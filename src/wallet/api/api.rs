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

    /*pub fn finalize_tx(
        &mut self,
        slate: &mut Slate,
        tx_proof: Option<&mut TxProof>,
    ) -> Result<bool, Error> {
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

                let tx_proof = tx_proof.map(|proof| {
                    proof.amount = context.amount;
                    proof.fee = context.fee;
                    for input in context.input_commits {
                        proof.inputs.push(input.clone());
                    }
                    for output in context.output_commits {
                        proof.outputs.push(output.clone());
                    }
                    proof
                });

                tx::update_stored_tx(&mut *w, slate, tx_proof)?;
                {
                    let mut batch = w.batch()?;
                    batch.delete_private_context(&slate.id.to_string())?;
                    batch.commit()?;
                }
                w.close()?;
                Ok(true)
            }
        }
    }*/

    /*pub fn cancel_tx(
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
    }*/

    /*
    fn update_outputs(&self, w: &mut W, update_all: bool) -> bool {
        let parent_key_id = w.get_parent_key_id();
        match updater::refresh_outputs(&mut *w, &parent_key_id, update_all) {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    pub fn get_stored_tx_proof(&mut self, id: u32) -> Result<TxProof, Error> {
        let mut w = self.wallet.lock();
        w.open_with_credentials()?;
        let parent_key_id = w.get_parent_key_id();
        let txs: Vec<TxLogEntry> =
            updater::retrieve_txs(&mut *w, Some(id), None, Some(&parent_key_id), false)?;
        if txs.len() != 1 {
            return Err(ErrorKind::TransactionHasNoProof)?;
        }
        let uuid = txs[0]
            .tx_slate_id
            .ok_or_else(|| ErrorKind::TransactionHasNoProof)?;
        w.get_stored_tx_proof(&uuid.to_string())
    }

    pub fn verify_tx_proof(
        &mut self,
        tx_proof: &TxProof,
    ) -> Result<
        (
            Option<GrinboxAddress>,
            GrinboxAddress,
            u64,
            Vec<pedersen::Commitment>,
            pedersen::Commitment,
        ),
        Error,
    > {
        let secp = &Secp256k1::with_caps(ContextFlag::Commit);

        let (destination, slate) = tx_proof
            .verify_extract(None)
            .map_err(|_| ErrorKind::VerifyProof)?;

        let inputs_ex = tx_proof.inputs.iter().collect::<HashSet<_>>();

        let mut inputs: Vec<pedersen::Commitment> = slate
            .tx
            .inputs()
            .iter()
            .map(|i| i.commitment())
            .filter(|c| !inputs_ex.contains(c))
            .collect();

        let outputs_ex = tx_proof.outputs.iter().collect::<HashSet<_>>();

        let outputs: Vec<pedersen::Commitment> = slate
            .tx
            .outputs()
            .iter()
            .map(|o| o.commitment())
            .filter(|c| !outputs_ex.contains(c))
            .collect();

        let excess = &slate.participant_data[1].public_blind_excess;

        let excess_parts: Vec<&PublicKey> = slate
            .participant_data
            .iter()
            .map(|p| &p.public_blind_excess)
            .collect();
        let excess_sum =
            PublicKey::from_combination(secp, excess_parts).map_err(|_| ErrorKind::VerifyProof)?;

        let commit_amount = secp.commit_value(tx_proof.amount)?;
        inputs.push(commit_amount);

        let commit_excess = secp.commit_sum(outputs.clone(), inputs)?;
        let pubkey_excess = commit_excess.to_pubkey(secp)?;

        if excess != &pubkey_excess {
            return Err(ErrorKind::VerifyProof.into());
        }

        let mut input_com: Vec<pedersen::Commitment> =
            slate.tx.inputs().iter().map(|i| i.commitment()).collect();

        let mut output_com: Vec<pedersen::Commitment> =
            slate.tx.outputs().iter().map(|o| o.commitment()).collect();

        input_com.push(secp.commit(0, slate.tx.offset.secret_key(secp)?)?);

        output_com.push(secp.commit_value(slate.fee)?);

        let excess_sum_com = secp.commit_sum(output_com, input_com)?;

        if excess_sum_com.to_pubkey(secp)? != excess_sum {
            return Err(ErrorKind::VerifyProof.into());
        }

        return Ok((
            destination,
            tx_proof.address.clone(),
            tx_proof.amount,
            outputs,
            excess_sum_com,
        ));
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

    /*pub fn initiate_receive_tx(
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
        let (slate, context, add_fn) =
            tx::create_receive_tx(&mut *w, amount, num_outputs, &parent_key_id, message)?;

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

    pub fn build_coinbase(&mut self, block_fees: &BlockFees) -> Result<CbData, Error> {
        let mut w = self.wallet.lock();
        w.open_with_credentials()?;
        let res = updater::build_coinbase(&mut *w, block_fees);
        w.close()?;
        res
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
    }*/
}
