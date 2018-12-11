use std::str::FromStr;
use std::fmt::{self, Display};

use common::{Error, Wallet713Error};
use regex::Regex;

#[derive(Debug, Clone)]
pub struct GrinboxAddress {
    pub public_key: String,
    pub domain: Option<String>,
    pub port: Option<u16>,
}

impl GrinboxAddress {
    pub fn local_to(&self, other: &GrinboxAddress) -> bool {
        let domain_match = match (&self.domain, &other.domain) {
            (None, None) => true,
            (None, Some(_)) => false,
            (Some(_), None) => false,
            (Some(a), Some(b)) => a == b
        };

        let port_match = match (&self.port, &other.port) {
            (None, None) => true,
            (None, Some(_)) => false,
            (Some(_), None) => false,
            (Some(a), Some(b)) => a == b
        };

        domain_match && port_match
    }
}

impl FromStr for GrinboxAddress {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let re = Regex::new(r"^(?P<public_key>[123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz]{52})(@(?P<domain>[a-zA-Z0-9\.]+)(:(?P<port>[0-9]*))?)?$").unwrap();
        let captures = re.captures(s);
        if captures.is_none() {
            Err(Wallet713Error::GrinboxAddressParsingError(s.to_string()))?;
        }

        let captures = captures.unwrap();
        let public_key = captures.name("public_key").unwrap().as_str().to_string();
        let domain = captures.name("domain").map(|m| m.as_str().to_string());
        let port = captures.name("port").map(|m| u16::from_str_radix(m.as_str(), 10).unwrap());
        Ok(Self {
            public_key,
            domain,
            port
        })
    }
}

impl Display for GrinboxAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.public_key)?;
        if let Some(ref domain) = self.domain {
            write!(f, "@{}", domain)?;
            if let Some(ref port) = self.port {
                write!(f, ":{}", port)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::GrinboxAddress;
    use std::str::FromStr;

    #[test]
    fn can_parse_grinbox_address_full() {
        let address_str = "xd6A7NwpB2yDevoShkZLPorZB2h7Aivf9JyjkngKywgzrog2VpnU@grinbox.io:5555";
        let address = GrinboxAddress::from_str(address_str).unwrap();
        assert_eq!("xd6A7NwpB2yDevoShkZLPorZB2h7Aivf9JyjkngKywgzrog2VpnU", address.public_key);
        assert_eq!(Some("grinbox.io".to_string()), address.domain);
        assert_eq!(Some(5555), address.port);
        assert_eq!(format!("{}", address), address_str);
    }

    #[test]
    fn can_parse_grinbox_address_no_port() {
        let address_str = "xd6A7NwpB2yDevoShkZLPorZB2h7Aivf9JyjkngKywgzrog2VpnU@grinbox.io";
        let address = GrinboxAddress::from_str(address_str).unwrap();
        assert_eq!("xd6A7NwpB2yDevoShkZLPorZB2h7Aivf9JyjkngKywgzrog2VpnU", address.public_key);
        assert_eq!(Some("grinbox.io".to_string()), address.domain);
        assert_eq!(None, address.port);
        assert_eq!(format!("{}", address), address_str);
    }

    #[test]
    fn can_parse_grinbox_address_no_domain() {
        let address_str = "xd6A7NwpB2yDevoShkZLPorZB2h7Aivf9JyjkngKywgzrog2VpnU";
        let address = GrinboxAddress::from_str(address_str).unwrap();
        assert_eq!("xd6A7NwpB2yDevoShkZLPorZB2h7Aivf9JyjkngKywgzrog2VpnU", address.public_key);
        assert_eq!(None, address.domain);
        assert_eq!(None, address.port);
        assert_eq!(format!("{}", address), address_str);
    }
}