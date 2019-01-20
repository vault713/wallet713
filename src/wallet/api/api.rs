use std::marker::PhantomData;
use std::sync::Arc;
use uuid::Uuid;

use grin_core::ser;
use grin_util::Mutex;
use grin_util::secp::{pedersen, ContextFlag, Secp256k1};

use super::types::{Transaction, Slate, Keychain, Identifier, NodeClient, TxWrapper, WalletBackend, AcctPathMapping, OutputData, TxLogEntry, TxLogEntryType, WalletInfo, ContextType, Error, ErrorKind};
use super::tx;
use super::keys;
use super::updater;

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
        let parent_key_id = w.get_parent_key_id();
        let tx = updater::retrieve_txs(&mut *w, None, Some(slate.id), Some(&parent_key_id), false)?;
        for t in &tx {
            if t.tx_type == TxLogEntryType::TxReceived {
                return Err(ErrorKind::TransactionAlreadyReceived(slate.id.to_string()).into());
            }
        }

        let res = tx::invoice_tx(&mut *w, slate, minimum_confirmations, max_outputs, num_change_outputs, selection_strategy_is_use_all, parent_key_id.clone(), message);
        w.close()?;
        res
    }

    pub fn accounts(&self) -> Result<Vec<AcctPathMapping>, Error> {
        let mut w = self.wallet.lock();
        keys::accounts(&mut *w)
    }

    pub fn create_account_path(&self, label: &str) -> Result<Identifier, Error> {
        let mut w = self.wallet.lock();
        keys::new_acct_path(&mut *w, label)
    }

    pub fn retrieve_outputs(
        &self,
        include_spent: bool,
        refresh_from_node: bool,
        tx_id: Option<u32>,
    ) -> Result<(bool, Vec<(OutputData, pedersen::Commitment)>), Error> {
        let mut w = self.wallet.lock();
        w.open_with_credentials()?;
        let parent_key_id = w.get_parent_key_id();

        let mut validated = false;
        if refresh_from_node {
            validated = self.update_outputs(&mut w, false);
        }

        let res = Ok((
            validated,
            updater::retrieve_outputs(&mut *w, include_spent, tx_id, Some(&parent_key_id))?,
        ));

        w.close()?;
        res
    }

    pub fn retrieve_txs(
        &self,
        refresh_from_node: bool,
        tx_id: Option<u32>,
        tx_slate_id: Option<Uuid>,
    ) -> Result<(bool, Vec<TxLogEntry>), Error> {
        let mut w = self.wallet.lock();
        w.open_with_credentials()?;
        let parent_key_id = w.get_parent_key_id();

        let mut validated = false;
        if refresh_from_node {
            validated = self.update_outputs(&mut w, false);
        }

        let res = Ok((
            validated,
            updater::retrieve_txs(&mut *w, tx_id, tx_slate_id, Some(&parent_key_id), false)?,
        ));

        w.close()?;
        res
    }

    pub fn retrieve_summary_info(
        &mut self,
        refresh_from_node: bool,
        minimum_confirmations: u64,
    ) -> Result<(bool, WalletInfo), Error> {
        let mut w = self.wallet.lock();
        w.open_with_credentials()?;
        let parent_key_id = w.get_parent_key_id();

        let mut validated = false;
        if refresh_from_node {
            validated = self.update_outputs(&mut w, false);
        }

        let wallet_info = updater::retrieve_info(&mut *w, &parent_key_id, minimum_confirmations)?;
        let res = Ok((validated, wallet_info));

        w.close()?;
        res
    }

    pub fn initiate_tx(
        &mut self,
        address: Option<String>,
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
        let parent_key_id = w.get_parent_key_id();
        let (slate, context, lock_fn) = tx::create_send_tx(
            &mut *w,
            address,
            amount,
            minimum_confirmations,
            max_outputs,
            num_change_outputs,
            selection_strategy_is_use_all,
            &parent_key_id,
            message,
        )?;

        // Save the aggsig context in our DB for when we
        // recieve the transaction back
        {
            let mut batch = w.batch()?;
            batch.save_private_context(&slate.id.to_string(), &context)?;
            batch.commit()?;
        }

        w.close()?;
        Ok((slate, lock_fn))
    }

    pub fn tx_lock_outputs(
        &mut self,
        tx: &Transaction,
        lock_fn: impl FnOnce(&mut W, &Transaction) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let mut w = self.wallet.lock();
        w.open_with_credentials()?;
        lock_fn(&mut *w, tx)?;
        Ok(())
    }

    pub fn finalize_tx(&mut self, slate: &mut Slate) -> Result<bool, Error> {
        let context = {
            let mut w = self.wallet.lock();
            w.open_with_credentials()?;
            let context = w.get_private_context(&slate.id.to_string())?;
            w.close()?;
            context
        };

        match context.context_type {
            ContextType::Tx => {
                let mut w = self.wallet.lock();
                w.open_with_credentials()?;
                tx::complete_tx(&mut *w, slate, &context)?;
                tx::update_stored_tx(&mut *w, slate)?;
                {
                    let mut batch = w.batch()?;
                    batch.delete_private_context(&slate.id.to_string())?;
                    batch.commit()?;
                }
                w.close()?;
                Ok(true)
            },
        }
    }

    pub fn cancel_tx(
        &mut self,
        tx_id: Option<u32>,
        tx_slate_id: Option<Uuid>,
    ) -> Result<(), Error> {
        let mut w = self.wallet.lock();
        w.open_with_credentials()?;
        let parent_key_id = w.get_parent_key_id();
        if !self.update_outputs(&mut w, false) {
            return Err(ErrorKind::TransactionCancellationError(
                "Can't contact running Grin node. Not Cancelling.",
            ))?;
        }
        tx::cancel_tx(&mut *w, &parent_key_id, tx_id, tx_slate_id)?;
        w.close()?;
        Ok(())
    }

    pub fn get_stored_tx(&self, uuid: &str) -> Result<Transaction, Error> {
        let w = self.wallet.lock();
        w.get_stored_tx(uuid)
    }

    pub fn post_tx(&self, tx: &Transaction, fluff: bool) -> Result<(), Error> {
        let tx_hex = grin_util::to_hex(ser::ser_vec(tx).unwrap());
        let client = {
            let mut w = self.wallet.lock();
            w.w2n_client().clone()
        };
        client.post_tx(&TxWrapper { tx_hex: tx_hex }, fluff)?;
        Ok(())
    }

    pub fn verify_slate_messages(&mut self, slate: &Slate) -> Result<(), Error> {
        let secp = Secp256k1::with_caps(ContextFlag::VerifyOnly);
        slate.verify_messages(&secp)?;
        Ok(())
    }

    pub fn restore(&mut self) -> Result<(), Error> {
        let mut w = self.wallet.lock();
        w.open_with_credentials()?;
        let res = w.restore();
        w.close()?;
        res
    }

    pub fn check_repair(&mut self) -> Result<(), Error> {
        let mut w = self.wallet.lock();
        w.open_with_credentials()?;
        self.update_outputs(&mut w, true);
        w.check_repair()?;
        w.close()?;
        Ok(())
    }

    pub fn node_height(&mut self) -> Result<(u64, bool), Error> {
        let res = {
            let mut w = self.wallet.lock();
            w.open_with_credentials()?;
            w.w2n_client().get_chain_height()
        };
        match res {
            Ok(height) => Ok((height, true)),
            Err(_) => {
                let outputs = self.retrieve_outputs(true, false, None)?;
                let height = match outputs.1.iter().map(|(out, _)| out.height).max() {
                    Some(height) => height,
                    None => 0,
                };
                Ok((height, false))
            }
        }
    }

    fn update_outputs(&self, w: &mut W, update_all: bool) -> bool {
        let parent_key_id = w.get_parent_key_id();
        match updater::refresh_outputs(&mut *w, &parent_key_id, update_all) {
            Ok(_) => true,
            Err(_) => false,
        }
    }
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
        let parent_key_id = w.get_parent_key_id();
        let (slate, context, add_fn) = tx::create_receive_tx(
            &mut *w,
            amount,
            num_outputs,
            &parent_key_id,
            message,
        )?;

        {
            let mut batch = w.batch()?;
            batch.save_private_context(&slate.id.to_string(), &context)?;
            batch.commit()?;
        }

        w.close()?;
        Ok((slate, add_fn))
    }

    pub fn tx_add_invoice_outputs(
        &mut self,
        slate: &Slate,
        add_fn: impl FnOnce(&mut W, &Transaction) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let mut w = self.wallet.lock();
        w.open_with_credentials()?;
        add_fn(&mut *w, &slate.tx)?;
        Ok(())
    }

    pub fn receive_tx(
        &mut self,
        address: Option<String>,
        slate: &mut Slate,
        message: Option<String>,
    ) -> Result<(), Error> {
        let mut w = self.wallet.lock();
        w.open_with_credentials()?;
        let parent_key_id = w.get_parent_key_id();
        // Don't do this multiple times
        let tx = updater::retrieve_txs(&mut *w, None, Some(slate.id), Some(&parent_key_id), false)?;
        for t in &tx {
            if t.tx_type == TxLogEntryType::TxReceived {
                return Err(ErrorKind::TransactionAlreadyReceived(slate.id.to_string()).into());
            }
        }
        let res = tx::receive_tx(&mut *w, address, slate, &parent_key_id, message);
        w.close()?;

        if let Err(e) = res {
            Err(e)
        } else {
            Ok(())
        }
    }
}
