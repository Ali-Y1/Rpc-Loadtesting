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
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::sync::atomic::{AtomicU64, Ordering, AtomicBool};
use tokio::time::{timeout, Duration};
use crate::utils::*;

pub mod utils;


pub async fn run() {
    let args = Cli::from_args();
    let client = Arc::new(Client::new());
    let max_connections = args.concurrent_connections;
    let connections_step = args.connections_step;
    let request_file = &args.request_file;

    // Read the JSON request from the file
    let json_request = match read_json_request_from_file(request_file).await {
        Ok(req) => req,
        Err(e) => {
            eprintln!("Error reading JSON request from file: {}", e);
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
        let stats = Arc::new(Mutex::new(Stats::default()));
        let mut handles = Vec::new();
        let start_time = Instant::now();
        let stop_flag = Arc::new(AtomicBool::new(false));
        let timeout_duration = Duration::from_millis(args.timeout);

        // Set up a signal handler for SIGINT and SIGTERM
        let mut sigint = signal(SignalKind::interrupt()).unwrap();
        let mut sigterm = signal(SignalKind::terminate()).unwrap();

        // Spawn a separate task to handle signals
        let stop_flag_clone = stop_flag.clone();
        let signal_handler = tokio::spawn(async move {
            tokio::select! {
                _ = sigint.recv() => {},
                _ = sigterm.recv() => {},
            }
            println!("Received kill signal, shutting down...");
            stop_flag_clone.store(true, Ordering::SeqCst);
        });

        for _ in 0..connections {
            let client = client.clone();
            let stats = stats.clone();
            let server_url = args.server_url.to_owned();
            let requests_per_connection = args.requests_per_connection;
            let stop_flag = stop_flag.clone();
            let json_request_clone = json_request.clone();
            if stop_flag.load(Ordering::SeqCst) {
                break;
            }

            let handle = tokio::spawn(async move {
                for _ in 0..requests_per_connection {
                    let request = json_request_clone.clone();
        
                    let start_time = Instant::now();
                    let result = match timeout(timeout_duration, send_json_rpc_request(&client, &server_url, &request)).await {
                        Ok(res) => res,
                        Err(_) => {
                            // Update the timeout_requests counter
                            let mut stats = stats.lock().unwrap();
                            stats.timeout_requests += 1;
                    
                            Err("Request timed out".into())
                        },
    
                    };
                    let elapsed_time = start_time.elapsed().as_millis();
    
                    let mut stats = stats.lock().unwrap();
                    stats.completed_requests.fetch_add(1, Ordering::Relaxed);
                    match result {
                        Ok(_) => {
                            stats.successful_requests += 1;
                            stats.total_response_time += elapsed_time;
                        }
                        Err(_) => {
                            stats.failed_requests += 1;
                        }
                    }
                }
            });
            handles.push(handle);

            if stop_flag.load(Ordering::SeqCst) {
                break;
            }
        }

        tokio::select! {
            _ = signal_handler => {},
            _ = future::join_all(handles) => {},
        }

        let elapsed_time = start_time.elapsed();
        let elapsed_seconds = elapsed_time.as_secs_f64();

        // Display the results
        let stats = stats.lock().unwrap();
        let total_requests = stats.completed_requests.load(Ordering::Relaxed);
        let average_response_time = if total_requests > 0 {
            stats.total_response_time / total_requests as u128
        } else {
            0
        };
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
     let file_path = "results.csv";
     let mut wtr = csv::Writer::from_path(file_path).unwrap();
 
     // Write headers
     wtr.write_record(headers).unwrap();
 
     // Write records
     for record in records {
         wtr.write_record(&record).unwrap();
     }
 
     // Flush the writer to ensure all data is written to the file
     wtr.flush().unwrap();
 
     println!("Results have been exported to {}", file_path);
 }