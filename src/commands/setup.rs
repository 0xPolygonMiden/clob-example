use core::panic;
use std::time::Duration;

use clap::Parser;
use miden_client::{
    accounts::{Account, AccountId, AccountStorageMode, AccountTemplate},
    assets::{FungibleAsset, TokenSymbol},
    crypto::FeltRng,
    notes::{NoteTag, NoteType},
    transactions::{build_swap_tag, TransactionRequest},
    Client, Word,
};
use tokio::time::sleep;

use crate::{
    constants::DB_FILE_PATH, note::create_partial_swap_notes_transaction_request,
    utils::clear_notes_tables,
};

// Setup COMMAND
// ================================================================================================

#[derive(Debug, Clone, Parser)]
#[clap(about = "Setup the order book")]
pub struct SetupCmd {}

impl SetupCmd {
    pub async fn execute(&self, client: &mut Client<impl FeltRng>) -> Result<(), String> {
        // Sync rollup state
        client.sync_state().await.unwrap();

        // Create faucet accounts
        let (faucet1, _) = Self::create_faucet(1000, "ASSETA", client);
        let (faucet2, _) = Self::create_faucet(1000, "ASSETB", client);

        // Create user account
        let (admin, _) = Self::create_wallet(client);
        let (user, _) = Self::create_wallet(client);

        // Mint assets for user
        Self::fund_wallet(faucet1.id(), 500, faucet2.id(), 500, admin.id(), client).await;
        Self::fund_wallet(faucet1.id(), 500, faucet2.id(), 500, user.id(), client).await;

        // Create 50 ASSETA/ASSETB swap notes
        Self::create_partial_swap_notes(
            50,
            faucet1.id(),
            500,
            faucet2.id(),
            500,
            admin.id(),
            client,
        )
        .await;

        // Create 50 ASSETB/ASSETA swap notes
        Self::create_partial_swap_notes(
            50,
            faucet2.id(),
            500,
            faucet1.id(),
            500,
            admin.id(),
            client,
        )
        .await;

        // Build note tags
        let swap_1_2_tag = build_swap_tag(NoteType::Public, faucet1.id(), faucet2.id()).unwrap();
        let swap_2_1_tag = build_swap_tag(NoteType::Public, faucet2.id(), faucet1.id()).unwrap();

        if swap_1_2_tag == swap_2_1_tag {
            panic!("Both asset tags should not be similar.");
        }

        // Sanitize client db
        clear_notes_tables(DB_FILE_PATH);

        Self::print_clob_data(
            faucet1.id(),
            faucet2.id(),
            admin.id(),
            user.id(),
            swap_1_2_tag,
            swap_2_1_tag,
        );

        println!("CLOB successfully setup.");

        Ok(())
    }

    async fn create_partial_swap_notes(
        num_notes: u8,
        faucet1: AccountId,
        total_asset_offering: u64,
        faucet2: AccountId,
        total_asset_requesting: u64,
        user: AccountId,
        client: &mut Client<impl FeltRng>,
    ) {
        let transaction_request = create_partial_swap_notes_transaction_request(
            num_notes,
            user,
            faucet1,
            total_asset_offering,
            faucet2,
            total_asset_requesting,
            client.rng(),
        )
        .unwrap();
        let tx_result = client.new_transaction(user, transaction_request).unwrap();
        client.submit_transaction(tx_result).await.unwrap();
    }

    async fn fund_wallet(
        faucet1: AccountId,
        asset_a_amount: u64,
        faucet2: AccountId,
        asset_b_amount: u64,
        user: AccountId,
        client: &mut Client<impl FeltRng>,
    ) {
        // Setup mint
        let note_type = NoteType::Public;

        // Mint AssetA
        let asset_a = FungibleAsset::new(faucet1, asset_a_amount).unwrap();
        let transaction_request =
            TransactionRequest::mint_fungible_asset(asset_a, user, note_type, client.rng())
                .unwrap();
        let tx_result = client
            .new_transaction(faucet1, transaction_request)
            .unwrap();
        let asset_a_note_id = tx_result.relevant_notes()[0].id();
        client.submit_transaction(tx_result).await.unwrap();

        // Mint AssetB
        let asset_b = FungibleAsset::new(faucet2, asset_b_amount).unwrap();
        let transaction_request =
            TransactionRequest::mint_fungible_asset(asset_b, user, note_type, client.rng())
                .unwrap();
        let tx_result = client
            .new_transaction(faucet2, transaction_request)
            .unwrap();
        let asset_b_note_id = tx_result.relevant_notes()[0].id();
        client.submit_transaction(tx_result).await.unwrap();

        // Sync rollup state
        sleep(Duration::from_secs(20)).await;
        client.sync_state().await.unwrap();

        // Fund receiving wallet
        let tx_request = TransactionRequest::consume_notes(vec![asset_a_note_id, asset_b_note_id]);
        let tx_result = client.new_transaction(user, tx_request).unwrap();
        client.submit_transaction(tx_result).await.unwrap();
    }

    fn create_wallet(client: &mut Client<impl FeltRng>) -> (Account, Word) {
        let wallet_template = AccountTemplate::BasicWallet {
            mutable_code: false,
            storage_mode: AccountStorageMode::Public,
        };
        client.new_account(wallet_template).unwrap()
    }

    fn create_faucet(
        max_supply: u64,
        token_symbol: &str,
        client: &mut Client<impl FeltRng>,
    ) -> (Account, Word) {
        let faucet_template = AccountTemplate::FungibleFaucet {
            token_symbol: TokenSymbol::new(token_symbol).unwrap(),
            decimals: 10,
            max_supply,
            storage_mode: AccountStorageMode::Public,
        };
        client.new_account(faucet_template).unwrap()
    }

    fn print_clob_data(
        faucet1: AccountId,
        faucet2: AccountId,
        admin: AccountId,
        user: AccountId,
        swap_1_2_tag: NoteTag,
        swap_2_1_tag: NoteTag,
    ) {
        println!("faucet1: {}", faucet1);
        println!("faucet2: {}", faucet2);
        println!("swap_1_2_tag: {}", swap_1_2_tag);
        println!("swap_2_1_tag: {}", swap_2_1_tag);
        println!("Admin: {}", admin);
        println!("User: {}", user);
    }
}
