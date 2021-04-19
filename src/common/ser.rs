// Copyright 2018 The Grin Developers
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

//! Sane serialization & deserialization of cryptographic structs into hex

//use epic_keychain::BlindingFactor;
//use epic_util::secp::key::PublicKey;
//use epic_util::secp::pedersen::{Commitment, RangeProof};
//use epic_util::secp::Signature;
//use epic_util::{from_hex, static_secp_instance, to_hex};
//use serde::{Deserialize, Deserializer, Serializer};

/*/// Serializes a secp PublicKey to and from hex
pub mod pubkey_serde {
	use super::*;

	///
	pub fn serialize<S>(key: &PublicKey, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
	{
		let static_secp = static_secp_instance();
		let static_secp = static_secp.lock();
		serializer.serialize_str(&to_hex(key.serialize_vec(&static_secp, true).to_vec()))
	}

	///
	pub fn deserialize<'de, D>(deserializer: D) -> Result<PublicKey, D::Error>
		where
			D: Deserializer<'de>,
	{
		use serde::de::Error;
		let static_secp = static_secp_instance();
		let static_secp = static_secp.lock();
		String::deserialize(deserializer)
			.and_then(|string| from_hex(string).map_err(|err| Error::custom(err.to_string())))
			.and_then(|bytes: Vec<u8>| {
				PublicKey::from_slice(&static_secp, &bytes)
					.map_err(|err| Error::custom(err.to_string()))
			})
	}
}*/

/*/// Serializes an Option<secp::Signature> to and from hex
pub mod option_sig_serde {
	use serde::de::Error;
	use super::*;

	///
	pub fn serialize<S>(sig: &Option<Signature>, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
	{
		let static_secp = static_secp_instance();
		let static_secp = static_secp.lock();
		match sig {
			Some(sig) => {
				serializer.serialize_str(&to_hex(sig.serialize_compact(&static_secp).to_vec()))
			}
			None => serializer.serialize_none(),
		}
	}

	///
	pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Signature>, D::Error>
		where
			D: Deserializer<'de>,
	{
		let static_secp = static_secp_instance();
		let static_secp = static_secp.lock();
		Option::<String>::deserialize(deserializer).and_then(|res| match res {
			Some(string) => from_hex(string.to_string())
				.map_err(|err| Error::custom(err.to_string()))
				.and_then(|bytes: Vec<u8>| {
					let mut b = [0u8; 64];
					b.copy_from_slice(&bytes[0..64]);
					Signature::from_compact(&static_secp, &b)
						.map(|val| Some(val))
						.map_err(|err| Error::custom(err.to_string()))
				}),
			None => Ok(None),
		})
	}

}*/

/*/// Serializes a secp::Signature to and from hex
pub mod sig_serde {
	use serde::de::Error;
	use super::*;

	///
	pub fn serialize<S>(sig: &Signature, serializer: S) -> Result<S::Ok, S::Error>
		where
			S: Serializer,
	{
		let static_secp = static_secp_instance();
		let static_secp = static_secp.lock();
		serializer.serialize_str(&to_hex(sig.serialize_compact(&static_secp).to_vec()))
	}

	///
	pub fn deserialize<'de, D>(deserializer: D) -> Result<Signature, D::Error>
		where
			D: Deserializer<'de>,
	{
		let static_secp = static_secp_instance();
		let static_secp = static_secp.lock();
		String::deserialize(deserializer)
			.and_then(|string| from_hex(string).map_err(|err| Error::custom(err.to_string())))
			.and_then(|bytes: Vec<u8>| {
				let mut b = [0u8; 64];
				b.copy_from_slice(&bytes[0..64]);
				Signature::from_compact(&static_secp, &b)
					.map_err(|err| Error::custom(err.to_string()))
			})
	}
}*/

/*/// Creates a BlindingFactor from a hex string
pub fn blind_from_hex<'de, D>(deserializer: D) -> Result<BlindingFactor, D::Error>
	where
		D: Deserializer<'de>,
{
	use serde::de::Error;
	String::deserialize(deserializer).and_then(|string| {
		BlindingFactor::from_hex(&string).map_err(|err| Error::custom(err.to_string()))
	})
}*/

/*/// Creates a RangeProof from a hex string
pub fn rangeproof_from_hex<'de, D>(deserializer: D) -> Result<RangeProof, D::Error>
	where
		D: Deserializer<'de>,
{
	use serde::de::{Error, IntoDeserializer};

	let val = String::deserialize(deserializer)
		.and_then(|string| from_hex(string).map_err(|err| Error::custom(err.to_string())))?;
	RangeProof::deserialize(val.into_deserializer())
}*/

/*/// Creates a Pedersen Commitment from a hex string
pub fn commitment_from_hex<'de, D>(deserializer: D) -> Result<Commitment, D::Error>
	where
		D: Deserializer<'de>,
{
	use serde::de::Error;
	String::deserialize(deserializer)
		.and_then(|string| from_hex(string).map_err(|err| Error::custom(err.to_string())))
		.and_then(|bytes: Vec<u8>| Ok(Commitment::from_vec(bytes.to_vec())))
}*/

/*/// Seralizes a byte string into hex
pub fn as_hex<T, S>(bytes: T, serializer: S) -> Result<S::Ok, S::Error>
	where
		T: AsRef<[u8]>,
		S: Serializer,
{
	serializer.serialize_str(&to_hex(bytes.as_ref().to_vec()))
}*/

/// Used to ensure u64s are serialised in json
/// as strings by default, since it can't be guaranteed that consumers
/// will know what to do with u64 literals (e.g. Javascript). However,
/// fields using this tag can be deserialized from literals or strings.
/// From solutions on:
/// https://github.com/serde-rs/json/issues/329
pub mod string_or_u64 {
	use serde::{de, Deserializer, Serializer};
	use std::fmt;

	/// serialize into a string
	pub fn serialize<T, S>(value: &T, serializer: S) -> Result<S::Ok, S::Error>
	where
		T: fmt::Display,
		S: Serializer,
	{
		serializer.collect_str(value)
	}

	/// deserialize from either literal or string
	pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
	where
		D: Deserializer<'de>,
	{
		struct Visitor;
		impl<'a> de::Visitor<'a> for Visitor {
			type Value = u64;
			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				write!(
					formatter,
					"a string containing digits or an int fitting into u64"
				)
			}
			fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
				Ok(v)
			}
			fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
			where
				E: de::Error,
			{
				s.parse().map_err(de::Error::custom)
			}
		}
		deserializer.deserialize_any(Visitor)
	}
}

/*/// As above, for Options
pub mod opt_string_or_u64 {
	use serde::{de, Deserializer, Serializer};
	use std::fmt;

	/// serialize into string or none
	pub fn serialize<T, S>(value: &Option<T>, serializer: S) -> Result<S::Ok, S::Error>
		where
			T: fmt::Display,
			S: Serializer,
	{
		match value {
			Some(v) => serializer.collect_str(v),
			None => serializer.serialize_none(),
		}
	}

	/// deser from 'null', literal or string
	pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
		where
			D: Deserializer<'de>,
	{
		struct Visitor;
		impl<'a> de::Visitor<'a> for Visitor {
			type Value = Option<u64>;
			fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
				write!(
					formatter,
					"null, a string containing digits or an int fitting into u64"
				)
			}
			fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E> {
				Ok(Some(v))
			}
			fn visit_str<E>(self, s: &str) -> Result<Self::Value, E>
				where
					E: de::Error,
			{
				let val: u64 = s.parse().map_err(de::Error::custom)?;
				Ok(Some(val))
			}
			fn visit_unit<E>(self) -> Result<Self::Value, E> {
				Ok(None)
			}
		}
		deserializer.deserialize_any(Visitor)
	}
}*/
