use grin_core::ser::{Readable, Writeable, Reader, Writer};
use grin_core::ser::Error as CoreError;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Contact {
    pub public_key: String,
    pub name: String,
}

impl Contact {
    pub fn new(public_key: &str, name: &str) -> Self {
        Self {
            public_key: public_key.to_string(),
            name: name.to_string()
        }
    }
}

impl Writeable for Contact {
    fn write<W: Writer>(&self, writer: &mut W) -> Result<(), CoreError> {
        writer.write_bytes(&serde_json::to_vec(self).map_err(|_| CoreError::CorruptedData)?)
    }
}

impl Readable for Contact {
    fn read(reader: &mut Reader) -> Result<Contact, CoreError> {
        let data = reader.read_bytes_len_prefix()?;
        serde_json::from_slice(&data[..]).map_err(|_| CoreError::CorruptedData)
    }
}