// Copyright 2019 The vault713 Developers
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use super::base58::{FromBase58, ToBase58};
use super::{ErrorKind, Result};
pub use epic_util::secp::key::{PublicKey, SecretKey};
use epic_util::secp::pedersen::Commitment;
pub use epic_util::secp::{Message, Secp256k1, Signature};
use sha2::{Digest, Sha256};
use std::fmt::Write;

pub const EPICBOX_ADDRESS_VERSION_MAINNET: [u8; 2] = [1, 0];
pub const EPICBOX_ADDRESS_VERSION_TESTNET: [u8; 2] = [1, 136];

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

impl Hex<Commitment> for Commitment {
	fn from_hex(str: &str) -> Result<Commitment> {
		let data = from_hex(str.to_string())?;
		Ok(Commitment::from_vec(data))
	}

	fn to_hex(&self) -> String {
		to_hex(self.0.to_vec())
	}
}

pub fn sign_challenge(challenge: &str, secret_key: &SecretKey) -> Result<Signature> {
	let mut hasher = Sha256::new();
	hasher.update(challenge.as_bytes());
	let message = Message::from_slice(hasher.finalize().as_slice())?;
	let secp = Secp256k1::new();
	secp.sign(&message, secret_key)
		.map_err(|_| ErrorKind::Secp.into())
}

pub fn verify_signature(
	challenge: &str,
	signature: &Signature,
	public_key: &PublicKey,
) -> Result<()> {
	let mut hasher = Sha256::new();
	hasher.update(challenge.as_bytes());
	let message = Message::from_slice(hasher.finalize().as_slice())?;
	let secp = Secp256k1::new();
	secp.verify(&message, signature, public_key)
		.map_err(|_| ErrorKind::Secp.into())
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
		.map(|b| u8::from_str_radix(b, 16).map_err(|_| ErrorKind::NumberParsingError.into()))
		.collect::<Result<Vec<u8>>>()?;
	Ok(vec)
}

fn split_n(s: &str, n: usize) -> Vec<&str> {
	(0..(s.len() - n + 1) / 2 + 1)
		.map(|i| &s[2 * i..2 * i + n])
		.collect()
}
