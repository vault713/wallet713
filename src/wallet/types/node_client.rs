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

use super::TxWrapper;
use crate::common::client;
use crate::wallet::ErrorKind;
use failure::Error;
use futures::stream;
use futures::Stream;
use grin_api::{Output, OutputListing, OutputType, Tip};
use grin_util::secp::pedersen::{Commitment, RangeProof};
use grin_util::to_hex;
use std::collections::HashMap;
use tokio::runtime::Runtime;

/// Node version info
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct NodeVersionInfo {
	/// Semver version string
	pub node_version: String,
	/// block header verson
	pub block_header_version: u16,
	/// Whether this version info was successfully verified from a node
	pub verified: Option<bool>,
}

/// Encapsulate all wallet-node communication functions. No functions within libwallet
/// should care about communication details
pub trait NodeClient: Sync + Send + Clone + 'static {
	/// Return the URL of the check node
	fn node_url(&self) -> &str;

	/// Set the node URL
	fn set_node_url(&mut self, node_url: &str);

	/// Return the node api secret
	fn node_api_secret(&self) -> Option<String>;

	/// Change the API secret
	fn set_node_api_secret(&mut self, node_api_secret: Option<String>);

	fn get_version_info(&mut self) -> Option<NodeVersionInfo>;

	/// Posts a transaction to a grin node
	fn post_tx(&self, tx: &TxWrapper, fluff: bool) -> Result<(), Error>;

	/// retrieves the current tip from the specified grin node
	fn get_chain_height(&self) -> Result<u64, Error>;

	/// retrieve a list of outputs from the specified grin node
	/// need "by_height" and "by_id" variants
	fn get_outputs_from_node(
		&self,
		wallet_outputs: Vec<Commitment>,
	) -> Result<HashMap<Commitment, (String, u64, u64)>, Error>;

	/// Get a list of outputs from the node by traversing the UTXO
	/// set in PMMR index order.
	/// Returns
	/// (last available output index, last insertion index retrieved,
	/// outputs(commit, proof, is_coinbase, height, mmr_index))
	fn get_outputs_by_pmmr_index(
		&self,
		start_height: u64,
		max_outputs: u64,
	) -> Result<(u64, u64, Vec<(Commitment, RangeProof, bool, u64, u64)>), Error>;
}

#[derive(Clone)]
pub struct HTTPNodeClient {
	node_url: String,
	node_api_secret: Option<String>,
	node_version_info: Option<NodeVersionInfo>,
}

impl HTTPNodeClient {
	/// Create a new client that will communicate with the given grin node
	pub fn new(node_url: &str, node_api_secret: Option<String>) -> HTTPNodeClient {
		HTTPNodeClient {
			node_url: node_url.to_owned(),
			node_api_secret: node_api_secret,
			node_version_info: None,
		}
	}
}

impl NodeClient for HTTPNodeClient {
	fn node_url(&self) -> &str {
		&self.node_url
	}
	fn node_api_secret(&self) -> Option<String> {
		self.node_api_secret.clone()
	}

	fn set_node_url(&mut self, node_url: &str) {
		self.node_url = node_url.to_owned();
	}

	fn set_node_api_secret(&mut self, node_api_secret: Option<String>) {
		self.node_api_secret = node_api_secret;
	}

	fn get_version_info(&mut self) -> Option<NodeVersionInfo> {
		if let Some(v) = self.node_version_info.as_ref() {
			return Some(v.clone());
		}
		let url = format!("{}/v1/version", self.node_url());
		let mut retval = match client::get::<NodeVersionInfo>(url.as_str(), self.node_api_secret())
		{
			Ok(n) => n,
			Err(e) => {
				// If node isn't available, allow offline functions
				// unfortunately have to parse string due to error structure
				let err_string = format!("{}", e);
				if err_string.contains("404") {
					return Some(NodeVersionInfo {
						node_version: "1.0.0".into(),
						block_header_version: 1,
						verified: Some(false),
					});
				} else {
					error!("Unable to contact Node to get version info: {}", e);
					return None;
				}
			}
		};
		retval.verified = Some(true);
		self.node_version_info = Some(retval.clone());
		Some(retval)
	}

	/// Posts a transaction to a grin node
	fn post_tx(&self, tx: &TxWrapper, fluff: bool) -> Result<(), Error> {
		let url;
		let dest = self.node_url();
		if fluff {
			url = format!("{}/v1/pool/push_tx?fluff", dest);
		} else {
			url = format!("{}/v1/pool/push_tx", dest);
		}
		let res = client::post_no_ret(url.as_str(), self.node_api_secret(), tx);
		if let Err(e) = res {
			let report = format!("Posting transaction to node: {}", e);
			error!("Post TX Error: {}", e);
			return Err(ErrorKind::ClientCallback(report).into());
		}
		Ok(())
	}

	/// Return the chain tip from a given node
	fn get_chain_height(&self) -> Result<u64, Error> {
		let addr = self.node_url();
		let url = format!("{}/v1/chain", addr);
		let res = client::get::<Tip>(url.as_str(), self.node_api_secret());
		match res {
			Err(e) => {
				let report = format!("Getting chain height from node: {}", e);
				error!("Get chain height error: {}", e);
				Err(ErrorKind::ClientCallback(report).into())
			}
			Ok(r) => Ok(r.height),
		}
	}

	/// Retrieve outputs from node
	fn get_outputs_from_node(
		&self,
		wallet_outputs: Vec<Commitment>,
	) -> Result<HashMap<Commitment, (String, u64, u64)>, Error> {
		let addr = self.node_url();
		// build the necessary query params -
		// ?id=xxx,yyy,zzz
		let query_params: Vec<String> = wallet_outputs
			.iter()
			.map(|commit| format!("{}", to_hex(commit.as_ref().to_vec())))
			.collect();

		// build a map of api outputs by commit so we can look them up efficiently
		let mut api_outputs: HashMap<Commitment, (String, u64, u64)> = HashMap::new();
		let mut tasks = Vec::new();

		for query_chunk in query_params.chunks(120) {
			let url = format!(
				"{}/v1/chain/outputs/byids?id={}",
				addr,
				query_chunk.join(","),
			);
			tasks.push(client::get_async::<Vec<Output>>(
				url.as_str(),
				self.node_api_secret(),
			));
		}

		let task = stream::futures_unordered(tasks).collect();

		let mut rt = Runtime::new().unwrap();
		let results = match rt.block_on(task) {
			Ok(outputs) => outputs,
			Err(e) => {
				let report = format!("Getting outputs by id: {}", e);
				error!("Outputs by id failed: {}", e);
				return Err(ErrorKind::ClientCallback(report).into());
			}
		};

		for res in results {
			for out in res {
				api_outputs.insert(
					out.commit.commit(),
					(to_hex(out.commit.to_vec()), out.height, out.mmr_index),
				);
			}
		}
		Ok(api_outputs)
	}

	fn get_outputs_by_pmmr_index(
		&self,
		start_height: u64,
		max_outputs: u64,
	) -> Result<(u64, u64, Vec<(Commitment, RangeProof, bool, u64, u64)>), Error> {
		let addr = self.node_url();
		let query_param = format!("start_index={}&max={}", start_height, max_outputs);

		let url = format!("{}/v1/txhashset/outputs?{}", addr, query_param,);

		let mut api_outputs: Vec<(Commitment, RangeProof, bool, u64, u64)> = Vec::new();

		match client::get::<OutputListing>(url.as_str(), self.node_api_secret()) {
			Ok(o) => {
				for out in o.outputs {
					let is_coinbase = match out.output_type {
						OutputType::Coinbase => true,
						OutputType::Transaction => false,
					};
					api_outputs.push((
						out.commit,
						out.range_proof().unwrap(),
						is_coinbase,
						out.block_height.unwrap(),
						out.mmr_index,
					));
				}

				Ok((o.highest_index, o.last_retrieved_index, api_outputs))
			}
			Err(e) => {
				// if we got anything other than 200 back from server, bye
				error!(
					"get_outputs_by_pmmr_index: error contacting {}. Error: {}",
					addr, e
				);
				let report = format!("outputs by pmmr index: {}", e);
				Err(ErrorKind::ClientCallback(report))?
			}
		}
	}
}
