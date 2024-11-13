use std::{hash::Hash, mem::replace};

use miden_client::{
    accounts::AccountId,
    assets::{Asset, FungibleAsset},
    crypto::FeltRng,
    notes::{
        build_swap_tag, Note, NoteAssets, NoteError, NoteExecutionHint, NoteExecutionMode,
        NoteInputs, NoteMetadata, NoteRecipient, NoteScript, NoteTag, NoteType,
    },
    transactions::{TransactionRequest, TransactionRequestError},
    Felt, Word,
};
use miden_lib::transaction::TransactionKernel;
use miden_objects::transaction::OutputNote;
use rand::{seq::SliceRandom, Rng};
use miden_client::ZERO;

pub fn create_partial_swap_notes_transaction_request(
    num_notes: u8,
    sender: AccountId,
    offering_faucet: AccountId,
    total_asset_offering: u64,
    requesting_faucet: AccountId,
    total_asset_requesting: u64,
    felt_rng: &mut impl FeltRng,
) -> Result<TransactionRequest, TransactionRequestError> {
    // Setup note args
    let mut own_output_notes = vec![];

    let note_type = NoteType::Public;
    let offering_distribution =
        generate_random_distribution(num_notes as usize, total_asset_offering);
    let requesting_distribution =
        generate_random_distribution(num_notes as usize, total_asset_requesting);

    for i in 0..num_notes {
        let offered_asset = Asset::Fungible(
            FungibleAsset::new(offering_faucet, offering_distribution[i as usize]).unwrap(),
        );
        let requested_asset = Asset::Fungible(
            FungibleAsset::new(requesting_faucet, requesting_distribution[i as usize]).unwrap(),
        );

        let swapp_note = create_swapp_note(
            sender,
            sender,
            offered_asset,
            requested_asset,
            note_type,
            Felt::new(0),
            felt_rng,
        )?;

        own_output_notes.push(OutputNote::Full(swapp_note));
    }

    TransactionRequest::new().with_own_output_notes(own_output_notes)
}

pub fn create_swapp_note<R: FeltRng>(
    sender: AccountId,
    payback_receiver: AccountId,
    offered_asset: Asset,
    requested_asset: Asset,
    note_type: NoteType,
    aux: Felt,
    _rng: &mut R,
) -> Result<Note, NoteError> {
    // build the tag for the SWAPP use case
    let swapp_tag = build_swap_tag(note_type, &offered_asset, &requested_asset)?;

    let payback_serial_num = [Felt::new(4), Felt::new(3), Felt::new(2), Felt::new(1)];
    let payback_recipient = build_p2id_recipient(payback_receiver, payback_serial_num)?;

    let payback_recipient_word: Word = payback_recipient.digest().into();
    let requested_asset_word: Word = requested_asset.into();
    let payback_tag = NoteTag::from_account_id(payback_receiver, NoteExecutionMode::Local)?;

    let assembler = TransactionKernel::assembler();
    let note_code = include_str!("scripts/SWAPP.masm");
    let note_script = NoteScript::compile(note_code, assembler).unwrap();
    let note_script_hash = note_script.hash();

    let inputs = NoteInputs::new(vec![
        payback_recipient_word[0],
        payback_recipient_word[1],
        payback_recipient_word[2],
        payback_recipient_word[3],
        requested_asset_word[0],
        requested_asset_word[1],
        requested_asset_word[2],
        requested_asset_word[3],
        payback_tag.inner().into(),
        NoteExecutionHint::always().into(),
        swapp_tag.inner().into(),
        ZERO,
        note_script_hash[0],
        note_script_hash[1],
        note_script_hash[2],
        note_script_hash[3],
    ]).unwrap();

    let serial_num = [Felt::new(1), Felt::new(2), Felt::new(3), Felt::new(4)];

    // build the outgoing note
    let metadata = NoteMetadata::new(sender, note_type, swapp_tag, NoteExecutionHint::always(), aux)?;
    let assets = NoteAssets::new(vec![offered_asset])?;
    let recipient =  NoteRecipient::new(serial_num, note_script, inputs);
    let note = Note::new(assets, metadata, recipient);

    Ok(note)
}

// HELPERS

fn build_p2id_recipient(target: AccountId, serial_num: Word) -> Result<NoteRecipient, NoteError> {
    let assembler = TransactionKernel::assembler();
    let note_code = include_str!("scripts/SWAPP.masm");
    let note_script = NoteScript::compile(note_code, assembler).unwrap();
    let note_inputs = NoteInputs::new(vec![target.into()])?;

    Ok(NoteRecipient::new(serial_num, note_script, note_inputs))
}

fn generate_random_distribution(n: usize, total: u64) -> Vec<u64> {
    if total < n as u64 {
        panic!("Total must at least be equal to n to make sure that all values are non-zero.")
    }

    let mut rng = rand::thread_rng();
    let mut result = Vec::with_capacity(n);
    let mut remaining = total;

    // Generate n-1 random numbers
    for _ in 0..n - 1 {
        if remaining == 0 {
            result.push(1); // Ensure non-zero
            continue;
        }

        let max = remaining.saturating_sub(n as u64 - result.len() as u64 - 1);
        let value = if max > 1 {
            rng.gen_range(1..=(total / n as u64))
        } else {
            1
        };

        result.push(value);
        remaining -= value;
    }

    // Add the last number to make the sum equal to total
    result.push(remaining.max(1));

    // Shuffle the vector to randomize the order
    result.shuffle(&mut rng);

    result
}
