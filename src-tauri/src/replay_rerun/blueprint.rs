//! The viewer layout.
//!
//! The SDK `rerun::blueprint` API does not expose view properties, the time series need a
//! `TimeAxis` property to show the full game instead of a window around the time cursor, so the
//! blueprint store is written here with the same archetypes the SDK uses.

use rerun::external::re_log_msg::{BlueprintActivationCommand, LogMsg};
use rerun::external::re_log_types::RecordingId;
use rerun::external::re_sdk_types::blueprint::archetypes::{
    ContainerBlueprint, PanelBlueprint, TimeAxis, ViewBlueprint, ViewContents, ViewportBlueprint,
};
use rerun::external::re_sdk_types::blueprint::components::{
    ColumnShare, ContainerKind, IncludedContent, LinkAxis, PanelState, QueryExpression,
    RootContainer, RowShare, TimeRange, ViewClass,
};
use rerun::external::re_sdk_types::components::Name;
use rerun::external::re_sdk_types::encodings::{self, Float32, TimeRangeBoundary};
use rerun::{entity_path, RecordingStream, RecordingStreamResult};
use uuid::Uuid;

use super::PLAYER_STATS;

/// Writes the blueprint archetypes, returns the blueprint paths of the containers and views so
/// that they can be nested.
struct BlueprintWriter {
    stream: RecordingStream,
}

impl BlueprintWriter {
    fn view(
        &self,
        class: &str,
        name: &str,
        origin: &str,
        contents: &[&str],
    ) -> RecordingStreamResult<String> {
        let path = format!("view/{}", Uuid::new_v4());
        self.stream.log(
            format!("{path}/ViewContents"),
            &ViewContents::new(
                contents
                    .iter()
                    .map(|query| QueryExpression(query.to_string().into())),
            ),
        )?;
        self.stream.log(
            path.as_str(),
            &ViewBlueprint::new(ViewClass(class.into()))
                .with_display_name(Name(name.into()))
                .with_space_origin(origin.to_string()),
        )?;
        Ok(path)
    }

    /// A time series that shows the whole game duration.
    fn time_series(&self, name: &str, origin: &str) -> RecordingStreamResult<String> {
        let path = self.view("TimeSeries", name, origin, &["$origin/**"])?;
        self.stream.log(
            format!("{path}/TimeAxis"),
            &TimeAxis::new()
                .with_link(LinkAxis::Independent)
                .with_view_range(TimeRange(encodings::TimeRange {
                    start: TimeRangeBoundary::Infinite,
                    end: TimeRangeBoundary::Infinite,
                })),
        )?;
        Ok(path)
    }

    fn container(
        &self,
        kind: ContainerKind,
        contents: Vec<String>,
        shares: &[f32],
    ) -> RecordingStreamResult<(Uuid, String)> {
        let id = Uuid::new_v4();
        let path = format!("container/{id}");
        let mut container = ContainerBlueprint::new(kind).with_contents(
            contents
                .into_iter()
                .map(|content| IncludedContent(content.into())),
        );
        let shares = shares.iter().map(|share| Float32(*share));
        container = match kind {
            ContainerKind::Horizontal => container.with_col_shares(shares.map(ColumnShare)),
            ContainerKind::Vertical => container.with_row_shares(shares.map(RowShare)),
            _ => container,
        };
        self.stream.log(path.as_str(), &container)?;
        Ok((id, path))
    }
}

/// Turns `minerals/used_in_progress_army` into `Minerals used in progress army`.
fn stat_title(stat: &str) -> String {
    let title = stat.replace(['/', '_'], " ");
    let mut chars = title.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => title,
    }
}

/// Sends the viewer layout, the map takes 60% of the width, the charts and the events log take
/// the rest and the side panels are collapsed.
pub fn send_blueprint(
    recording: &RecordingStream,
    players: &[String],
) -> RecordingStreamResult<()> {
    let application_id = recording
        .store_info()
        .map(|info| info.application_id().clone())
        .unwrap_or_else(|| super::viewer::RERUN_APP_ID.into());
    let (stream, storage) = rerun::RecordingStreamBuilder::new(application_id)
        .recording_id(RecordingId::random())
        .blueprint()
        .memory()?;
    // Required for the viewer to identify blueprint data.
    stream.set_time_sequence("blueprint", 0);
    let writer = BlueprintWriter { stream };

    let map = writer.view(
        "3D",
        "Map",
        "/",
        &[
            "+ /map/**",
            "+ /units/**",
            "+ /deaths/**",
            "+ /commands/**",
            "+ /cameras/**",
        ],
    )?;

    let mut charts = vec![
        writer.time_series("Minerals", "/stats/minerals/current")?,
        writer.time_series("Vespene", "/stats/vespene/current")?,
        writer.time_series("Mineral rate", "/stats/minerals/collection_rate")?,
        writer.time_series("Vespene rate", "/stats/vespene/collection_rate")?,
        writer.time_series("Supply", "/stats/food")?,
        writer.time_series("Workers", "/stats/workers_active_count")?,
        writer.time_series("Decisions", "/decisions")?,
    ];
    for player in players {
        charts.push(writer.time_series(
            &format!("Units {player}"),
            &entity_path!("unit_types", player).to_string(),
        )?);
    }
    // The rest of the economy stats, the ones above already have their own tab.
    for stat in PLAYER_STATS.iter().filter(|stat| {
        !matches!(
            **stat,
            "minerals/current"
                | "vespene/current"
                | "minerals/collection_rate"
                | "vespene/collection_rate"
                | "food/used"
                | "food/made"
                | "workers_active_count"
        )
    }) {
        charts.push(writer.time_series(&stat_title(stat), &format!("/stats/{stat}"))?);
    }
    let (_, charts) = writer.container(ContainerKind::Tabs, charts, &[])?;

    let events = writer.view("TextLog", "Events", "/events", &["$origin/**"])?;
    let (_, side_column) =
        writer.container(ContainerKind::Vertical, vec![charts, events], &[2.0, 1.0])?;
    let (root_id, _) = writer.container(
        ContainerKind::Horizontal,
        vec![map, side_column],
        &[3.0, 2.0],
    )?;

    writer.stream.log(
        "viewport",
        &ViewportBlueprint::new().with_root_container(RootContainer(root_id.into())),
    )?;
    for panel in ["blueprint_panel", "selection_panel"] {
        writer.stream.log(
            panel,
            &PanelBlueprint::new().with_state(PanelState::Collapsed),
        )?;
    }

    let msgs = storage.take();
    let Some(blueprint_id) = msgs.iter().find_map(|msg| match msg {
        LogMsg::SetStoreInfo(info) => Some(info.info.store_id.clone()),
        _ => None,
    }) else {
        tracing::error!("Blueprint without store info, not sending it");
        return Ok(());
    };
    recording.send_blueprint(
        msgs,
        BlueprintActivationCommand {
            blueprint_id,
            make_active: true,
            make_default: true,
        },
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_titles_stats() {
        assert_eq!(
            stat_title("minerals/used_in_progress_army"),
            "Minerals used in progress army"
        );
    }
}
