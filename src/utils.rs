use miden_client::{
    accounts::{AccountData, AccountId},
    assets::{Asset, FungibleAsset},
    auth::StoreAuthenticator,
    config::{Endpoint, RpcConfig},
    crypto::{FeltRng, RpoRandomCoin},
    notes::NoteType,
    rpc::TonicRpcClient,
    store::sqlite_store::{config::SqliteStoreConfig, SqliteStore},
    transactions::{
        request::{TransactionRequest, TransactionRequestError},
        OutputNote,
    },
    Client, Felt,
};
use miden_lib::{
    notes::create_swap_note,
    utils::{Deserializable, Serializable},
};
use rand::Rng;
use std::{
    fs::{self, File},
    io::{self, Read, Write},
    path::Path,
    rc::Rc,
};

// Client Setup
// ================================================================================================

pub fn setup_client() -> Client<
    TonicRpcClient,
    RpoRandomCoin,
    SqliteStore,
    StoreAuthenticator<RpoRandomCoin, SqliteStore>,
> {
    let store_config = SqliteStoreConfig::default();
    let store = Rc::new(SqliteStore::new(&store_config).unwrap());
    let mut rng = rand::thread_rng();
    let coin_seed: [u64; 4] = rng.gen();
    let rng = RpoRandomCoin::new(coin_seed.map(Felt::new));
    let authenticator = StoreAuthenticator::new_with_rng(store.clone(), rng);
    let rpc_config = RpcConfig {
        endpoint: Endpoint::new("http".to_string(), "localhost".to_string(), 57291),
        timeout_ms: 10000,
    };
    let in_debug_mode = true;
    Client::new(
        TonicRpcClient::new(&rpc_config),
        rng,
        store,
        authenticator,
        in_debug_mode,
    )
}

// Transaction Request Creation
// ================================================================================================

pub fn create_swap_notes_transaction_request(
    num: u8,
    sender: AccountId,
    offering_faucet: AccountId,
    requesting_faucet: AccountId,
    rng: &mut impl FeltRng,
) -> Result<TransactionRequest, TransactionRequestError> {
    let mut expected_future_notes = vec![];
    let mut own_output_notes = vec![];
    let note_type = NoteType::Public;
    let aux = Felt::new(0);
    let offered_asset = Asset::Fungible(FungibleAsset::new(offering_faucet, 1).unwrap());
    let requested_asset = Asset::Fungible(FungibleAsset::new(requesting_faucet, 1).unwrap());

    // Create 50 offering/requesting swap notes
    for _ in 0..num {
        let (created_note, payback_note_details) =
            create_swap_note(sender, offered_asset, requested_asset, note_type, aux, rng)?;
        expected_future_notes.push(payback_note_details);
        own_output_notes.push(OutputNote::Full(created_note));
    }

    // Create 50 requesting/offering swap notes
    for _ in 0..num {
        let (created_note, payback_note_details) =
            create_swap_note(sender, requested_asset, offered_asset, note_type, aux, rng)?;
        expected_future_notes.push(payback_note_details);
        own_output_notes.push(OutputNote::Full(created_note));
    }

    TransactionRequest::new()
        .with_expected_future_notes(expected_future_notes)
        .with_own_output_notes(own_output_notes)
}

// AccountData I/O
// ================================================================================================

pub fn export_account_data(account_data: &AccountData, filename: &str) -> io::Result<()> {
    let serialized = account_data.to_bytes();
    fs::create_dir_all("accounts")?;
    let file_path = Path::new("accounts").join(format!("{}.mac", filename));
    let mut file = File::create(file_path)?;
    file.write_all(&serialized)?;
    Ok(())
}

pub fn import_account_data(file_path: &str) -> io::Result<AccountData> {
    let mut file = File::open(file_path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    AccountData::read_from_bytes(&buffer)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))
}

pub fn load_accounts() -> io::Result<Vec<AccountData>> {
    let accounts_dir = Path::new("accounts");

    if !accounts_dir.exists() {
        return Ok(Vec::new());
    }

    let mut accounts = Vec::new();

    for entry in fs::read_dir(accounts_dir)? {
        let entry = entry?;
        let path = entry.path();
        let path_str = path.to_str().unwrap();

        match import_account_data(path_str) {
            Ok(account_data) => accounts.push(account_data),
            Err(e) => eprintln!(
                "Error importing account data from {} : {}",
                path_str,
                e.to_string()
            ),
        }
    }

    Ok(accounts)
}
