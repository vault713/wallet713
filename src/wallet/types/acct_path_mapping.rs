use grin_core::ser;
use super::Identifier;

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