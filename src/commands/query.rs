use clap::Parser;

use miden_client::{crypto::FeltRng, Client};

use super::sync::SyncCmd;

#[derive(Debug, Clone, Parser)]
#[clap(about = "Query rollup for notes with a certain tag")]
pub struct QueryCmd {
    /// Tags to be queried from the rollup
    #[clap(required = true)]
    pub tags: Vec<u32>,
}

impl QueryCmd {
    pub async fn execute(&self, client: &mut Client<impl FeltRng>) -> Result<(), String> {
        for tag in self.tags.clone() {
            client
                .add_note_tag(tag.into())
                .await
                .map_err(|e| e.to_string())?;
        }

        // Sync rollup state
        let sync_command = SyncCmd {};
        sync_command.execute(client).await?;

        Ok(())
    }
}
