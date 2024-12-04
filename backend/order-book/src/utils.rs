use miden_client::{
    accounts::AccountId,
    assets::{Asset, FungibleAsset},
    config::{Endpoint, RpcConfig},
    crypto::{FeltRng, RpoRandomCoin},
    notes::NoteTag,
    rpc::TonicRpcClient,
    store::{
        sqlite_store::{config::SqliteStoreConfig, SqliteStore},
        InputNoteRecord, NoteFilter, StoreAuthenticator,
    },
    transactions::NoteArgs,
    Client, Felt,
};
use miden_tx::{LocalTransactionProver, ProvingOptions};
use rand::Rng;
use rusqlite::Connection;
use std::sync::Arc;

use crate::order::Order;

// Client Setup
// ================================================================================================

pub async fn setup_client() -> Client<impl FeltRng> {
    let store_config = SqliteStoreConfig::default();
    let store = SqliteStore::new(&store_config).await.unwrap();
    let store = Arc::new(store);

    let mut rng = rand::thread_rng();
    let coin_seed: [u64; 4] = rng.gen();

    let rng = RpoRandomCoin::new(coin_seed.map(Felt::new));
    let authenticator = StoreAuthenticator::new_with_rng(store.clone(), rng);
    let tx_prover = LocalTransactionProver::new(ProvingOptions::default());

    let rpc_config = RpcConfig {
        endpoint: Endpoint::new("http".to_string(), "localhost".to_string(), 57291),
        timeout_ms: 10000,
    };

    let in_debug_mode = true;

    Client::new(
        Box::new(TonicRpcClient::new(&rpc_config)),
        rng,
        store,
        Arc::new(authenticator),
        Arc::new(tx_prover),
        in_debug_mode,
    )
}

pub async fn get_notes_by_tag(client: &Client<impl FeltRng>, tag: NoteTag) -> Vec<InputNoteRecord> {
    let notes = client.get_input_notes(NoteFilter::Unspent).await.unwrap();

    notes
        .into_iter()
        .filter_map(|note| {
            note.clone().metadata().and_then(|metadata| {
                if metadata.tag() == tag {
                    Some(note)
                } else {
                    None
                }
            })
        })
        .collect()
}

pub fn get_assets_from_swap_note(note: &InputNoteRecord) -> (Asset, Asset) {
    let source_asset =
        Asset::Fungible(note.assets().iter().collect::<Vec<&Asset>>()[0].unwrap_fungible());
    let target_faucet = AccountId::try_from(note.details().inputs().values()[7]).unwrap();
    let target_amount = note.details().inputs().values()[4].as_int();
    let target_asset = Asset::Fungible(FungibleAsset::new(target_faucet, target_amount).unwrap());
    (source_asset, target_asset)
}

pub fn print_order_table(title: &str, orders: &[Order]) {
    let mut table = Vec::new();
    table.push("+--------------------------------------------------------------------+--------------------+------------------+--------------------+------------------+----------+".to_string());
    table.push("| Note ID                                                            | Requested Asset    | Amount Requested | Offered Asset      | Offered Amount   | Price    |".to_string());
    table.push("+--------------------------------------------------------------------+--------------------+------------------+--------------------+------------------+----------+".to_string());

    for order in orders {
        let note_id = order
            .id()
            .map_or_else(|| "N/A".to_string(), |id| id.to_string());
        let source_asset_faucet_id = order.source_asset().faucet_id().to_string();
        let source_asset_amount = order.source_asset().unwrap_fungible().amount();
        let target_asset_faucet_id = order.target_asset().faucet_id().to_string();
        let target_asset_amount = order.target_asset().unwrap_fungible().amount();

        table.push(format!(
            "| {:<66} | {:<16} | {:<16} | {:<16} | {:<16} | {:<8.2} |",
            note_id,
            target_asset_faucet_id,
            target_asset_amount,
            source_asset_faucet_id,
            source_asset_amount,
            order.price()
        ));
    }

    table.push("+--------------------------------------------------------------------+--------------------+------------------+--------------------+------------------+----------+\n".to_string());

    // Print title
    println!("{}\n", title);

    // Print table
    for line in table {
        println!("{}", line);
    }
}

pub fn print_balance_update(orders: &[Order], args: &[NoteArgs]) {
    if orders.is_empty() {
        println!("No orders to process. Your balance will not change.");
        return;
    }

    let mut total_source_asset = 0u64;
    let mut total_target_asset = 0u64;
    let source_faucet_id = orders[0].target_asset().faucet_id();
    let target_faucet_id = orders[0].source_asset().faucet_id();

    for (i, order) in orders.into_iter().enumerate() {
        total_source_asset += args[i][0].as_int();
        total_target_asset += order.source_asset().unwrap_fungible().amount();
    }

    println!("Balance Update Preview:");
    println!("------------------------");
    println!("Assets you will receive:");
    println!("  Faucet ID: {}", target_faucet_id);
    println!("  Amount: {}", total_target_asset);
    println!("\nAssets you will spend:");
    println!("  Faucet ID: {}", source_faucet_id);
    println!("  Amount: {}", total_source_asset);
    println!("------------------------");
}

pub fn clear_notes_tables(db_path: &str) {
    // Open a connection to the SQLite database
    let conn = Connection::open(db_path).unwrap();

    // Execute the DELETE commands
    conn.execute_batch(
        "
        DELETE FROM output_notes;
        DELETE FROM input_notes;
    ",
    )
    .unwrap();

    println!("Both output_notes and input_notes tables have been cleared.");
}
