use miden_lib::transaction::TransactionKernel;
use miden_objects::{
    accounts::AccountId,
    assets::Asset,
    crypto::utils::Serializable,
    notes::{
        Note, NoteAssets, NoteExecutionHint, NoteExecutionMode, NoteInputs, NoteMetadata,
        NoteRecipient, NoteScript, NoteTag, NoteType,
    },
    transaction::{ExecutedTransaction, ProvenTransaction},
    Felt, NoteError, Word,
};
use miden_prover::ProvingOptions;
use miden_tx::{TransactionProver, TransactionVerifier, TransactionVerifierError};
use miden_vm::Assembler;
use vm_processor::utils::Deserializable;

use miden_client::transactions::build_swap_tag;

use miden_objects::Hasher;
// HELPER FUNCTIONS
// ================================================================================================

#[cfg(test)]
pub fn prove_and_verify_transaction(
    executed_transaction: ExecutedTransaction,
) -> Result<(), TransactionVerifierError> {
    let executed_transaction_id = executed_transaction.id();
    // Prove the transaction

    let proof_options = ProvingOptions::default();
    let prover = TransactionProver::new(proof_options);
    let proven_transaction = prover.prove_transaction(executed_transaction).unwrap();

    assert_eq!(proven_transaction.id(), executed_transaction_id);

    // Serialize & deserialize the ProvenTransaction
    let serialised_transaction = proven_transaction.to_bytes();
    let proven_transaction = ProvenTransaction::read_from_bytes(&serialised_transaction).unwrap();

    // Verify that the generated proof is valid
    let verifier = TransactionVerifier::new(miden_objects::MIN_PROOF_SECURITY_LEVEL);

    verifier.verify(proven_transaction)
}

pub fn create_p2id_note(
    sender: AccountId,
    target: AccountId,
    assets: Vec<Asset>,
    note_type: NoteType,
    aux: Felt,
    serial_num: [Felt; 4],
) -> Result<Note, NoteError> {
    let assembler: Assembler = TransactionKernel::assembler_testing().with_debug_mode(true);
    let note_code = include_str!("../notes/P2ID.masm");

    let note_script = NoteScript::compile(note_code, assembler).unwrap();

    let inputs = NoteInputs::new(vec![target.into()])?;
    let tag = NoteTag::from_account_id(target, NoteExecutionMode::Local)?;

    let metadata = NoteMetadata::new(sender, note_type, tag, NoteExecutionHint::always(), aux)?;
    let vault = NoteAssets::new(assets)?;
    let recipient = NoteRecipient::new(serial_num, note_script, inputs);
    Ok(Note::new(vault, metadata, recipient))
}

/// Generates a SWAP note - swap of assets between two accounts - and returns the note as well as
/// [NoteDetails] for the payback note.
///
/// This script enables a swap of 2 assets between the `sender` account and any other account that
/// is willing to consume the note. The consumer will receive the `offered_asset` and will create a
/// new P2ID note with `sender` as target, containing the `requested_asset`.
///
/// # Errors
/// Returns an error if deserialization or compilation of the `SWAP` script fails.
pub fn create_partial_swap_note(
    creator: AccountId,
    last_consumer: AccountId,
    offered_asset: Asset,
    requested_asset: Asset,
    swap_serial_num: [Felt; 4],
    fill_number: u64,
) -> Result<Note, NoteError> {
    let assembler: Assembler = TransactionKernel::assembler_testing();

    let note_code = include_str!("../notes/SWAPp.masm");
    let note_script = NoteScript::compile(note_code, assembler).unwrap();
    let note_type = NoteType::Public;

    let requested_asset_word: Word = requested_asset.into();
    let tag = build_swap_tag(
        note_type,
        offered_asset.faucet_id(),
        requested_asset.faucet_id(),
    )?;

    let inputs = NoteInputs::new(vec![
        requested_asset_word[0],
        requested_asset_word[1],
        requested_asset_word[2],
        requested_asset_word[3],
        tag.inner().into(),
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
        Felt::new(fill_number),
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
        creator.into(),
    ])?;

    let aux = Felt::new(0);

    // build the outgoing note
    let metadata = NoteMetadata::new(
        last_consumer,
        note_type,
        tag,
        NoteExecutionHint::always(),
        aux,
    )?;

    let assets = NoteAssets::new(vec![offered_asset])?;
    let recipient = NoteRecipient::new(swap_serial_num, note_script.clone(), inputs.clone());
    let note = Note::new(assets.clone(), metadata, recipient.clone());

    Ok(note)
}

pub fn compute_p2id_serial_num(swap_serial_num: [Felt; 4], swap_count: u64) -> [Felt; 4] {
    let swap_count_word = [
        Felt::new(swap_count),
        Felt::new(0),
        Felt::new(0),
        Felt::new(0),
    ];
    let p2id_serial_num = Hasher::merge(&[swap_serial_num.into(), swap_count_word.into()]);

    p2id_serial_num.into()
}
