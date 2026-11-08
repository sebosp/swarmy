//! Charts rendered with ApexCharts the way the rust-ui charts do (https://leptos.rust-ui.com):
//! the components write their data as `data-chart-*` attributes of a container and
//! `public/charts/chart_init.js` renders the containers, see the attributes there.

use leptos::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use swarmy_common::{GamesWins, UnitMonthUsage};

/// The colors of the series, tailwind indigo-500, green-500, sky-500, yellow-500, purple-500.
/// The series colors of the theme, `--chart-N` of styles.css, in a fixed order: a series keeps
/// its color when the others are filtered out.
pub const CHART_COLORS: [&str; 8] = [
    "var(--chart-1)",
    "var(--chart-2)",
    "var(--chart-3)",
    "var(--chart-4)",
    "var(--chart-5)",
    "var(--chart-6)",
    "var(--chart-7)",
    "var(--chart-8)",
];

/// A unique id per chart, so that the charts are destroyed when their container is removed.
fn next_chart_id(kind: &str) -> String {
    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
    format!("{kind}-{}", NEXT_ID.fetch_add(1, Ordering::Relaxed))
}

fn to_json<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_string(value).unwrap_or_default()
}

/// The data of a chart, one or more series of values for the same labels.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct ChartData {
    pub labels: Vec<String>,
    /// The name and the values of each series, the values align with the labels, none is a gap.
    pub series: Vec<(String, Vec<Option<f64>>)>,
    /// The CSS colors of the series, [`CHART_COLORS`] when empty.
    pub colors: Vec<String>,
    /// The tooltip of each label, the values of the series when empty.
    pub tooltips: Vec<String>,
    /// The maximum of the y axis, so that several charts share the scale.
    pub y_max: Option<f64>,
    /// Appended to the values, i.e. `%`.
    pub value_suffix: String,
    /// `normal` or `100%` to stack the series, empty for side by side.
    pub stack_type: String,
    /// The line curve, `smooth` by default, `straight` or `stepline`.
    pub curve: String,
    /// Vertical lines at the labels, with a text or none: i.e. when a patch starts.
    pub markers: Vec<(String, String)>,
    /// The decimals of the numbers of the y axis.
    pub y_decimals: u32,
    /// The view draws its own legend, i.e. one that filters the series.
    pub hide_legend: bool,
}

impl ChartData {
    pub fn new(labels: Vec<String>) -> Self {
        Self {
            labels,
            ..Default::default()
        }
    }

    pub fn with_series(mut self, name: &str, values: Vec<f64>) -> Self {
        self.series
            .push((name.to_string(), values.into_iter().map(Some).collect()));
        self
    }

    /// A series with gaps, i.e. the patches without the unit.
    pub fn with_optional_series(mut self, name: &str, values: Vec<Option<f64>>) -> Self {
        self.series.push((name.to_string(), values));
        self
    }

    pub fn with_owned_colors(mut self, colors: Vec<String>) -> Self {
        self.colors = colors;
        self
    }

    pub fn with_colors(mut self, colors: &[&str]) -> Self {
        self.colors = colors.iter().map(|color| color.to_string()).collect();
        self
    }

    pub fn with_tooltips(mut self, tooltips: Vec<String>) -> Self {
        self.tooltips = tooltips;
        self
    }

    pub fn with_y_max(mut self, y_max: f64) -> Self {
        self.y_max = Some(y_max);
        self
    }

    pub fn with_value_suffix(mut self, suffix: &str) -> Self {
        self.value_suffix = suffix.to_string();
        self
    }

    pub fn with_curve(mut self, curve: &str) -> Self {
        self.curve = curve.to_string();
        self
    }

    pub fn with_markers(mut self, markers: Vec<(String, String)>) -> Self {
        self.markers = markers;
        self
    }

    pub fn with_y_decimals(mut self, decimals: u32) -> Self {
        self.y_decimals = decimals;
        self
    }

    pub fn stacked(mut self, stack_type: &str) -> Self {
        self.stack_type = stack_type.to_string();
        self
    }

    /// Without the legend of the chart, the view draws its own.
    pub fn without_legend(mut self) -> Self {
        self.hide_legend = true;
        self
    }
}

/// The chart container, `name` is the ApexCharts type, `BarChart` or `LineChart`.
#[component]
fn ApexChart(
    name: &'static str,
    data: ChartData,
    /// The tailwind classes of the container, its height is the height of the chart.
    class: &'static str,
) -> impl IntoView {
    let values: Vec<&Vec<Option<f64>>> = data.series.iter().map(|(_, values)| values).collect();
    let names: Vec<&String> = data.series.iter().map(|(name, _)| name).collect();
    // A single series is written as a flat array.
    let values_json = match values.as_slice() {
        [single] => to_json(single),
        _ => to_json(&values),
    };
    view! {
        <div
            id=next_chart_id(name)
            // The chart is drawn at the width of the container, it must not widen its parent.
            // Clipped across only: the tooltips may go past the top and the bottom.
            class=format!("w-full min-w-0 overflow-x-clip {class}")
            data-name=name
            data-chart-values=values_json
            data-chart-labels=to_json(&data.labels)
            data-chart-series-names=to_json(&names)
            data-chart-colors=(!data.colors.is_empty()).then(|| to_json(&data.colors))
            data-chart-tooltips=(!data.tooltips.is_empty()).then(|| to_json(&data.tooltips))
            data-chart-y-max=data.y_max.map(|y_max| y_max.to_string())
            data-chart-value-suffix=data.value_suffix
            data-chart-stack-type=(!data.stack_type.is_empty()).then_some(data.stack_type)
            data-chart-curve=(!data.curve.is_empty()).then_some(data.curve)
            data-chart-markers=(!data.markers.is_empty()).then(|| to_json(&data.markers))
            data-chart-y-decimals=(data.y_decimals > 0).then(|| data.y_decimals.to_string())
            data-chart-legend=data.hide_legend.then_some("off")
        ></div>
    }
}

#[component]
pub fn BarChart(data: ChartData, #[prop(default = "h-48")] class: &'static str) -> impl IntoView {
    view! { <ApexChart name="BarChart" data class /> }
}

#[component]
pub fn LineChart(data: ChartData, #[prop(default = "h-48")] class: &'static str) -> impl IntoView {
    view! { <ApexChart name="LineChart" data class /> }
}

/// One line around an even middle, as the evaluation graph of a chess game: the values go from
/// -50, the second side wins for sure, to 50, the first one does. The area between the line and
/// the middle has the first color of `data` above it and the second one below.
#[component]
pub fn AdvantageChart(
    data: ChartData,
    #[prop(default = "h-48")] class: &'static str,
) -> impl IntoView {
    view! { <ApexChart name="AdvantageChart" data class /> }
}

/// The games (gray) and wins (green) of each label, i.e. per month, as bars.
#[component]
pub fn GamesWinsChart(data: Vec<GamesWins>) -> impl IntoView {
    let tooltips = data
        .iter()
        .map(|entry| {
            format!(
                "{}: {} wins out of {} games",
                entry.label, entry.wins, entry.games
            )
        })
        .collect();
    let chart = ChartData::new(data.iter().map(|entry| entry.label.clone()).collect())
        .with_series(
            "Games",
            data.iter().map(|entry| entry.games as f64).collect(),
        )
        .with_series("Wins", data.iter().map(|entry| entry.wins as f64).collect())
        .with_colors(&["var(--chart-grid)", CHART_COLORS[1]])
        .with_tooltips(tooltips);
    view! { <BarChart data=chart /> }
}

/// The percentage of the games of each month in which a unit was made, as bars. Hovering a bar
/// shows the games and the units made per game.
#[component]
pub fn UnitUsageChart(data: Vec<UnitMonthUsage>) -> impl IntoView {
    let tooltips = data
        .iter()
        .map(|entry| {
            format!(
                "{}: made in {} of {} games ({:.0}%), {:.1} per game in which it was made",
                entry.month,
                entry.games_with_unit,
                entry.games,
                entry.games_percent(),
                entry.born_per_game(),
            )
        })
        .collect();
    let chart = ChartData::new(data.iter().map(|entry| entry.month.clone()).collect())
        .with_series(
            "Games with the unit",
            data.iter()
                .map(|entry| (entry.games_percent() * 10.).round() / 10.)
                .collect(),
        )
        .with_colors(&[CHART_COLORS[2]])
        .with_tooltips(tooltips)
        .with_value_suffix("%");
    view! { <BarChart data=chart /> }
}
