use core::panic;
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
    time::Duration,
};

use clap::Parser;
use miden_client::{
    accounts::{Account, AccountId, AccountStorageType, AccountTemplate},
    assets::{FungibleAsset, TokenSymbol},
    auth::TransactionAuthenticator,
    crypto::FeltRng,
    notes::{NoteTag, NoteType},
    rpc::NodeRpcClient,
    store::Store,
    transactions::{build_swap_tag, request::TransactionRequest},
    Client, Word,
};
use serde::{Deserialize, Serialize};
use tokio::time::sleep;

use crate::{
    constants::{CLOB_DATA_FILE_PATH, DB_FILE_PATH},
    utils::{clear_notes_tables, create_swap_notes_transaction_request},
};

//
// ================================================================================================

#[derive(Debug, Serialize, Deserialize)]
pub struct Clob {
    pub faucet1: AccountId,
    pub faucet1_name: String,
    pub faucet2: AccountId,
    pub faucet2_name: String,
    pub user: AccountId,
    pub swap_1_2_tag: NoteTag,
    pub swap_2_1_tag: NoteTag,
}

// Setup COMMAND
// ================================================================================================

#[derive(Debug, Clone, Parser)]
#[clap(about = "Setup the order book")]
pub struct SetupCmd {}

impl SetupCmd {
    pub async fn execute<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        &self,
        client: &mut Client<N, R, S, A>,
    ) -> Result<(), String> {
        // Sync rollup state
        client.sync_state().await.unwrap();

        // Create faucet accounts
        let (faucet1, _) = Self::create_faucet(2000, "ASSETA", client);
        let (faucet2, _) = Self::create_faucet(2000, "ASSETB", client);

        // Create user account
        let (user, _) = Self::create_wallet(client);
        let (user_2, _) = Self::create_wallet(client);

        // Mint assets for user
        Self::fund_user_wallet(faucet1.id(), 1000, faucet2.id(), 1000, user.id(), client).await;
        Self::fund_user_wallet(faucet1.id(), 5, faucet2.id(), 5, user_2.id(), client).await;

        // Create 50 ASSETA/ASSETB swap notes
        Self::create_swap_notes(50, faucet1.id(), 500, faucet2.id(), 500, user.id(), client).await;

        // Create 50 ASSETB/ASSETA swap notes
        Self::create_swap_notes(50, faucet2.id(), 500, faucet1.id(), 500, user.id(), client).await;

        // Build note tags
        let swap_1_2_tag = build_swap_tag(NoteType::Public, faucet1.id(), faucet2.id()).unwrap();
        let swap_2_1_tag = build_swap_tag(NoteType::Public, faucet2.id(), faucet1.id()).unwrap();

        if swap_1_2_tag == swap_2_1_tag {
            panic!("Both asset tags should not be similar.");
        }

        // Sanitize client db
        clear_notes_tables(DB_FILE_PATH).unwrap();

        Self::print_clob_data(
            faucet1.id(),
            faucet2.id(),
            user.id(),
            swap_1_2_tag,
            swap_2_1_tag,
        );

        Self::export_clob_data(
            faucet1.id(),
            "BTC",
            faucet2.id(),
            "ETH",
            user.id(),
            swap_1_2_tag,
            swap_2_1_tag,
        )
        .unwrap();

        println!("CLOB successfully setup.");
        println!("user 2: {:?}", user_2.id().to_hex());

        Ok(())
    }

    async fn create_swap_notes<
        N: NodeRpcClient,
        R: FeltRng,
        S: Store,
        A: TransactionAuthenticator,
    >(
        num_notes: u8,
        faucet1: AccountId,
        total_asset_offering: u64,
        faucet2: AccountId,
        total_asset_requesting: u64,
        user: AccountId,
        client: &mut Client<N, R, S, A>,
    ) {
        let transaction_request = create_swap_notes_transaction_request(
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

    async fn fund_user_wallet<
        N: NodeRpcClient,
        R: FeltRng,
        S: Store,
        A: TransactionAuthenticator,
    >(
        faucet1: AccountId,
        asset_a_amount: u64,
        faucet2: AccountId,
        asset_b_amount: u64,
        user: AccountId,
        client: &mut Client<N, R, S, A>,
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

    fn create_wallet<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        client: &mut Client<N, R, S, A>,
    ) -> (Account, Word) {
        let wallet_template = AccountTemplate::BasicWallet {
            mutable_code: false,
            storage_type: AccountStorageType::OnChain,
        };
        client.new_account(wallet_template).unwrap()
    }

    fn create_faucet<N: NodeRpcClient, R: FeltRng, S: Store, A: TransactionAuthenticator>(
        max_supply: u64,
        token_symbol: &str,
        client: &mut Client<N, R, S, A>,
    ) -> (Account, Word) {
        let faucet_template = AccountTemplate::FungibleFaucet {
            token_symbol: TokenSymbol::new(token_symbol).unwrap(),
            decimals: 10,
            max_supply,
            storage_type: AccountStorageType::OnChain,
        };
        client.new_account(faucet_template).unwrap()
    }

    fn print_clob_data(
        faucet1: AccountId,
        faucet2: AccountId,
        user: AccountId,
        swap_1_2_tag: NoteTag,
        swap_2_1_tag: NoteTag,
    ) {
        println!("faucet1: {}", faucet1);
        println!("faucet2: {}", faucet2);
        println!("swap_1_2_tag: {}", swap_1_2_tag);
        println!("swap_2_1_tag: {}", swap_2_1_tag);
        println!("User: {}", user);
    }

    fn export_clob_data(
        faucet1: AccountId,
        faucet1_name: &str,
        faucet2: AccountId,
        faucet2_name: &str,
        user: AccountId,
        swap_1_2_tag: NoteTag,
        swap_2_1_tag: NoteTag,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let clob = Clob {
            faucet1,
            faucet1_name: faucet1_name.to_string(),
            faucet2,
            faucet2_name: faucet2_name.to_string(),
            user,
            swap_1_2_tag,
            swap_2_1_tag,
        };

        // Serialize the struct to a TOML string
        let toml_string = toml::to_string(&clob)?;

        // Write the TOML string to a file
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(CLOB_DATA_FILE_PATH)?;

        file.write_all(toml_string.as_bytes())?;

        Ok(())
    }

    pub fn import_clob_data() -> Result<Clob, Box<dyn std::error::Error>> {
        // Check if file exists
        if !Path::new(CLOB_DATA_FILE_PATH).exists() {
            return Err(format!("CLOB data file not found: {}", CLOB_DATA_FILE_PATH).into());
        }

        // Read the TOML file contents
        let mut file = File::open(CLOB_DATA_FILE_PATH)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        // Deserialize the TOML string into a Clob struct
        let clob: Clob = toml::from_str(&contents)?;

        Ok(clob)
    }
}
