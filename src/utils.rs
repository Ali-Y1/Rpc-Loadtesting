use csv::Writer;
use futures::future;
use jsonrpc_core::IoHandler;
use jsonrpc_http_server::{AccessControlAllowOrigin, DomainsValidation, ServerBuilder};
use jsonrpc_derive::rpc;
use log::{debug, warn, error, info};
use serde::{Deserialize, Serialize};
use structopt::StructOpt;
use serde_json::{Map, Value};
use reqwest::Client;
use tokio::signal::unix::{SignalKind, signal};
use std::error::Error;
use std::fs::File;
use std::io::BufRead;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Instant;
use std::sync::atomic::{AtomicU64, Ordering, AtomicBool};
use tokio::time::{timeout, Duration};
use std::path::PathBuf;
use std::{fs, fmt, io};


#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct JsonRequest {
    pub id: u64,
    pub jsonrpc: String,
    pub method: String,
    pub params: Vec<Value>,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonResponse {
    pub id: u64,
    pub jsonrpc: String,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
}
#[derive(Default)]
pub struct Stats {
    pub completed_requests: AtomicU64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_response_time: u128,
    pub timeout_requests: usize,
}

#[derive(StructOpt)]
pub struct Cli {
    #[structopt(short = "t", long = "timeout", default_value = "15000", help = "Request timeout in milliseconds")]
    pub timeout: u64,
    #[structopt(short = "u", long = "url", required = true, help = "List of server URLs separated by commas")]
    pub server_urls: Vec<String>,
    #[structopt(short = "c", long = "connections", help = "Number of concurrent connections to establish")]
    pub concurrent_connections: u32,
    #[structopt(short = "r", long = "requests", default_value = "0", help = "Number of requests per connection (0 for time-based test)")]
    pub requests_per_connection: u32,
    #[structopt(short = "s", long = "step", default_value = "0", help = "Connection step size for testing with varying connection counts")]
    pub connections_step: u32,
    #[structopt(short = "f", long = "file", help = "Path to the file containing the JSON-RPC request")]
    pub request_file: PathBuf,
    #[structopt(short = "d", long = "duration", default_value = "0", help = "Test duration in seconds (ignored if requests_per_connection is set)")]
    pub test_duration: u64,
    #[structopt(short = "o",long = "output",default_value = "results.csv",help = "Output filename for the results (CSV format)")]
    pub output_filename: String,
    #[structopt(short = "v", long = "verbose", parse(from_occurrences), help = "Increase output verbosity")]
    pub verbosity: u8,
    #[structopt(short = "p", long = "pipe", help = "Take requests from stdin")]
    pub pipe: bool,
}

pub async fn read_json_request_from_file(file_path: &PathBuf) -> Result<JsonRequest, Box<dyn Error>> {
    let contents = fs::read_to_string(file_path)?;
    let json_request: JsonRequest = serde_json::from_str(&contents)?;
    debug!("Read JSON request from file: {:?}", json_request);
    Ok(json_request)
}

#[derive(Debug)]
enum CustomJsonRpcError {
    HttpError(reqwest::StatusCode),
    JsonRpcError(JsonResponse),
}

impl fmt::Display for CustomJsonRpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CustomJsonRpcError::HttpError(status) => write!(f, "HTTP error: {}", status),
            CustomJsonRpcError::JsonRpcError(response) => write!(f, "JSON-RPC error: {:?}", response),
        }
    }
}

impl Error for CustomJsonRpcError {}

pub async fn send_json_rpc_request(
    client: &Client,
    server_url: &str,
    request: &JsonRequest,
) -> Result<(), Box<dyn Error>> {
    debug!("Sending JSON-RPC request: {:?}", request);
    let response = client.post(server_url)
        .json(request)
        .send()
        .await?;

    if response.status().is_success() {
        let content_length = response.content_length().unwrap_or(1000);
        if content_length < 1000 {
            let json_response: JsonResponse = response.json().await?;
            return Err(CustomJsonRpcError::JsonRpcError(json_response).into());
            // return Ok(())
        }
        Ok(())
    } else {
        Err(CustomJsonRpcError::HttpError(response.status()).into())
    }
}
pub async fn export_to_csv(filename: &str, headers: &[&str], records: &[Vec<String>]) -> Result<(), Box<dyn Error>> {
    info!("Exporting results to CSV file: {}", filename);
    let file = File::create(filename)?;
    let mut writer = Writer::from_writer(file);
    info!("Writing headers to the CSV file");
    writer.write_record(headers)?;
    info!("Writing records to the CSV file");
    for record in records {
        writer.write_record(record)?;
    }
    info!("Flushing the writer and finalizing the CSV file");
    writer.flush()?;
    Ok(())
}

pub async fn process_ethspam_output(tx: mpsc::Sender<JsonRequest>, stop_flag: Arc<AtomicBool>) {
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        if stop_flag.load(Ordering::SeqCst) {
            break;
        }

        let line = line.unwrap();
        let json_request: serde_json::Result<JsonRequest> = serde_json::from_str(&line);

        match json_request {
            Ok(request) => {
                // Send the JsonRequest to the channel
                tx.send(request).unwrap();
            }
            Err(e) => {
                eprintln!("Error parsing JSON RPC request: {}", e);
            }
        }
    }
}