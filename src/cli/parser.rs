use clap::{App, AppSettings, SubCommand, Arg, ArgMatches};
use common::Error;

#[derive(Clone)]
pub struct Parser {}

impl<'a, 'b> Parser {
    pub fn parse(command: &str) -> Result<ArgMatches, Error> {
        let command = command.trim();
        let matches = Parser::parser().get_matches_from_safe(command.split_whitespace())?;
        Ok(matches)
    }

    fn parser() -> App<'a, 'b> {
        App::new("")
            .setting(AppSettings::NoBinaryName)
            .subcommand(
                SubCommand::with_name("challenge")
                    .about("outputs the current challenge")
            )
            .subcommand(
                SubCommand::with_name("exit")
                    .about("exits wallet713 cli")
            )
            .subcommand(
                SubCommand::with_name("config")
                    .about("configures wallet713")
                    .arg(
                        Arg::from_usage("-g, --generate-keys 'generate new set of grinbox keys'")
                    )
                    .arg(
                        Arg::from_usage("[data-path] -d, --data-path=<data path> 'the wallet data directory'")
                    )
                    .arg(
                        Arg::from_usage("[uri] -u, --uri=<URI> 'the grinbox uri'")
                    )
                    .arg(
                        Arg::from_usage("[private-key] --private-key=<private-key> 'the grinbox private key'")
                    )
                    .arg(
                        Arg::from_usage("[node-uri] -n, --node-uri=<uri> 'the grin node uri'")
                    )
                    .arg(
                        Arg::from_usage("[node-secret] -s, --secret=<node-secret> 'the grin node api secret'")
                    )
            )
            .subcommand(
                SubCommand::with_name("init")
                    .about("initializes the wallet")
                    .arg(
                        Arg::from_usage("[password] -p, --password=<password> 'the password to use'")
                    )
            )
            .subcommand(
                SubCommand::with_name("info")
                    .about("displays wallet info")
                    .arg(
                        Arg::from_usage("[password] -p, --password=<password> 'the password to use'")
                    )
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
                                Arg::from_usage("<public-key> 'the contact public key'")
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
                        Arg::from_usage("[show-spent] -spent, --show-spent 'show spent outputs'")
                    )
            )
            .subcommand(
                SubCommand::with_name("listen")
                    .about("listens to incoming slates to your grinbox account")
                    .arg(
                        Arg::from_usage("[password] -p, --password=<password> 'the password to use'")
                    )
            )
            .subcommand(
                SubCommand::with_name("subscribe")
                    .about("subscribes to incoming slates")
            )
            .subcommand(
                SubCommand::with_name("unsubscribe")
                    .about("removes incoming slates subscription")
            )
            .subcommand(
                SubCommand::with_name("stop")
                    .about("stops the slate listener")
            )
            .subcommand(
                SubCommand::with_name("send")
                    .about("sends grins to a grinbox subject")
                    .arg(
                        Arg::from_usage("-t, --to=<subject> 'the subject to send grins to'")
                    )
                    .arg(
                        Arg::from_usage("<amount> 'the amount of grins to send'")
                    )
            )
            .subcommand(
                SubCommand::with_name("repost")
                    .about("reposts an existing transaction.")
                    .arg(
                        Arg::from_usage("-i, --id=<id> 'the transaction id'")
                    )
                    .arg(
                        Arg::from_usage("[password] -p, --password=<password> 'the password to use'")
                    )
            )
            .subcommand(
                SubCommand::with_name("cancel")
                    .about("cancels an existing transaction.")
                    .arg(
                        Arg::from_usage("-i, --id=<id> 'the transaction id'")
                    )
                    .arg(
                        Arg::from_usage("[password] -p, --password=<password> 'the password to use'")
                    )
            )
            .subcommand(
                SubCommand::with_name("restore")
                    .about("restores your wallet from existing seed")
            )
    }
}