use grin_util::secp::pedersen;
use uuid::Uuid;

use crate::common::config::{Wallet713Config, WalletConfig};
use crate::common::crypto::Hex;
use crate::common::hasher::derive_address_key;
use crate::common::{ErrorKind, Result};
use crate::contacts::AddressBook;
use crate::wallet::types::TxProof;

use super::api::Owner;
use super::backend::Backend;
use super::types::{
    Arc, BlockFees, CbData, ExtKeychain, HTTPNodeClient, Mutex, OutputData, NodeClient, SecretKey,
    Slate, Transaction, TxLogEntry, WalletBackend, WalletInfo, WalletInst, WalletSeed,
};

pub struct Wallet {
    active_account: String,
    pub backend: Option<Arc<Mutex<Backend<HTTPNodeClient, ExtKeychain>>>>,
    max_auto_accept_invoice: Option<u64>,
}

impl Wallet {
    pub fn new(max_auto_accept_invoice: Option<u64>) -> Self {
        Self {
            active_account: "default".to_string(),
            backend: None,
            max_auto_accept_invoice,
        }
    }

    /*
    pub fn initiate_receive_tx(&self, amount: u64, num_outputs: usize) -> Result<Slate> {
        let wallet = self.get_wallet_instance()?;
        let mut s: Slate = Slate::blank(0);
        controller::foreign_single_use(wallet.clone(), |api| {
            let (slate, add_fn) = api.initiate_receive_tx(amount, num_outputs, None)?;
            api.tx_add_invoice_outputs(&slate, add_fn)?;
            s = slate;
            Ok(())
        })?;
        Ok(s)
    }

    pub fn process_receiver_initiated_slate(&self, slate: &mut Slate) -> Result<()> {
        // reject by default unless wallet is set to auto accept invoices under a certain threshold
        let max_auto_accept_invoice = self
            .max_auto_accept_invoice
            .ok_or(ErrorKind::DoesNotAcceptInvoices)?;

        if slate.amount > max_auto_accept_invoice {
            Err(ErrorKind::InvoiceAmountTooBig(slate.amount))?;
        }

        let wallet = self.get_wallet_instance()?;

        controller::owner_single_use(wallet.clone(), |api| {
            let lock_fn = api.invoice_tx(slate, 10, 500, 1, false, None)?;
            api.tx_lock_outputs(&slate.tx, lock_fn)?;
            Ok(())
        })?;
        Ok(())
    }

    pub fn derive_address_key(&self, index: u32) -> Result<SecretKey> {
        let wallet = self.get_wallet_instance()?;
        let mut w = wallet.lock();
        w.open_with_credentials()?;
        derive_address_key(w.keychain(), index).map_err(|e| e.into())
    }*/
}