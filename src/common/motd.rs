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

use clap::crate_version;
use colored::Colorize;
use failure::Error;
use epic_api::client;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::io;
use std::io::Write;

#[derive(Debug, Serialize, Deserialize)]
pub struct MOTD {
	#[serde(default)]
	pub message: Option<String>,
	#[serde(default)]
	pub update_message: Option<String>,
	#[serde(default)]
	pub urgent: Option<bool>,
	#[serde(default)]
	pub version: Option<Version>,
}

pub fn get_motd() -> Result<(), Error> {
	let crate_version = Version::parse(crate_version!())?;

	let motd: MOTD = client::get(
		"https://raw.githubusercontent.com/vault713/wallet713/master/motd.json",
		None,
	)?;

	if let Some(version) = motd.version {
		if version > crate_version {
			let update_message = match motd.update_message {
				None => String::new(),
				Some(um) => um,
			};

			println!(
				"{} {}",
				"A new version of wallet713 is available!".bold(),
				update_message
			);
			println!();
			println!("Upgrade by running:");
			println!(" curl https://wallet.713.mw/install.sh -sSf | sh");
			println!();
		}
	}

	if let Some(m) = motd.message {
		println!("{}", m.bold());
		println!();
	}

	if motd.urgent.unwrap_or(false) {
		println!("{}", "Press ENTER to continue".bright_red().bold());
		let mut line = String::new();
		io::stdout().flush().unwrap();
		io::stdin().read_line(&mut line).unwrap();
	}

	Ok(())
}
