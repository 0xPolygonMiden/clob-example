use std::collections::BTreeMap;

use miden_lib::{
    notes::{create_swap_note, create_swapp_note},
    transaction::TransactionKernel,
};
use miden_objects::{
    notes::NoteType,
    testing::account_code::DEFAULT_AUTH_SCRIPT,
    transaction::{TransactionArgs, TransactionScript},
};
use miden_tx::testing::mock_chain::{Auth, MockChain};
use vm_processor::{crypto::RpoRandomCoin, Felt};

#[test]
fn test_swapp_script() {
    // Setup
    // --------------------------------------------------------------------------------------------
    let mut chain = MockChain::new();

    // create assets
    let faucet_1 = chain.add_existing_faucet(Auth::NoAuth, "BTC", 10);
    let faucet_2 = chain.add_existing_faucet(Auth::NoAuth, "ETH", 10);

    let offered_asset = faucet_1.mint(10);
    let requested_asset = faucet_2.mint(10);

    // create sender and target account
    let sender = chain.add_new_wallet(Auth::BasicAuth, vec![offered_asset]);
    let target = chain.add_existing_wallet(Auth::BasicAuth, vec![requested_asset]);

    let note = create_swapp_note(
        sender.id(),
        offered_asset,
        requested_asset,
        NoteType::Public,
        Felt::new(27),
        &mut RpoRandomCoin::new([Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)]),
    )
    .unwrap();

    // add note to chain
    chain.add_note(note.clone());
    chain.seal_block(None);

    // EXECUTE TX
    // --------------------------------------------------------------------------------------------
    let transaction_script =
        TransactionScript::compile(DEFAULT_AUTH_SCRIPT, vec![], TransactionKernel::assembler())
            .unwrap();

    let mut tx_context = chain
        .build_tx_context(target.id())
        .tx_script(transaction_script.clone())
        .build();

    let note_args = [Felt::new(9), Felt::new(0), Felt::new(0), Felt::new(0)];

    let note_args_map = BTreeMap::from([(note.id(), note_args)]);

    let tx_args = TransactionArgs::new(
        Some(transaction_script),
        Some(note_args_map),
        tx_context.tx_args().advice_inputs().clone().map,
    );

    tx_context.set_tx_args(tx_args);

    let executed_transaction = tx_context.execute().unwrap();

    println!("Executed transaction: {:#?}", executed_transaction);
}
