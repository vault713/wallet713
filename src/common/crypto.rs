use rand::OsRng;
use secp256k1::{Secp256k1, Message};
pub use secp256k1::{SecretKey, PublicKey, Signature};

use super::base58::{ToBase58, FromBase58};
use common::{Wallet713Error, Result};
use sha2::{Sha256, Digest};

pub const BASE58_CHECK_VERSION_GRIN_TX: [u8; 2] = [1, 120];

pub trait Hex<T> {
    fn from_hex(str: &str) -> Result<T>;
    fn to_hex(&self) -> String;
}

pub trait Base58<T> {
    fn from_base58(str: &str) -> Result<T>;
    fn to_base58(&self) -> String;

    fn from_base58_check(str: &str, version_bytes: usize) -> Result<T>;
    fn to_base58_check(&self, version: Vec<u8>) -> String;
}

impl Hex<PublicKey> for PublicKey {
    fn from_hex(str: &str) -> Result<PublicKey> {
        let secp = Secp256k1::new();
        let hex = from_hex(str.to_string())?;
        let key = PublicKey::from_slice(&secp, &hex)?;
        Ok(key)
    }

    fn to_hex(&self) -> String {
        to_hex(self.serialize().to_vec())
    }
}

impl Base58<PublicKey> for PublicKey {
    fn from_base58(str: &str) -> Result<PublicKey> {
        let secp = Secp256k1::new();
        let str = str::from_base58(str)?;
        let key = PublicKey::from_slice(&secp, &str)?;
        Ok(key)
    }

    fn to_base58(&self) -> String {
        self.serialize().to_base58()
    }

    fn from_base58_check(str: &str, version_bytes: usize) -> Result<PublicKey> {
        let secp = Secp256k1::new();
        let str = str::from_base58_check(str, version_bytes)?;
        let key = PublicKey::from_slice(&secp, &str.1)?;
        Ok(key)
    }

    fn to_base58_check(&self, version: Vec<u8>) -> String {
        self.serialize().to_base58_check(version)
    }
}

impl Hex<Signature> for Signature {
    fn from_hex(str: &str) -> Result<Signature> {
        let secp = Secp256k1::new();
        let hex = from_hex(str.to_string())?;
        let signature = Signature::from_der(&secp, &hex)?;
        Ok(signature)
    }

    fn to_hex(&self) -> String {
        let secp = Secp256k1::new();
        let signature = self.serialize_der(&secp);
        to_hex(signature)
    }
}

impl Hex<SecretKey> for SecretKey {
    fn from_hex(str: &str) -> Result<SecretKey> {
        let secp = Secp256k1::new();
        let data = from_hex(str.to_string())?;
        let key = SecretKey::from_slice(&secp, &data)?;
        Ok(key)
    }

    fn to_hex(&self) -> String {
        self.to_string()
    }
}

pub fn generate_keypair() -> (SecretKey, PublicKey) {
    let secp = Secp256k1::new();
    let mut rng = OsRng::new().expect("OsRng");
    secp.generate_keypair(&mut rng)
}

pub fn public_key_from_secret_key(secret_key: &SecretKey) -> PublicKey {
    let secp = Secp256k1::new();
    PublicKey::from_secret_key(&secp, secret_key)
}

pub fn sign_challenge(challenge: &str, secret_key: &SecretKey) -> Result<Signature> {
    let mut hasher = Sha256::new();
    hasher.input(challenge.as_bytes());
    let message = Message::from_slice(hasher.result().as_slice())?;
    let secp = Secp256k1::new();
    let signature = secp.sign(&message, secret_key);
    Ok(signature)
}

pub fn verify_signature(challenge: &str, signature: &Signature, public_key: &PublicKey) -> Result<()> {
    let mut hasher = Sha256::new();
    hasher.input(challenge.as_bytes());
    let message = Message::from_slice(hasher.result().as_slice())?;
    let secp = Secp256k1::new();
    secp.verify(&message, signature, public_key)?;
    Ok(())
}

use std::fmt::Write;

/// Encode the provided bytes into a hex string
pub fn to_hex(bytes: Vec<u8>) -> String {
    let mut s = String::new();
    for byte in bytes {
        write!(&mut s, "{:02x}", byte).expect("Unable to write");
    }
    s
}

/// Decode a hex string into bytes.
pub fn from_hex(hex_str: String) -> Result<Vec<u8>> {
    if hex_str.len() % 2 == 1 {
        Err(Wallet713Error::NumberParsingError)?;
    }
    let hex_trim = if &hex_str[..2] == "0x" {
        hex_str[2..].to_owned()
    } else {
        hex_str.clone()
    };
    let vec = split_n(&hex_trim.trim()[..], 2)
        .iter()
        .map(|b| {
            u8::from_str_radix(b, 16).map_err(|_| {
                Wallet713Error::NumberParsingError
            })
        })
        .collect::<std::result::Result<Vec<u8>, Wallet713Error>>()?;
    Ok(vec)
}

fn split_n(s: &str, n: usize) -> Vec<&str> {
    (0..(s.len() - n + 1) / 2 + 1)
        .map(|i| &s[2 * i..2 * i + n])
        .collect()
}
