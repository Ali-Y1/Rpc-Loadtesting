use std::{sync::{mpsc, atomic::AtomicBool, Arc}, thread};
use load_testing::{run, utils::{Cli, JsonRequest, process_ethspam_output}};
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
    let (tx, rx) = mpsc::channel::<JsonRequest>();

    // Create an Arc<AtomicBool> to signal the producer_thread to stop
    let stop_flag = Arc::new(AtomicBool::new(false));

    if args.pipe {
        // Spawn a separate task for the process_ethspam_output function
        let producer_task = {
            let tx = tx.clone();
            let stop_flag = stop_flag.clone();
            tokio::spawn(async move {
                process_ethspam_output(tx, stop_flag).await;
            })
        };

        // Call the run function and pass the receiver end of the channel
        run(Some(rx), Some(stop_flag.clone())).await;

        // Wait for the producer_task to finish
        let _ = producer_task.await;
    } else {
        // Call the run function without using the pipe
        // You may need to modify the run function to not require the channel if you don't use the pipe
        run(None, None).await;
    }
 }