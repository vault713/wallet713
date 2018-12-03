#[macro_use] extern crate serde_derive;
extern crate clap;
extern crate colored;
extern crate ws;
extern crate futures;
extern crate tokio;
extern crate secp256k1;
extern crate rand;
extern crate sha2;
extern crate digest;

extern crate grin_wallet;
extern crate grin_keychain;
extern crate grin_util;
extern crate grin_core;

use clap::ArgMatches;
use colored::*;

use grin_core::{core};

#[macro_use] mod common;
mod grinbox;
mod wallet;
mod cli;

use common::config::Wallet713Config;
use common::error::Error;
use common::crypto::*;
use wallet::Wallet;
use cli::Parser;

fn config(args: &ArgMatches, silent: bool) -> Result<Wallet713Config, Error> {
	let mut config;
	let mut any_matches = false;
    let exists = Wallet713Config::exists();
	if exists {
		config = Wallet713Config::from_file()?;
	} else {
		config = Wallet713Config::default()?;
	}

    if let Some(data_path) = args.value_of("data-path") {
        config.wallet713_data_path = data_path.to_string();
        any_matches = true;
    }

	if let Some(uri) = args.value_of("uri") {
		config.grinbox_uri = uri.to_string();
		any_matches = true;
	}
	
    if let Some(account) = args.value_of("private-key") {
        config.grinbox_private_key = account.to_string();
        any_matches = true;
    }

    if let Some(node_uri) = args.value_of("node-uri") {
        config.grin_node_uri = node_uri.to_string();
        any_matches = true;
    }

    if let Some(node_secret) = args.value_of("node-secret") {
        config.grin_node_secret = Some(node_secret.to_string());
        any_matches = true;
    }

    if !exists || args.is_present("generate-keys") {
        let (pr, _) = generate_keypair();
        config.grinbox_private_key = pr.to_string();
        any_matches = exists;
    }

	config.to_file()?;

    if !any_matches && !silent {
        cli_message!("{}", config);
    }

    Ok(config)
}

fn listen(wallet: &mut Wallet, password: &str) -> Result<(), Error> {
	if Wallet713Config::exists() {
		let config = Wallet713Config::from_file().map_err(|_| {
            Error::generic("could not load config!")
        })?;
		if config.grinbox_private_key.is_empty() {
            Err(Error::generic("grinbox keypair not set!"))
		} else if config.grinbox_uri.is_empty() {
            Err(Error::generic("grinbox uri not set!"))
		} else {
            wallet.start_client(password, &config.grinbox_uri[..], &config.grinbox_private_key[..])?;
		    Ok(())
        }
	} else {
		Err(Error::generic(NO_CONFIG))
	}
}

const WELCOME_HEADER: &str = r#"
Welcome to wallet713

"#;

const NO_CONFIG: &str = r#"
Wallet713 config not found!
Use `config` command to set one up
"#;


const WELCOME_FOOTER: &str = r#"Use `listen` to connect to grinbox or `help` to see available commands
"#;

fn welcome() -> Result<(), Error> {
    let config = config(&ArgMatches::new(), true)?;

    let secret_key = SecretKey::from_hex(&config.grinbox_private_key)?;
    let public_key = common::crypto::public_key_from_secret_key(&secret_key);
    let public_key = public_key.to_base58_check(common::crypto::BASE58_CHECK_VERSION_GRIN_TX.to_vec());

	print!("{}", WELCOME_HEADER.bright_yellow().bold());
    println!("{}: {}", "Your 713.grinbox address".bright_yellow(), public_key.bright_green());
	println!("{}", WELCOME_FOOTER.bright_blue().bold());

    Ok(())
}

fn handle<T>(result: Result<T, Error>) {
    if let Err(e) = result {
        cli_message!("{}", e);
    }
}

fn main() {
	welcome().unwrap_or_else(|e| {
        panic!("{}: could not read or create config! {}", "ERROR".bright_red(), e);
    });

    let mut wallet = Wallet::new();
    let account = "default".to_owned();
    loop {
        let account = account.clone();
        cli_message!();
        let mut command = String::new();
        std::io::stdin().read_line(&mut command).expect("oops!");

        let result = Parser::parse(&command[..]);
        match result {
            Ok(matches) => {
                match matches.subcommand_name() {
                    None => {},
                    Some("exit") => std::process::exit(0),
                    Some("config") => {
                        handle(config(matches.subcommand_matches("config").unwrap(), false));
                    },
                    Some("init") => {
                        let password = matches.subcommand_matches("init").unwrap().value_of("password").unwrap_or("");
                        handle(wallet.init(password));
                    },
                    Some("listen") => {
                        let password = matches.subcommand_matches("listen").unwrap().value_of("password").unwrap_or("");
                        handle(listen(&mut wallet, password));
                    },
                    Some("subscribe") => {
                        handle(wallet.subscribe());
                    },
                    Some("unsubscribe") => {
                        handle(wallet.unsubscribe());
                    },
                    Some("stop") => {
                        handle(wallet.stop_client());
                    },
                    Some("info") => {
                        let password = matches.subcommand_matches("info").unwrap().value_of("password").unwrap_or("");
                        handle(wallet.info(password, &account[..]));
                    },
                    Some("txs") => {
                        let password = matches.subcommand_matches("txs").unwrap().value_of("password").unwrap_or("");
                        handle(wallet.txs(password, &account[..]));
                    },
                    Some("outputs") => {
                        let password = matches.subcommand_matches("outputs").unwrap().value_of("password").unwrap_or("");
                        let show_spent = matches.subcommand_matches("outputs").unwrap().is_present("show-spent");
                        handle(wallet.outputs(password, &account[..], show_spent));
                    },
                    Some("repost") => {
                        let password = matches.subcommand_matches("repost").unwrap().value_of("password").unwrap_or("");
                        let id = matches.subcommand_matches("repost").unwrap().value_of("id").unwrap();
                        let id = id.parse::<u32>().map_err(|_| {
                            Error::generic("invalid id!")
                        });
                        match id {
                            Ok(id) => handle(wallet.repost(password, id, false)),
                            Err(e) => cli_message!("{}", e),
                        }
                    },
                    Some("cancel") => {
                        let password = matches.subcommand_matches("cancel").unwrap().value_of("password").unwrap_or("");
                        let id = matches.subcommand_matches("cancel").unwrap().value_of("id").unwrap();
                        let id = id.parse::<u32>().map_err(|_| {
                            Error::generic("invalid id!")
                        });
                        match id {
                            Ok(id) => handle(wallet.cancel(password, id)),
                            Err(e) => cli_message!("{}", e),
                        }
                    },
                    Some("send") => {
                        let args = matches.subcommand_matches("send").unwrap();
                        let password = args.value_of("password").unwrap_or("");
                        let to = args.value_of("to").unwrap();
                        let amount = args.value_of("amount").unwrap();
                        let amount = core::amount_from_hr_string(amount).map_err(|_| {
                           Error::generic("invalid amount given!")
                        });

                        if let Err(e) = amount {
                            cli_message!("{}", e);
                            continue;
                        }

                        let amount = amount.unwrap();
                        let result = wallet.send(password, &account[..], to, amount, 10, "all", 1, 500);
                        match result {
                            Ok(slate) => {
                                cli_message!("slate [{}] for [{}] grins sent successfully to [{}]",
                                        slate.id.to_string().bright_green(),
                                        core::amount_to_hr_string(slate.amount, false).bright_green(),
                                        to.bright_green()
                                    );
                            },
                            Err(e) => cli_message!("{}", e)
                        }
                    },
                    Some("restore") => {
                        let password = matches.subcommand_matches("restore").unwrap().value_of("password").unwrap_or("");
                        handle(wallet.restore(password));
                    },
                    Some("challenge") => {
                        cli_message!("{}", wallet.client.get_challenge());
                    },
                    Some(subcommand) => {
                        cli_message!("{}: subcommand `{}` not implemented!", "ERROR".bright_red(), subcommand.bright_green());
                    },
                }
            },
            Err(err) => cli_message!("{}", err),
        }
    }
}
