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

use crate::common::ErrorKind;
use crate::wallet::types::{InitTxArgs, InitTxSendArgs};
use clap::ArgMatches;
use grin_core::core::amount_from_hr_string;
use std::str::FromStr;

macro_rules! usage {
	( $r:expr ) => {
		return Err(ErrorKind::Usage($r.usage().to_owned()));
	};
}

#[derive(Clone, Debug)]
pub enum AccountArgs<'a> {
	Create(&'a str),
	Switch(&'a str),
}

#[derive(Clone, Debug)]
pub enum SendCommandType<'a> {
	Estimate,
	File(&'a str),
	Address,
}

#[derive(Clone, Debug)]
pub enum ProofArgs<'a> {
	Export(u32, &'a str),
	Verify(&'a str),
}

#[derive(Clone, Debug)]
pub enum ContactArgs<'a> {
	Add(&'a str, &'a str),
	Remove(&'a str),
}

#[derive(Clone, Debug)]
pub enum AddressArgs {
	Display,
	Next,
	Prev,
	Index(u32),
}

#[derive(Clone, Debug)]
pub enum SeedArgs {
	Display,
	Recover,
}

fn required<'a>(args: &'a ArgMatches, name: &str) -> Result<&'a str, ErrorKind> {
	args.value_of(name)
		.ok_or_else(|| ErrorKind::Argument(name.to_owned()))
}

fn parse<T>(arg: &str) -> Result<T, ErrorKind>
where
	T: FromStr,
{
	arg.parse::<T>()
		.map_err(|_| ErrorKind::ParseNumber(arg.to_owned()))
}

pub fn account_command<'a>(args: &'a ArgMatches) -> Result<AccountArgs<'a>, ErrorKind> {
	let account_args = match args.subcommand() {
		("create", Some(args)) => AccountArgs::Create(required(args, "name")?),
		("switch", Some(args)) => AccountArgs::Switch(required(args, "name")?),
		(_, _) => {
			usage!(args);
		}
	};
	Ok(account_args)
}

pub fn send_command<'a>(
	args: &'a ArgMatches,
) -> Result<(SendCommandType<'a>, InitTxArgs), ErrorKind> {
	let mut init_args = InitTxArgs::default();

	let amount = required(args, "amount")?;
	init_args.amount =
		amount_from_hr_string(amount).map_err(|_| ErrorKind::ParseNumber(amount.to_owned()))?;
	if let Some(confirmations) = args.value_of("confirmations") {
		init_args.minimum_confirmations = parse(confirmations)?;
	}
	if let Some(change_outputs) = args.value_of("change_outputs") {
		init_args.num_change_outputs = parse(change_outputs)?;
	}
	init_args.selection_strategy_is_use_all = match args.value_of("strategy") {
		Some("all") => true,
		_ => false,
	};
	init_args.message = args.value_of("message").map(|m| m.to_owned());
	if let Some(version) = args.value_of("version") {
		init_args.target_slate_version = Some(parse(version)?);
	}

	let cmd_type = if let Some(address) = args.value_of("address") {
		init_args.send_args = Some(InitTxSendArgs {
			method: None,
			dest: address.to_owned(),
			finalize: true,
			post_tx: true,
			fluff: args.is_present("fluff"),
		});
		SendCommandType::Address
	} else if let Some(file) = args.value_of("file_name") {
		SendCommandType::File(file)
	} else if args.is_present("estimate") {
		init_args.estimate_only = Some(true);
		SendCommandType::Estimate
	} else {
		usage!(args);
	};

	Ok((cmd_type, init_args))
}

pub fn finalize_command<'a>(args: &'a ArgMatches) -> Result<(&'a str, bool), ErrorKind> {
	Ok((required(args, "file_name")?, args.is_present("fluff")))
}

pub fn repost_command(args: &ArgMatches) -> Result<(u32, bool), ErrorKind> {
	Ok((parse(required(args, "index")?)?, args.is_present("fluff")))
}

pub fn cancel_command(args: &ArgMatches) -> Result<u32, ErrorKind> {
	Ok(parse(required(args, "index")?)?)
}

pub fn repair_command(args: &ArgMatches) -> Result<bool, ErrorKind> {
	Ok(args.is_present("delete_unconfirmed"))
}

pub fn listen_command<'a>(args: &'a ArgMatches) -> Result<(&'a str, bool), ErrorKind> {
	Ok((
		args.value_of("type").unwrap_or(""),
		args.is_present("owner"),
	))
}

pub fn receive_command<'a>(args: &'a ArgMatches) -> Result<(&'a str, Option<&'a str>), ErrorKind> {
	Ok((required(args, "file_name")?, args.value_of("message")))
}

pub fn proof_command<'a>(args: &'a ArgMatches) -> Result<ProofArgs<'a>, ErrorKind> {
	let proof_args = match args.subcommand() {
		("export", Some(args)) => ProofArgs::Export(
			parse(required(args, "index")?)?,
			required(args, "file_name")?,
		),
		("verify", Some(args)) => ProofArgs::Verify(required(args, "file_name")?),
		(_, _) => {
			usage!(args);
		}
	};
	Ok(proof_args)
}

pub fn contact_command<'a>(args: &'a ArgMatches) -> Result<ContactArgs<'a>, ErrorKind> {
	let contact_args = match args.subcommand() {
		("add", Some(args)) => {
			ContactArgs::Add(required(args, "name")?, required(args, "address")?)
		}
		("remove", Some(args)) => ContactArgs::Remove(required(args, "name")?),
		(_, _) => {
			usage!(args);
		}
	};
	Ok(contact_args)
}

pub fn address_command(args: &ArgMatches) -> Result<AddressArgs, ErrorKind> {
	let address_args = if args.is_present("next") {
		AddressArgs::Next
	} else if args.is_present("prev") {
		AddressArgs::Prev
	} else if let Some(index) = args.value_of("index") {
		AddressArgs::Index(parse(index)?)
	} else {
		AddressArgs::Display
	};
	Ok(address_args)
}

pub fn seed_command(args: &ArgMatches) -> Result<SeedArgs, ErrorKind> {
	let seed_args = match args.subcommand() {
		("display", _) => SeedArgs::Display,
		("recover", _) => SeedArgs::Recover,
		(_, _) => {
			usage!(args);
		}
	};
	Ok(seed_args)
}
