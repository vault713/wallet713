use colored::Colorize;
use failure::Error;
use grin_api::client;
use semver::Version;

#[derive(Debug, Serialize, Deserialize)]
pub struct MOTD {
    #[serde(default)]
    pub message: Option<String>,
    pub version: Option<String>,
}

pub fn get_motd() -> Result<(), Error> {
    let crate_version = Version::parse(crate_version!())?;
    println!("crate version: {:?}", crate_version);

    let motd: MOTD = client::get(
        "https://raw.githubusercontent.com/jaspervdm/wallet713/qol_update/motd.json",
        None,
    )?;

    if let Some(m) = motd.message {
        println!("{}", m.bold());
    }

    if let Some(v) = motd.version {
        let version = Version::parse(&v)?;
        if version > crate_version {
            println!("A new version of wallet713 is available, updating is recommended");
        }
    }

    Ok(())
}
