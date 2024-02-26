use clap::Parser;
use s2protocol::SC2ReplayFilters;
use swarmy::*;

use rerun::external::re_memory::AccountingAllocator;

#[global_allocator]
static GLOBAL: AccountingAllocator<mimalloc::MiMalloc> =
    AccountingAllocator::new(mimalloc::MiMalloc);

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

    /// Filters a specific player id.
    #[arg(long)]
    filter_player_id: Option<u8>,

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

    /// An output RRD file to generate once the input has been processed.
    #[arg(long)]
    output: Option<String>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    let filters = SC2ReplayFilters {
        player_id: cli.filter_player_id,
        unit_tag: cli.filter_unit_tag,
        min_loop: cli.filter_min_loop,
        max_loop: cli.filter_max_loop,
        event_type: cli.filter_event_type,
        unit_name: cli.filter_unit_name,
        max_events: cli.filter_max_events,
        include_stats: cli.include_stats,
    };
    tracing::info!("Filters: {:?}", filters);
    let sc2_rerun = SC2Rerun::new(&cli.source, filters)?;
    if let Some(output) = cli.output {
        sc2_rerun.save_to_file(&output)?;
    } else {
        sc2_rerun.show()?;
    }
    Ok(())
}
