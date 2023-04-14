use clap::Parser;
use rerun::time;
use swarmy::*;
use tracing_subscriber;

use rerun::external::re_memory::AccountingAllocator;

#[global_allocator]
static GLOBAL: AccountingAllocator<mimalloc::MiMalloc> =
    AccountingAllocator::new(mimalloc::MiMalloc);

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    source: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let cli = Cli::parse();
    let mut session = rerun::SessionBuilder::new("swarmy-rerun").buffered();
    let (mpq, file_contents) = read_mpq(&cli.source);
    let game_timeline = rerun::time::Timeline::new("game_timeline", time::TimeType::Sequence);
    add_game_events(&mpq, &file_contents, &mut session, &game_timeline)?;
    add_tracker_events(&mpq, &file_contents, &mut session, &game_timeline)?;
    rerun::native_viewer::show(&session)?;
    Ok(())
}
