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

mod api;
mod broker;
#[macro_use]
mod common;
mod contacts;
mod controller;
mod internal;
mod wallet;

use clap::{crate_version, App, Arg, ArgMatches};
use colored::*;
use common::config::Wallet713Config;
use common::{ErrorKind, Result, RuntimeMode};
use contacts::{AddressBook, Backend};
use controller::cli::CLI;
use epic_core::global::{set_mining_mode, ChainTypes};
use wallet::create_container;

fn do_config(
	args: &ArgMatches,
	chain: &Option<ChainTypes>,
	silent: bool,
	new_address_index: Option<u32>,
	config_path: Option<&str>,
) -> Result<Wallet713Config> {
	let mut config;
	let mut any_matches = false;
	let exists = Wallet713Config::exists(config_path, &chain)?;
	if exists {
		config = Wallet713Config::from_file(config_path, &chain)?;
	} else {
		config = Wallet713Config::default(&chain)?;
	}

	if let Some(data_path) = args.value_of("data-path") {
		config.wallet713_data_path = data_path.to_string();
		any_matches = true;
	}

	if let Some(domain) = args.value_of("domain") {
		config.grinbox_domain = domain.to_string();
		any_matches = true;
	}

	if let Some(port) = args.value_of("port") {
		let port = u16::from_str_radix(port, 10).map_err(|_| ErrorKind::NumberParsingError)?;
		config.grinbox_port = Some(port);
		any_matches = true;
	}

	if let Some(node_uri) = args.value_of("node-uri") {
		config.grin_node_uri = Some(node_uri.to_string());
		any_matches = true;
	}

	if let Some(node_secret) = args.value_of("node-secret") {
		config.grin_node_secret = Some(node_secret.to_string());
		any_matches = true;
	}

	if new_address_index.is_some() {
		config.grinbox_address_index = new_address_index;
		any_matches = true;
	}

	config.to_file(config_path.map(|p| p.to_owned()))?;

	if !any_matches && !silent {
		cli_message!("{}", config);
	}

	Ok(config)
}

fn welcome(args: &ArgMatches, runtime_mode: &RuntimeMode) -> Result<Wallet713Config> {
	let chain: Option<ChainTypes> = match args.is_present("floonet") {
		true => Some(ChainTypes::Floonet),
		false => Some(ChainTypes::Mainnet),
	};

	unsafe {
		common::set_runtime_mode(runtime_mode);
	};

	let config = do_config(args, &chain, true, None, args.value_of("config-path"))?;
	set_mining_mode(config.chain.clone().unwrap_or(ChainTypes::Mainnet));

	Ok(config)
}

fn main() {
	enable_ansi_support();

	let matches = App::new("wallet713")
        .version(crate_version!())
        .arg(Arg::from_usage("[config-path] -c, --config=<config-path> 'the path to the config file'"))
        .arg(Arg::from_usage("[log-config-path] -l, --log-config-path=<log-config-path> 'the path to the log config file'"))
        .arg(Arg::from_usage("[account] -a, --account=<account> 'the account to use'"))
        .arg(Arg::from_usage("[daemon] -d, --daemon 'run daemon'"))
        .arg(Arg::from_usage("[floonet] -f, --floonet 'use floonet'"))
        .get_matches();

	let runtime_mode = match matches.is_present("daemon") {
		true => RuntimeMode::Daemon,
		false => RuntimeMode::Cli,
	};

	let config: Wallet713Config = welcome(&matches, &runtime_mode).unwrap_or_else(|e| {
		panic!(
			"{}: could not read or create config! {}",
			"ERROR".bright_red(),
			e
		);
	});

	if runtime_mode == RuntimeMode::Daemon {
		env_logger::init();
	}

	let data_path_buf = config.get_data_path().unwrap();
	let data_path = data_path_buf.to_str().unwrap();

	let address_book_backend =
		Backend::new(data_path).expect("could not create address book backend!");
	let address_book = AddressBook::new(Box::new(address_book_backend))
		.expect("could not create an address book!");

	let container = create_container(config, address_book).unwrap();

	let cli = CLI::new(container);
	cli.start();

	press_any_key();
}

#[cfg(windows)]
pub fn enable_ansi_support() {
	if !ansi_term::enable_ansi_support().is_ok() {
		colored::control::set_override(false);
	}
}

#[cfg(not(windows))]
pub fn enable_ansi_support() {}

#[cfg(windows)]
pub fn press_any_key() {
	dont_disappear::any_key_to_continue::default();
}

#[cfg(not(windows))]
pub fn press_any_key() {}
