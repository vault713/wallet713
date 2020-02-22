// Copyright 2019 The Grin Developers
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

//! JSON-RPC Stub generation for the Owner API

use crate::common::Keychain;
use crate::wallet::api::Owner;
use crate::wallet::types::{
	AcctPathMapping, Identifier, InitTxArgs, NodeClient, NodeHeightResult, OutputCommitMapping,
	Slate, Transaction, TxLogEntry, WalletBackend, WalletInfo,
};
use crate::wallet::ErrorKind;
use easy_jsonrpc_mw;
use uuid::Uuid;

/// Public definition used to generate Owner jsonrpc api.
/// * When running with defaults, the V2 api is available at
/// `localhost:3420/v2/owner`
/// * The endpoint only supports POST operations, with the json-rpc request as the body
#[easy_jsonrpc_mw::rpc]
pub trait OwnerRpc {
	fn accounts(&self) -> Result<Vec<AcctPathMapping>, ErrorKind>;
	fn create_account_path(&self, label: &String) -> Result<Identifier, ErrorKind>;
	fn set_active_account(&self, label: &String) -> Result<(), ErrorKind>;
	fn retrieve_outputs(
		&self,
		include_spent: bool,
		refresh_from_node: bool,
		tx_id: Option<u32>,
	) -> Result<(bool, Vec<OutputCommitMapping>), ErrorKind>;
	fn retrieve_txs(
		&self,
		refresh_from_node: bool,
		tx_id: Option<u32>,
		tx_slate_id: Option<Uuid>,
	) -> Result<(bool, Vec<TxLogEntry>), ErrorKind>;
	fn retrieve_summary_info(
		&self,
		refresh_from_node: bool,
		minimum_confirmations: u64,
	) -> Result<(bool, WalletInfo), ErrorKind>;
	fn init_send_tx(&self, args: InitTxArgs) -> Result<Slate, ErrorKind>;
	//	fn issue_invoice_tx(&self, args: IssueInvoiceTxArgs) -> Result<Slate, ErrorKind>;
	//	fn process_invoice_tx(&self, slate: &Slate, args: InitTxArgs) -> Result<Slate, ErrorKind>;
	fn tx_lock_outputs(&self, slate: Slate, participant_id: usize) -> Result<(), ErrorKind>;
	fn finalize_tx(&self, slate: Slate) -> Result<Slate, ErrorKind>;
	fn post_tx(&self, tx: &Transaction, fluff: bool) -> Result<(), ErrorKind>;
	fn cancel_tx(&self, tx_id: Option<u32>, tx_slate_id: Option<Uuid>) -> Result<(), ErrorKind>;
	fn get_stored_tx(&self, slate_id: &Uuid) -> Result<Option<Transaction>, ErrorKind>;
	fn verify_slate_messages(&self, slate: &Slate) -> Result<(), ErrorKind>;
	fn restore(&self) -> Result<(), ErrorKind>;
	fn check_repair(&self, delete_unconfirmed: bool) -> Result<(), ErrorKind>;
	fn node_height(&self) -> Result<NodeHeightResult, ErrorKind>;
}

impl<W, C, K> OwnerRpc for Owner<W, C, K>
where
	W: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
{
	fn accounts(&self) -> Result<Vec<AcctPathMapping>, ErrorKind> {
		Owner::accounts(self).map_err(|e| ErrorKind::GenericError(e.to_string()))
	}

	fn create_account_path(&self, label: &String) -> Result<Identifier, ErrorKind> {
		Owner::create_account_path(self, label).map_err(|e| ErrorKind::GenericError(e.to_string()))
	}

	fn set_active_account(&self, label: &String) -> Result<(), ErrorKind> {
		Owner::set_active_account(self, label).map_err(|e| ErrorKind::GenericError(e.to_string()))
	}

	fn retrieve_outputs(
		&self,
		include_spent: bool,
		refresh_from_node: bool,
		tx_id: Option<u32>,
	) -> Result<(bool, Vec<OutputCommitMapping>), ErrorKind> {
		Owner::retrieve_outputs(self, include_spent, refresh_from_node, tx_id)
			.map(|x| (x.0, x.2))
			.map_err(|e| ErrorKind::GenericError(e.to_string()))
	}

	fn retrieve_txs(
		&self,
		refresh_from_node: bool,
		tx_id: Option<u32>,
		tx_slate_id: Option<Uuid>,
	) -> Result<(bool, Vec<TxLogEntry>), ErrorKind> {
		Owner::retrieve_txs(self, refresh_from_node, false, false, tx_id, tx_slate_id)
			.map(|x| (x.0, x.2))
			.map_err(|e| ErrorKind::GenericError(e.to_string()))
	}

	fn retrieve_summary_info(
		&self,
		refresh_from_node: bool,
		minimum_confirmations: u64,
	) -> Result<(bool, WalletInfo), ErrorKind> {
		Owner::retrieve_summary_info(self, refresh_from_node, minimum_confirmations)
			.map_err(|e| ErrorKind::GenericError(e.to_string()))
	}

	fn init_send_tx(&self, args: InitTxArgs) -> Result<Slate, ErrorKind> {
		Owner::init_send_tx(self, args).map_err(|e| ErrorKind::GenericError(e.to_string()))
	}

	/*fn issue_invoice_tx(&self, args: IssueInvoiceTxArgs) -> Result<Slate, ErrorKind> {
		Owner::issue_invoice_tx(self, args).map_err(|e| e.kind())
	}*/

	/*fn process_invoice_tx(&self, slate: &Slate, args: InitTxArgs) -> Result<Slate, ErrorKind> {
		Owner::process_invoice_tx(self, slate, args).map_err(|e| e.kind())
	}*/

	fn tx_lock_outputs(&self, mut slate: Slate, participant_id: usize) -> Result<(), ErrorKind> {
		Owner::tx_lock_outputs(
			self,
			&mut slate,
			participant_id,
			Some("http owner api".to_owned()),
		)
		.map_err(|e| ErrorKind::GenericError(e.to_string()))
	}

	fn finalize_tx(&self, mut slate: Slate) -> Result<Slate, ErrorKind> {
		Owner::finalize_tx(self, &mut slate, None)
			.map_err(|e| ErrorKind::GenericError(e.to_string()))
	}

	fn post_tx(&self, tx: &Transaction, fluff: bool) -> Result<(), ErrorKind> {
		Owner::post_tx(self, tx, fluff).map_err(|e| ErrorKind::GenericError(e.to_string()))
	}

	fn cancel_tx(&self, tx_id: Option<u32>, tx_slate_id: Option<Uuid>) -> Result<(), ErrorKind> {
		Owner::cancel_tx(self, tx_id, tx_slate_id)
			.map_err(|e| ErrorKind::GenericError(e.to_string()))
	}

	fn get_stored_tx(&self, slate_id: &Uuid) -> Result<Option<Transaction>, ErrorKind> {
		Owner::get_stored_tx(self, slate_id).map_err(|e| ErrorKind::GenericError(e.to_string()))
	}

	fn verify_slate_messages(&self, slate: &Slate) -> Result<(), ErrorKind> {
		Owner::verify_slate_messages(self, slate)
			.map_err(|e| ErrorKind::GenericError(e.to_string()))
	}

	fn restore(&self) -> Result<(), ErrorKind> {
		Owner::restore(self).map_err(|e| ErrorKind::GenericError(e.to_string()))
	}

	fn check_repair(&self, delete_unconfirmed: bool) -> Result<(), ErrorKind> {
		Owner::check_repair(self, delete_unconfirmed)
			.map_err(|e| ErrorKind::GenericError(e.to_string()))
	}

	fn node_height(&self) -> Result<NodeHeightResult, ErrorKind> {
		Owner::node_height(self).map_err(|e| ErrorKind::GenericError(e.to_string()))
	}
}
