#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate prettytable;
#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_json;
#[macro_use]
extern crate gotham_derive;
#[macro_use]
extern crate clap;
extern crate env_logger;
extern crate blake2_rfc;
extern crate chrono;
extern crate ansi_term;
extern crate colored;
extern crate digest;
extern crate dirs;
extern crate easy_jsonrpc;
extern crate failure;
extern crate futures;
extern crate gotham;
extern crate hmac;
extern crate hyper;
extern crate mime;
extern crate parking_lot;
extern crate rand;
extern crate regex;
extern crate ring;
extern crate ripemd160;
extern crate rpassword;
extern crate rustyline;
extern crate serde;
extern crate sha2;
extern crate term;
extern crate tokio;
extern crate url;
extern crate uuid;
extern crate ws;
extern crate semver;

extern crate grin_api;
#[macro_use]
extern crate grin_core;
extern crate grin_keychain;
extern crate grin_store;
extern crate grin_util;

use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::path::Path;

use clap::{App, Arg, ArgMatches};
use colored::*;
use grin_core::core;
use grin_core::global::{is_mainnet, set_mining_mode, ChainTypes};
use url::Url;

mod api;
mod broker;
mod cli;
#[macro_use]
mod common;
mod contacts;
mod controller;
mod internal;
mod wallet;

//use api::router::{build_foreign_api_router, build_owner_api_router};
use cli::Parser;
use common::config::Wallet713Config;
use common::{ErrorKind, Result, RuntimeMode, COLORED_PROMPT, PROMPT, post};
use wallet::Wallet;

use crate::wallet::types::{Arc, Mutex, Slate, TxProof};
use crate::common::motd::get_motd;

use contacts::{Address, AddressBook, AddressType, Backend, Contact, GrinboxAddress};

const CLI_HISTORY_PATH: &str = ".history";

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

    config.to_file(config_path)?;

    if !any_matches && !silent {
        cli_message!("{}", config);
    }

    Ok(config)
}

/*fn do_contacts(args: &ArgMatches, address_book: Arc<Mutex<AddressBook>>) -> Result<()> {
    let mut address_book = address_book.lock();
    if let Some(add_args) = args.subcommand_matches("add") {
        let name = add_args.value_of("name").expect("missing argument: name");
        let address = add_args
            .value_of("address")
            .expect("missing argument: address");

        // try parse as a general address and fallback to grinbox address
        let contact_address = Address::parse(address);
        let contact_address: Result<Box<Address>> = match contact_address {
            Ok(address) => Ok(address),
            Err(e) => {
                Ok(Box::new(GrinboxAddress::from_str(address).map_err(|_| e)?) as Box<Address>)
            }
        };

        let contact = Contact::new(name, contact_address?)?;
        address_book.add_contact(&contact)?;
    } else if let Some(add_args) = args.subcommand_matches("remove") {
        let name = add_args.value_of("name").unwrap();
        address_book.remove_contact(name)?;
    } else {
        let contacts: Vec<()> = address_book
            .contacts()
            .map(|contact| {
                cli_message!("@{} = {}", contact.name, contact.address);
                ()
            })
            .collect();

        if contacts.len() == 0 {
            cli_message!(
                "your contact list is empty. consider using `contacts add` to add a new contact."
            );
        }
    }
    Ok(())
}*/

const WELCOME_FOOTER: &str = r#"Use `help` to see available commands
"#;

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

use broker::{
    CloseReason, GrinboxPublisher, GrinboxSubscriber, KeybasePublisher, KeybaseSubscriber,
    Publisher, Subscriber, SubscriptionHandler,
};
use std::borrow::Borrow;


/*fn start_grinbox_listener(
    config: &Wallet713Config,
    wallet: Arc<Mutex<Wallet>>,
    address_book: Arc<Mutex<AddressBook>>,
) -> Result<(GrinboxPublisher, GrinboxSubscriber, std::thread::JoinHandle<()>)> {
    // make sure wallet is not locked, if it is try to unlock with no passphrase
    {
        let mut wallet = wallet.lock();
        if wallet.is_locked() {
            wallet.unlock(config, "default", "")?;
        }
    }

    cli_message!("starting grinbox listener...");
    let grinbox_address = config.get_grinbox_address()?;
    let grinbox_secret_key = config.get_grinbox_secret_key()?;

    let grinbox_publisher = GrinboxPublisher::new(
        &grinbox_address,
        &grinbox_secret_key,
        config.grinbox_protocol_unsecure(),
    )?;

    let grinbox_subscriber = GrinboxSubscriber::new(
        &grinbox_publisher
    )?;

    let cloned_publisher = grinbox_publisher.clone();
    let mut cloned_subscriber = grinbox_subscriber.clone();
    let grinbox_listener_handle = std::thread::spawn(move || {
        let controller = Controller::new(
            &grinbox_address.stripped(),
            wallet.clone(),
            address_book.clone(),
            Box::new(cloned_publisher),
        )
        .expect("could not start grinbox controller!");
        cloned_subscriber
            .start(Box::new(controller))
            .expect("something went wrong!");
    });
    Ok((grinbox_publisher, grinbox_subscriber, grinbox_listener_handle))
}

fn start_keybase_listener(
    config: &Wallet713Config,
    wallet: Arc<Mutex<Wallet>>,
    address_book: Arc<Mutex<AddressBook>>,
) -> Result<(KeybasePublisher, KeybaseSubscriber, std::thread::JoinHandle<()>)> {
    // make sure wallet is not locked, if it is try to unlock with no passphrase
    {
        let mut wallet = wallet.lock();
        if wallet.is_locked() {
            wallet.unlock(config, "default", "")?;
        }
    }

    cli_message!("starting keybase listener...");
    let keybase_subscriber = KeybaseSubscriber::new()?;
    let keybase_publisher = KeybasePublisher::new(config.default_keybase_ttl.clone())?;

    let mut cloned_subscriber = keybase_subscriber.clone();
    let cloned_publisher = keybase_publisher.clone();
    let keybase_listener_handle = std::thread::spawn(move || {
        let controller = Controller::new(
            "keybase",
            wallet.clone(),
            address_book.clone(),
            Box::new(cloned_publisher),
        )
        .expect("could not start keybase controller!");
        cloned_subscriber
            .start(Box::new(controller))
            .expect("something went wrong!");
    });
    Ok((keybase_publisher, keybase_subscriber, keybase_listener_handle))
}*/


fn main() {
    enable_ansi_support();

    let matches = App::new("wallet713")
        .version(crate_version!())
        .arg(Arg::from_usage("[config-path] -c, --config=<config-path> 'the path to the config file'"))
        .arg(Arg::from_usage("[log-config-path] -l, --log-config-path=<log-config-path> 'the path to the log config file'"))
        .arg(Arg::from_usage("[account] -a, --account=<account> 'the account to use'"))
        .arg(Arg::from_usage("[passphrase] -p, --passphrase=<passphrase> 'the passphrase to use'").min_values(0))
        .arg(Arg::from_usage("[daemon] -d, --daemon 'run daemon'"))
        .arg(Arg::from_usage("[floonet] -f, --floonet 'use floonet'"))
        .get_matches();

    let runtime_mode = match matches.is_present("daemon") {
        true => RuntimeMode::Daemon,
        false => RuntimeMode::Cli,
    };

    let mut config: Wallet713Config = welcome(&matches, &runtime_mode).unwrap_or_else(|e| {
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

    if config.check_updates() {
        get_motd().unwrap_or(());
    }

    // TODO: clean this up
    use grin_keychain::ExtKeychain;
    use wallet::{Container, create_container};
    use wallet::types::{HTTPNodeClient, WalletSeed};


//    let backend = wallet::Backend::<HTTPNodeClient, ExtKeychain>::new(&wallet_config, "", node_client).unwrap();
    let container = create_container(config, Some(address_book)).unwrap();

    use controller::cli::CLI;
    let cli = CLI::new(container);
    cli.start();

    return;


    let wallet = Wallet::new(config.max_auto_accept_invoice);
    let wallet = Arc::new(Mutex::new(wallet));

    let mut grinbox_broker: Option<(GrinboxPublisher, GrinboxSubscriber)> = None;
    let mut keybase_broker: Option<(KeybasePublisher, KeybaseSubscriber)> = None;

    /*let has_seed = Wallet::seed_exists(&config);
    if !has_seed {
        let mut line = String::new();

        println!("{}", "Please choose an option".bright_green().bold());
        println!(" 1) {} a new wallet", "init".bold());
        println!(" 2) {} from mnemonic", "recover".bold());
        println!(" 3) {}", "exit".bold());
        println!();
        print!("{}", "> ".cyan());
        io::stdout().flush().unwrap();

        if io::stdin().read_line(&mut line).unwrap() == 0 {
            println!("{}: invalid option", "ERROR".bright_red());
            std::process::exit(1);
        }

        println!();

        let line = line.trim();
        let mut out_is_safe = false;
        match line {
            "1" | "init" | "" => {
                println!("{}", "Initialising a new wallet".bold());
                println!();
                println!("Set an optional password to secure your wallet with. Leave blank for no password.");
                println!();
                if let Err(err) = do_command("init -p", &mut config, wallet.clone(), address_book.clone(), &mut keybase_broker, &mut grinbox_broker, &mut out_is_safe) {
                    println!("{}: {}", "ERROR".bright_red(), err);
                    std::process::exit(1);
                }
            },
            "2" | "recover" | "restore" => {
                println!("{}", "Recovering from mnemonic".bold());
                print!("Mnemonic: ");
                io::stdout().flush().unwrap();
                let mut line = String::new();
                if io::stdin().read_line(&mut line).unwrap() == 0 {
                    println!("{}: invalid mnemonic", "ERROR".bright_red());
                    std::process::exit(1);
                }
                let line = line.trim();
                println!();
                println!("Set an optional password to secure your wallet with. Leave blank for no password.");
                println!();
                // TODO: refactor this
                let cmd = format!("recover -m {} -p", line);
                if let Err(err) = do_command(&cmd, &mut config, wallet.clone(), address_book.clone(), &mut keybase_broker, &mut grinbox_broker, &mut out_is_safe) {
                    println!("{}: {}", "ERROR".bright_red(), err);
                    std::process::exit(1);
                }
            },
            "3" | "exit" => {
                return;
            },
            _ => {
                println!("{}: invalid option", "ERROR".bright_red());
                std::process::exit(1);
            },
        }

        println!();
    }*/

    /*if wallet.lock().is_locked() {
        let account = matches.value_of("account").unwrap_or("default").to_string();
        let has_wallet = if matches.is_present("passphrase") {
            let passphrase = password_prompt(matches.value_of("passphrase"));
            let result = wallet.lock().unlock(&config, &account, &passphrase);
            if let Err(ref err) = result {
                println!("{}: {}", "ERROR".bright_red(), err);
                std::process::exit(1);
            }
            result.is_ok()
        }
        else {
            wallet.lock().unlock(&config, &account, "").is_ok()
        };

        if has_wallet {
            let der = derive_address_key(&mut config, wallet.clone(), &mut grinbox_broker);
            if der.is_err() {
                cli_message!("{}: {}", "ERROR".bright_red(), der.unwrap_err());
            }
        }
        else {
            println!(
                "{}",
                "Unlock your existing wallet or type `init` to initiate a new one"
                    .bright_blue()
                    .bold()
            );
        }
    }*/

    cli_message!("{}", WELCOME_FOOTER.bright_blue());

    let mut grinbox_listener_handle: Option<std::thread::JoinHandle<()>> = None;
    let mut keybase_listener_handle: Option<std::thread::JoinHandle<()>> = None;
    let mut owner_api_handle: Option<std::thread::JoinHandle<()>> = None;
    let mut foreign_api_handle: Option<std::thread::JoinHandle<()>> = None;

    /*if config.grinbox_listener_auto_start() {
        let result = start_grinbox_listener(&config, wallet.clone(), address_book.clone());
        match result {
            Err(e) => cli_message!("{}: {}", "ERROR".bright_red(), e),
            Ok((publisher, subscriber, handle)) => {
                grinbox_broker = Some((publisher, subscriber));
                grinbox_listener_handle = Some(handle);
            },
        }
    }

    if config.keybase_listener_auto_start() {
        let result = start_keybase_listener(&config, wallet.clone(), address_book.clone());
        match result {
            Err(e) => cli_message!("{}: {}", "ERROR".bright_red(), e),
            Ok((publisher, subscriber, handle)) => {
                keybase_broker = Some((publisher, subscriber));
                keybase_listener_handle = Some(handle);
            },
        }
    }

    if config.owner_api() || config.foreign_api() {
        owner_api_handle = match config.owner_api {
            Some(true) => {
                cli_message!(
                    "starting listener for owner api on [{}]",
                    config.owner_api_address().bright_green()
                );
                if config.owner_api_secret.is_none() {
                    cli_message!(
                        "{}: no api secret for owner api, it is recommended to set one.",
                        "WARNING".bright_yellow()
                    );
                }
                let router = build_owner_api_router(
                    wallet.clone(),
                    grinbox_broker.clone(),
                    keybase_broker.clone(),
                    config.owner_api_secret.clone(),
                    config.owner_api_include_foreign,
                );
                let address = config.owner_api_address();
                Some(std::thread::spawn(move || {
                    gotham::start(address, router);
                }))
            }
            _ => None,
        };

        foreign_api_handle = match config.foreign_api {
            Some(true) => {
                cli_message!(
                    "starting listener for foreign api on [{}]",
                    config.foreign_api_address().bright_green()
                );
                if config.foreign_api_secret.is_none() {
                    cli_message!(
                        "{}: no api secret for foreign api, it is recommended to set one.",
                        "WARNING".bright_yellow()
                    );
                }
                let router = build_foreign_api_router(
                    wallet.clone(),
                    grinbox_broker.clone(),
                    keybase_broker.clone(),
                    config.foreign_api_secret.clone(),
                );
                let address = config.foreign_api_address();
                Some(std::thread::spawn(move || {
                    gotham::start(address, router);
                }))
            }
            _ => None,
        };
    };

    if runtime_mode == RuntimeMode::Daemon {
        let mut listening = false;
        if let Some(handle) = grinbox_listener_handle {
            handle.join().unwrap();
            listening = true;
        }

        if let Some(handle) = keybase_listener_handle {
            handle.join().unwrap();
            listening = true;
        }

        if let Some(handle) = owner_api_handle {
            handle.join().unwrap();
            listening = true;
        }

        if let Some(handle) = foreign_api_handle {
            handle.join().unwrap();
            listening = true;
        }

        if !listening {
            warn!("no listener configured, exiting");
        }

        return;
    }*/

    /*let wallet713_home_path_buf = Wallet713Config::default_home_path(&config.chain).unwrap();
    let wallet713_home_path = wallet713_home_path_buf.to_str().unwrap();

    if let Some(path) = Path::new(wallet713_home_path)
        .join(CLI_HISTORY_PATH)
        .to_str()
    {
        rl.load_history(path).is_ok();
    }*/

    /*loop {
        let command = rl.readline(PROMPT);
        match command {
            Ok(command) => {
                let command = command.trim();

                if command == "exit" {
                    break;
                }

                let mut out_is_safe = false;
                let result = do_command(
                    &command,
                    &mut config,
                    wallet.clone(),
                    address_book.clone(),
                    &mut keybase_broker,
                    &mut grinbox_broker,
                    &mut out_is_safe,
                );

                if let Err(err) = result {
                    cli_message!("{}", err);
                }

                if out_is_safe {
                    rl.add_history_entry(command);
                }
            }
            Err(_) => {
                break;
            }
        }
    }*/

    /*if let Some(path) = Path::new(wallet713_home_path)
        .join(CLI_HISTORY_PATH)
        .to_str()
    {
        rl.save_history(path).is_ok();
    }*/
}

/*fn derive_address_key(
    config: &mut Wallet713Config,
    wallet: Arc<Mutex<Wallet>>,
    grinbox_broker: &mut Option<(GrinboxPublisher, GrinboxSubscriber)>,
) -> Result<()> {
    if grinbox_broker.is_some() {
        return Err(ErrorKind::HasListener.into());
    }
    let index = config.grinbox_address_index();
    let key = wallet.lock().derive_address_key(index)?;
    config.grinbox_address_key = Some(key);
    show_address(config, false)?;
    Ok(())
}

fn show_address(config: &Wallet713Config, include_index: bool) -> Result<()> {
    println!(
        "{}: {}",
        "Your grinbox address".bright_yellow(),
        config.get_grinbox_address()?.stripped().bright_green()
    );
    if include_index {
        println!(
            "Derived with index [{}]",
            config.grinbox_address_index().to_string().bright_blue()
        );
    }
    Ok(())
}*/

/*fn do_command(
    command: &str,
    config: &mut Wallet713Config,
    wallet: Arc<Mutex<Wallet>>,
    address_book: Arc<Mutex<AddressBook>>,
    keybase_broker: &mut Option<(KeybasePublisher, KeybaseSubscriber)>,
    grinbox_broker: &mut Option<(GrinboxPublisher, GrinboxSubscriber)>,
    out_is_safe: &mut bool,
) -> Result<()> {
    *out_is_safe = true;
    let home_dir = dirs::home_dir()
        .map(|p| p.to_str().unwrap().to_string())
        .unwrap_or("~".to_string());

    let matches = Parser::parse(command)?;
    match matches.subcommand_name() {
        Some("config") => {
            let args = matches.subcommand_matches("config").unwrap();

            let new_address_index = match args.is_present("generate-address") {
                false => None,
                true => Some({
                    let index = match args.value_of("generate-address-index") {
                        Some(index) => u32::from_str_radix(index, 10)
                            .map_err(|_| ErrorKind::NumberParsingError)?,
                        None => config.grinbox_address_index() + 1,
                    };
                    config.grinbox_address_index = Some(index);
                    index
                }),
            };

            *config = do_config(
                args,
                &config.chain,
                false,
                new_address_index,
                config.config_home.as_ref().map(|x| &**x),
            )?;

            if new_address_index.is_some() {
                derive_address_key(config, wallet, grinbox_broker)?;
                cli_message!(
                    "Derived with index [{}]",
                    config.grinbox_address_index().to_string().bright_blue()
                );
            }
        }
        Some("address") => {
            show_address(config, true)?;
        }
        Some("contacts") => {
            let arg_matches = matches.subcommand_matches("contacts").unwrap();
            do_contacts(&arg_matches, address_book.clone())?;
        }
        Some("invoice") => {
            let args = matches.subcommand_matches("invoice").unwrap();
            let to = args.value_of("to").unwrap();
            let outputs = args.value_of("outputs").unwrap_or("1");
            let outputs = usize::from_str_radix(outputs, 10)
                .map_err(|_| ErrorKind::InvalidNumOutputs(outputs.to_string()))?;
            let amount = args.value_of("amount").unwrap();
            let amount = core::amount_from_hr_string(amount)
                .map_err(|_| ErrorKind::InvalidAmount(amount.to_string()))?;

            let mut to = to.to_string();
            let mut display_to = None;
            if to.starts_with("@") {
                let contact = address_book.lock().get_contact(&to[1..])?;
                to = contact.get_address().to_string();
                display_to = Some(contact.get_name().to_string());
            }

            // try parse as a general address
            let address = Address::parse(&to);
            let address: Result<Box<Address>> = match address {
                Ok(address) => Ok(address),
                Err(e) => {
                    Ok(Box::new(GrinboxAddress::from_str(&to).map_err(|_| e)?) as Box<Address>)
                }
            };

            let to = address?;
            if display_to.is_none() {
                display_to = Some(to.stripped());
            }
            let slate: Result<Slate> = match to.address_type() {
                AddressType::Keybase => {
                    if let Some((publisher, _)) = keybase_broker {
                        let slate = wallet.lock().initiate_receive_tx(amount, outputs)?;
                        publisher.post_slate(&slate, to.borrow())?;
                        Ok(slate)
                    } else {
                        Err(ErrorKind::ClosedListener("keybase".to_string()))?
                    }
                }
                AddressType::Grinbox => {
                    if let Some((publisher, _)) = grinbox_broker {
                        let slate = wallet.lock().initiate_receive_tx(amount, outputs)?;
                        publisher.post_slate(&slate, to.borrow())?;
                        Ok(slate)
                    } else {
                        Err(ErrorKind::ClosedListener("grinbox".to_string()))?
                    }
                }
                _ => Err(ErrorKind::HttpRequest.into()),
            };

            let slate = slate?;
            cli_message!(
                "invoice slate [{}] for [{}] grins sent successfully to [{}]",
                slate.id.to_string().bright_green(),
                core::amount_to_hr_string(slate.amount, false).bright_green(),
                display_to.unwrap().bright_green()
            );
        }
        Some("recover") => {
            *out_is_safe = false;
            if keybase_broker.is_some() || grinbox_broker.is_some() {
                return Err(ErrorKind::HasListener.into());
            }
            let args = matches.subcommand_matches("recover").unwrap();
            let passphrase = match args.is_present("passphrase") {
                true => password_prompt(args.value_of("passphrase")),
                false => "".to_string(),
            };
            *out_is_safe = args.value_of("passphrase").is_none();

            if let Some(words) = args.values_of("words") {
                println!("recovering... please wait as this could take a few minutes to complete.");
                let words: Vec<&str> = words.collect();
                {
                    let mut w = wallet.lock();
                    w.restore_seed(config, &words, passphrase.as_str())?;
                    w.init(config, "default", passphrase.as_str(), false)?;
                    w.restore_state()?;
                }

                derive_address_key(config, wallet, grinbox_broker)?;
                if passphrase.is_empty() {
                    println!("{}: wallet with no passphrase.", "WARNING".bright_yellow());
                }

                println!("wallet restoration done!");
                *out_is_safe = false;
                return Ok(());
            } else if args.is_present("display") {
                wallet.lock().show_mnemonic(config, &passphrase)?;
                return Ok(());
            }
        }
    };

    Ok(())
}*/

#[cfg(windows)]
pub fn enable_ansi_support() {
    if !ansi_term::enable_ansi_support().is_ok() {
        colored::control::set_override(false);
    }
}

#[cfg(not(windows))]
pub fn enable_ansi_support() {
}