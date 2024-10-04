use clap::Parser;
use jsonrpc_http_server::ServerBuilder;

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long = "endpoint", default_value = "127.0.0.1:3030")]
    endpoint: String,
}

fn main() {
    dotenv::dotenv().ok();
    tracing_subscriber::fmt::Subscriber::builder().init();

    let args = Args::parse();

    let io = jsonrpc_core::IoHandler::new();

    let server = ServerBuilder::new(io)
        .threads(3)
        .max_request_body_size(32_000_000)
        .start_http(&args.endpoint.parse().unwrap())
        .unwrap();

    server.wait();
}
