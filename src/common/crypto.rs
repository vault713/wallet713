pub use grin_util::secp::{Message, Secp256k1, Signature};
pub use grin_util::secp::key::{PublicKey, SecretKey};

use std::fmt::Write;
use super::base58::{ToBase58, FromBase58};
use common::{ErrorKind, Result};
use sha2::{Sha256, Digest};
use rand::Rng;
use rand::thread_rng;
use ring::aead;
use ring::{digest, pbkdf2};

pub const GRINBOX_ADDRESS_VERSION_MAINNET: [u8; 2] = [1, 11];
pub const GRINBOX_ADDRESS_VERSION_TESTNET: [u8; 2] = [1, 120];

pub trait Hex<T> {
    fn from_hex(str: &str) -> Result<T>;
    fn to_hex(&self) -> String;
}

pub trait Base58<T> {
    fn from_base58(str: &str) -> Result<T>;
    fn to_base58(&self) -> String;

    fn from_base58_check(str: &str, version_bytes: Vec<u8>) -> Result<T>;
    fn to_base58_check(&self, version: Vec<u8>) -> String;
}

fn serialize_public_key(public_key: &PublicKey) -> Vec<u8> {
    let secp = Secp256k1::new();
    let ser = public_key.serialize_vec(&secp, true);
    ser[..].to_vec()
}

impl Hex<PublicKey> for PublicKey {
    fn from_hex(str: &str) -> Result<PublicKey> {
        let secp = Secp256k1::new();
        let hex = from_hex(str.to_string())?;
        PublicKey::from_slice(&secp, &hex).map_err(|_| ErrorKind::InvalidBase58Key.into())
    }

    fn to_hex(&self) -> String {
        to_hex(serialize_public_key(self))
    }
}

impl Base58<PublicKey> for PublicKey {
    fn from_base58(str: &str) -> Result<PublicKey> {
        let secp = Secp256k1::new();
        let str = str::from_base58(str)?;
        PublicKey::from_slice(&secp, &str).map_err(|_| ErrorKind::InvalidBase58Key.into())
    }

    fn to_base58(&self) -> String {
        serialize_public_key(self).to_base58()
    }

    fn from_base58_check(str: &str, version_expect: Vec<u8>) -> Result<PublicKey> {
        let secp = Secp256k1::new();
        let n_version = version_expect.len();
        let (version_actual, key_bytes) = str::from_base58_check(str, n_version)?;
        if version_actual != version_expect {
            return Err(ErrorKind::InvalidBase58Version.into());
        }
        PublicKey::from_slice(&secp, &key_bytes).map_err(|_| ErrorKind::InvalidBase58Key.into())
    }

    fn to_base58_check(&self, version: Vec<u8>) -> String {
        serialize_public_key(self).to_base58_check(version)
    }
}

impl Hex<Signature> for Signature {
    fn from_hex(str: &str) -> Result<Signature> {
        let secp = Secp256k1::new();
        let hex = from_hex(str.to_string())?;
        Signature::from_der(&secp, &hex).map_err(|_| ErrorKind::Secp.into())
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
        SecretKey::from_slice(&secp, &data).map_err(|_| ErrorKind::Secp.into())
    }

    fn to_hex(&self) -> String {
        to_hex(self.0.to_vec())
    }
}

pub fn public_key_from_secret_key(secret_key: &SecretKey) -> Result<PublicKey> {
    let secp = Secp256k1::new();
    PublicKey::from_secret_key(&secp, secret_key).map_err(|_| ErrorKind::Secp.into())
}

pub fn sign_challenge(challenge: &str, secret_key: &SecretKey) -> Result<Signature> {
    let mut hasher = Sha256::new();
    hasher.input(challenge.as_bytes());
    let message = Message::from_slice(hasher.result().as_slice())?;
    let secp = Secp256k1::new();
    secp.sign(&message, secret_key).map_err(|_| ErrorKind::Secp.into())
}

pub fn verify_signature(challenge: &str, signature: &Signature, public_key: &PublicKey) -> Result<()> {
    let mut hasher = Sha256::new();
    hasher.input(challenge.as_bytes());
    let message = Message::from_slice(hasher.result().as_slice())?;
    let secp = Secp256k1::new();
    secp.verify(&message, signature, public_key).map_err(|_| ErrorKind::Secp.into())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedMessage {
    encrypted_message: String,
    salt: String,
    nonce: String,
}

impl EncryptedMessage {
    pub fn new(message: String, receiver_public_key: &PublicKey, secret_key: &SecretKey) -> Result<EncryptedMessage> {
        let secp = Secp256k1::new();
        let mut common_secret = receiver_public_key.clone();
        common_secret.mul_assign(&secp, secret_key).map_err(|_| ErrorKind::Encryption)?;
        let common_secret_ser = common_secret.serialize_vec(&secp, true);
        let common_secret_slice = &common_secret_ser[1..33];

        let salt: [u8; 8] = thread_rng().gen();
        let nonce: [u8; 12] = thread_rng().gen();
        let mut key = [0; 32];
        pbkdf2::derive(&digest::SHA512, 100, &salt, common_secret_slice, &mut key);
        let mut enc_bytes = message.as_bytes().to_vec();
        let suffix_len = aead::CHACHA20_POLY1305.tag_len();
        for _ in 0..suffix_len {
            enc_bytes.push(0);
        }
        let sealing_key = aead::SealingKey::new(&aead::CHACHA20_POLY1305, &key)
            .map_err(|_| ErrorKind::Encryption)?;
        aead::seal_in_place(&sealing_key, &nonce, &[], &mut enc_bytes, suffix_len)
            .map_err(|_| ErrorKind::Encryption)?;

        Ok(EncryptedMessage {
            encrypted_message: to_hex(enc_bytes),
            salt: to_hex(salt.to_vec()),
            nonce: to_hex(nonce.to_vec()),
        })
    }

    pub fn key(&self, sender_public_key: &PublicKey, secret_key: &SecretKey) -> Result<[u8; 32]> {
        let salt = from_hex(self.salt.clone()).map_err(|_| ErrorKind::Decryption)?;

        let secp = Secp256k1::new();
        let mut common_secret = sender_public_key.clone();
        common_secret.mul_assign(&secp, secret_key).map_err(|_| ErrorKind::Decryption)?;
        let common_secret_ser = common_secret.serialize_vec(&secp, true);
        let common_secret_slice = &common_secret_ser[1..33];

        let mut key = [0; 32];
        pbkdf2::derive(&digest::SHA512, 100, &salt, common_secret_slice, &mut key);

        Ok(key)
    }

    pub fn decrypt_with_key(&self, key: &[u8; 32]) -> Result<String> {
        let mut encrypted_message = from_hex(self.encrypted_message.clone()).map_err(|_| ErrorKind::Decryption)?;
        let nonce = from_hex(self.nonce.clone()).map_err(|_| ErrorKind::Decryption)?;

        let opening_key = aead::OpeningKey::new(&aead::CHACHA20_POLY1305, key)
            .map_err(|_| ErrorKind::Decryption)?;
        let decrypted_data = aead::open_in_place(&opening_key, &nonce, &[], 0, &mut encrypted_message)
            .map_err(|_| ErrorKind::Decryption)?;

        String::from_utf8(decrypted_data.to_vec()).map_err(|_| ErrorKind::Decryption.into())
    }
}


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
        Err(ErrorKind::NumberParsingError)?;
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
                ErrorKind::NumberParsingError.into()
            })
        })
        .collect::<Result<Vec<u8>>>()?;
    Ok(vec)
}

fn split_n(s: &str, n: usize) -> Vec<&str> {
    (0..(s.len() - n + 1) / 2 + 1)
        .map(|i| &s[2 * i..2 * i + n])
        .collect()
}
