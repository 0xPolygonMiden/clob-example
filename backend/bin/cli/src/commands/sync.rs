use clap::Parser;

use miden_client::{crypto::FeltRng, Client};

#[derive(Debug, Clone, Parser)]
#[clap(about = "Sync rollup state")]
pub struct SyncCmd {}

impl SyncCmd {
    pub async fn execute(&self, client: &mut Client<impl FeltRng>) -> Result<(), String> {
        client.sync_state().await?;
        println!("Sync successful.");
        Ok(())
    }
}
