use anyhow::Result;
use clap::Parser;
use script::{get_kroma_host_cli_by_l1_head_hash, init_fetcher, parse_u64, utils::PreviewReport};
use std::{fs::File, io::Write};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// L2 block number for derivation.
    #[arg(short, long, value_parser = parse_u64)]
    l2_block: u64,

    /// L1 distance from the origin.
    #[arg(short, long, default_value_t = 100)]
    distance: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let fetcher = init_fetcher();

    let report_base = PreviewReport::from_fetcher(args.l2_block, Some(&fetcher)).await;
    let report_head = report_base.l1_head(args.distance, Some(&fetcher)).await;
    let host_cli =
        get_kroma_host_cli_by_l1_head_hash(args.l2_block, report_head.hash, Some(&fetcher)).await?;

    println!("> PreviewReport: {:#?}", report_base);
    println!("> L1 Head: {:#?}", report_head);
    println!("> Host CLI: {:#?}", host_cli);

    let json_string = serde_json::to_string_pretty(&host_cli)?;
    let mut file = File::create("host_cli_output.json")?;
    file.write_all(json_string.as_bytes())?;
    println!("> Host CLI was saved");

    Ok(())
}
