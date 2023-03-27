use csv::Writer;
use futures::future;
use jsonrpc_core::IoHandler;
use jsonrpc_http_server::{AccessControlAllowOrigin, DomainsValidation, ServerBuilder};
use jsonrpc_derive::rpc;
use serde::{Deserialize, Serialize};
use structopt::StructOpt;
use serde_json::{Map, Value};
use reqwest::Client;
use tokio::signal::unix::{SignalKind, signal};
use std::error::Error;
use std::fs::File;
use std::sync::{Arc, RwLock};
use std::time::Instant;
use std::sync::atomic::{AtomicU64, Ordering, AtomicBool};
use tokio::time::{timeout, Duration};
use log::{debug, info, error};
use chrono::prelude::*;
use indicatif::{ProgressBar, ProgressStyle};
use std::collections::HashMap;
use rand::prelude::*;
use crate::utils::*;

pub mod utils;


pub async fn run() {
    let args = Cli::from_args();
    let client = Arc::new(Client::new());
    let max_connections = args.concurrent_connections;
    let connections_step = args.connections_step;
    let request_file = &args.request_file;
    let start_time = Utc::now();
    let server_urls = args.server_urls;
    info!("Started test at {}", start_time);

    // Read the JSON request from the file
    let json_request = match read_json_request_from_file(request_file).await {
        Ok(req) => {
        info!("Successfully read JSON request from file");
        req
    },
        Err(e) => {
            error!("Error reading JSON request from file: {}", e);
            return;
        }
    };

    let headers = &[
        "connections",
        "total_requests",
        "successful_requests",
        "failed_requests",
        "average_response_time",
        "average_requests_per_second",
        "elapsed_time",
        "timeout_requests",
    ];

    let mut records = Vec::new();
    let connections_range = if connections_step > 0 {
        (1..=max_connections).step_by(connections_step as usize).collect::<Vec<_>>()
    } else {
        vec![max_connections]
    };
    for connections in connections_range {
        let stats = Arc::new(RwLock::new(Stats::default()));
        let mut handles = Vec::new();
        let start_time = Instant::now();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let timeout_duration = Duration::from_millis(args.timeout);
        let error_counts = Arc::new(RwLock::new(HashMap::new()));
        // Set up a signal handler for SIGINT and SIGTERM
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        let mut sigterm = signal(SignalKind::terminate()).unwrap();
        let progress_bar = ProgressBar::new(connections as u64 * args.requests_per_connection as u64);
        progress_bar.set_style(ProgressStyle::default_bar()
            .template("[{elapsed_precise}] |{bar:40.cyan/blue}| {pos:>7}/{len:7} ({eta}) ({per_sec}) {msg}")
            .progress_chars(" * "));

        // Spawn a separate task to handle signals
        let stop_flag_clone = stop_flag.clone();
        let signal_handler = tokio::spawn(async move {
            tokio::select! {
                _ = sigint.recv() => {},
                _ = sigterm.recv() => {},
            }
            info!("Kill signal received, shutting down");
            stop_flag_clone.store(true, Ordering::SeqCst);
        });
        
        for _ in 0..connections {
            let progress_bar = progress_bar.clone();
            let client = client.clone();
            let stats = stats.clone();
            let server_urls = server_urls.clone();
            let test_duration = args.test_duration;
            let stop_flag_clone = stop_flag.clone();
            let json_request_clone = json_request.clone();

            if stop_flag_clone.load(Ordering::SeqCst) {
                break;
            }
            let error_counts = error_counts.clone();
            let handle = tokio::spawn(async move {
                let mut request_count = 0;
                let test_start_time = Instant::now();
                while (test_duration == 0 || Instant::now().duration_since(test_start_time) < Duration::from_secs(test_duration))
                    && (args.requests_per_connection == 0 || request_count < args.requests_per_connection)
                {
                    let request = json_request_clone.clone();
                    let server_url = {
                        let mut rng = thread_rng();
                        server_urls.choose(&mut rng).unwrap().to_owned()
                    };
                    let start_time = Instant::now();
                    let result = match timeout(timeout_duration, send_json_rpc_request(&client, &server_url, &request)).await {
                        Ok(res) => res,
                        Err(_) => {
                            // Update the timeout_requests counter
                            let mut stats = stats.write().unwrap();
                            stats.timeout_requests += 1;
                    
                            Err("Request timed out".into())
                        },
                    };
                    let elapsed_time = start_time.elapsed().as_millis();
    
                    let mut stats = stats.write().unwrap();
                    stats.completed_requests.fetch_add(1, Ordering::Relaxed);
                    progress_bar.inc(1);
                    match result {
                        Ok(_) => {
                            stats.successful_requests += 1;
                            stats.total_response_time += elapsed_time;
                            debug!("Request succeeded, response time: {} ms", elapsed_time);
                        }
                        Err(e) => {
                            stats.failed_requests += 1;
                            {
                                let mut error_counts = error_counts.write().unwrap();
                                let count = error_counts.entry(e.to_string()).or_insert(0);
                                *count += 1;
                                // progress_bar.println(format!("{}({})", e,count));
                                //print!("{}({})", e,count);
                            }
                            //error!("Request failed with error: {}", e);
                        }
                    }
                    
                    request_count += 1;
                    if stop_flag_clone.load(Ordering::SeqCst) {
                        break;
                    }
                }
            });
            handles.push(handle);

            if stop_flag.load(Ordering::SeqCst) {
                break;
            }
        }

        tokio::select! {
            _ = signal_handler => { info!("Signal handler completed");},
            _ = future::join_all(handles) => {
                progress_bar.finish_with_message("Completed requests");
                 info!("All connection tasks completed");},
        }
        
        let elapsed_time = start_time.elapsed();
        let elapsed_seconds = elapsed_time.as_secs_f64();

        // Display the results
        
        let mut stats = stats.write().unwrap();
        let total_requests = stats.completed_requests.load(Ordering::Relaxed);
        let average_response_time = if total_requests > 0 {
            stats.total_response_time / total_requests as u128
        } else {
            0
        };
        // TODO: make the avg req calculation more accurate
        //let average_requests_per_second = stats.successful_requests as f64 / elapsed_seconds;

        let average_requests_per_second = total_requests as f64 / elapsed_seconds;

        println!("\n{:=^50}", format!(" Results for {} Connections ", connections));
        println!("{:<24}|{:>25}", "Total requests", total_requests);
        println!("{:<24}|{:>25}", "Successful requests", stats.successful_requests);
        println!("{:<24}|{:>25}", "Failed requests", stats.failed_requests);
        println!("{:<24}|{:>25}", "Timeout requests", stats.timeout_requests);
        println!("{:<24}|{:>25.2}", "Avg requests per second", average_requests_per_second);
        println!("{:<24}|{:>25} ms", "Average response time", average_response_time);
        println!("{:<24}|{:>25.2} s", "Elapsed time", elapsed_seconds);
        println!("{:=<50}", "");
        
        println!("\nError counts:");
        let mut error_counts = error_counts.write().unwrap();
        for (error, count) in error_counts.iter() {
            println!("{}: {}", error, count);
        }

        // Store the results
        records.push(vec![
            connections.to_string(),
            total_requests.to_string(),
            stats.successful_requests.to_string(),
            stats.failed_requests.to_string(),
            average_response_time.to_string(),
            format!("{:.2}", average_requests_per_second),
            format!("{:.2}", elapsed_seconds),
            stats.timeout_requests.to_string(),
        ]);
        if stop_flag.load(Ordering::SeqCst) {
            break;
        }
        
    }
    
     // Export the results to a CSV file
     let file_path = &args.output_filename;
     let mut wtr = csv::Writer::from_path(file_path).unwrap();
 
     // Write headers
     wtr.write_record(headers).unwrap();
 
     // Write records
     for record in records {
         wtr.write_record(&record).unwrap();
     }
 
     // Flush the writer to ensure all data is written to the file
     wtr.flush().unwrap();
 
     info!("Results exported to {}", file_path);
 }