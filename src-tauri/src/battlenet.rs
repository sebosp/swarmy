//! The StarCraft II community API of Battle.net, see
//! https://community.developer.battle.net/documentation/starcraft-2/community-apis: the profile and
//! the ladders of the season of an account, the `region-realm-id` of its toon handle.
//!
//! The Client ID and Secret of the user's Battle.net application are kept by the backend in
//! `battlenet.json` of its data directory, readable by its user only, and never sent back to the
//! views; `SWARMY_BATTLENET_CLIENT_ID` and `SWARMY_BATTLENET_CLIENT_SECRET` replace them (tests,
//! servers). The app sets them in Config; a server (`swarmy --serve`) takes them from its
//! environment or its data directory only, the browser version has no Config for them and the
//! server refuses `get_battlenet_status` and `set_battlenet_credentials`
//! (`web::DESKTOP_COMMANDS`). The OAuth token is of the client credentials flow, it needs no
//! redirect.

use crate::host::SwarmyHost;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};
use swarmy_common::*;

const CREDENTIALS_FILE: &str = "battlenet.json";
const TOKEN_URL: &str = "https://oauth.battle.net/token";
/// The profiles fetched, kept in the data directory of the backend: the Player view shows a
/// profile at once instead of waiting the seconds Battle.net takes (a profile is one request plus
/// one per ladder of the season), also after a restart.
const PROFILES_FILE: &str = "battlenet-profiles.json";
/// A profile is served as it is this long; older, it is shown at once and fetched again in the
/// background, the ladders change with each game.
const PROFILE_FRESH: Duration = Duration::from_secs(30 * 60);
/// A profile older than this is not shown, Battle.net is asked first.
const PROFILE_KEEP: Duration = Duration::from_secs(7 * 24 * 3600);

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
struct Credentials {
    client_id: String,
    client_secret: String,
}

fn credentials_path(host: &dyn SwarmyHost) -> Result<PathBuf, SwarmyError> {
    Ok(host.data_dir()?.join(CREDENTIALS_FILE))
}

/// The credentials of the environment, else the ones saved from Config.
fn credentials(host: &dyn SwarmyHost) -> Option<(Credentials, bool)> {
    if let (Ok(client_id), Ok(client_secret)) = (
        std::env::var("SWARMY_BATTLENET_CLIENT_ID"),
        std::env::var("SWARMY_BATTLENET_CLIENT_SECRET"),
    ) && !client_id.is_empty()
        && !client_secret.is_empty()
    {
        return Some((
            Credentials {
                client_id,
                client_secret,
            },
            true,
        ));
    }
    let path = credentials_path(host).ok()?;
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text)
        .ok()
        .map(|credentials| (credentials, false))
}

/// Writes the credentials readable by the user only.
fn save_credentials(host: &dyn SwarmyHost, credentials: &Credentials) -> Result<(), SwarmyError> {
    let path = credentials_path(host)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let text = serde_json::to_string(credentials)?;
    #[cfg(unix)]
    {
        use std::io::Write;
        use std::os::unix::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)?;
        file.write_all(text.as_bytes())?;
    }
    #[cfg(not(unix))]
    std::fs::write(&path, text)?;
    Ok(())
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

static TOKENS: Mutex<Option<(String, String, Instant)>> = Mutex::new(None);

/// The next request asks Battle.net for a new token.
fn forget_token() {
    if let Ok(mut tokens) = TOKENS.lock() {
        *tokens = None;
    }
}

/// The token of the credentials, kept until shortly before it expires.
async fn token(credentials: &Credentials) -> Result<String, SwarmyError> {
    if let Some((client_id, token, expires)) = TOKENS.lock().ok().and_then(|tokens| tokens.clone())
        && client_id == credentials.client_id
        && Instant::now() < expires
    {
        return Ok(token);
    }
    let response = reqwest::Client::new()
        .post(TOKEN_URL)
        .basic_auth(&credentials.client_id, Some(&credentials.client_secret))
        .form(&[("grant_type", "client_credentials")])
        .send()
        .await
        .map_err(|err| SwarmyError::Other(format!("Battle.net cannot be reached: {err}")))?;
    if !response.status().is_success() {
        return Err(SwarmyError::Other(format!(
            "Battle.net refused the Client ID and Secret ({}), check them in Config",
            response.status()
        )));
    }
    let token: TokenResponse = response
        .json()
        .await
        .map_err(|err| SwarmyError::Other(format!("Unexpected answer of Battle.net: {err}")))?;
    let expires = Instant::now() + Duration::from_secs(token.expires_in.saturating_sub(300));
    if let Ok(mut tokens) = TOKENS.lock() {
        *tokens = Some((
            credentials.client_id.clone(),
            token.access_token.clone(),
            expires,
        ));
    }
    Ok(token.access_token)
}

/// The API host of a region of a toon handle: 1 Americas, 2 Europe, 3 Korea and Taiwan.
fn api_host(region: u8) -> Result<&'static str, SwarmyError> {
    match region {
        1 => Ok("https://us.api.blizzard.com"),
        2 => Ok("https://eu.api.blizzard.com"),
        3 => Ok("https://kr.api.blizzard.com"),
        _ => Err(SwarmyError::Other(format!(
            "Battle.net has no community API for the region {region} (China)"
        ))),
    }
}

/// `region-realm-id` of an account.
fn parse_account(account: &str) -> Result<(u8, u32, u64), SwarmyError> {
    let parts: Vec<&str> = account.split('-').collect();
    match parts.as_slice() {
        [region, realm, id] => Ok((
            region.parse().map_err(|_| bad_account(account))?,
            realm.parse().map_err(|_| bad_account(account))?,
            id.parse().map_err(|_| bad_account(account))?,
        )),
        _ => Err(bad_account(account)),
    }
}

fn bad_account(account: &str) -> SwarmyError {
    SwarmyError::Other(format!("{account} is not an account, i.e. 2-1-315071"))
}

async fn get_json<T: serde::de::DeserializeOwned>(
    url: &str,
    token: &str,
) -> Result<T, SwarmyError> {
    let response = reqwest::Client::new()
        .get(url)
        .query(&[("locale", "en_US")])
        .bearer_auth(token)
        .send()
        .await
        .map_err(|err| SwarmyError::Other(format!("Battle.net cannot be reached: {err}")))?;
    match response.status() {
        status if status.is_success() => response
            .json()
            .await
            .map_err(|err| SwarmyError::Other(format!("Unexpected answer of Battle.net: {err}"))),
        reqwest::StatusCode::NOT_FOUND => Err(SwarmyError::Other(
            "Battle.net has no profile for this account".to_string(),
        )),
        status => Err(SwarmyError::Other(format!("Battle.net answered {status}"))),
    }
}

// The parts of the answers that are read.
#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ProfileAnswer {
    summary: ProfileSummary,
    career: Career,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ProfileSummary {
    display_name: String,
    clan_tag: String,
    portrait: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Career {
    total_career_games: u32,
    total_games_this_season: u32,
    #[serde(rename = "best1v1Finish")]
    best_1v1_finish: Option<Finish>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct Finish {
    league_name: Option<String>,
    times_achieved: u32,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct LadderSummary {
    show_case_entries: Vec<ShowCaseEntry>,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ShowCaseEntry {
    ladder_id: String,
    team: ShowCaseTeam,
    league_name: String,
    rank: u32,
    wins: u32,
    losses: u32,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct ShowCaseTeam {
    localized_game_mode: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct LadderAnswer {
    ranks_and_pools: Vec<RankAndPool>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RankAndPool {
    mmr: Option<i32>,
}

/// The profile and the ladders of the season of an account.
async fn fetch_profile(
    credentials: &Credentials,
    account: &str,
) -> Result<BattleNetProfile, SwarmyError> {
    let (region, realm, id) = parse_account(account)?;
    let host = api_host(region)?;
    let token = token(credentials).await?;
    let base = format!("{host}/sc2/profile/{region}/{realm}/{id}");
    let profile: ProfileAnswer = get_json(&base, &token).await?;
    let summary: LadderSummary = get_json(&format!("{base}/ladder/summary"), &token)
        .await
        .unwrap_or_default();
    // The MMR is in the ladder itself: the ladders are asked at once, not one after the other.
    let ladders =
        futures_util::future::join_all(summary.show_case_entries.into_iter().map(|entry| {
            let url = format!("{base}/ladder/{}", entry.ladder_id);
            let token = token.clone();
            async move {
                let mmr = get_json::<LadderAnswer>(&url, &token)
                    .await
                    .ok()
                    .and_then(|ladder| ladder.ranks_and_pools.first().and_then(|pool| pool.mmr));
                BattleNetLadder {
                    mode: entry.team.localized_game_mode,
                    league: entry.league_name,
                    rank: entry.rank,
                    wins: entry.wins,
                    losses: entry.losses,
                    mmr,
                }
            }
        }))
        .await;
    let best = profile.career.best_1v1_finish.unwrap_or_default();
    Ok(BattleNetProfile {
        account: account.to_string(),
        display_name: profile.summary.display_name,
        clan_tag: profile.summary.clan_tag,
        profile_url: format!("https://starcraft2.blizzard.com/profile/{region}/{realm}/{id}"),
        portrait_url: profile.summary.portrait,
        best_1v1_league: best.league_name.unwrap_or_default(),
        best_1v1_times: best.times_achieved,
        total_career_games: profile.career.total_career_games,
        games_this_season: profile.career.total_games_this_season,
        ladders,
    })
}

/// Whether the backend has credentials, and whether Battle.net accepts them now: a new token is
/// asked, so that Config tells a revoked secret.
pub async fn get_battlenet_status(host: &dyn SwarmyHost) -> Result<ApiResponse, SwarmyError> {
    let status = match credentials(host) {
        Some((credentials, from_environment)) => {
            forget_token();
            let checked = token(&credentials).await;
            BattleNetStatus {
                configured: true,
                from_environment,
                working: checked.is_ok(),
                error: checked.err().map(|err| err.to_string()).unwrap_or_default(),
            }
        }
        None => BattleNetStatus::default(),
    };
    Ok(ApiResponseBuilder::new().process_result(Ok(status)))
}

/// Saves the credentials once Battle.net accepts them; both empty removes them.
pub async fn set_battlenet_credentials(
    host: &dyn SwarmyHost,
    client_id: String,
    client_secret: String,
) -> Result<ApiResponse, SwarmyError> {
    let (client_id, client_secret) = (
        client_id.trim().to_string(),
        client_secret.trim().to_string(),
    );
    if client_id.is_empty() && client_secret.is_empty() {
        let path = credentials_path(host)?;
        if path.exists() {
            std::fs::remove_file(path)?;
        }
        return Ok(ApiResponseBuilder::new().process_result(Ok("Removed")));
    }
    let credentials = Credentials {
        client_id,
        client_secret,
    };
    forget_token();
    let result = match token(&credentials).await {
        Ok(_) => save_credentials(host, &credentials).map(|_| "Saved, Battle.net accepted them"),
        Err(err) => Err(err),
    };
    Ok(ApiResponseBuilder::new().process_result(result))
}

/// A profile of the cache, with when Battle.net answered it (seconds since the epoch).
#[derive(Debug, Clone, serde::Serialize, Deserialize)]
struct CachedProfile {
    profile: BattleNetProfile,
    fetched_at: u64,
}

impl CachedProfile {
    fn age(&self) -> Duration {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|since| since.as_secs())
            .unwrap_or_default();
        Duration::from_secs(now.saturating_sub(self.fetched_at))
    }
}

/// The profiles fetched, by account: in memory, loaded from [`PROFILES_FILE`] the first time and
/// written back after each fetch.
static PROFILES: Mutex<Option<HashMap<String, CachedProfile>>> = Mutex::new(None);
/// The accounts being fetched in the background, so that a profile asked twice is fetched once.
static REFRESHING: Mutex<Option<HashSet<String>>> = Mutex::new(None);

fn profiles_path(host: &dyn SwarmyHost) -> Option<PathBuf> {
    host.data_dir().ok().map(|dir| dir.join(PROFILES_FILE))
}

/// The cached profile of the account, if it is younger than [`PROFILE_KEEP`].
fn cached_profile(path: Option<&Path>, account: &str) -> Option<CachedProfile> {
    let mut profiles = PROFILES.lock().ok()?;
    let profiles = profiles.get_or_insert_with(|| {
        path.and_then(|path| std::fs::read_to_string(path).ok())
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    });
    profiles
        .get(account)
        .filter(|cached| cached.age() < PROFILE_KEEP)
        .cloned()
}

/// Keeps a profile just fetched, in memory and in the file.
fn remember_profile(path: Option<&Path>, account: &str, profile: &BattleNetProfile) {
    let Ok(mut profiles) = PROFILES.lock() else {
        return;
    };
    let profiles = profiles.get_or_insert_with(HashMap::new);
    profiles.insert(
        account.to_string(),
        CachedProfile {
            profile: profile.clone(),
            fetched_at: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .map(|since| since.as_secs())
                .unwrap_or_default(),
        },
    );
    // The profiles are public data, the file can be read by anyone.
    if let Some(path) = path {
        let written = serde_json::to_string(&*profiles)
            .map_err(SwarmyError::from)
            .and_then(|text| {
                if let Some(dir) = path.parent() {
                    std::fs::create_dir_all(dir)?;
                }
                std::fs::write(path, text).map_err(SwarmyError::from)
            });
        if let Err(err) = written {
            tracing::warn!("Unable to write {}: {err}", path.display());
        }
    }
}

/// Fetches a profile again in the background, once at a time per account.
fn refresh_in_background(credentials: Credentials, path: Option<PathBuf>, account: String) {
    if let Ok(mut refreshing) = REFRESHING.lock()
        && !refreshing
            .get_or_insert_with(HashSet::new)
            .insert(account.clone())
    {
        return;
    }
    tokio::spawn(async move {
        match fetch_profile(&credentials, &account).await {
            Ok(profile) => remember_profile(path.as_deref(), &account, &profile),
            Err(err) => tracing::warn!("Unable to refresh the Battle.net profile {account}: {err}"),
        }
        if let Ok(mut refreshing) = REFRESHING.lock()
            && let Some(refreshing) = refreshing.as_mut()
        {
            refreshing.remove(&account);
        }
    });
}

/// The Battle.net profile of an account: from the cache when it is younger than
/// [`PROFILE_FRESH`]; an older one is answered at once and fetched again in the background; one
/// not cached is fetched first. The cache survives a restart ([`PROFILES_FILE`]).
pub async fn query_battlenet_profile(
    host: &dyn SwarmyHost,
    account: String,
) -> Result<ApiResponse, SwarmyError> {
    let path = profiles_path(host);
    let cached = cached_profile(path.as_deref(), &account);
    let credentials = credentials(host).map(|(credentials, _)| credentials);
    if let Some(cached) = cached {
        if cached.age() >= PROFILE_FRESH
            && let Some(credentials) = credentials
        {
            refresh_in_background(credentials, path.clone(), account.clone());
        }
        return Ok(ApiResponseBuilder::new().process_result(Ok(cached.profile)));
    }
    let Some(credentials) = credentials else {
        return Ok(
            ApiResponseBuilder::new()
                .process_result(Err::<BattleNetProfile, _>(SwarmyError::Other(
                "No Battle.net API credentials: the app takes a Client ID and Secret in Config, \
                     a server in its environment (SWARMY_BATTLENET_CLIENT_ID and _SECRET)"
                    .to_string(),
            ))),
        );
    };
    let result = fetch_profile(&credentials, &account).await;
    if let Ok(profile) = &result {
        remember_profile(path.as_deref(), &account, profile);
    }
    Ok(ApiResponseBuilder::new().process_result(result))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_reads_the_accounts() {
        assert_eq!(parse_account("2-1-315071").unwrap(), (2, 1, 315071));
        assert!(parse_account("Serral").is_err());
        assert!(api_host(5).is_err());
    }

    /// The profile of Serral's European account, with the credentials of the environment.
    #[tokio::test]
    #[ignore]
    async fn it_reads_a_profile() {
        let credentials = Credentials {
            client_id: std::env::var("SWARMY_BATTLENET_CLIENT_ID").unwrap(),
            client_secret: std::env::var("SWARMY_BATTLENET_CLIENT_SECRET").unwrap(),
        };
        let profile = fetch_profile(&credentials, "2-1-315071").await.unwrap();
        println!("{profile:#?}");
        assert_eq!(profile.display_name, "Serral");
        assert!(profile.total_career_games > 0);
    }
}
