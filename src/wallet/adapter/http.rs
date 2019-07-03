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

/// HTTP Wallet 'plugin' implementation

use failure::Error;
use grin_api::client;
use grin_api::Error as APIError;
use serde::Serialize;
use serde_json::Value;

use crate::wallet::types::VersionedSlate;
use crate::wallet::ErrorKind;
use super::Adapter;

#[derive(Clone)]
pub struct HTTPAdapter {}

impl HTTPAdapter {
	/// Create
	pub fn new() -> Box<Self> {
		Box::new(Self {})
	}

	/// Check version of the other wallet
	fn check_other_version(&self, url: &str) -> Result<(), Error> {
		let req = json!({
			"jsonrpc": "2.0",
			"method": "check_version",
			"id": 1,
			"params": []
		});

		let res: String = post(url, None, &req).map_err(|e| {
			let report = format!("Performing version check (is recipient listening?): {}", e);
			error!("{}", report);
			ErrorKind::ClientCallback(report)
		})?;

		let res: Value = serde_json::from_str(&res).unwrap();
		trace!("Response: {}", res);
		if res["error"] != json!(null) {
			let report = format!(
				"Posting transaction slate: Error: {}, Message: {}",
				res["error"]["code"], res["error"]["message"]
			);
			error!("{}", report);
			return Err(ErrorKind::ClientCallback(report).into());
		}

		let resp_value = res["result"]["Ok"].clone();
		trace!("resp_value: {}", resp_value.clone());
		let foreign_api_version: u16 =
			serde_json::from_value(resp_value["foreign_api_version"].clone()).unwrap();
		let supported_slate_versions: Vec<String> =
			serde_json::from_value(resp_value["supported_slate_versions"].clone()).unwrap();

		// trivial tests for now, but will be expanded later
		if foreign_api_version < 2 {
			let report = format!("Other wallet reports unrecognized API format.");
			error!("{}", report);
			return Err(ErrorKind::ClientCallback(report).into());
		}

		if !supported_slate_versions.contains(&"V2".to_owned()) {
			let report = format!("Unable to negotiate slate format with other wallet.");
			error!("{}", report);
			return Err(ErrorKind::ClientCallback(report).into());
		}

		Ok(())
	}
}

impl Adapter for HTTPAdapter {
	fn supports_sync(&self) -> bool {
		true
	}

	fn send_tx_sync(&self, dest: &str, slate: &VersionedSlate) -> Result<VersionedSlate, Error> {
		if &dest[..4] != "http" {
			let err_str = format!(
				"dest formatted as {} but send -d expected stdout or http://IP:port",
				dest
			);
			error!("{}", err_str);
			return Err(ErrorKind::Uri.into());
		}
		let url = format!("{}/v2/foreign", dest);
		debug!("Posting transaction slate to {}", url);

		self.check_other_version(&url)?;

		// Note: not using easy-jsonrpc as don't want the dependencies in this crate
		let req = json!({
			"jsonrpc": "2.0",
			"method": "receive_tx",
			"id": 1,
			"params": [
			    slate,
				null,
				null
			]
		});
		trace!("Sending receive_tx request: {}", req);

		let res: String = post(url.as_str(), None, &req).map_err(|e| {
			let report = format!("Posting transaction slate (is recipient listening?): {}", e);
			error!("{}", report);
			ErrorKind::ClientCallback(report)
		})?;

		let res: Value = serde_json::from_str(&res).unwrap();
		trace!("Response: {}", res);
		if res["error"] != json!(null) {
			let report = format!(
				"Posting transaction slate: Error: {}, Message: {}",
				res["error"]["code"], res["error"]["message"]
			);
			error!("{}", report);
			return Err(ErrorKind::ClientCallback(report).into());
		}

		let slate_value = res["result"]["Ok"].clone();
		trace!("slate_value: {}", slate_value);

		let slate: VersionedSlate = serde_json::from_str(&serde_json::to_string(&slate_value).unwrap())?;
		Ok(slate)
	}

	fn send_tx_async(&self, _dest: &str, _slate: &VersionedSlate) -> Result<(), Error> {
		unimplemented!();
	}
}

pub fn post<IN>(url: &str, api_secret: Option<String>, input: &IN) -> Result<String, APIError>
where
	IN: Serialize,
{
	let req = client::create_post_request(url, api_secret, input)?;
	let res = client::send_request(req)?;
	Ok(res)
}
