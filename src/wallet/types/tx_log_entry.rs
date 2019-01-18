use grin_core::ser;
use uuid::Uuid;
use chrono::prelude::*;

use super::{Identifier, TxLogEntryType};

/// Optional transaction information, recorded when an event happens
/// to add or remove funds from a wallet. One Transaction log entry
/// maps to one or many outputs
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TxLogEntry {
    /// BIP32 account path used for creating this tx
    pub parent_key_id: Identifier,
    /// Local id for this transaction (distinct from a slate transaction id)
    pub id: u32,
    /// Slate transaction this entry is associated with, if any
    pub tx_slate_id: Option<Uuid>,
    /// Transaction type (as above)
    pub tx_type: TxLogEntryType,
    /// Time this tx entry was created
    /// #[serde(with = "tx_date_format")]
    pub creation_ts: DateTime<Utc>,
    /// Time this tx was confirmed (by this wallet)
    /// #[serde(default, with = "opt_tx_date_format")]
    pub confirmation_ts: Option<DateTime<Utc>>,
    /// Whether the inputs+outputs involved in this transaction have been
    /// confirmed (In all cases either all outputs involved in a tx should be
    /// confirmed, or none should be; otherwise there's a deeper problem)
    pub confirmed: bool,
    /// number of inputs involved in TX
    pub num_inputs: usize,
    /// number of outputs involved in TX
    pub num_outputs: usize,
    /// Amount credited via this transaction
    pub amount_credited: u64,
    /// Amount debited via this transaction
    pub amount_debited: u64,
    /// Fee
    pub fee: Option<u64>,
}

impl TxLogEntry {
    /// Return a new blank with TS initialised with next entry
    pub fn new(parent_key_id: Identifier, t: TxLogEntryType, id: u32) -> Self {
        TxLogEntry {
            parent_key_id: parent_key_id,
            tx_type: t,
            id: id,
            tx_slate_id: None,
            creation_ts: Utc::now(),
            confirmation_ts: None,
            confirmed: false,
            amount_credited: 0,
            amount_debited: 0,
            num_inputs: 0,
            num_outputs: 0,
            fee: None,
        }
    }

    /// Update confirmation TS with now
    pub fn update_confirmation_ts(&mut self) {
        self.confirmation_ts = Some(Utc::now());
    }
}

impl ser::Writeable for TxLogEntry {
    fn write<W: ser::Writer>(&self, writer: &mut W) -> Result<(), ser::Error> {
        writer.write_bytes(&serde_json::to_vec(self).map_err(|_| ser::Error::CorruptedData)?)
    }
}

impl ser::Readable for TxLogEntry {
    fn read(reader: &mut dyn ser::Reader) -> Result<TxLogEntry, ser::Error> {
        let data = reader.read_bytes_len_prefix()?;
        serde_json::from_slice(&data[..]).map_err(|_| ser::Error::CorruptedData)
    }
}