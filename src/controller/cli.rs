use super::args::{
	self, AccountArgs, AddressArgs, ContactArgs, ProofArgs, SeedArgs, SendCommandType,
};
use super::display::{self, InitialPromptOption};
use crate::api::listener::ListenerInterface;
use crate::common::motd::get_motd;
use crate::common::{Arc, ErrorKind, Keychain, Mutex};
use crate::contacts::Address;
use crate::wallet::api::{Foreign, Owner};
use crate::wallet::types::{NodeClient, TxProof, VersionedSlate, WalletBackend};
use crate::wallet::Container;
use clap::{App, ArgMatches};
use colored::Colorize;
use failure::Error;
use grin_core::core::amount_to_hr_string;
use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::Hinter;
use rustyline::{CompletionType, Config, Context, EditMode, Editor, Helper, OutputStreamType};
use semver::Version;
use std::borrow::Cow::{self, Borrowed, Owned};
use std::fs::File;
use std::io::{Read, Write};

const COLORED_PROMPT: &'static str = "\x1b[36mwallet713>\x1b[0m ";
const PROMPT: &'static str = "wallet713> ";
const HISTORY_PATH: &str = ".history";

pub struct CLI<W, C, K>
where
	W: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
{
	api: Owner<W, C, K>,
	foreign: Foreign<W, C, K>,
}

impl<W, C, K> CLI<W, C, K>
where
	W: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
{
	pub fn new(container: Arc<Mutex<Container<W, C, K>>>) -> Self {
		Self {
			api: Owner::new(container.clone()),
			foreign: Foreign::new(container),
		}
	}

	pub fn start(&self) {
		match self.real_start() {
			Err(e) => display::error(e),
			Ok(_) => {}
		}
	}

	fn real_start(&self) -> Result<(), Error> {
		let has_seed = self.api.has_seed()?;

		if has_seed {
			self.api.set_password(display::password_prompt()?)?;
			println!(
				"{}",
				format!("\nWelcome to wallet713 v{}\n", crate_version!())
					.bright_yellow()
					.bold()
			);
			self.api.connect()?;
		} else if self.initial_prompt()? {
			return Ok(());
		}

		if self.api.config().check_updates() {
			let _ = get_motd();
		}

		if !self.check_node_version() {
			return Ok(());
		}

		println!("Use `help` to see available commands");
		println!();

		self.start_listeners()?;
		self.command_loop();
		Ok(())
	}

	fn initial_prompt(&self) -> Result<bool, Error> {
		match display::initial_prompt()? {
			InitialPromptOption::Init => {
				self.init_wallet()?;
				Ok(false)
			}
			InitialPromptOption::Recover => {
				self.recover_wallet(false)?;
				Ok(false)
			}
			InitialPromptOption::Exit => Ok(true),
		}
	}

	fn init_wallet(&self) -> Result<(), Error> {
		println!("{}", "Initialising a new wallet".bold());
		println!();
		println!(
			"Set an optional password to secure your wallet with. Leave blank for no password."
		);
		println!();
		let password = display::password_prompt()?;
		self.api.set_seed(None, password, false)?;
		display::mnemonic(self.api.get_seed()?, true);
		self.api.connect()?;
		Ok(())
	}

	fn recover_wallet(&self, overwrite: bool) -> Result<(), Error> {
		let mnemonic = display::mnemonic_prompt()?;
		println!();
		println!(
			"Set an optional password to secure your wallet with. Leave blank for no password."
		);
		println!();
		let password = display::password_prompt()?;
		self.api.set_seed(Some(mnemonic), password, overwrite)?;
		self.api.connect()?;
		self.api.clear()?;
		println!("Recovering wallet..");
		self.api.restore()?;
		println!("Wallet recovered successfully");
		Ok(())
	}

	fn check_node_version(&self) -> bool {
		if let Some(v) = self.api.node_version() {
			if Version::parse(&v.node_version) < Version::parse("2.0.0-beta.1") {
				let version = if v.node_version == "1.0.0" {
					"1.x.x series"
				} else {
					&v.node_version
				};
				println!("The Grin Node in use (version {}) is outdated and incompatible with this wallet version.", version);
				println!("Please update the node to version 2.0.0 or later and try again.");
				return false;
			}
		}
		true
	}

	fn start_listeners(&self) -> Result<(), Error> {
		let config = self.api.config();
		if config.grinbox_listener_auto_start() {
			if let Err(e) = self.api.start_listener(ListenerInterface::Grinbox) {
				display::error(e);
			}
		}
		if config.keybase_listener_auto_start() {
			if let Err(e) = self.api.start_listener(ListenerInterface::Keybase) {
				display::error(e);
			}
		}
		if config.foreign_api() {
			if let Err(e) = self.api.start_listener(ListenerInterface::ForeignHttp) {
				display::error(e);
			}
		}
		if config.owner_api() {
			if let Err(e) = self.api.start_listener(ListenerInterface::OwnerHttp) {
				display::error(e);
			}
		}

		Ok(())
	}

	fn command_loop(&self) {
		let editor = Config::builder()
			.history_ignore_space(true)
			.completion_type(CompletionType::List)
			.edit_mode(EditMode::Emacs)
			.output_stream(OutputStreamType::Stdout)
			.build();

		let mut reader = Editor::with_config(editor);
		reader.set_helper(Some(EditorHelper(
			FilenameCompleter::new(),
			MatchingBracketHighlighter::new(),
		)));

		let history_file = self
			.api
			.config()
			.get_data_path()
			.unwrap()
			.parent()
			.unwrap()
			.join(HISTORY_PATH);
		if history_file.exists() {
			let _ = reader.load_history(&history_file);
		}

		let yml = load_yaml!("commands.yml");
		let mut app = App::from_yaml(yml).version(crate_version!());

		loop {
			match reader.readline(PROMPT) {
				Ok(command) => {
					if command.is_empty() {
						continue;
					}

					let args = app.get_matches_from_safe_borrow(command.trim().split_whitespace());
					let done = match args {
						Ok(args) => match self.command(args) {
							Ok(done) => done,
							Err(err) => {
								cli_message!("{} {}", "Error:".bright_red(), err);
								false
							}
						},
						Err(err) => {
							match err.kind {
								clap::ErrorKind::HelpDisplayed => {
									cli_message!("{}", err);
								}
								_ => {
									cli_message!("{} {}", "Error:".bright_red(), err);
								}
							}
							false
						}
					};
					reader.add_history_entry(command);
					if done {
						println!();
						break;
					}
				}
				Err(err) => {
					println!("Unable to read line: {}", err);
					break;
				}
			}
		}

		let _ = reader.save_history(&history_file);
	}

	fn command(&self, args: ArgMatches) -> Result<bool, Error> {
		let home_dir = dirs::home_dir()
			.map(|p| p.to_str().unwrap().to_string())
			.unwrap_or("~".to_string());

		match args.subcommand() {
			("account", Some(m)) => match args::account_command(m)? {
				AccountArgs::Create(name) => {
					self.api.create_account_path(name)?;
					println!("Account '{}' created", name);
				}
				AccountArgs::Switch(name) => {
					self.api.set_active_account(name)?;
					println!("Switched to account '{}'", name);
				}
			},
			("accounts", _) => {
				display::accounts(self.api.accounts()?);
			}
			("address", Some(m)) => {
				let mut idx = self.api.config().grinbox_address_index();
				match args::address_command(m)? {
					AddressArgs::Display => {
						println!(
							"Your grinbox address is {}",
							self.api.grinbox_address()?.stripped().bright_green()
						);
					}
					AddressArgs::Next => {
						idx = idx.saturating_add(1);
						self.api.set_grinbox_address_index(idx)?;
					}
					AddressArgs::Prev => {
						idx = idx.saturating_sub(1);
						self.api.set_grinbox_address_index(idx)?;
					}
					AddressArgs::Index(i) => {
						idx = i;
						self.api.set_grinbox_address_index(idx)?;
					}
				};
				cli_message!(
					"Using grinbox address index {}",
					idx.to_string().bright_green()
				);
			}
			("cancel", Some(m)) => {
				let index = args::cancel_command(m)?;
				self.api.cancel_tx(Some(index), None)?;
				println!("Transaction cancelled successfully");
			}
			("check", Some(m)) => {
				let delete_unconfirmed = args::repair_command(m)?;
				println!("Checking and repairing wallet..");
				self.api.check_repair(delete_unconfirmed)?;
				println!("Wallet repaired successfully");
			}
			("contact", Some(m)) => match args::contact_command(m)? {
				ContactArgs::Add(name, address) => {
					self.api.add_contact(name, address)?;
					println!("Contact {} added", name.bright_green());
				}
				ContactArgs::Remove(name) => {
					self.api.remove_contact(name)?;
					println!("Contact {} removed", name.bright_green());
				}
			},
			("contacts", _) => {
				display::contacts(self.api.contacts()?);
			}
			("exit", _) => {
				let _ = self.api.stop_listeners();
				return Ok(true);
			}
			("finalize", Some(m)) => {
				let (file_name, fluff) = args::finalize_command(m)?;
				let mut file = File::open(file_name.replace("~", &home_dir))?;
				let mut slate = String::new();
				file.read_to_string(&mut slate)?;
				let slate: VersionedSlate =
					serde_json::from_str(&slate).map_err(|_| ErrorKind::ParseSlate)?;
				let slate = self.api.finalize_tx(&slate.into(), None)?;
				self.api.post_tx(&slate.tx, fluff)?;
				println!("Transaction finalized and posted successfully");
			}
			("info", _) => {
				let account = self.api.active_account()?;
				let (validated, wallet_info) = self.api.retrieve_summary_info(true, 10)?;
				display::info(&account, &wallet_info, validated, true);
			}
			("listen", Some(m)) => {
				let interface = match args::listen_command(m)? {
					("grinbox", _) | ("", _) => ListenerInterface::Grinbox,
					("keybase", _) => ListenerInterface::Keybase,
					("http", true) => ListenerInterface::OwnerHttp,
					("http", false) => ListenerInterface::ForeignHttp,
					_ => {
						return Err(ErrorKind::IncorrectListenerInterface.into());
					}
				};
				self.api.start_listener(interface)?;
			}
			("outputs", Some(m)) => {
				let account = self.api.active_account()?;
				let (validated, height, outputs) =
					self.api
						.retrieve_outputs(m.is_present("spent"), true, None)?;
				let height = match height {
					Some(h) => h,
					None => self.api.node_height()?.height,
				};
				display::outputs(&account, height, validated, outputs, true);
			}
			("proof", Some(m)) => {
				let (sender, receiver, amount, outputs, excess) = match args::proof_command(m)? {
					ProofArgs::Export(index, file_name) => {
						println!("A");
						let tx_proof = self
							.api
							.get_stored_tx_proof(Some(index), None)?
							.ok_or(ErrorKind::TransactionHasNoProof)?;
						println!("B");
						let verify = self.api.verify_tx_proof(&tx_proof)?;
						println!("C");
						let mut file = File::create(file_name.replace("~", &home_dir))?;
						file.write_all(serde_json::to_string(&tx_proof)?.as_bytes())?;
						println!("Proof exported to {}", file_name.bright_green());
						verify
					}
					ProofArgs::Verify(file_name) => {
						let mut file = File::open(file_name.replace("~", &home_dir))?;
						let mut tx_proof = String::new();
						file.read_to_string(&mut tx_proof)?;
						let tx_proof: TxProof = serde_json::from_str(&tx_proof)?;
						self.api.verify_tx_proof(&tx_proof)?
					}
				};
				display::proof(sender, receiver, amount, outputs, excess);
			}
			("receive", Some(m)) => {
				let (file_name, message) = args::receive_command(m)?;
				let mut file = File::open(file_name.replace("~", &home_dir))?;
				let mut slate = String::new();
				file.read_to_string(&mut slate)?;
				let slate: VersionedSlate =
					serde_json::from_str(&slate).map_err(|_| ErrorKind::ParseSlate)?;
				let version = slate.version().clone();
				let slate = slate.into();
				let slate = self.foreign.receive_tx(
					&slate,
					None,
					Some("file".to_owned()),
					message.map(|m| m.to_owned()),
				)?;
				let mut file_out =
					File::create(&format!("{}.response", file_name.replace("~", &home_dir)))?;
				let slate = VersionedSlate::into_version(slate, version);
				file_out.write_all(serde_json::to_string(&slate)?.as_bytes())?;
				cli_message!(
					"Response slate file {} created successfully",
					format!("{}.response", file_name.bright_green())
				);
			}
			("repost", Some(m)) => {
				let (index, fluff) = args::repost_command(m)?;
				let slate_id = self.api.repost_tx(Some(index), None, fluff)?;
				println!(
					"Transaction {} reposted successfully",
					slate_id.to_string().bright_green()
				);
			}
			("restore", _) => {
				println!("Restoring wallet..");
				self.api.restore()?;
				println!("Wallet restored successfully");
			}
			("seed", Some(m)) => {
				match args::seed_command(m)? {
					SeedArgs::Display => {
						display::mnemonic(self.api.get_seed()?, false);
					}
					SeedArgs::Recover => {
						self.api.stop_listeners()?;
						self.api.disconnect()?;
						self.recover_wallet(true)?;
					}
				};
			}
			("send", Some(m)) => {
				let (cmd_type, args) = args::send_command(m)?;

				match cmd_type {
					SendCommandType::Address => {
						self.api.init_send_tx(args)?;
					}
					SendCommandType::File(file_name) => {
						let slate = self.api.init_send_tx(args)?;
						let mut file = File::create(file_name.replace("~", &home_dir))?;
						file.write_all(serde_json::to_string_pretty(&slate)?.as_bytes())?;
						self.api
							.tx_lock_outputs(&slate, 0, Some("file".to_owned()))?;

						println!(
							"Slate {} for {} grin saved to {}",
							slate.id.to_string().bright_green(),
							amount_to_hr_string(slate.amount, false).bright_green(),
							file_name.bright_green()
						);
					}
					SendCommandType::Estimate => {
						let strategies = vec!["smallest", "all"]
							.into_iter()
							.map(|strategy| {
								let mut init_args = args.clone();
								init_args.selection_strategy_is_use_all = strategy == "all";
								let slate = self.api.init_send_tx(init_args).unwrap();
								(strategy, slate.amount, slate.fee)
							})
							.collect();
						display::estimate(args.amount, strategies, true);
					}
				}
			}
			("stop", Some(m)) => {
				let interface = match args::listen_command(m)? {
					("grinbox", _) | ("", _) => ListenerInterface::Grinbox,
					("keybase", _) => ListenerInterface::Keybase,
					("http", true) => ListenerInterface::OwnerHttp,
					("http", false) => ListenerInterface::ForeignHttp,
					_ => {
						return Err(ErrorKind::IncorrectListenerInterface.into());
					}
				};
				self.api.stop_listener(interface)?;
			}
			("txs", _) => {
				let account = self.api.active_account()?;
				let (validated, height, txs, contacts, proofs) =
					self.api.retrieve_txs(true, true, true, None, None)?;
				let height = match height {
					Some(h) => h,
					None => self.api.node_height()?.height,
				};
				display::txs(
					&account, height, validated, &txs, proofs, contacts, true, true,
				);
			}
			_ => {
				cli_message!("Unknown command");
			}
		}

		Ok(false)
	}
}

struct EditorHelper(FilenameCompleter, MatchingBracketHighlighter);

impl Completer for EditorHelper {
	type Candidate = Pair;

	fn complete(
		&self,
		line: &str,
		pos: usize,
		ctx: &Context<'_>,
	) -> std::result::Result<(usize, Vec<Pair>), ReadlineError> {
		self.0.complete(line, pos, ctx)
	}
}

impl Hinter for EditorHelper {
	fn hint(&self, _line: &str, _pos: usize, _ctx: &Context<'_>) -> Option<String> {
		None
	}
}

impl Highlighter for EditorHelper {
	fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
		self.1.highlight(line, pos)
	}

	fn highlight_prompt<'b, 's: 'b, 'p: 'b>(
		&'s self,
		prompt: &'p str,
		default: bool,
	) -> Cow<'b, str> {
		if default {
			Borrowed(COLORED_PROMPT)
		} else {
			Borrowed(prompt)
		}
	}

	fn highlight_hint<'h>(&self, hint: &'h str) -> Cow<'h, str> {
		Owned("\x1b[1m".to_owned() + hint + "\x1b[m")
	}

	fn highlight_char(&self, line: &str, pos: usize) -> bool {
		self.1.highlight_char(line, pos)
	}
}

impl Helper for EditorHelper {}
