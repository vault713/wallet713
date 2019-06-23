use clap::{App, ArgMatches};
use colored::Colorize;
use failure::Error;
use grin_core::core::amount_to_hr_string;
use rpassword::prompt_password_stdout;
use rustyline::completion::{Completer, FilenameCompleter, Pair};
use rustyline::error::ReadlineError;
use rustyline::highlight::{Highlighter, MatchingBracketHighlighter};
use rustyline::hint::Hinter;
use rustyline::{CompletionType, Config, EditMode, Editor, Helper, OutputStreamType};
use std::borrow::Cow::{self, Borrowed, Owned};
use std::fs::File;
use std::io::{Read, Write};
use crate::common::{Arc, ErrorKind, Keychain, Mutex};
use crate::wallet::api::owner::Owner;
use crate::wallet::types::{NodeClient, Slate, WalletBackend};
use crate::wallet::Container;
use super::args::{self, AccountArgs, SendCommandType};
use super::display::{self, InitialPromptOption};

const COLORED_PROMPT: &'static str = "\x1b[36mwallet713>\x1b[0m ";
const PROMPT: &'static str = "wallet713> ";

pub struct CLI<W, C, K>
where
	W: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
{
    api: Owner<W, C, K>
}

impl<W, C, K> CLI<W, C, K>
where
	W: WalletBackend<C, K>,
	C: NodeClient,
	K: Keychain,
{
    pub fn new(container: Arc<Mutex<Container<W, C, K>>>) -> Self {
        Self {
            api: Owner::new(container),
        }
    }

    pub fn start(&self) {
        match self.real_start() {
            Err(e) => display::error(e),
            Ok(_) => {},
        }
    }

    fn real_start(&self) -> Result<(), Error> {
        let has_seed = self.api.has_seed()?;

        if has_seed {
            self.api.set_password(display::password_prompt()?)?;
            self.api.connect()?;
        }
        else if self.initial_prompt()? {
            return Ok(());
        }

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
                self.recover_wallet()?;
                Ok(false)
            }
            InitialPromptOption::Exit => {
                Ok(true)
            }
        }
    }

    fn init_wallet(&self) -> Result<(), Error> {
        println!("{}", "Initialising a new wallet".bold());
        println!();
        println!("Set an optional password to secure your wallet with. Leave blank for no password.");
        println!();
        let password = display::password_prompt()?;
        self.api.set_seed(None, password)?;
        display::mnemonic(self.api.get_seed()?);
        self.api.connect()?;
        Ok(())
    }

    fn recover_wallet(&self) -> Result<(), Error> {
        let mnemonic = display::mnemonic_prompt()?;
        println!();
	    println!("Set an optional password to secure your wallet with. Leave blank for no password.");
	    println!();
	    let password = display::password_prompt()?;
        self.api.set_seed(Some(mnemonic), password)?;
        self.api.connect()?;
        self.api.clear()?;
        println!("Restoring wallet..");
        self.api.restore()?;
        println!("Wallet restored successfully");
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

        let yml = load_yaml!("commands.yml");
        let mut app = App::from_yaml(yml);

        for command in reader.iter(PROMPT) {
            match command {
                Ok(command) => {
                    if command.is_empty() {
                        continue;
                    }

                    let args = app.get_matches_from_safe_borrow(command.trim().split_whitespace());
                    match args {
                        Ok(args) => {
                            match self.command(args) {
                                Ok(done) => {
                                    if done {
                                        break;
                                    }
                                },
                                Err(err) => {
                                    cli_message!("{} {}", "Error:".bright_red(), err);
                                }
                            };
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
                        }
                    };
                },
                Err(err) => {
                    println!("Unable to read line: {}", err);
                    break;
                }
            }
        }
    }

    fn command(&self, args: ArgMatches) -> Result<bool, Error> {
        let home_dir = dirs::home_dir()
            .map(|p| p.to_str().unwrap().to_string())
            .unwrap_or("~".to_string());

        match args.subcommand() {
            ("account", Some(m)) => {
                match args::account_command(m)? {
                    AccountArgs::Create(name) => {
                        self.api.create_account_path(name)?;
                        cli_message!("Account '{}' created", name);
                    },
                    AccountArgs::Switch(name) => {
                        self.api.set_active_account(name)?;
                        cli_message!("Switched to account '{}'", name);
                    }
                }
            }
            ("accounts", _) => {
                display::accounts(self.api.accounts()?);
            }
            ("cancel", Some(m)) => {
                let index = args::cancel_command(m)?;
                self.api.cancel_tx(Some(index), None)?;
                cli_message!("Transaction cancelled successfully");
            }
            ("check", Some(m)) => {
                let delete_unconfirmed = args::repair_command(m)?;
                println!("Checking and repairing wallet..");
                self.api.check_repair(delete_unconfirmed)?;
                cli_message!("Wallet repaired successfully");
            }
            ("exit", _) => {
                return Ok(true);
            }
            ("finalize", Some(m)) => {
                let (file_name, fluff) = args::finalize_command(m)?;
                let mut file = File::open(file_name.replace("~", &home_dir))?;
                let mut slate = String::new();
                file.read_to_string(&mut slate)?;
                let mut slate = Slate::deserialize_upgrade(&slate)?;
                let slate = self.api.finalize_tx(&slate)?;
                self.api.post_tx(&slate.tx, fluff)?;
                cli_message!("Transaction finalized and posted successfully");
            }
            ("info", _) => {
                let account = self.api.active_account()?;
                let (validated, wallet_info) = self.api.retrieve_summary_info(true, 10)?;
                display::info(&account, &wallet_info, validated, true);

            }
            ("listen", Some(m)) => {
                let t = args::listen_command(m)?;
                match t {
                    "grinbox" | "" => {
                        self.api.start_grinbox_listener()?;
                    },
                    _ => {
                        return Err(ErrorKind::UnknownListenerType(t.to_owned()).into());
                    }
                }
            }
            ("outputs", Some(m)) => {
                let account = self.api.active_account()?;
                let (validated, height, outputs) = self.api.retrieve_outputs(m.is_present("spent"), true, None)?;
                let height = match height {
                    Some(h) => h,
                    None => self.api.node_height()?.1
                };
                display::outputs(&account, height, validated, outputs, true);
            }
            ("repost", Some(m)) => {
                let (index, fluff) = args::repost_command(m)?;
                self.api.repost_tx(Some(index), None, fluff)?;
                cli_message!("Transaction reposted successfully");
            }
            ("restore", _) => {
                println!("Restoring wallet..");
                self.api.restore()?;
                cli_message!("Wallet restored successfully");
            }
            ("send", Some(m)) => {
                let (cmd_type, args) = args::send_command(m)?;

                match cmd_type {
                    SendCommandType::Address => {
                        let dest = args.send_args.as_ref().unwrap().dest.clone();
                        let slate = self.api.init_send_tx(args)?;
                        cli_message!(
                            "Slate {} for {} grin sent successfully to {}",
                            slate.id.to_string().bright_green(),
                            amount_to_hr_string(slate.amount, false).bright_green(),
                            dest.bright_green()
                        );
                    }
                    SendCommandType::File(file_name) => {
                        let slate = self.api.init_send_tx(args)?;
                        let mut file = File::create(file_name.replace("~", &home_dir))?;
                        file.write_all(serde_json::to_string_pretty(&slate)?.as_bytes())?;
                        self.api.tx_lock_outputs(&slate, 0, Some("file".to_owned()))?;

                        cli_message!(
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
                let t = args::listen_command(m)?;
                match t {
                    "grinbox" | "" => {
                        self.api.stop_grinbox_listener()?;
                    },
                    _ => {
                        return Err(ErrorKind::UnknownListenerType(t.to_owned()).into());
                    }
                }
            }
            ("txs", _) => {
                let account = self.api.active_account()?;
                let (validated, height, txs, contacts, proofs) = self.api.retrieve_txs(true, true, true, None, None)?;
                let height = match height {
                    Some(h) => h,
                    None => self.api.node_height()?.1
                };
                display::txs(&account, height, validated, &txs, proofs, contacts, true, true);
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
    ) -> std::result::Result<(usize, Vec<Pair>), ReadlineError> {
        self.0.complete(line, pos)
    }
}

impl Hinter for EditorHelper {
    fn hint(&self, _line: &str, _pos: usize) -> Option<String> {
        None
    }
}

impl Highlighter for EditorHelper {
    fn highlight<'l>(&self, line: &'l str, pos: usize) -> Cow<'l, str> {
        self.1.highlight(line, pos)
    }

    fn highlight_prompt<'p>(&self, prompt: &'p str) -> Cow<'p, str> {
        if prompt == PROMPT {
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