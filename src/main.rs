use clap::Parser;
use swarmy::*;
use tracing_subscriber;

use rerun::external::re_memory::AccountingAllocator;

#[global_allocator]
static GLOBAL: AccountingAllocator<mimalloc::MiMalloc> =
    AccountingAllocator::new(mimalloc::MiMalloc);

// TODO: There a ratio between tracker events and game events.
// This ratio is still to be verified.

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Source file, the SC2Replay extension usually.
    #[arg(short, long, value_name = "FILE")]
    source: String,

    /// Filters a specific user id.
    #[arg(short, long, value_name = "USER_ID")]
    filter_user_id: Option<i64>,

    /// Filters a specific unit tag.
    #[arg(short, long, value_name = "UNIT_TAG")]
    filter_unit_tag: Option<i64>,

    /// Allows setting up a min event loop, in game_event units
    #[arg(short, long, value_name = "MIN")]
    min: Option<i64>,

    /// Allows setting up a max event loop
    #[arg(short, long, value_name = "MAX")]
    max: Option<i64>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    let mut sc2_replay = SC2Replay::new(&cli.source)?;
    sc2_replay.show()?;
    Ok(())
}
