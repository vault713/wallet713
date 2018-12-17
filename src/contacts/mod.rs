mod types;
mod backend;
pub use self::backend::LMDBBackend;
pub use self::types::{Address, AddressType, GrinboxAddress, KeybaseAddress, Contact, AddressBook, };

#[cfg(test)]
mod test {
    use super::{Contact, Address, AddressType, GrinboxAddress, KeybaseAddress, LMDBBackend, AddressBook};
    use std::str::FromStr;
    use uuid::Uuid;

    #[test]
    fn can_add_and_get_contact_no_duplicates() {
        let name = format!("{}", Uuid::new_v4());
        let backend = LMDBBackend::new("./target/tests").unwrap();
        let mut address_book = AddressBook::new(Box::new(backend)).unwrap();
        let address_str = format!("keybase://{}", &name[0..5]);
        let address = Address::parse(&address_str).unwrap();
        let contact = Contact::new(&name, address).unwrap();
        address_book.add_contact(&contact).unwrap();
        let contact = address_book.get_contact(&name).unwrap();
        assert_eq!(&name, contact.get_name());
        assert!(AddressType::Keybase == contact.get_address().address_type());
        assert_eq!(format!("{}", contact.get_address()), address_str);
        assert_eq!(address_book.add_contact(&contact).is_err(), true);
        assert_eq!(address_book.remove_contact(&name).is_ok(), true);
        assert_eq!(address_book.add_contact(&contact).is_ok(), true);
        assert_eq!(address_book.get_contact_by_address(&address_str).unwrap().get_name(), &name);
    }

    #[test]
    fn can_parse_keybase_contact() {
        let address_str = "keybase://keybase_username";
        let address = Address::parse(address_str).unwrap();
        let contact = Contact::new("test", address).unwrap();
        assert!(AddressType::Keybase == contact.get_address().address_type());
        assert_eq!(format!("{}", contact.get_address()), address_str);
    }

    #[test]
    fn can_parse_grinbox_contact() {
        let address_str = "grinbox://xd6A7NwpB2yDevoShkZLPorZB2h7Aivf9JyjkngKywgzrog2VpnU@grinbox.io:5555";
        let address = Address::parse(address_str).unwrap();
        let contact = Contact::new("test", address).unwrap();
        assert!(AddressType::Grinbox == contact.get_address().address_type());
        assert_eq!(format!("{}", contact.get_address()), address_str);
    }

    #[test]
    fn can_parse_keybase_address_full() {
        let address_str = "keybase://keybase_username";
        let address = KeybaseAddress::from_str(address_str).unwrap();
        assert_eq!("keybase_username", address.username);
        assert_eq!(format!("{}", address), address_str);
    }

    #[test]
    fn can_parse_grinbox_address_full() {
        let address_str = "grinbox://xd6A7NwpB2yDevoShkZLPorZB2h7Aivf9JyjkngKywgzrog2VpnU@grinbox.io:5555";
        let address = GrinboxAddress::from_str(address_str).unwrap();
        assert_eq!("xd6A7NwpB2yDevoShkZLPorZB2h7Aivf9JyjkngKywgzrog2VpnU", address.public_key);
        assert_eq!(Some("grinbox.io".to_string()), address.domain);
        assert_eq!(Some(5555), address.port);
        assert_eq!(format!("{}", address), address_str);
    }

    #[test]
    fn can_parse_grinbox_address_no_port() {
        let address_str = "grinbox://xd6A7NwpB2yDevoShkZLPorZB2h7Aivf9JyjkngKywgzrog2VpnU@grinbox.io";
        let address = GrinboxAddress::from_str(address_str).unwrap();
        assert_eq!("xd6A7NwpB2yDevoShkZLPorZB2h7Aivf9JyjkngKywgzrog2VpnU", address.public_key);
        assert_eq!(Some("grinbox.io".to_string()), address.domain);
        assert_eq!(None, address.port);
        assert_eq!(format!("{}", address), address_str);
    }

    #[test]
    fn can_parse_grinbox_address_no_domain() {
        let address_str = "grinbox://xd6A7NwpB2yDevoShkZLPorZB2h7Aivf9JyjkngKywgzrog2VpnU";
        let address = GrinboxAddress::from_str(address_str).unwrap();
        assert_eq!("xd6A7NwpB2yDevoShkZLPorZB2h7Aivf9JyjkngKywgzrog2VpnU", address.public_key);
        assert_eq!(None, address.domain);
        assert_eq!(None, address.port);
        assert_eq!(format!("{}", address), address_str);
    }
}