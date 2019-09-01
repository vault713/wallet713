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

//! JSON-RPC Stub generation for the Foreign API

use crate::common::Keychain;
use crate::wallet::api::types::VersionInfo;
use crate::wallet::api::Foreign;
use crate::wallet::types::{BlockFees, CbData, NodeClient, Slate, VersionedSlate, WalletBackend};
use crate::wallet::ErrorKind;
use easy_jsonrpc;

/// Public definition used to generate Foreign jsonrpc api.
/// * When running with defaults, the V2 api is available at
/// `localhost:3415/v2/foreign`
/// * The endpoint only supports POST operations, with the json-rpc request as the body
#[easy_jsonrpc::rpc]
pub trait ForeignRpc {
	fn check_version(&self) -> Result<VersionInfo, ErrorKind>;
	fn build_coinbase(&self, block_fees: &BlockFees) -> Result<CbData, ErrorKind>;
	fn verify_slate_messages(&self, slate: &Slate) -> Result<(), ErrorKind>;
	fn receive_tx(
		&self,
		slate: VersionedSlate,
		dest_acct_name: Option<String>,
		message: Option<String>,
	) -> Result<VersionedSlate, ErrorKind>;
	//	fn finalize_invoice_tx(&self, slate: &Slate) -> Result<Slate, ErrorKind>;
}

impl<W, C, K> ForeignRpc for Foreign<W, C, K>
where
	W: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
{
	fn check_version(&self) -> Result<VersionInfo, ErrorKind> {
		Foreign::check_version(self).map_err(|e| ErrorKind::GenericError(e.to_string()))
		// TODO: use ErrorKind everywhere
	}

	fn build_coinbase(&self, block_fees: &BlockFees) -> Result<CbData, ErrorKind> {
		Foreign::build_coinbase(self, block_fees)
			.map_err(|e| ErrorKind::GenericError(e.to_string()))
	}

	fn verify_slate_messages(&self, slate: &Slate) -> Result<(), ErrorKind> {
		Foreign::verify_slate_messages(self, slate)
			.map_err(|e| ErrorKind::GenericError(e.to_string()))
	}

	fn receive_tx(
		&self,
		slate: VersionedSlate,
		dest_acct_name: Option<String>,
		message: Option<String>,
	) -> Result<VersionedSlate, ErrorKind> {
		let version = slate.version();
		let slate: Slate = slate.into();
		let slate = Foreign::receive_tx(
			self,
			&slate,
			dest_acct_name.as_ref().map(String::as_str),
			Some("http".to_owned()),
			message,
		)
		.map_err(|e| ErrorKind::GenericError(e.to_string()))?;

		Ok(VersionedSlate::into_version(slate, version))
	}

	/*fn finalize_invoice_tx(&self, slate: &Slate) -> Result<Slate, ErrorKind> {
		Foreign::finalize_invoice_tx(self, slate)
			.map_err(|e| ErrorKind::GenericError(e.to_string()))
	}*/
}
