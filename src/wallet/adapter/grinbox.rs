/// Grinbox 'plugin' implementation

use failure::Error;
use grin_api::client;
use grin_api::Error as APIError;
use serde::Serialize;
use serde_json::Value;

use crate::broker::{GrinboxPublisher, Publisher};
use crate::contacts::{Address, GrinboxAddress};
use crate::wallet::types::Slate;
use crate::wallet::ErrorKind;
use super::Adapter;

#[derive(Clone)]
pub struct GrinboxAdapter<'a> {
    publisher: Option<&'a GrinboxPublisher>,
}

impl<'a> GrinboxAdapter<'a> {
	/// Create
	pub fn new(publisher: Option<&'a GrinboxPublisher>) -> Self {
        Self {
            publisher,
        }
	}
}

impl<'a> Adapter for GrinboxAdapter<'a> {
	fn supports_sync(&self) -> bool {
		false
	}

	fn send_tx_sync(&self, _dest: &str, _slate: &Slate) -> Result<Slate, Error> {
		unimplemented!();
	}

	fn send_tx_async(&self, dest: &str, slate: &Slate) -> Result<(), Error> {
		let publisher = self.publisher.ok_or(ErrorKind::GrinboxNoListener)?;
        let address = GrinboxAddress::from_str(dest)?;
        publisher.post_slate(slate, &address)
	}

	fn receive_tx_async(&self, _params: &str) -> Result<Slate, Error> {
		unimplemented!();
	}
}
