use clap::Parser;
use s2protocol::SC2ReplayFilters;
use swarmy::*;
use tracing_subscriber;

use rerun::external::re_memory::AccountingAllocator;

#[global_allocator]
static GLOBAL: AccountingAllocator<mimalloc::MiMalloc> =
    AccountingAllocator::new(mimalloc::MiMalloc);

// TODO: There a ratio between tracker events and game events.
// This ratio is still to be verified.

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Source file, the SC2Replay extension usually.
    #[arg(short, long, value_name = "FILE")]
    source: String,

    /// Whether to include the player stats. This should be later move into a filter where specific
    /// event types can be excluded/included but for now this is just clutter.
    #[arg(long, default_value_t = false)]
    include_stats: bool,

    /// Filters a specific user id.
    #[arg(long)]
    filter_user_id: Option<i64>,

    /// Filters a specific unit tag.
    #[arg(long)]
    filter_unit_tag: Option<i64>,

    /// Allows setting up a min event loop, in game_event units
    #[arg(long)]
    filter_min_loop: Option<i64>,

    /// Allows setting up a max event loop
    #[arg(long)]
    filter_max_loop: Option<i64>,

    /// Only show game of specific types
    #[arg(long)]
    filter_event_type: Option<String>,

    /// Only show game of specific types
    #[arg(long)]
    filter_unit_name: Option<String>,

    /// Allows processing a max ammount of events of each type.
    #[arg(long)]
    filter_max_events: Option<usize>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    let filters = SC2ReplayFilters {
        user_id: cli.filter_user_id,
        unit_tag: cli.filter_unit_tag,
        min_loop: cli.filter_min_loop,
        max_loop: cli.filter_max_loop,
        event_type: cli.filter_event_type,
        unit_name: cli.filter_unit_name,
        max_events: cli.filter_max_events,
    };
    tracing::info!("Filters: {:?}", filters);
    let mut sc2_rerun = SC2Rerun::new(&cli.source, filters, cli.include_stats)?;
    sc2_rerun.add_events()?;
    sc2_rerun.show()?;
    Ok(())
}
