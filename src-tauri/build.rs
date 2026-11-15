/// The libraries whose versions are shown in the Home view, read from the workspace lock file.
const VERSIONED_DEPENDENCIES: [&str; 4] = ["s2protocol", "rerun", "polars", "tauri"];

fn main() {
    println!("cargo:rerun-if-changed=../Cargo.lock");
    let lock_file = std::fs::read_to_string("../Cargo.lock").unwrap_or_default();
    for dependency in VERSIONED_DEPENDENCIES {
        let version = lock_file
            .split("[[package]]")
            .find_map(|package| {
                let mut lines = package.lines().map(str::trim);
                let name = lines.find(|line| line.starts_with("name = "))?;
                if name != format!("name = \"{dependency}\"") {
                    return None;
                }
                let version = lines.find(|line| line.starts_with("version = "))?;
                Some(
                    version
                        .trim_start_matches("version = ")
                        .trim_matches('"')
                        .to_string(),
                )
            })
            .unwrap_or_else(|| "unknown".to_string());
        println!(
            "cargo:rustc-env=SWARMY_{}_VERSION={version}",
            dependency.to_uppercase()
        );
    }
    git_revision();
    tauri_build::build()
}

/// The commit swarmy is built from and whether the tree has uncommitted changes, for the links
/// to the queries in the repository (`AppVersions::git_sha`). `SWARMY_GIT_SHA` and
/// `SWARMY_GIT_DIRTY` in the environment win: a build from a copy of the tree without `.git`
/// (the deploy script) sets them.
fn git_revision() {
    println!("cargo:rerun-if-env-changed=SWARMY_GIT_SHA");
    println!("cargo:rerun-if-env-changed=SWARMY_GIT_DIRTY");
    println!("cargo:rerun-if-changed=../.git/HEAD");
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
    };
    let sha = std::env::var("SWARMY_GIT_SHA")
        .ok()
        .or_else(|| git(&["rev-parse", "HEAD"]))
        .unwrap_or_default();
    let dirty = match std::env::var("SWARMY_GIT_DIRTY") {
        Ok(value) => value == "1" || value == "true",
        Err(_) => git(&["status", "--porcelain"]).is_none_or(|status| !status.is_empty()),
    };
    println!("cargo:rustc-env=SWARMY_GIT_SHA={sha}");
    println!("cargo:rustc-env=SWARMY_GIT_DIRTY={}", dirty as u8);
}
