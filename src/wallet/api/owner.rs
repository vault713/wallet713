// Copyright 2018 The Grin & vault713 Developers
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
use futures::sync::oneshot;
use futures::Future;
use grin_core::core::hash::Hashed;
use grin_core::core::Transaction;
use grin_core::ser::ser_vec;
use grin_keychain::Identifier;
use grin_util::secp::key::PublicKey;
use grin_util::secp::pedersen::Commitment;
use grin_util::{ZeroingString, to_hex};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::marker::PhantomData;
use std::thread::{JoinHandle, spawn};
use uuid::Uuid;

use crate::api::router::build_foreign_api_router;
use crate::broker::{Controller, GrinboxPublisher, GrinboxSubscriber, Subscriber};
use crate::common::config::Wallet713Config;
use crate::common::hasher::derive_address_key;
use crate::common::{Arc, Keychain, Mutex, MutexGuard};
use crate::contacts::{Address, AddressType, GrinboxAddress, parse_address};
use crate::wallet::adapter::{Adapter, GrinboxAdapter, HTTPAdapter};
use crate::wallet::{Container, ErrorKind};
use crate::internal::*;
use crate::wallet::types::{
    AcctPathMapping, InitTxArgs, NodeClient, OutputCommitMapping, Slate, SlateVersion, TxLogEntry,
	TxProof, TxWrapper, VersionedSlate, WalletBackend, WalletInfo,
};

pub struct Owner<W, C, K>
where
	W: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
{
    container: Arc<Mutex<Container<W, C, K>>>,
	phantom_k: PhantomData<K>,
	phantom_c: PhantomData<C>,
}

impl<W, C, K> Owner<W, C, K>
where
	W: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
{
    pub fn new(container: Arc<Mutex<Container<W, C, K>>>) -> Self {
		Owner {
			container,
			phantom_k: PhantomData,
			phantom_c: PhantomData,
		}
	}

	/// Convenience function that opens and closes the wallet with the stored credentials
	fn open_and_close<F, X>(&self, f: F) -> Result<X, Error>
	where
		F: FnOnce(&mut MutexGuard<Container<W, C, K>>) -> Result<X, Error>
	{
		let mut c = self.container.lock();
		{
			let w = c.backend()?;
			w.open_with_credentials()?;
		}
		let res = f(&mut c);
		{
			// Always try to close wallet
			// Operation still considered successful, even if closing failed
			let w = c.backend();
			if w.is_ok() {
				let _ = w.unwrap().close();
			}
		}
		res
	}

	pub fn has_seed(&self) -> Result<bool, Error> {
		let mut c = self.container.lock();
		let w = c.raw_backend();
		w.has_seed()
	}

	pub fn get_seed(&self) -> Result<ZeroingString, Error> {
		let mut c = self.container.lock();
		let w = c.raw_backend();
		w.get_seed()
	}

	pub fn set_seed(&self, mnemonic: Option<ZeroingString>, password: ZeroingString) -> Result<(), Error> {
		let mut c = self.container.lock();
		let w = c.raw_backend();
		w.set_seed(mnemonic, password, false)
	}

	/// Set the password to attempt to decrypt the seed with
	pub fn set_password(&self, password: ZeroingString) -> Result<(), Error> {
		let mut c = self.container.lock();
		let w = c.raw_backend();
		w.set_password(password)
	}

	/// Connect to the backend
	pub fn connect(&self) -> Result<(), Error> {
		let mut c = self.container.lock();
		let w = c.raw_backend();
		w.connect()
	}

	/// Clear the wallet of its contents
	pub fn clear(&self) -> Result<(), Error> {
		let mut c = self.container.lock();
		let w = c.raw_backend();
		w.clear()
	}

	pub fn get_config(&self) -> Wallet713Config {
		let c = self.container.lock();
		c.config.clone()
	}

	pub fn start_grinbox_listener(&self) -> Result<GrinboxAddress, Error> {
		self.open_and_close(|c| {
			if !c.backend()?.has_seed()? {
				return Err(ErrorKind::NoSeed.into());
			}

			if c.grinbox.is_some() {
				return Err(ErrorKind::GrinboxListener.into());
			}

			let index = c.config.grinbox_address_index();
			let keychain = c.backend()?.keychain();
			let sec_key = derive_address_key(keychain, index)?;
			let pub_key = PublicKey::from_secret_key(keychain.secp(), &sec_key)?;
			let address = GrinboxAddress::new(
				pub_key,
				Some(c.config.grinbox_domain.clone()),
				c.config.grinbox_port,
			);

			let publisher = GrinboxPublisher::new(
				&address,
				&sec_key,
				c.config.grinbox_protocol_unsecure(),
			)?;

			let subscriber = GrinboxSubscriber::new(&publisher)?;

			let caddress = address.clone();
			let cpublisher = publisher.clone();
			let mut csubscriber = subscriber.clone();
			let ccontainer = self.container.clone();
			let handle = spawn(move || {
				let controller = Controller::new(
					&caddress.stripped(),
					ccontainer,
					cpublisher,
				)
				.expect("could not start grinbox controller!");
				csubscriber
					.start(controller)
					.expect("something went wrong!");
				()
			});

			c.grinbox = Some((address.clone(), publisher, subscriber, handle));
			Ok(address)
		})
	}

	pub fn stop_grinbox_listener(&self) -> Result<(), Error> {
        let mut c = self.container.lock();
        match c.grinbox.take() {
            None => Ok(()),
            Some((address, publisher, subscriber, handle)) => {
                subscriber.stop();
                let _ = handle.join();
                c.grinbox = None;
                Ok(())
            }
        }
	}

	pub fn start_keybase_listener() -> Result<(), Error> {
		Ok(())
	}

	pub fn stop_keybase_listener() -> Result<(), Error> {
		Ok(())
	}

	pub fn start_foreign_http_listener(&self) -> Result<String, Error> {
        let lock = self.container.clone();
        let mut c = self.container.lock();
        if c.foreign_http.is_some() {
            return Err(ErrorKind::ForeignHttpListener.into());
        }
        let (stop_send, stop_recv) = oneshot::channel::<()>();
        let address = c.config.foreign_api_address();
        let router = build_foreign_api_router(self.container.clone(), c.config.foreign_api_secret.clone());
        let server = gotham::init_server(address.clone(), router);
        let fut = server
            .select(stop_recv.map(|_| ()).map_err(|_| ()))
            .map(|(res, _)| res)
            .map_err(|(error, _)| error);
        let handle = spawn(move || {
            tokio::run(fut);
            ()
        });

        c.foreign_http = Some((stop_send, handle));
        Ok(address)
	}

	pub fn stop_foreign_http_listener(&self) -> Result<(), Error> {
		let mut c = self.container.lock();
        match c.foreign_http.take() {
            None => Ok(()),
            Some((stop, handle)) => {
                let _ = stop.send(());
                let _ = handle.join();
                c.foreign_http = None;
                Ok(())
            }
        }
	}

	pub fn accounts(&self) -> Result<Vec<AcctPathMapping>, Error> {
		let mut c = self.container.lock();
		let w = c.backend()?;
        keys::accounts(w)
	}

    pub fn create_account_path(&self, label: &str) -> Result<Identifier, Error> {
		let mut c = self.container.lock();
		let w = c.backend()?;
        keys::new_acct_path(w, label)
	}

	pub fn active_account(&self) -> Result<String, Error> {
		let c = self.container.lock();
		Ok(c.account.clone())
	}

	pub fn set_active_account(&self, label: &str) -> Result<(), Error> {
		let mut c = self.container.lock();
		let w = c.backend()?;
		w.set_parent_key_id_by_name(label)?;
		c.account = label.to_owned();
		Ok(())
	}

    pub fn retrieve_outputs(
		&self,
		include_spent: bool,
		refresh_from_node: bool,
		tx_id: Option<u32>,
	) -> Result<(bool, Option<u64>, Vec<OutputCommitMapping>), Error> {
		self.open_and_close(|c| {
			let w = c.backend()?;
			let parent_key_id = w.get_parent_key_id();
			let mut validated = false;
			let mut height = None;
			if refresh_from_node {
				if let Ok(h) = updater::refresh_outputs(w, &parent_key_id, false) {
					validated = true;
					height = Some(h);
				}
			}

			let outputs = updater::retrieve_outputs(w, include_spent, tx_id, Some(&parent_key_id))?;
			Ok((
				validated,
				height,
				outputs,
			))
		})
	}

	pub fn retrieve_txs(
		&self,
		refresh_from_node: bool,
		check_contacts: bool,
		check_proofs: bool,
		tx_id: Option<u32>,
		tx_slate_id: Option<Uuid>,
	) -> Result<(bool, Option<u64>, Vec<TxLogEntry>, HashMap<String, String>, HashMap<Uuid, bool>), Error> {
		self.open_and_close(|c| {
			let w = c.backend()?;
			let parent_key_id = w.get_parent_key_id();

			let mut validated = false;
			let mut height = None;
			if refresh_from_node {
				if let Ok(h) = updater::refresh_outputs(w, &parent_key_id, false) {
					validated = true;
					height = Some(h);
				}
			}

			let (txs, proofs) = updater::retrieve_txs(w, tx_id, tx_slate_id, Some(&parent_key_id), false, check_proofs)?;

			let mut contacts = HashMap::new();
			match (check_contacts, &c.address_book) {
				(true, Some(book)) => for tx in &txs {
					if let Some(a) = &tx.address {
						match book.get_contact(a) {
							Ok(Some(con)) => {
								contacts.insert(a.clone(), con.name.clone());
							},
							_ => {}
						}
					}
				},
				_ => {},
			}

			Ok((validated, height, txs, contacts, proofs))
		})
	}

	fn retrieve_tx(
		&self,
		tx_id: Option<u32>,
		tx_slate_id: Option<Uuid>,
	) -> Result<TxLogEntry, Error> {
		let mut tx_id_string = String::new();
		if let Some(tx_id) = tx_id {
			tx_id_string = tx_id.to_string();
		} else if let Some(tx_slate_id) = tx_slate_id {
			tx_id_string = tx_slate_id.to_string();
		}

		let (_, _, txs, _, _) = self.retrieve_txs(true, false, false, tx_id, tx_slate_id)?;
		match txs.into_iter().next() {
			Some(t) => Ok(t),
			None => {
				Err(ErrorKind::TransactionDoesntExist(tx_id_string).into())
			}
		}
	}

    pub fn retrieve_summary_info(
		&self,
		refresh_from_node: bool,
		minimum_confirmations: u64,
	) -> Result<(bool, WalletInfo), Error> {
		self.open_and_close(|c| {
			let w = c.backend()?;
			let parent_key_id = w.get_parent_key_id();

			let mut validated = false;
			if refresh_from_node {
				validated = updater::refresh_outputs(w, &parent_key_id, false).is_ok();
			}

			let wallet_info = updater::retrieve_info(w, &parent_key_id, minimum_confirmations)?;
			Ok((validated, wallet_info))
		})
	}

    pub fn init_send_tx(&self, mut args: InitTxArgs) -> Result<Slate, Error> {
		if let Some(sa) = &mut args.send_args {
			if sa.dest.starts_with("@") {
				// Look up contact by address
				let c = self.container.lock();
				if let Some(contacts) = &c.address_book {
					let contact = contacts.get_contact(&sa.dest[1..])?;
					sa.dest = contact
						.ok_or(ErrorKind::ContactNotFound(sa.dest.clone()))?
						.address;
				}
				else {
					return Err(ErrorKind::NoAddressBook.into());
				}
			}

			if sa.method.is_none() {
				// Try to infer method from the address
				let address = parse_address(&sa.dest)?;
				sa.method = Some(match address.address_type() {
					AddressType::Http => "http",
					AddressType::Grinbox => "grinbox",
					AddressType::Keybase => "keybase",
				}.to_owned());
				sa.dest = address.stripped();
			}

		}
		let mut send_args = args.send_args.clone();
		let version = match args.target_slate_version {
			Some(v) => SlateVersion::try_from(v)?,
			None => SlateVersion::default(),
		};
		let mut slate = self.open_and_close(|c| {
			let w = c.backend()?;
			tx::init_send_tx(w, args)
		})?;

		// Helper functionality. If send arguments exist, attempt to send
		match &mut send_args {
			Some(sa) => {
				let vslate = VersionedSlate::into_version(slate.clone(), version);
				let sync = match sa.method.clone().unwrap().as_ref() {
					"http" => {
						slate = HTTPAdapter::new()
							.send_tx_sync(&sa.dest, &vslate)?
							.into();
						true
					}
					"grinbox" => {
						sa.finalize = false;
						sa.post_tx = false;
                        GrinboxAdapter::new(&self.container)
							.send_tx_async(&sa.dest, &vslate)?;
						false
					}
					/*"keybase" => {
						sa.finalize = false;
						sa.post_tx = false;
						let c = self.container.lock();
						let publisher = c.keybase_publisher.as_ref().ok_or(ErrorKind::KeybaseNoListener)?;
						publisher.post_slate(&slate)?;
					}*/
					_ => {
						error!("unsupported payment method");
						return Err(ErrorKind::ClientCallback(
							"unsupported payment method".to_owned(),
						))?;
					}
				};
				self.tx_lock_outputs(&slate, 0, Some(sa.dest.clone()))?;
				if sync {
					let slate = match sa.finalize {
						true => self.finalize_tx(&slate, None)?,
						false => slate,
					};

					if sa.post_tx {
						self.post_tx(&slate.tx, sa.fluff)?;
					}
					Ok(slate)
				}
				else {
					Ok(slate)
				}
			}
			None => Ok(slate),
		}
	}

	/*pub fn issue_invoice_tx(&self, args: IssueInvoiceTxArgs) -> Result<Slate, Error> {
		let mut w = self.wallet.lock();
		w.open_with_credentials()?;
		let slate = owner::issue_invoice_tx(&mut *w, args, self.doctest_mode)?;
		w.close()?;
		Ok(slate)
	}*/

    /*pub fn process_invoice_tx(&self, slate: &Slate, args: InitTxArgs) -> Result<Slate, Error> {
		let mut w = self.wallet.lock();
		w.open_with_credentials()?;
		let slate = owner::process_invoice_tx(&mut *w, slate, args, self.doctest_mode)?;
		w.close()?;
		Ok(slate)
	}*/

	pub fn tx_lock_outputs(&self, slate: &Slate, participant_id: usize, address: Option<String>) -> Result<(), Error> {
		self.open_and_close(|c| {
			let w = c.backend()?;
			tx::tx_lock_outputs(w, slate, participant_id, address)
		})
	}

	pub fn finalize_tx(&self, slate: &Slate, tx_proof: Option<&mut TxProof>) -> Result<Slate, Error> {
		self.open_and_close(|c| {
			let w = c.backend()?;
			let mut slate = slate.clone();
			slate = tx::finalize_tx(w, &slate, tx_proof)?;
			Ok(slate)
		})
	}

	pub fn post_tx(&self, tx: &Transaction, fluff: bool) -> Result<(), Error> {
		self.open_and_close(|c| {
			let w = c.backend()?;
			let tx_hex = to_hex(ser_vec(tx).unwrap());
			let res = w.w2n_client().post_tx(&TxWrapper { tx_hex }, fluff);
			if let Err(e) = res {
				error!("api: post_tx: failed with error: {}", e);
				Err(e)
			} else {
				debug!(
					"api: post_tx: successfully posted tx: {}, fluff? {}",
					tx.hash(),
					fluff
				);
				Ok(())
			}
		})
	}

	pub fn cancel_tx(&self, tx_id: Option<u32>, tx_slate_id: Option<Uuid>) -> Result<(), Error> {
		self.open_and_close(|c| {
			let w = c.backend()?;
			let parent_key_id = w.get_parent_key_id();
			if updater::refresh_outputs(w, &parent_key_id, false).is_err() {
				return Err(ErrorKind::Node.into());
			}

			tx::cancel_tx(w, &parent_key_id, tx_id, tx_slate_id)
		})
	}

	pub fn get_stored_tx(&self, slate_id: &Uuid) -> Result<Option<Transaction>, Error> {
		let mut c = self.container.lock();
		let w = c.backend()?;
		w.get_stored_tx(&slate_id.to_string())
	}

	pub fn repost_tx(&self, tx_id: Option<u32>, tx_slate_id: Option<Uuid>, fluff: bool) -> Result<(), Error> {
		let tx_entry = self.retrieve_tx(tx_id, tx_slate_id)?;
		if tx_entry.confirmed {
			return Err(ErrorKind::TransactionAlreadyConfirmed.into());
		}
		let slate_id = tx_entry.tx_slate_id.ok_or(ErrorKind::TransactionProofNotStored)?;
		let mut c = self.container.lock();
		let w = c.backend()?;
		let tx = w.get_stored_tx(&slate_id.to_string())?.ok_or(ErrorKind::TransactionNotStored)?;
		self.post_tx(&tx, fluff)
	}

	pub fn verify_slate_messages(&self, slate: &Slate) -> Result<(), Error> {
		slate.verify_messages()
	}

	pub fn get_stored_tx_proof(&self, tx_id: Option<u32>, tx_slate_id: Option<Uuid>) -> Result<Option<TxProof>, Error> {
		let tx_entry = self.retrieve_tx(tx_id, tx_slate_id)?;
		let slate_id = match tx_entry.tx_slate_id {
			Some(id) => id,
			None => {
				return Ok(None);
			}
		};
		let mut c = self.container.lock();
		let w = c.backend()?;
		w.get_stored_tx_proof(&slate_id.to_string())
	}

	pub fn verify_tx_proof(&self, tx_proof: &TxProof) ->
	Result<
		(
			GrinboxAddress,				// sender address
			GrinboxAddress,				// receiver address
			u64,						// amount
			Vec<Commitment>,			// receiver outputs
			Commitment,					// kernel excess
		),
		Error,
	>
	{
		tx::verify_tx_proof(tx_proof)
	}

	pub fn restore(&self) -> Result<(), Error> {
		self.open_and_close(|c| {
			let w = c.backend()?;
			w.restore()
		})
	}

	pub fn check_repair(&self, delete_unconfirmed: bool) -> Result<(), Error> {
		self.open_and_close(|c| {
			let w = c.backend()?;
			let parent_key_id = w.get_parent_key_id();
			updater::refresh_outputs(w, &parent_key_id, true)?;
			w.check_repair(delete_unconfirmed)
		})
	}

	pub fn node_height(&self) -> Result<(bool, u64), Error> {
		self.open_and_close(|c| {
			let w = c.backend()?;
			updater::node_height(w)
		})
	}
}
