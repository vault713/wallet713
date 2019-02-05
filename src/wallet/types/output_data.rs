use super::{Identifier, OutputStatus};
use grin_core::ser;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct OutputData {
    /// Root key_id that the key for this output is derived from
    pub root_key_id: Identifier,
    /// Derived key for this output
    pub key_id: Identifier,
    /// How many derivations down from the root key
    pub n_child: u32,
    /// The actual commit, optionally stored
    pub commit: Option<String>,
    /// PMMR Index, used on restore in case of duplicate wallets using the same
    /// key_id (2 wallets using same seed, for instance
    pub mmr_index: Option<u64>,
    /// Value of the output, necessary to rebuild the commitment
    pub value: u64,
    /// Current status of the output
    pub status: OutputStatus,
    /// Height of the output
    pub height: u64,
    /// Height we are locked until
    pub lock_height: u64,
    /// Is this a coinbase output? Is it subject to coinbase locktime?
    pub is_coinbase: bool,
    /// Optional corresponding internal entry in tx entry log
    pub tx_log_entry: Option<u32>,
}

impl OutputData {
    /// Lock a given output to avoid conflicting use
    pub fn lock(&mut self) {
        self.status = OutputStatus::Locked;
    }

    /// How many confirmations has this output received?
    /// If height == 0 then we are either Unconfirmed or the output was
    /// cut-through
    /// so we do not actually know how many confirmations this output had (and
    /// never will).
    pub fn num_confirmations(&self, current_height: u64) -> u64 {
        if self.height > current_height {
            return 0;
        }
        if self.status == OutputStatus::Unconfirmed {
            0
        } else if self.height == 0 {
            0
        } else {
            // if an output has height n and we are at block n
            // then we have a single confirmation (the block it originated in)
            1 + (current_height - self.height)
        }
    }

    /// Check if output is eligible to spend based on state and height and
    /// confirmations
    pub fn eligible_to_spend(&self, current_height: u64, minimum_confirmations: u64) -> bool {
        if [OutputStatus::Spent, OutputStatus::Locked].contains(&self.status) {
            return false;
        } else if self.status == OutputStatus::Unconfirmed && self.is_coinbase {
            return false;
        } else if self.lock_height > current_height {
            return false;
        } else if self.status == OutputStatus::Unspent
            && self.num_confirmations(current_height) >= minimum_confirmations
        {
            return true;
        } else if self.status == OutputStatus::Unconfirmed && minimum_confirmations == 0 {
            return true;
        } else {
            return false;
        }
    }

    /// Marks this output as unspent if it was previously unconfirmed
    pub fn mark_unspent(&mut self) {
        match self.status {
            OutputStatus::Unconfirmed => self.status = OutputStatus::Unspent,
            _ => (),
        }
    }

    /// Mark an output as spent
    pub fn mark_spent(&mut self) {
        match self.status {
            OutputStatus::Unspent => self.status = OutputStatus::Spent,
            OutputStatus::Locked => self.status = OutputStatus::Spent,
            _ => (),
        }
    }
}

impl ser::Writeable for OutputData {
    fn write<W: ser::Writer>(&self, writer: &mut W) -> Result<(), ser::Error> {
        writer.write_bytes(&serde_json::to_vec(self).map_err(|_| ser::Error::CorruptedData)?)
    }
}

impl ser::Readable for OutputData {
    fn read(reader: &mut dyn ser::Reader) -> Result<OutputData, ser::Error> {
        let data = reader.read_bytes_len_prefix()?;
        serde_json::from_slice(&data[..]).map_err(|_| ser::Error::CorruptedData)
    }
}
