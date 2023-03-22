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
./target/release/load_testing_tool \
    --server-url http://your-json-rpc-server-url.com \
    --concurrent-connections 100 \
    --connections-step 10 \
    --requests-per-connection 1000 \
    --timeout 5000 \
    --json-file path/to/your/json/file.json
```
- `--server-url`: The URL of the JSON-RPC server to test.
- `--concurrent-connections`: The maximum number of concurrent connections to test.
- `--connections-step`: The step size for increasing the number of concurrent connections in each iteration.
- `--requests-per-connection`: The number of requests to send per connection.
- `--timeout`: The request timeout duration in milliseconds.
- `--json-file`: The path to the JSON file containing the JSON-RPC request body.
3. Analyze the results displayed in the terminal and exported to the results.csv file.
## 📜 License
This project is licensed under the MIT License.