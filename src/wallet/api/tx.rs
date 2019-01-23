use uuid::Uuid;

use super::types::{WalletBackend, Context, TxLogEntryType, Error, ErrorKind, NodeClient, Keychain, Identifier, Slate, Transaction};
use super::selection;
use super::updater;

use crate::wallet::types::TxProof;

pub fn receive_tx<T: ?Sized, C, K>(
    wallet: &mut T,
    address: Option<String>,
    slate: &mut Slate,
    parent_key_id: &Identifier,
    message: Option<String>,
) -> Result<(), Error>
    where
        T: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    // create an output using the amount in the slate
    let (_, mut context, receiver_create_fn) =
        selection::build_recipient_output_with_slate(wallet, address, slate, parent_key_id.clone())?;

    // fill public keys
    let _ = slate.fill_round_1(
        wallet.keychain(),
        &mut context.sec_key,
        &context.sec_nonce,
        1,
        message,
    )?;

    // perform partial sig
    let _ = slate.fill_round_2(wallet.keychain(), &context.sec_key, &context.sec_nonce, 1)?;

    // Save output in wallet
    let _ = receiver_create_fn(wallet);

    Ok(())
}

pub fn create_send_tx<T: ?Sized, C, K>(
    wallet: &mut T,
    address: Option<String>,
    amount: u64,
    minimum_confirmations: u64,
    max_outputs: usize,
    num_change_outputs: usize,
    selection_strategy_is_use_all: bool,
    parent_key_id: &Identifier,
    message: Option<String>,
) -> Result<
    (
        Slate,
        Context,
        impl FnOnce(&mut T, &Transaction) -> Result<(), Error>,
    ),
    Error,
>
    where
        T: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    // Get lock height
    let current_height = wallet.w2n_client().get_chain_height()?;
    // ensure outputs we're selecting are up to date
    updater::refresh_outputs(wallet, parent_key_id, false)?;

    let lock_height = current_height;

    // Sender selects outputs into a new slate and save our corresponding keys in
    // a transaction context. The secret key in our transaction context will be
    // randomly selected. This returns the public slate, and a closure that locks
    // our inputs and outputs once we're convinced the transaction exchange went
    // according to plan
    // This function is just a big helper to do all of that, in theory
    // this process can be split up in any way
    let (mut slate, mut context, sender_lock_fn) = selection::build_send_tx_slate(
        wallet,
        address,
        2,
        amount,
        current_height,
        minimum_confirmations,
        lock_height,
        max_outputs,
        num_change_outputs,
        selection_strategy_is_use_all,
        parent_key_id.clone(),
    )?;

    // Generate a kernel offset and subtract from our context's secret key. Store
    // the offset in the slate's transaction kernel, and adds our public key
    // information to the slate
    let _ = slate.fill_round_1(
        wallet.keychain(),
        &mut context.sec_key,
        &context.sec_nonce,
        0,
        message,
    )?;

    Ok((slate, context, sender_lock_fn))
}

pub fn complete_tx<T: ?Sized, C, K>(
    wallet: &mut T,
    slate: &mut Slate,
    context: &Context,
) -> Result<(), Error>
    where
        T: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    let _ = slate.fill_round_2(wallet.keychain(), &context.sec_key, &context.sec_nonce, 0)?;
    // Final transaction can be built by anyone at this stage
    let res = slate.finalize(wallet.keychain());
    if let Err(e) = res {
        Err(ErrorKind::LibTX(e.kind()))?
    }
    Ok(())
}

pub fn cancel_tx<T: ?Sized, C, K>(
    wallet: &mut T,
    parent_key_id: &Identifier,
    tx_id: Option<u32>,
    tx_slate_id: Option<Uuid>,
) -> Result<(), Error>
    where
        T: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    let mut tx_id_string = String::new();
    if let Some(tx_id) = tx_id {
        tx_id_string = tx_id.to_string();
    } else if let Some(tx_slate_id) = tx_slate_id {
        tx_id_string = tx_slate_id.to_string();
    }
    let tx_vec = updater::retrieve_txs(wallet, tx_id, tx_slate_id, Some(&parent_key_id), false)?;
    if tx_vec.len() != 1 {
        return Err(ErrorKind::TransactionDoesntExist(tx_id_string))?;
    }
    let tx = tx_vec[0].clone();
    if tx.tx_type != TxLogEntryType::TxSent && tx.tx_type != TxLogEntryType::TxReceived {
        return Err(ErrorKind::TransactionNotCancellable(tx_id_string))?;
    }
    if tx.confirmed == true {
        return Err(ErrorKind::TransactionNotCancellable(tx_id_string))?;
    }
    // get outputs associated with tx
    let res = updater::retrieve_outputs(wallet, false, Some(tx.id), Some(&parent_key_id))?;
    let outputs = res.iter().map(|(out, _)| out).cloned().collect();
    updater::cancel_tx_and_outputs(wallet, tx, outputs)?;
    Ok(())
}

pub fn update_stored_tx<T: ?Sized, C, K>(wallet: &mut T, slate: &Slate, tx_proof: Option<&mut TxProof>) -> Result<(), Error>
    where
        T: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    // finalize command
    let tx_vec = updater::retrieve_txs(wallet, None, Some(slate.id), None, false)?;
    let mut tx = None;
    // don't want to assume this is the right tx, in case of self-sending
    for t in tx_vec {
        if t.tx_type == TxLogEntryType::TxSent {
            tx = Some(t.clone());
            break;
        }
    };

    if tx.is_none() {
        return Err(ErrorKind::TransactionDoesntExist(slate.id.to_string()).into());
    }

    let mut batch = wallet.batch()?;
    batch.store_tx(&slate.id.to_string(), &slate.tx)?;
    if tx_proof.is_some() {
        batch.store_tx_proof(&slate.id.to_string(), tx_proof.unwrap())?;
    }
    batch.commit()?;

    Ok(())
}

pub fn invoice_tx<T: ?Sized, C, K>(
    wallet: &mut T,
    slate: &mut Slate,
    minimum_confirmations: u64,
    max_outputs: usize,
    num_change_outputs: usize,
    selection_strategy_is_use_all: bool,
    parent_key_id: Identifier,
    message: Option<String>,
) -> Result<(
impl FnOnce(&mut T, &Transaction) -> Result<(), Error>
), Error>
    where
        T: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    updater::refresh_outputs(wallet, &parent_key_id, false)?;

    let (mut context, update_sender_wallet_fn) = selection::build_recipient_input_with_slate(
        wallet,
        slate,
        minimum_confirmations,
        max_outputs,
        num_change_outputs,
        selection_strategy_is_use_all,
        parent_key_id,
    )?;

    slate.fill_round_1(
        wallet.keychain(),
        &mut context.sec_key,
        &context.sec_nonce,
        1,
        message,
    )?;

    slate.fill_round_2(wallet.keychain(), &context.sec_key, &context.sec_nonce, 1)?;

    Ok(update_sender_wallet_fn)
}

pub fn create_receive_tx<T: ?Sized, C, K>(
    wallet: &mut T,
    amount: u64,
    num_outputs: usize,
    parent_key_id: &Identifier,
    message: Option<String>,
) -> Result<
    (
        Slate,
        Context,
        impl FnOnce(&mut T, &Transaction) -> Result<(), Error>,
    ),
    Error,
>
    where
        T: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    // Get lock height
    let current_height = wallet.w2n_client().get_chain_height()?;
    let lock_height = current_height;

    let (mut slate, mut context, add_fn) = selection::build_receive_tx_slate(
        wallet,
        2,
        amount,
        num_outputs,
        current_height,
        lock_height,
        parent_key_id.clone(),
    )?;

    let _ = slate.fill_round_1(
        wallet.keychain(),
        &mut context.sec_key,
        &context.sec_nonce,
        0,
        message,
    )?;

    Ok((slate, context, add_fn))
}
