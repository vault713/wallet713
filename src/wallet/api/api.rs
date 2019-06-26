use failure::Error;
use grin_core::ser;
use grin_util::secp::key::PublicKey;
use grin_util::secp::pedersen;
use grin_util::secp::{ContextFlag, Secp256k1};
use std::collections::HashSet;
use std::marker::PhantomData;
use uuid::Uuid;

use crate::common::ErrorKind;
use crate::contacts::GrinboxAddress;
use crate::wallet::types::{
    AcctPathMapping, Arc, BlockFees, CbData, ContextType, Identifier, Keychain,
    Mutex, NodeClient, OutputData, Slate, Transaction, TxLogEntry, TxLogEntryType, TxProof,
    TxWrapper, WalletBackend, WalletInfo,
};
use super::tx;

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

    /*pub fn invoice_tx(
        &mut self,
        slate: &mut Slate,
        minimum_confirmations: u64,
        max_outputs: usize,
        num_change_outputs: usize,
        selection_strategy_is_use_all: bool,
        message: Option<String>,
    ) -> Result<(impl FnOnce(&mut W, &Transaction) -> Result<(), Error>), Error> {
        let mut w = self.wallet.lock();
        w.open_with_credentials()?;
        let parent_key_id = w.get_parent_key_id();
        let tx = updater::retrieve_txs(&mut *w, None, Some(slate.id), Some(&parent_key_id), false)?;
        for t in &tx {
            if t.tx_type == TxLogEntryType::TxReceived {
                return Err(ErrorKind::TransactionAlreadyReceived(slate.id.to_string()).into());
            }
        }

        let res = tx::invoice_tx(
            &mut *w,
            slate,
            minimum_confirmations,
            max_outputs,
            num_change_outputs,
            selection_strategy_is_use_all,
            parent_key_id.clone(),
            message,
        );
        w.close()?;
        res
    }*/
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

    /*pub fn tx_add_invoice_outputs(
        &mut self,
        slate: &Slate,
        add_fn: impl FnOnce(&mut W, &Transaction) -> Result<(), Error>,
    ) -> Result<(), Error> {
        let mut w = self.wallet.lock();
        w.open_with_credentials()?;
        add_fn(&mut *w, &slate.tx)?;
        Ok(())
    }*/
}
