# 🚀 Load Testing Tool for JSON-RPC Servers 🔧

This load testing tool is designed to stress test JSON-RPC servers by sending a high volume of requests. It allows you to test the server's performance under different levels of concurrent connections and requests per connection.

## 🌟 Features

- Configurable concurrent connections and requests per connection
- Customizable JSON-RPC request body loaded from a JSON file
- Adjustable request timeout duration
- Displays results for each level of concurrent connections, including total requests, successful requests, failed requests, average response time, and average requests per second
- Exports results to a CSV file for further analysis

## 📚 Prerequisites

- [Rust](https://www.rust-lang.org/tools/install)
- [Tokio](https://tokio.rs/)

## 📥 Installation

1. Clone this repository:
```
git clone https://github.com/yourusername/load-testing-tool.git
```
2. Change into the cloned directory:
```
cd load-testing-tool
```

3. Build the project:
```
cargo build --release
```

4. Find the compiled binary in the `./target/release` folder.

## 🔍 Usage

1. Create a JSON file containing the JSON-RPC request body you want to use in the test. For example:

```json
{
    "method": "eth_getLogs",
    "params": [
        {
            "fromBlock":"latest",
            "toBlock":"latest"
        }
    ],
    "id": 1,
    "jsonrpc": "2.0"
}
```
2. Run the load testing tool with the required arguments:
```
./target/release/json_rpc_load_tester [OPTIONS]
```
Replace [OPTIONS] with the appropriate command-line options for your test. Available options include:

- `-t, --timeout`: Request timeout in milliseconds (default: 15000)
- `-u, --url`: URL of the JSON-RPC server to test (required)
- `-c, --connections`: Number of concurrent connections to establish (required)
- `-r, --requests`: Number of requests per connection (0 for time-based test, default: 0)
- `-s, --step`: Connection step size for testing with varying connection counts (default: 0)
- `-f, --file`: Path to the file containing the JSON-RPC request (required)
- `-d, --duration`: Test duration in seconds 
3. Analyze the results displayed in the terminal and exported to the results.csv file.
## 📜 License
This project is licensed under the MIT License.