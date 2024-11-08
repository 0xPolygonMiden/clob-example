use clap::Parser;

use crate::{
    commands::{
        init::InitCmd, list::ListCmd, order::OrderCmd, query::QueryCmd, setup::SetupCmd,
        sync::SyncCmd,
    },
    utils::setup_client,
};

/// CLI actions
#[derive(Debug, Parser)]
pub enum Command {
    Init(InitCmd),
    Setup(SetupCmd),
    Order(OrderCmd),
    List(ListCmd),
    Sync(SyncCmd),
    Query(QueryCmd),
}

/// Root CLI struct
#[derive(Parser, Debug)]
#[clap(
    name = "Miden-order-book",
    about = "Miden order book cli",
    version,
    rename_all = "kebab-case"
)]
pub struct Cli {
    #[clap(subcommand)]
    action: Command,
}

impl Cli {
    pub async fn execute(&self) -> Result<(), String> {
        // Setup client
        let mut client = setup_client().await;

        // Execute Cli commands
        match &self.action {
            Command::Setup(setup) => setup.execute(&mut client).await,
            Command::Order(order) => order.execute(&mut client).await,
            Command::Sync(sync) => sync.execute(&mut client).await,
            Command::Init(init) => init.execute(),
            Command::Query(query) => query.execute(&mut client).await,
            Command::List(list) => list.execute(&client).await,
        }
    }
}
