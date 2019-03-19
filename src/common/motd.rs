use colored::Colorize;
use failure::Error;
use grin_api::client;
use semver::Version;
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
    pub version: Option<String>,
}

pub fn get_motd() -> Result<(), Error> {
    let crate_version = Version::parse(crate_version!())?;

    let motd: MOTD = client::get(
        "https://raw.githubusercontent.com/jaspervdm/wallet713/qol_update/motd.json",
        None,
    )?;

    if let Some(v) = motd.version {
        let version = Version::parse(&v)?;
        if version > crate_version {
            let update_message = match motd.update_message {
                None => String::new(),
                Some(um) => um,
            };

            println!("{}{}", "A new version of wallet713 is available!".bold(), update_message);
            println!();
            println!("Upgrade by running:");
            println!(" curl https://install.wallet.713.mw -sSf | sh");
            println!();

            if motd.urgent.unwrap_or(false) {
                println!("{}", "Press ENTER to continue".bright_red().bold());
                let mut line = String::new();
                io::stdout().flush().unwrap();
                io::stdin().read_line(&mut line).unwrap();
            }
        }
    }

    if let Some(m) = motd.message {
        println!("{}", m.bold());
        println!();
    }

    Ok(())
}
