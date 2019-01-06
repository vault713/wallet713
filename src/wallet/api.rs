use std::marker::PhantomData;
use std::sync::Arc;

use grin_util::Mutex;
use grin_wallet::libwallet::internal::{keys, updater, selection};
use grin_wallet::libwallet::types::{
    OutputStatus, Context, NodeClient, OutputData, TxLogEntry, TxLogEntryType, WalletBackend,
};
use grin_wallet::libwallet::{Error, ErrorKind};
use grin_core::core::Transaction;
use grin_core::core::amount_to_hr_string;
use grin_keychain::{Keychain, Identifier};
use grin_core::libtx::slate::Slate;
use grin_core::libtx::{build, tx_fee};

pub struct Wallet713OwnerAPI<W: ?Sized, C, K>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    pub wallet: Arc<Mutex<W>>,
    phantom: PhantomData<K>,
    phantom_c: PhantomData<C>,
}

pub struct Wallet713ForeignAPI<W: ?Sized, C, K>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    pub wallet: Arc<Mutex<W>>,
    phantom: PhantomData<K>,
    phantom_c: PhantomData<C>,
}

impl<W: ?Sized, C, K> Wallet713OwnerAPI<W, C, K>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    pub fn new(wallet_in: Arc<Mutex<W>>) -> Self {
        Self {
            wallet: wallet_in,
            phantom: PhantomData,
            phantom_c: PhantomData,
        }
    }

    pub fn invoice_tx(
        &mut self,
        dest_acct_name: Option<&str>,
        slate: &mut Slate,
        minimum_confirmations: u64,
        max_outputs: usize,
        num_change_outputs: usize,
        selection_strategy_is_use_all: bool,
        message: Option<String>,
    ) -> Result<(
        impl FnOnce(&mut W, &Transaction) -> Result<(), Error>
    ), Error> {
        let mut w = self.wallet.lock();
        w.open_with_credentials()?;
        let parent_key_id = match dest_acct_name {
            Some(d) => {
                let pm = w.get_acct_path(d.to_owned())?;
                match pm {
                    Some(p) => p.path,
                    None => w.parent_key_id(),
                }
            }
            None => w.parent_key_id(),
        };

        let tx = updater::retrieve_txs(&mut *w, None, Some(slate.id), Some(&parent_key_id))?;
        for t in &tx {
            if t.tx_type == TxLogEntryType::TxReceived {
                return Err(ErrorKind::TransactionAlreadyReceived(slate.id.to_string()).into());
            }
        }

        let res = invoice_tx(&mut *w, slate, minimum_confirmations, max_outputs, num_change_outputs, selection_strategy_is_use_all, parent_key_id.clone(), message);
        w.close()?;
        res
    }
}

fn invoice_tx<T: ?Sized, C, K>(
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
    let current_height = wallet.w2n_client().get_chain_height()?;

    updater::refresh_outputs(wallet, &parent_key_id)?;

    let lock_height = slate.lock_height;
    let amount = slate.amount;
    let num_outputs = slate.tx.outputs().len();

    let (elems, inputs, change_amounts_derivations, _amount, fee) = select_send_tx(
        wallet,
        amount,
        num_outputs,
        current_height,
        minimum_confirmations,
        lock_height,
        max_outputs,
        num_change_outputs,
        selection_strategy_is_use_all,
        &parent_key_id,
    )?;

    slate.fee = fee;
    let slate_id = slate.id.clone();

    let keychain = wallet.keychain().clone();

    let blinding = slate.add_transaction_elements(&keychain, elems)?;

    let mut context = Context::new(
        wallet.keychain().secp(),
        blinding.secret_key(&keychain.secp()).unwrap(),
    );

    for input in inputs {
        context.add_input(&input.key_id);
    }

    for (_, id) in &change_amounts_derivations {
        context.add_output(&id);
    }

    let lock_inputs = context.get_inputs().clone();
    let _lock_outputs = context.get_outputs().clone();

    let update_sender_wallet_fn = move |wallet: &mut T, tx: &Transaction| {
        let tx_entry = {
            let mut batch = wallet.batch()?;
            let log_id = batch.next_tx_log_id(&parent_key_id)?;
            let mut t = TxLogEntry::new(parent_key_id.clone(), TxLogEntryType::TxReceived, log_id);
            t.tx_slate_id = Some(slate_id);
            let filename = format!("{}.grintx", slate_id);
            t.stored_tx = Some(filename);
            t.fee = Some(fee);
            let mut amount_debited = 0;
            t.num_inputs = lock_inputs.len();
            for id in lock_inputs {
                let mut coin = batch.get(&id).unwrap();
                coin.tx_log_entry = Some(log_id);
                amount_debited = amount_debited + coin.value;
                batch.lock_output(&mut coin)?;
            }

            t.amount_debited = amount_debited;

            for (change_amount, id) in &change_amounts_derivations {
                t.num_outputs += 1;
                t.amount_credited += change_amount;
                batch.save(OutputData {
                    root_key_id: parent_key_id.clone(),
                    key_id: id.clone(),
                    n_child: id.to_path().last_path_index(),
                    value: change_amount.clone(),
                    status: OutputStatus::Unconfirmed,
                    height: current_height,
                    lock_height: 0,
                    is_coinbase: false,
                    tx_log_entry: Some(log_id),
                })?;
            }
            batch.save_tx_log_entry(t.clone(), &parent_key_id)?;
            batch.commit()?;
            t
        };
        wallet.store_tx(&format!("{}", tx_entry.tx_slate_id.unwrap()), tx)?;
        Ok(())
    };

    let _ = slate.fill_round_1(
        wallet.keychain(),
        &mut context.sec_key,
        &context.sec_nonce,
        1,
        message,
    )?;

    let _ = slate.fill_round_2(wallet.keychain(), &context.sec_key, &context.sec_nonce, 1)?;

    Ok(update_sender_wallet_fn)
}

impl<W: ?Sized, C, K> Wallet713ForeignAPI<W, C, K>
    where
        W: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    pub fn new(wallet_in: Arc<Mutex<W>>) -> Self {
        Self {
            wallet: wallet_in,
            phantom: PhantomData,
            phantom_c: PhantomData,
        }
    }

    pub fn initiate_receive_tx(
        &mut self,
        src_acct_name: Option<&str>,
        amount: u64,
        num_outputs: usize,
        message: Option<String>,
    ) -> Result<
        (
            Slate,
            impl FnOnce(&mut W, &Transaction) -> Result<(), Error>,
        ),
        Error,
    > {
        let mut w = self.wallet.lock();
        w.open_with_credentials()?;
        let parent_key_id = match src_acct_name {
            Some(d) => {
                let pm = w.get_acct_path(d.to_owned())?;
                match pm {
                    Some(p) => p.path,
                    None => w.parent_key_id(),
                }
            }
            None => w.parent_key_id(),
        };

        let (slate, context, add_fn) = create_receive_tx(
            &mut *w,
            amount,
            num_outputs,
            &parent_key_id,
            message,
        )?;

        {
            let mut batch = w.batch()?;
            batch.save_private_context(slate.id.as_bytes(), &context)?;
            batch.commit()?;
        }

        w.close()?;
        Ok((slate, add_fn))
    }

    pub fn tx_add_outputs(
        &mut self,
        slate: &Slate,
        add_fn: impl FnOnce(&mut W, &Transaction) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let mut w = self.wallet.lock();
        w.open_with_credentials()?;
        add_fn(&mut *w, &slate.tx)?;
        Ok(())
    }
}

fn create_receive_tx<T: ?Sized, C, K>(
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

    let (mut slate, mut context, add_fn) = build_receive_tx_slate(
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

fn build_receive_tx_slate<T: ?Sized, C, K>(
    wallet: &mut T,
    num_participants: usize,
    amount: u64,
    num_outputs: usize,
    current_height: u64,
    lock_height: u64,
    parent_key_id: Identifier,
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
    let mut slate = Slate::blank(num_participants);
    slate.amount = amount;
    slate.height = current_height;
    slate.lock_height = lock_height;

    let mut elems = vec![];
    let mut key_ids_and_amounts = vec![];

    let mut remaining_amount = amount;
    for i in 0..num_outputs {
        let key_id = keys::next_available_key(wallet).unwrap();
        let output_amount: u64 = if i == num_outputs - 1 {
            remaining_amount
        } else {
            amount / (num_outputs as u64)
        };
        if output_amount > 0 {
            key_ids_and_amounts.push((key_id.clone(), output_amount));
            elems.push(build::output(output_amount, key_id.clone()));
            remaining_amount -= output_amount;
        }
    }

    let keychain = wallet.keychain().clone();
    let blinding =
        slate.add_transaction_elements(&keychain, elems)?;

    let mut context = Context::new(
        keychain.secp(),
        blinding
            .secret_key(wallet.keychain().clone().secp())
            .unwrap(),
    );

    let key_ids_and_amounts_inner = key_ids_and_amounts.clone();

    for (key_id, _) in key_ids_and_amounts {
        context.add_output(&key_id.clone());
    }

    let slate_id = slate.id.clone();
    let wallet_add_fn = move |wallet: &mut T, tx: &Transaction| {
        let tx_log_entry = {
            let mut batch = wallet.batch()?;
            let log_id = batch.next_tx_log_id(&parent_key_id)?;
            let mut t = TxLogEntry::new(parent_key_id.clone(), TxLogEntryType::TxSent, log_id);
            let filename = format!("{}.grintx", slate_id);
            t.stored_tx = Some(filename);
            t.tx_slate_id = Some(slate_id);
            t.amount_credited = amount;
            t.num_outputs = num_outputs;
            for (key_id, amount) in key_ids_and_amounts_inner {
                batch.save(OutputData {
                    root_key_id: parent_key_id.clone(),
                    key_id: key_id.clone(),
                    n_child: key_id.to_path().last_path_index(),
                    value: amount,
                    status: OutputStatus::Unconfirmed,
                    height: current_height,
                    lock_height: 0,
                    is_coinbase: false,
                    tx_log_entry: Some(log_id),
                })?;
            }
            batch.save_tx_log_entry(t.clone(), &parent_key_id)?;
            batch.commit()?;
            t
        };
        wallet.store_tx(&format!("{}", tx_log_entry.tx_slate_id.unwrap()), tx)?;
        Ok(())
    };
    Ok((slate, context, wallet_add_fn))
}


/// Builds a transaction to send to someone from the HD seed associated with the
/// wallet and the amount to send. Handles reading through the wallet data file,
/// selecting outputs to spend and building the change.
fn select_send_tx<T: ?Sized, C, K>(
    wallet: &mut T,
    amount: u64,
    num_outputs: usize,
    current_height: u64,
    minimum_confirmations: u64,
    lock_height: u64,
    max_outputs: usize,
    change_outputs: usize,
    selection_strategy_is_use_all: bool,
    parent_key_id: &Identifier,
) -> Result<
    (
        Vec<Box<build::Append<K>>>,
        Vec<OutputData>,
        Vec<(u64, Identifier)>, // change amounts and derivations
        u64,                    // amount
        u64,                    // fee
    ),
    Error,
>
    where
        T: WalletBackend<C, K>,
        C: NodeClient,
        K: Keychain,
{
    // select some spendable coins from the wallet
    let (max_outputs, coins) = selection::select_coins(
        wallet,
        amount,
        current_height,
        minimum_confirmations,
        max_outputs,
        selection_strategy_is_use_all,
        parent_key_id,
    );

    // sender is responsible for setting the fee on the partial tx
    // recipient should double check the fee calculation and not blindly trust the
    // sender

    // TODO - Is it safe to spend without a change output? (1 input -> 1 output)
    // TODO - Does this not potentially reveal the senders private key?
    //
    // First attempt to spend without change
    let mut fee = tx_fee(coins.len(), num_outputs, 1, None);
    let mut total: u64 = coins.iter().map(|c| c.value).sum();
    let mut amount_with_fee = amount + fee;

    if total == 0 {
        return Err(ErrorKind::NotEnoughFunds {
            available: 0,
            available_disp: amount_to_hr_string(0, false),
            needed: amount_with_fee as u64,
            needed_disp: amount_to_hr_string(amount_with_fee as u64, false),
        })?;
    }

    // The amount with fee is more than the total values of our max outputs
    if total < amount_with_fee && coins.len() == max_outputs {
        return Err(ErrorKind::NotEnoughFunds {
            available: total,
            available_disp: amount_to_hr_string(total, false),
            needed: amount_with_fee as u64,
            needed_disp: amount_to_hr_string(amount_with_fee as u64, false),
        })?;
    }

    let num_outputs = change_outputs + num_outputs;

    // We need to add a change address or amount with fee is more than total
    if total != amount_with_fee {
        fee = tx_fee(coins.len(), num_outputs, 1, None);
        amount_with_fee = amount + fee;

        // Here check if we have enough outputs for the amount including fee otherwise
        // look for other outputs and check again
        while total < amount_with_fee {
            // End the loop if we have selected all the outputs and still not enough funds
            if coins.len() == max_outputs {
                return Err(ErrorKind::NotEnoughFunds {
                    available: total as u64,
                    available_disp: amount_to_hr_string(total as u64, false),
                    needed: amount_with_fee as u64,
                    needed_disp: amount_to_hr_string(amount_with_fee as u64, false),
                })?;
            }

            // select some spendable coins from the wallet
            let (_, coins) = selection::select_coins(
                wallet,
                amount_with_fee,
                current_height,
                minimum_confirmations,
                max_outputs,
                selection_strategy_is_use_all,
                parent_key_id,
            );
            fee = tx_fee(coins.len(), num_outputs, 1, None);
            total = coins.iter().map(|c| c.value).sum();
            amount_with_fee = amount + fee;
        }
    }

    // build transaction skeleton with inputs and change
    let (mut parts, change_amounts_derivations) =
        selection::inputs_and_change(&coins, wallet, amount, fee, change_outputs)?;

    // This is more proof of concept than anything but here we set lock_height
    // on tx being sent (based on current chain height via api).
    parts.push(build::with_lock_height(lock_height));

    Ok((parts, coins, change_amounts_derivations, amount, fee))
}