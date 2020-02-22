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

use super::Identifier;
use grin_core::ser;
use serde::{Deserialize, Serialize};

/// Map of named accounts to BIP32 paths
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AcctPathMapping {
	/// label used by user
	pub label: String,
	/// Corresponding parent BIP32 derivation path
	pub path: Identifier,
}

impl ser::Writeable for AcctPathMapping {
	fn write<W: ser::Writer>(&self, writer: &mut W) -> Result<(), ser::Error> {
		writer.write_bytes(&serde_json::to_vec(self).map_err(|_| ser::Error::CorruptedData)?)
	}
}

impl ser::Readable for AcctPathMapping {
	fn read(reader: &mut dyn ser::Reader) -> Result<AcctPathMapping, ser::Error> {
		let data = reader.read_bytes_len_prefix()?;
		serde_json::from_slice(&data[..]).map_err(|_| ser::Error::CorruptedData)
	}
}
