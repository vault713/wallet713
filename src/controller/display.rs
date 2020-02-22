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

use crate::common::ErrorKind;
use crate::contacts::{Contact, GrinboxAddress};
use crate::wallet::types::{
	AcctPathMapping, OutputCommitMapping, OutputStatus, TxLogEntry, WalletInfo,
};
use clap::crate_version;
use colored::Colorize;
use failure::Error;
use grin_core::core::amount_to_hr_string;
use grin_core::global::{coinbase_maturity, is_mainnet};
use grin_util::secp::pedersen::Commitment;
use grin_util::{to_hex, ZeroingString};
use prettytable::format::consts::{FORMAT_NO_BORDER_LINE_SEPARATOR, FORMAT_NO_COLSEP};
use prettytable::{cell, row, table};
use rpassword::prompt_password_stdout;
use std::collections::HashMap;
use std::fmt::Display;
use std::io::{self, Write};
use std::ops::Deref;
use uuid::Uuid;

pub enum InitialPromptOption {
	Init,
	Recover,
	Exit,
}

pub fn password_prompt() -> Result<ZeroingString, Error> {
	let password = match prompt_password_stdout("Password: ") {
		Ok(p) => p,
		Err(_) => {
			return Err(
				ErrorKind::GenericError("Unable to read password prompt".to_owned()).into(),
			);
		}
	};

	Ok(password.into())
}

pub fn error<D>(msg: D)
where
	D: Display,
{
	println!("{} {}", "ERROR:".bright_red(), msg);
}

///
pub fn initial_prompt() -> Result<InitialPromptOption, Error> {
	println!(
		"{}",
		format!("\nWelcome to wallet713 v{}\n", crate_version!())
			.bright_yellow()
			.bold()
	);

	println!("{}", "Please choose an option".bright_green().bold());
	println!(" 1) {} a new wallet", "init".bold());
	println!(" 2) {} from mnemonic", "recover".bold());
	println!(" 3) {}", "exit".bold());
	println!();
	print!("{}", "> ".cyan());
	io::stdout().flush().unwrap();

	let mut line = String::new();
	if io::stdin().read_line(&mut line).unwrap() == 0 {
		return Err(ErrorKind::GenericError("Invalid option".to_owned()).into());
	}
	println!();
	let line = line.trim();
	Ok(match line {
		"1" | "init" | "" => InitialPromptOption::Init,
		"2" | "recover" | "restore" => InitialPromptOption::Recover,
		"3" | "exit" => InitialPromptOption::Exit,
		_ => {
			return Err(ErrorKind::GenericError("Invalid option".to_owned()).into());
		}
	})
}

pub fn mnemonic_prompt() -> Result<ZeroingString, Error> {
	println!("{}", "Recovering from mnemonic".bold());
	print!("Enter your mnemonic: ");
	io::stdout().flush().unwrap();

	let mut line = String::new();
	if io::stdin().read_line(&mut line).unwrap() == 0 {
		return Err(ErrorKind::GenericError("Invalid mnemonic".to_owned()).into());
	}
	let line = line.trim();
	Ok(line.into())
}

pub fn mnemonic(mnemonic: ZeroingString, confirm: bool) {
	println!("Your recovery phrase is:");
	println!();
	println!("{}", mnemonic.deref());
	if confirm {
		println!();
		println!("Please back-up these words in a non-digital format.");
		println!(
			"{}",
			"Press ENTER when you have done so".bright_green().bold()
		);
		let mut line = String::new();
		io::stdout().flush().unwrap();
		io::stdin().read_line(&mut line).unwrap();
	}
}

/// Display summary info in a pretty way
pub fn estimate(
	amount: u64,
	strategies: Vec<(
		&str, // strategy
		u64,  // total amount to be locked
		u64,  // fee
	)>,
	dark_background_color_scheme: bool,
) {
	println!(
		"\n____ Estimation for sending {} ____\n",
		amount_to_hr_string(amount, true)
	);

	let mut table = table!();

	table.set_titles(row![
		bMG->"Selection strategy",
		bMG->"Fee",
		bMG->"Amount locked",
	]);

	for (strategy, total, fee) in strategies {
		if dark_background_color_scheme {
			table.add_row(row![
				bFC->strategy,
				FR->amount_to_hr_string(fee, true),
				FY->amount_to_hr_string(total, false),
			]);
		} else {
			table.add_row(row![
				bFD->strategy,
				FR->amount_to_hr_string(fee, true),
				FY->amount_to_hr_string(total, false),
			]);
		}
	}
	table.set_format(*FORMAT_NO_COLSEP);
	table.printstd();
	println!();
}

/// Display list of wallet accounts in a pretty way
pub fn accounts(acct_mappings: Vec<AcctPathMapping>) {
	println!("\n____ Wallet Accounts ____\n",);
	let mut table = table!();

	table.set_titles(row![
		mMG->"Name",
		bMG->"Parent BIP-32 Derivation Path",
	]);
	for m in acct_mappings {
		table.add_row(row![
			bFC->m.label,
			bGC->m.path.to_bip_32_string(),
		]);
	}
	table.set_format(*FORMAT_NO_BORDER_LINE_SEPARATOR);
	table.printstd();
	println!();
}

/// Display outputs in a pretty way
pub fn outputs(
	account: &str,
	cur_height: u64,
	validated: bool,
	outputs: Vec<OutputCommitMapping>,
	dark_background_color_scheme: bool,
) {
	println!(
		"\n____ Wallet Outputs - Account '{}' - Height {} ____\n",
		account, cur_height
	);

	let mut table = table!();

	table.set_titles(row![
		bMG->"Output Commitment",
		bMG->"Block Height",
		bMG->"Locked Until",
		bMG->"Status",
		bMG->"Coinbase?",
		bMG->"# Confirms",
		bMG->"Value",
		bMG->"Tx"
	]);

	for m in outputs {
		let commit = format!("{}", to_hex(m.commit.as_ref().to_vec()));
		let height = format!("{}", m.output.height);
		let lock_height = if m.output.lock_height > 0 {
			format!("{}", m.output.lock_height)
		} else {
			"".to_owned()
		};
		let is_coinbase = if m.output.is_coinbase { "yes" } else { "" }.to_owned();

		// Mark unconfirmed coinbase outputs as "Mining" instead of "Unconfirmed"
		let status = match m.output.status {
			OutputStatus::Unconfirmed if m.output.is_coinbase => "Mining".to_string(),
			_ => format!("{}", m.output.status),
		};

		let num_confirmations = format!("{}", m.output.num_confirmations(cur_height));
		let value = format!("{}", amount_to_hr_string(m.output.value, false));
		let tx = match m.output.tx_log_entry {
			None => "".to_owned(),
			Some(t) => t.to_string(),
		};

		if dark_background_color_scheme {
			table.add_row(row![
				bFC->commit,
				bFB->height,
				bFB->lock_height,
				bFR->status,
				bFY->is_coinbase,
				bFB->num_confirmations,
				bFG->value,
				bFC->tx,
			]);
		} else {
			table.add_row(row![
				bFD->commit,
				bFB->height,
				bFB->lock_height,
				bFR->status,
				bFD->is_coinbase,
				bFB->num_confirmations,
				bFG->value,
				bFD->tx,
			]);
		}
	}

	table.set_format(*FORMAT_NO_COLSEP);
	table.printstd();
	println!();

	if !validated {
		println!(
			"\nWARNING: Wallet failed to verify data. \
			 The above is from local cache and possibly invalid! \
			 (is your `grin server` offline or broken?)"
		);
	}
}

/// Display transaction log in a pretty way
pub fn txs(
	account: &str,
	cur_height: u64,
	validated: bool,
	txs: &Vec<TxLogEntry>,
	proofs: HashMap<Uuid, bool>,
	contacts: HashMap<String, String>,
	include_status: bool,
	dark_background_color_scheme: bool,
) {
	println!(
		"\n____ Transaction Log - Account '{}' - Height {} ____\n",
		account, cur_height
	);

	let mut table = table!();

	table.set_titles(row![
		bMG->"Index",
		bMG->"Type",
		bMG->"TXID",
		bMG->"Address",
		bMG->"Creation Time",
		bMG->"Confirmed?",
		bMG->"Confirmation Time",
		bMG->"Amount",
		bMG->"Fee",
		bMG->"Proof?",
	]);

	for t in txs {
		let id = format!("{}", t.id);
		let entry_type = format!("{}", t.tx_type);
		let slate_id = match &t.tx_slate_id {
			Some(m) => to_hex(m.as_bytes()[..4].to_vec()),
			None => "".to_owned(),
		};
		let address = match &t.address {
			Some(a) => match contacts.get(a) {
				Some(c) => format!("@{}", c),
				None => a.clone(),
			},
			None => "".to_owned(),
		};
		let creation_ts = format!("{}", t.creation_ts.format("%Y-%m-%d %H:%M:%S"));
		let confirmed = if t.confirmed { "yes" } else { "" }.to_owned();
		let confirmation_ts = match t.confirmation_ts {
			Some(m) => format!("{}", m.format("%Y-%m-%d %H:%M:%S")),
			None => "".to_owned(),
		};
		let mut amount: i64 = t.amount_credited as i64 - t.amount_debited as i64;
		if let Some(fee) = t.fee {
			amount += fee as i64;
		}
		let amount = if amount > 0 {
			format!(" {}", amount_to_hr_string(amount as u64, true))
		} else {
			format!("-{}", amount_to_hr_string((-amount) as u64, true))
		};
		let fee = match t.fee {
			Some(f) => amount_to_hr_string(f, true),
			None => "".to_owned(),
		};
		let proof = match &t.tx_slate_id {
			Some(m) if proofs.contains_key(m) => "yes".to_owned(),
			_ => "".to_owned(),
		};
		if dark_background_color_scheme {
			table.add_row(row![
				bFC->id,
				bFC->entry_type,
				bFB->slate_id,
				bFY->address,
				bFB->creation_ts,
				bFG->confirmed,
				bFB->confirmation_ts,
				bFY->amount,
				bFC->fee,
				bFG->proof,
			]);
		} else {
			table.add_row(row![
				bFD->id,
				bFb->entry_type,
				bFB->slate_id,
				bFG->address,
				bFB->creation_ts,
				bFg->confirmed,
				bFB->confirmation_ts,
				bFG->amount,
				bFD->fee,
				bFg->proof,
			]);
		}
	}

	table.set_format(*FORMAT_NO_COLSEP);
	table.printstd();
	println!();

	if !validated && include_status {
		println!(
			"\nWARNING: Wallet failed to verify data. \
			 The above is from local cache and possibly invalid! \
			 (is your `grin server` offline or broken?)"
		);
	}
}

/// Display summary info in a pretty way
pub fn info(
	account: &str,
	wallet_info: &WalletInfo,
	validated: bool,
	dark_background_color_scheme: bool,
) {
	println!(
		"\n____ Wallet Summary Info - Account '{}' - Height {} ____\n",
		account, wallet_info.last_confirmed_height,
	);

	let mut table = table!();

	if dark_background_color_scheme {
		table.add_row(row![
			bFG->"Confirmed Total",
			FG->amount_to_hr_string(wallet_info.total, false)
		]);
		// Only dispay "Immature Coinbase" if we have related outputs in the wallet.
		// This row just introduces confusion if the wallet does not receive coinbase rewards.
		if wallet_info.amount_immature > 0 {
			table.add_row(row![
				bFY->format!("Immature Coinbase (< {})", coinbase_maturity()),
				FY->amount_to_hr_string(wallet_info.amount_immature, false)
			]);
		}
		table.add_row(row![
			bFY->format!("Awaiting Confirmation (< {})", wallet_info.minimum_confirmations),
			FY->amount_to_hr_string(wallet_info.amount_awaiting_confirmation, false)
		]);
		table.add_row(row![
			bFB->format!("Awaiting Finalization"),
			FB->amount_to_hr_string(wallet_info.amount_awaiting_finalization, false)
		]);
		table.add_row(row![
			Fr->"Locked by previous transaction",
			Fr->amount_to_hr_string(wallet_info.amount_locked, false)
		]);
		table.add_row(row![
			Fw->"--------------------------------",
			Fw->"-------------"
		]);
		table.add_row(row![
			bFG->"Currently Spendable",
			FG->amount_to_hr_string(wallet_info.amount_currently_spendable, false)
		]);
	} else {
		table.add_row(row![
			bFG->"Total",
			FG->amount_to_hr_string(wallet_info.total, false)
		]);
		// Only dispay "Immature Coinbase" if we have related outputs in the wallet.
		// This row just introduces confusion if the wallet does not receive coinbase rewards.
		if wallet_info.amount_immature > 0 {
			table.add_row(row![
				bFB->format!("Immature Coinbase (< {})", coinbase_maturity()),
				FB->amount_to_hr_string(wallet_info.amount_immature, false)
			]);
		}
		table.add_row(row![
			bFB->format!("Awaiting Confirmation (< {})", wallet_info.minimum_confirmations),
			FB->amount_to_hr_string(wallet_info.amount_awaiting_confirmation, false)
		]);
		table.add_row(row![
			Fr->"Locked by previous transaction",
			Fr->amount_to_hr_string(wallet_info.amount_locked, false)
		]);
		table.add_row(row![
			Fw->"--------------------------------",
			Fw->"-------------"
		]);
		table.add_row(row![
			bFG->"Currently Spendable",
			FG->amount_to_hr_string(wallet_info.amount_currently_spendable, false)
		]);
	};
	table.set_format(*FORMAT_NO_BORDER_LINE_SEPARATOR);
	table.printstd();
	println!();
	if !validated {
		println!(
			"\nWARNING: Wallet failed to verify data against a live chain. \
			 The above is from local cache and only valid up to the given height! \
			 (is your `grin server` offline or broken?)"
		);
	}
}

pub fn proof(
	sender: GrinboxAddress,
	receiver: GrinboxAddress,
	amount: u64,
	outputs: Vec<Commitment>,
	excess: Commitment,
) {
	let outputs = outputs
		.iter()
		.map(|o| to_hex(o.0.to_vec()))
		.collect::<Vec<_>>();
	let excess = to_hex(excess.0.to_vec());

	println!(
		"This file proves that {} grin was sent to {} from {}",
		amount_to_hr_string(amount, false).bright_green(),
		format!("{}", receiver).bright_green(),
		format!("{}", sender).bright_green()
	);

	println!("\nOutputs:");
	for output in outputs {
		println!("   {}", output.bright_magenta());
	}
	println!("Kernel excess:");
	println!("   {}", excess.bright_magenta());
	println!("\n{}: this proof should only be considered valid if the kernel is actually on-chain with sufficient confirmations", "WARNING".bright_yellow());
	println!("Please use a grin block explorer to verify this is the case. for example:");
	let prefix = match is_mainnet() {
		true => "",
		false => "floonet.",
	};
	cli_message!("   https://{}grinscan.net/kernel/{}", prefix, excess);
}

/// Display list of contacts in a pretty way
pub fn contacts(contacts: Vec<Contact>) {
	println!("\n____ Contacts ____\n",);
	let mut table = table!();

	table.set_titles(row![
		mMG->"Name",
		bMG->"Address",
	]);
	for c in contacts {
		table.add_row(row![
			bFC->c.name,
			bGC->c.address,
		]);
	}
	table.set_format(*FORMAT_NO_BORDER_LINE_SEPARATOR);
	table.printstd();
	println!();
}
