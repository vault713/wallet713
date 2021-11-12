// Copyright 2019 The vault713 Developers
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

use crate::api::router::{build_foreign_api_router, build_owner_api_router};
use crate::broker::{
	Controller, EpicboxPublisher, EpicboxSubscriber, KeybasePublisher, KeybaseSubscriber,
	Publisher, Subscriber,
};
use crate::common::hasher::derive_address_key;
use crate::common::{Arc, Keychain, Mutex, MutexGuard};
use crate::contacts::{Address, EpicboxAddress, KeybaseAddress};
use crate::wallet::types::{NodeClient, VersionedSlate, WalletBackend};
use crate::wallet::Container;
use failure::Error;
use futures::sync::oneshot;
use futures::Future;
use epic_util::secp::key::PublicKey;
use std::fmt;
use std::thread::{spawn, JoinHandle};

pub trait Listener: Sync + Send + 'static {
	fn interface(&self) -> ListenerInterface;
	fn address(&self) -> String;
	fn publish(&self, slate: &VersionedSlate, to: &String) -> Result<(), Error>;
	fn stop(self: Box<Self>) -> Result<(), Error>;
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Hash)]
pub enum ListenerInterface {
	Epicbox,
	Keybase,
	ForeignHttp,
	OwnerHttp,
}

impl fmt::Display for ListenerInterface {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match *self {
			ListenerInterface::Epicbox => write!(f, "Epicbox"),
			ListenerInterface::Keybase => write!(f, "Keybase"),
			ListenerInterface::ForeignHttp => write!(f, "Foreign HTTP"),
			ListenerInterface::OwnerHttp => write!(f, "Owner HTTP"),
		}
	}
}

pub struct EpicboxListener {
	address: EpicboxAddress,
	publisher: EpicboxPublisher,
	subscriber: EpicboxSubscriber,
	handle: JoinHandle<()>,
}

impl Listener for EpicboxListener {
	fn interface(&self) -> ListenerInterface {
		ListenerInterface::Epicbox
	}

	fn address(&self) -> String {
		self.address.stripped()
	}

	fn publish(&self, slate: &VersionedSlate, to: &String) -> Result<(), Error> {
		let address = EpicboxAddress::from_str(to)?;
		self.publisher.post_slate(slate, &address)
	}

	fn stop(self: Box<Self>) -> Result<(), Error> {
		let s = *self;
		s.subscriber.stop();
		let _ = s.handle.join();
		Ok(())
	}
}

pub struct KeybaseListener {
	address: String,
	publisher: KeybasePublisher,
	subscriber: KeybaseSubscriber,
	handle: JoinHandle<()>,
}

impl Listener for KeybaseListener {
	fn interface(&self) -> ListenerInterface {
		ListenerInterface::Keybase
	}

	fn address(&self) -> String {
		self.address.clone()
	}

	fn publish(&self, slate: &VersionedSlate, to: &String) -> Result<(), Error> {
		let address = KeybaseAddress::from_str(to)?;
		self.publisher.post_slate(slate, &address)
	}

	fn stop(self: Box<Self>) -> Result<(), Error> {
		let s = *self;
		s.subscriber.stop();
		let _ = s.handle.join();
		Ok(())
	}
}

pub struct ForeignHttpListener {
	address: String,
	stop: oneshot::Sender<()>,
	handle: JoinHandle<()>,
}

impl Listener for ForeignHttpListener {
	fn interface(&self) -> ListenerInterface {
		ListenerInterface::ForeignHttp
	}

	fn address(&self) -> String {
		self.address.clone()
	}

	fn publish(&self, _slate: &VersionedSlate, _to: &String) -> Result<(), Error> {
		unimplemented!();
	}

	fn stop(self: Box<Self>) -> Result<(), Error> {
		let s = *self;
		let _ = s.stop.send(());
		let _ = s.handle;
		Ok(())
	}
}

pub struct OwnerHttpListener {
	address: String,
	stop: oneshot::Sender<()>,
	handle: JoinHandle<()>,
}

impl Listener for OwnerHttpListener {
	fn interface(&self) -> ListenerInterface {
		ListenerInterface::OwnerHttp
	}

	fn address(&self) -> String {
		self.address.clone()
	}

	fn publish(&self, _slate: &VersionedSlate, _to: &String) -> Result<(), Error> {
		unimplemented!();
	}

	fn stop(self: Box<Self>) -> Result<(), Error> {
		let s = *self;
		let _ = s.stop.send(());
		let _ = s.handle;
		Ok(())
	}
}

pub fn start_epicbox<W, C, K>(
	container: Arc<Mutex<Container<W, C, K>>>,
	c: &mut MutexGuard<Container<W, C, K>>,
) -> Result<Box<dyn Listener>, Error>
where
	W: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
{
	let index = c.config.epicbox_address_index();
	let keychain = c.backend()?.keychain();
	let sec_key = derive_address_key(keychain, index)?;
	let pub_key = PublicKey::from_secret_key(keychain.secp(), &sec_key)?;

	let address = EpicboxAddress::new(
		pub_key,
		Some(c.config.epicbox_domain.clone()),
		c.config.epicbox_port,
	);

	let publisher =
		EpicboxPublisher::new(&address, &sec_key, c.config.epicbox_protocol_unsecure())?;

	let subscriber = EpicboxSubscriber::new(&publisher)?;

	let caddress = address.clone();
	let mut csubscriber = subscriber.clone();
	let cpublisher = publisher.clone();
	let handle = spawn(move || {
		let controller = Controller::new(&caddress.stripped(), container, cpublisher)
			.expect("could not start epicbox controller!");
		csubscriber
			.start(controller)
			.expect("something went wrong!");
		()
	});

	Ok(Box::new(EpicboxListener {
		address,
		publisher,
		subscriber,
		handle,
	}))
}

pub fn start_keybase<W, C, K>(
	container: Arc<Mutex<Container<W, C, K>>>,
	c: &mut MutexGuard<Container<W, C, K>>,
) -> Result<Box<dyn Listener>, Error>
where
	W: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
{
	let subscriber = KeybaseSubscriber::new()?;
	let publisher = KeybasePublisher::new(c.config.default_keybase_ttl.clone())?;

	let mut csubscriber = subscriber.clone();
	let cpublisher = publisher.clone();
	let handle = spawn(move || {
		let controller = Controller::new("keybase", container, cpublisher)
			.expect("could not start keybase controller!");
		csubscriber
			.start(controller)
			.expect("something went wrong!");
		()
	});

	Ok(Box::new(KeybaseListener {
		address: String::from("keybase"),
		publisher,
		subscriber,
		handle,
	}))
}

pub fn start_foreign_http<W, C, K>(
	container: Arc<Mutex<Container<W, C, K>>>,
	c: &mut MutexGuard<Container<W, C, K>>,
) -> Result<Box<dyn Listener>, Error>
where
	W: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
{
	let (stop, stop_recv) = oneshot::channel::<()>();
	let address = c.config.foreign_api_address();
	let router = build_foreign_api_router(container, c.config.foreign_api_secret.clone());
	let server = gotham::init_server(address.clone(), router);
	let fut = stop_recv
		.map_err(|_| ())
		.select(server)
		.map(|(res, _)| res)
		.map_err(|(error, _)| error);
	let handle = spawn(move || {
		tokio::run(fut);
		()
	});

	Ok(Box::new(ForeignHttpListener {
		address,
		stop,
		handle,
	}))
}

pub fn start_owner_http<W, C, K>(
	container: Arc<Mutex<Container<W, C, K>>>,
	c: &mut MutexGuard<Container<W, C, K>>,
) -> Result<Box<dyn Listener>, Error>
where
	W: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
{
	let (stop, stop_recv) = oneshot::channel::<()>();
	let address = c.config.owner_api_address();
	let router = build_owner_api_router(container, c.config.owner_api_secret.clone());
	let server = gotham::init_server(address.clone(), router);
	let fut = stop_recv
		.map_err(|_| ())
		.select(server)
		.map(|(res, _)| res)
		.map_err(|(error, _)| error);
	let handle = spawn(move || {
		tokio::run(fut);
		()
	});

	Ok(Box::new(OwnerHttpListener {
		address,
		stop,
		handle,
	}))
}
