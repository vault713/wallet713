use clap::{App, AppSettings, Arg, ArgGroup, ArgMatches, SubCommand};
use common::Result;

#[derive(Clone)]
pub struct Parser {}

impl<'a, 'b> Parser {
    pub fn parse(command: &str) -> Result<ArgMatches> {
        let command = command.trim();
        let matches = Parser::parser().get_matches_from_safe(command.split_whitespace())?;
        Ok(matches)
    }

    fn parser() -> App<'a, 'b> {
        App::new("")
            .setting(AppSettings::NoBinaryName)
            .subcommand(
                SubCommand::with_name("exit")
                    .about("exits wallet713 cli")
            )
            .subcommand(
                SubCommand::with_name("config")
                    .about("configures wallet713")
                    .arg(
                        Arg::from_usage("[generate-address] -g, --generate-next-address 'generate new grinbox address, supports optional index `-i`'")
                    )
                    .arg(
                        Arg::from_usage("[generate-address-index] -i, --index=<index> 'use this index for grinbox address generation'")
                    )
                    .arg(
                        Arg::from_usage("[data-path] -d, --data-path=<data path> 'the wallet data directory'")
                    )
                    .arg(
                        Arg::from_usage("[domain] --domain=<domain> 'the grinbox domain'")
                    )
                    .arg(
                        Arg::from_usage("[port] -p, --port=<port> 'the grinbox port'")
                    )
                    .arg(
                        Arg::from_usage("[node-uri] -n, --node-uri=<uri> 'the grin node uri'")
                    )
                    .arg(
                        Arg::from_usage("[node-secret] -s, --secret=<node-secret> 'the grin node api secret'")
                    )
            )
            .subcommand(
                SubCommand::with_name("address")
                    .about("shows your current grinbox address")
            )
            .subcommand(
                SubCommand::with_name("init")
                    .about("initializes the wallet")
                    .arg(
                        Arg::from_usage("[passphrase] -p, --passphrase=<passphrase> 'the passphrase to use'")
                            .min_values(0)
                    )
            )
            .subcommand(
                SubCommand::with_name("lock")
                    .about("locks the wallet")
            )
            .subcommand(
                SubCommand::with_name("unlock")
                    .about("unlocks the wallet")
                    .arg(
                        Arg::from_usage("[account] -a, --account=<account> 'the account to use'")
                    )
                    .arg(
                        Arg::from_usage("[passphrase] -p, --passphrase=<passphrase> 'the passphrase to use'")
                            .min_values(0)
                    )
            )
            .subcommand(
                SubCommand::with_name("account")
                    .about("create a new account or switch to an existing account")
                    .subcommand(
                        SubCommand::with_name("create")
                            .about("creates a new account")
                            .arg(
                                Arg::from_usage("<name> 'the account name'")
                            )
                    )
                    .subcommand(
                        SubCommand::with_name("switch")
                            .about("switches to the given account")
                            .arg(
                                Arg::from_usage("<name> 'the account name'")
                            )
                            .arg(
                                Arg::from_usage("[account] -a, --account=<account> 'the account to use'")
                            )
                            .arg(
                                Arg::from_usage("[passphrase] -p, --passphrase=<passphrase> 'the passphrase to use'")
                                    .min_values(0)
                            )
                    )
            )
            .subcommand(
                SubCommand::with_name("accounts")
                    .about("lists available accounts")
            )
            .subcommand(
                SubCommand::with_name("info")
                    .about("displays wallet info")
            )
            .subcommand(
                SubCommand::with_name("contacts")
                    .about("manages your list of known contacts")
                    .subcommand(
                        SubCommand::with_name("add")
                            .about("adds a new contact")
                            .arg(
                                Arg::from_usage("<name> 'the contact name'")
                            )
                            .arg(
                                Arg::from_usage("<address> 'the contact address'")
                            )
                    )
                    .subcommand(
                        SubCommand::with_name("remove")
                            .about("removes an existing contact")
                            .arg(
                                Arg::from_usage("<name> 'the contact name'")
                            )
                    )
            )
            .subcommand(
                SubCommand::with_name("txs")
                    .about("displays transactions")
            )
            .subcommand(
                SubCommand::with_name("outputs")
                    .about("displays outputs")
                    .arg(
                        Arg::from_usage("[show-spent] -s, --show-spent 'show spent outputs'")
                    )
            )
            .subcommand(
                SubCommand::with_name("listen")
                    .about("listens to incoming slates to your grinbox account or keybase")
                    .arg(
                        Arg::from_usage("[grinbox] -g, --grinbox 'start the grinbox listener'")
                    )
                    .arg(
                        Arg::from_usage("[keybase] -k, --keybase 'start the keybase listener'")
                    )
            )
            .subcommand(
                SubCommand::with_name("stop")
                    .about("stops the slate listener")
                    .arg(
                        Arg::from_usage("[grinbox] -g, --grinbox 'stop the grinbox listener'")
                    )
                    .arg(
                        Arg::from_usage("[keybase] -k, --keybase 'stop the keybase listener'")
                    )
            )
            .subcommand(
                SubCommand::with_name("send")
                    .about("sends grins to an address")
                    .arg(
                        Arg::from_usage("[to] -t, --to=<address> 'the address to send grins to'")
                    )
                    .arg(
                        Arg::from_usage("[file] -f, --file=<file> 'the file to store the slate in'")
                    )
                    .group(ArgGroup::with_name("destination")
                        .args(&["to", "file"])
                        .required(true)
                    )
                    .arg(
                        Arg::from_usage("<amount> 'the amount of grins to send'")
                    )
                    .arg(
                        Arg::from_usage("[strategy] -s, --strategy=<strategy> 'the input selection strategy (all/smallest). Default: smallest'")
                    )
                    .arg(
                        Arg::from_usage("[confirmations] -c, --confirmations=<confirmations> 'the number of confirmations required for inputs'")
                    )
                    .arg(
                        Arg::from_usage("[change-outputs] -o, --change-outputs=<change-outputs> 'the number of change outputs'")
                    )
                    .arg(
                        Arg::from_usage("[message] -g, --message=<message> 'the message to include in the tx'")
                    )
            )
            .subcommand(
                SubCommand::with_name("invoice")
                    .about("sends invoice to an address")
                    .arg(
                        Arg::from_usage("-t, --to=<address> 'the address to send grins to'")
                    )
                    .arg(
                        Arg::from_usage("<amount> 'the amount of grins to send'")
                    )
                    .arg(
                        Arg::from_usage("[outputs] -o, --outputs=<outputs> 'the number of outputs'")
                    )
            )
            .subcommand(
                SubCommand::with_name("repost")
                    .about("reposts an existing transaction.")
                    .arg(
                        Arg::from_usage("-i, --id=<id> 'the transaction id'")
                    )
            )
            .subcommand(
                SubCommand::with_name("cancel")
                    .about("cancels an existing transaction.")
                    .arg(
                        Arg::from_usage("-i, --id=<id> 'the transaction id'")
                    )
            )
            .subcommand(
                SubCommand::with_name("restore")
                    .about("restores your wallet from existing seed")
                    .arg(
                        Arg::from_usage("[passphrase] -p, --passphrase=<passphrase> 'the passphrase to use'")
                            .min_values(0)
                    )
            )
            .subcommand(
                SubCommand::with_name("recover")
                    .about("recover wallet from mnemonic or displays the current mnemonic")
                    .arg(
                        Arg::from_usage("[passphrase] -p, --passphrase=<passphrase> 'the passphrase to use'")
                            .min_values(0)
                    )
                    .arg(
                        Arg::from_usage("[words] -m, --mnemonic=<words>... 'the seed mnemonic'")
                    )
                    .arg(
                        Arg::from_usage("[display] -d, --display= 'display the current mnemonic'")
                    )
                    .group(ArgGroup::with_name("method")
                        .args(&["words", "display"])
                        .required(true)
                    )

            )
            .subcommand(
                SubCommand::with_name("receive")
                    .about("receives a sender initiated slate from file and produces signed slate")
                    .arg(
                        Arg::from_usage("-f, --file=<file> 'the slate file'")
                    )
            )
            .subcommand(
                SubCommand::with_name("finalize")
                    .about("finalizes a slate response file and posts the transaction")
                    .arg(
                        Arg::from_usage("-f, --file=<file> 'the slate file'")
                    )
            )
            .subcommand(
                SubCommand::with_name("check")
                    .about("checks a wallet's outputs against a live node, repairing and restoring missing outputs if required")
            )
            .subcommand(
                SubCommand::with_name("export-proof")
                    .about("exports a transaction proof to a file")
                    .arg(
                        Arg::from_usage("-i, --id=<id> 'the transaction id'")
                    )
                    .arg(
                        Arg::from_usage("-f, --file=<file> 'the file to write to'")
                    )
            )
            .subcommand(
                SubCommand::with_name("verify-proof")
                    .about("verifies a transaction proof")
                    .arg(
                        Arg::from_usage("-f, --file=<file> 'the file to read from'")
                    )
            )
    }
}
