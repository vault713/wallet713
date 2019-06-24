// Copyright 2018 The Grin Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use failure::Error;
use grin_keychain::{Identifier, Keychain};
use uuid::Uuid;
use crate::wallet::types::{
	Context, InitTxArgs, NodeClient, Slate, TxLogEntryType, WalletBackend
};
use crate::wallet::ErrorKind;
use super::selection;
use super::updater;

const USER_MESSAGE_MAX_LEN: usize = 256;

/// Initiate tx as sender
pub fn init_send_tx<T: ?Sized, C, K>(
	w: &mut T,
	args: InitTxArgs,
) -> Result<Slate, Error>
where
	T: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
{
	let parent_key_id = match args.src_acct_name {
		Some(d) => {
			let pm = w.get_acct_path(&d)?;
			match pm {
				Some(p) => p.path,
				None => w.get_parent_key_id(),
			}
		}
		None => w.get_parent_key_id(),
	};

    let message = args.message.map(|m| {
        let mut m = m.clone();
        m.truncate(USER_MESSAGE_MAX_LEN);
        m
    });

	let mut slate = new_tx_slate(w, args.amount, 2)?;

	// If we just want to estimate, just send the results back
	if let Some(true) = args.estimate_only {
		let (total, fee) = estimate_send_tx(
			w,
			args.amount,
			args.minimum_confirmations,
			args.max_outputs as usize,
			args.num_change_outputs as usize,
			args.selection_strategy_is_use_all,
			&parent_key_id,
		)?;
		slate.amount = total;
		slate.fee = fee;
		return Ok(slate);
	}

	let context = add_inputs_to_slate(
		w,
		&mut slate,
		args.minimum_confirmations,
		args.max_outputs as usize,
		args.num_change_outputs as usize,
		args.selection_strategy_is_use_all,
		&parent_key_id,
		0,
		message,
		true,
	)?;

	// Save the aggsig context in our DB for when we receive the transaction back
	{
		let mut batch = w.batch()?;
		batch.save_private_context(slate.id.as_bytes(), 0, &context)?;
		batch.commit()?;
	}
	if let Some(v) = args.target_slate_version {
		slate.version_info.orig_version = v;
	}
	Ok(slate)
}

/// Creates a new slate for a transaction, can be called by anyone involved in
/// the transaction (sender(s), receiver(s))
pub fn new_tx_slate<T: ?Sized, C, K>(
	wallet: &mut T,
	amount: u64,
	num_participants: usize,
) -> Result<Slate, Error>
where
	T: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
{
	let current_height = wallet.w2n_client().get_chain_height()?;
	let mut slate = Slate::blank(num_participants);
	slate.amount = amount;
	slate.height = current_height;

	// Set the lock_height explicitly to 0 here.
	// This will generate a Plain kernel (rather than a HeightLocked kernel).
	slate.lock_height = 0;

	Ok(slate)
}

/// Estimates locked amount and fee for the transaction without creating one
pub fn estimate_send_tx<T: ?Sized, C, K>(
	wallet: &mut T,
	amount: u64,
	minimum_confirmations: u64,
	max_outputs: usize,
	num_change_outputs: usize,
	selection_strategy_is_use_all: bool,
	parent_key_id: &Identifier,
) -> Result<
	(
		u64, // total
		u64, // fee
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

	// Sender selects outputs into a new slate and save our corresponding keys in
	// a transaction context. The secret key in our transaction context will be
	// randomly selected. This returns the public slate, and a closure that locks
	// our inputs and outputs once we're convinced the transaction exchange went
	// according to plan
	// This function is just a big helper to do all of that, in theory
	// this process can be split up in any way
	let (_, total, _, fee) = selection::select_coins_and_fee(
		wallet,
		amount,
		current_height,
		minimum_confirmations,
		max_outputs,
		num_change_outputs,
		selection_strategy_is_use_all,
		parent_key_id,
	)?;
	Ok((total, fee))
}

/// Add inputs to the slate (effectively becoming the sender)
pub fn add_inputs_to_slate<T: ?Sized, C, K>(
	wallet: &mut T,
	slate: &mut Slate,
	minimum_confirmations: u64,
	max_outputs: usize,
	num_change_outputs: usize,
	selection_strategy_is_use_all: bool,
	parent_key_id: &Identifier,
	participant_id: usize,
	message: Option<String>,
	is_initator: bool,
) -> Result<Context, Error>
where
	T: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
{
	// sender should always refresh outputs
	updater::refresh_outputs(wallet, parent_key_id, false)?;

	// Sender selects outputs into a new slate and save our corresponding keys in
	// a transaction context. The secret key in our transaction context will be
	// randomly selected. This returns the public slate, and a closure that locks
	// our inputs and outputs once we're convinced the transaction exchange went
	// according to plan
	// This function is just a big helper to do all of that, in theory
	// this process can be split up in any way
	let mut context = selection::build_send_tx(
		wallet,
		slate,
		minimum_confirmations,
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
		participant_id,
		message,
	)?;

	if !is_initator {
		// perform partial sig
		let _ = slate.fill_round_2(
			wallet.keychain(),
			&context.sec_key,
			&context.sec_nonce,
			participant_id,
		)?;
	}

	Ok(context)
}

/// Add receiver output to the slate
pub fn add_output_to_slate<T: ?Sized, C, K>(
	wallet: &mut T,
	slate: &mut Slate,
	parent_key_id: &Identifier,
	participant_id: usize,
	message: Option<String>,
	is_initiator: bool,
) -> Result<Context, Error>
where
	T: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
{
	// create an output using the amount in the slate
	let (_, mut context) =
		selection::build_recipient_output(wallet, slate, parent_key_id.clone())?;

	// fill public keys
	let _ = slate.fill_round_1(
		wallet.keychain(),
		&mut context.sec_key,
		&context.sec_nonce,
		participant_id,
		message,
	)?;

	if !is_initiator {
		// perform partial sig
		let _ = slate.fill_round_2(
			wallet.keychain(),
			&context.sec_key,
			&context.sec_nonce,
			participant_id,
		)?;
	}

	Ok(context)
}

/// Complete a transaction
pub fn complete_tx<T: ?Sized, C, K>(
	wallet: &mut T,
	slate: &mut Slate,
	participant_id: usize,
	context: &Context,
) -> Result<(), Error>
where
	T: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
{
	let _ = slate.fill_round_2(
		wallet.keychain(),
		&context.sec_key,
		&context.sec_nonce,
		participant_id,
	)?;

	// Final transaction can be built by anyone at this stage
	slate.finalize(wallet.keychain())?;
	Ok(())
}

/// Rollback outputs associated with a transaction in the wallet
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
	let (tx_vec, _) = updater::retrieve_txs(wallet, tx_id, tx_slate_id, Some(&parent_key_id), false, false)?;
	let tx = match tx_vec.into_iter().next() {
		Some(t) => t,
		None => {
			return Err(ErrorKind::TransactionDoesntExist(tx_id_string))?;
		}
	};
	if (tx.tx_type != TxLogEntryType::TxSent && tx.tx_type != TxLogEntryType::TxReceived) || tx.confirmed {
		return Err(ErrorKind::TransactionNotCancellable(tx_id_string))?;
	}
	// get outputs associated with tx
	let res = updater::retrieve_outputs(wallet, false, Some(tx.id), Some(&parent_key_id))?;
	let outputs = res.iter().map(|m| m.output.clone()).collect();
	updater::cancel_tx_and_outputs(wallet, tx, outputs, parent_key_id)?;
	Ok(())
}

/// Update the stored transaction (this update needs to happen when the TX is finalised)
pub fn update_stored_tx<T: ?Sized, C, K>(
	wallet: &mut T,
	slate: &Slate,
	is_invoiced: bool,
) -> Result<(), Error>
where
	T: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
{
	// finalize command
	let (tx_vec, _) = updater::retrieve_txs(wallet, None, Some(slate.id), None, false, false)?;
	let mut tx = None;
	// don't want to assume this is the right tx, in case of self-sending
	for t in tx_vec {
		if t.tx_type == TxLogEntryType::TxSent && !is_invoiced {
			tx = Some(t.clone());
			break;
		}
		if t.tx_type == TxLogEntryType::TxReceived && is_invoiced {
			tx = Some(t.clone());
			break;
		}
	}
	let tx = match tx {
		Some(t) => t,
		None => return Err(ErrorKind::TransactionDoesntExist(slate.id.to_string()))?,
	};
	{
		let mut batch = wallet.batch()?;
		batch.store_tx(&tx.tx_slate_id.unwrap().to_string(), &slate.tx)?;
		batch.commit()?;
	}
	Ok(())
}

/// Lock sender outputs
pub fn tx_lock_outputs<T: ?Sized, C, K>(
	wallet: &mut T,
	slate: &Slate,
	participant_id: usize,
	address: Option<String>,
) -> Result<(), Error>
where
	T: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
{
	let context = wallet.get_private_context(slate.id.as_bytes(), participant_id)?;
	selection::lock_tx_context(wallet, slate, address, &context)
}

/// Finalize slate
pub fn finalize_tx<T: ?Sized, C, K>(wallet: &mut T, slate: &Slate) -> Result<Slate, Error>
where
	T: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
{
	let mut s = slate.clone();
	let context = wallet.get_private_context(s.id.as_bytes(), 0)?;
	complete_tx(wallet, &mut s, 0, &context)?;
	update_stored_tx(wallet, &mut s, false)?;
	{
		let mut batch = wallet.batch()?;
		batch.delete_private_context(s.id.as_bytes(), 0)?;
		batch.commit()?;
	}
	Ok(s)
}

/// Receive a tx as recipient
pub fn receive_tx<T: ?Sized, C, K>(
	w: &mut T,
	slate: &Slate,
	dest_acct_name: Option<&str>,
	message: Option<String>,
) -> Result<Slate, Error>
where
	T: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
{
	let mut ret_slate = slate.clone();
	let parent_key_id = match dest_acct_name {
		Some(d) => {
			let pm = w.get_acct_path(d)?;
			match pm {
				Some(p) => p.path,
				None => w.get_parent_key_id(),
			}
		}
		None => w.get_parent_key_id(),
	};
	// Don't do this multiple times
	let (tx, _) = updater::retrieve_txs(
		w,
		None,
		Some(ret_slate.id),
		Some(&parent_key_id),
		false,
		false,
	)?;
	for t in &tx {
		if t.tx_type == TxLogEntryType::TxReceived {
			return Err(ErrorKind::TransactionAlreadyReceived(ret_slate.id.to_string()).into());
		}
	}

	let message = match message {
		Some(mut m) => {
			m.truncate(USER_MESSAGE_MAX_LEN);
			Some(m)
		}
		None => None,
	};

	add_output_to_slate(
		w,
		&mut ret_slate,
		&parent_key_id,
		1,
		message,
		false,
	)?;
	Ok(ret_slate)
}