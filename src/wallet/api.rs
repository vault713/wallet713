use std::marker::PhantomData;
use std::sync::Arc;
use uuid::Uuid;

use grin_util::Mutex;
use grin_wallet::libwallet::internal::{keys, tx, updater, selection};
use grin_wallet::libwallet::types::{
    OutputStatus, Context, AcctPathMapping, BlockFees, CbData, NodeClient, OutputData, TxLogEntry, TxLogEntryType, TxWrapper, WalletBackend, WalletInfo,
};
use grin_wallet::libwallet::{Error, ErrorKind};
use grin_core::core::{Input as TxInput, OutputFeatures, Transaction, TxKernel};
use grin_keychain::{BlindSum, BlindingFactor, Keychain, Identifier};
use grin_core::libtx::slate::Slate;
use grin_core::libtx::{tx_fee, build};

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
        // Don't do this multiple times
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

    // refresh outputs
    updater::refresh_outputs(wallet, &parent_key_id)?;

    let lock_height = slate.lock_height;
    let amount = slate.amount;

    let (elems, inputs, change_amounts_derivations, amount, fee) = selection::select_send_tx(
        wallet,
        amount,
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

    // Create our own private context
    let mut context = Context::new(
        wallet.keychain().secp(),
        blinding.secret_key(&keychain.secp()).unwrap(),
    );

    // Store our private identifiers for each input
    for input in inputs {
        context.add_input(&input.key_id);
    }

    // Store change output(s)
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
            t.tx_hex = Some(filename);
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

            // write the output representing our change
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

    Ok((update_sender_wallet_fn))
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
        minimum_confirmations: u64,
        max_outputs: usize,
        num_change_outputs: usize,
        selection_strategy_is_use_all: bool,
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
            minimum_confirmations,
            max_outputs,
            num_change_outputs,
            selection_strategy_is_use_all,
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

    /// Receive a transaction from a sender
    pub fn receive_invoice_tx(
        &mut self,
        slate: &mut Slate,
        dest_acct_name: Option<&str>,
        message: Option<String>,
    ) -> Result<(), Error> {
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
        // Don't do this multiple times
        let tx = updater::retrieve_txs(&mut *w, None, Some(slate.id), Some(&parent_key_id))?;
        for t in &tx {
            if t.tx_type == TxLogEntryType::TxSent {
                return Err(ErrorKind::TransactionAlreadyReceived(slate.id.to_string()).into());
            }
        }
        let res = tx::receive_tx(&mut *w, slate, &parent_key_id, message);
        w.close()?;

        if let Err(e) = res {
            Err(e)
        } else {
            Ok(())
        }
    }
}

/// Issue a new transaction to the provided sender by spending some of our
/// wallet
fn create_receive_tx<T: ?Sized, C, K>(
    wallet: &mut T,
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
///    updater::refresh_outputs(wallet, parent_key_id)?;

    let lock_height = current_height;

    // Sender selects outputs into a new slate and save our corresponding keys in
    // a transaction context. The secret key in our transaction context will be
    // randomly selected. This returns the public slate, and a closure that locks
    // our inputs and outputs once we're convinced the transaction exchange went
    // according to plan
    // This function is just a big helper to do all of that, in theory
    // this process can be split up in any way
    let (mut slate, mut context, add_fn) = build_receive_tx_slate(
        wallet,
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

    Ok((slate, context, add_fn))
}

/// Initialize a transaction on the sender side, returns a corresponding
/// libwallet transaction slate with the appropriate inputs selected,
/// and saves the private wallet identifiers of our selected outputs
/// into our transaction context

fn build_receive_tx_slate<T: ?Sized, C, K>(
    wallet: &mut T,
    num_participants: usize,
    amount: u64,
    current_height: u64,
    minimum_confirmations: u64,
    lock_height: u64,
    max_outputs: usize,
    change_outputs: usize,
    selection_strategy_is_use_all: bool,
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
    // Create a potential output for this transaction
    let key_id = keys::next_available_key(wallet).unwrap();

    // create a slate
    let mut slate = Slate::blank(num_participants);
    slate.amount = amount;
    slate.height = current_height;
    slate.lock_height = lock_height;

    let keychain = wallet.keychain().clone();
    let blinding =
        slate.add_transaction_elements(&keychain, vec![build::output(amount, key_id.clone())])?;

    // Add blinding sum to our context
    let mut context = Context::new(
        keychain.secp(),
        blinding
            .secret_key(wallet.keychain().clone().secp())
            .unwrap(),
    );

    context.add_output(&key_id);

    // Create closure that adds the output to recipient's wallet
    // (up to the caller to decide when to do)
    let slate_id = slate.id.clone();
    let key_id_inner = key_id.clone();
    let wallet_add_fn = move |wallet: &mut T, tx: &Transaction| {
        let tx_log_entry = {
            let mut batch = wallet.batch()?;
            let log_id = batch.next_tx_log_id(&parent_key_id)?;
            let mut t = TxLogEntry::new(parent_key_id.clone(), TxLogEntryType::TxSent, log_id);
            t.tx_slate_id = Some(slate_id);
            t.amount_credited = amount;
            t.num_outputs = 1;
            batch.save(OutputData {
                root_key_id: parent_key_id.clone(),
                key_id: key_id_inner.clone(),
                n_child: key_id_inner.to_path().last_path_index(),
                value: amount,
                status: OutputStatus::Unconfirmed,
                height: current_height,
                lock_height: 0,
                is_coinbase: false,
                tx_log_entry: Some(log_id),
            })?;
            batch.save_tx_log_entry(t.clone(), &parent_key_id)?;
            batch.commit()?;
            t
        };
        wallet.store_tx(&format!("{}", tx_log_entry.tx_slate_id.unwrap()), tx)?;
        Ok(())
    };
    Ok((slate, context, wallet_add_fn))
}
