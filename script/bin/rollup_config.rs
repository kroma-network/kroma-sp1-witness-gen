use script::init_fetcher;
use serde_json::{Result, Value};
use std::fs::File;
use std::io::BufReader;

#[tokio::main]
async fn main() -> Result<()> {
    let fetcher = init_fetcher();

    let l2_chain_id = fetcher.get_l2_chain_id().await.unwrap();

    let file_name = format!("configs/{}/rollup.json", l2_chain_id);
    let file = File::open(file_name).unwrap();
    let reader = BufReader::new(file);

    let rollup_config: Value = serde_json::from_reader(reader).unwrap();
    println!("> Rollup config related to {:?} was generated", l2_chain_id);
    println!("> config: {:#?}", rollup_config);

    Ok(())
}
