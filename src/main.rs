use clap::Parser;
use miden_order_book::cli::Cli;

#[tokio::main]
async fn main() -> Result<(), String> {
    env_logger::init();

    let cli = Cli::parse();

    cli.execute().await
}
