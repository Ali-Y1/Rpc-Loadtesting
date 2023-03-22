use load_testing::{run, utils::Cli};
use log::{LevelFilter, SetLoggerError};
use simple_logger::SimpleLogger;
use structopt::StructOpt;

#[tokio::main]
async fn main() {
    let args = Cli::from_args();

    // Set the logging level based on the verbosity flag
    let log_level = match args.verbosity {
        0 => LevelFilter::Warn,
        1 => LevelFilter::Info,
        2 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    SimpleLogger::new().with_level(log_level).init().unwrap();
    run().await;
 }