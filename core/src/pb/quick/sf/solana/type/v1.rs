// Automatically generated rust module for 'type.proto' file

#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]
#![allow(unknown_lints)]
#![allow(clippy::all)]
#![cfg_attr(rustfmt, rustfmt_skip)]


use std::borrow::Cow;
use quick_protobuf::{MessageInfo, MessageRead, MessageWrite, BytesReader, Writer, WriterBackend, Result};
use quick_protobuf::sizeofs::*;
use super::super::super::super::*;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum RewardType {
    Unspecified = 0,
    Fee = 1,
    Rent = 2,
    Staking = 3,
    Voting = 4,
}

impl Default for RewardType {
    fn default() -> Self {
        RewardType::Unspecified
    }
}

impl From<i32> for RewardType {
    fn from(i: i32) -> Self {
        match i {
            0 => RewardType::Unspecified,
            1 => RewardType::Fee,
            2 => RewardType::Rent,
            3 => RewardType::Staking,
            4 => RewardType::Voting,
            _ => Self::default(),
        }
    }
}

impl<'a> From<&'a str> for RewardType {
    fn from(s: &'a str) -> Self {
        match s {
            "Unspecified" => RewardType::Unspecified,
            "Fee" => RewardType::Fee,
            "Rent" => RewardType::Rent,
            "Staking" => RewardType::Staking,
            "Voting" => RewardType::Voting,
            _ => Self::default(),
        }
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Block<'a> {
    pub previous_blockhash: Cow<'a, str>,
    pub blockhash: Cow<'a, str>,
    pub parent_slot: u64,
    pub transactions: Vec<sf::solana::r#type::v1::ConfirmedTransaction<'a>>,
    pub rewards: Vec<sf::solana::r#type::v1::Reward<'a>>,
    pub block_time: Option<sf::solana::r#type::v1::UnixTimestamp>,
    pub block_height: Option<sf::solana::r#type::v1::BlockHeight>,
    pub slot: u64,
}

impl<'a> MessageRead<'a> for Block<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.previous_blockhash = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(18) => msg.blockhash = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(24) => msg.parent_slot = r.read_uint64(bytes)?,
                Ok(34) => msg.transactions.push(r.read_message::<sf::solana::r#type::v1::ConfirmedTransaction>(bytes)?),
                Ok(42) => msg.rewards.push(r.read_message::<sf::solana::r#type::v1::Reward>(bytes)?),
                Ok(50) => msg.block_time = Some(r.read_message::<sf::solana::r#type::v1::UnixTimestamp>(bytes)?),
                Ok(58) => msg.block_height = Some(r.read_message::<sf::solana::r#type::v1::BlockHeight>(bytes)?),
                Ok(160) => msg.slot = r.read_uint64(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for Block<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.previous_blockhash == "" { 0 } else { 1 + sizeof_len((&self.previous_blockhash).len()) }
        + if self.blockhash == "" { 0 } else { 1 + sizeof_len((&self.blockhash).len()) }
        + if self.parent_slot == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.parent_slot) as u64) }
        + self.transactions.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.rewards.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.block_time.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
        + self.block_height.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
        + if self.slot == 0u64 { 0 } else { 2 + sizeof_varint(*(&self.slot) as u64) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.previous_blockhash != "" { w.write_with_tag(10, |w| w.write_string(&**&self.previous_blockhash))?; }
        if self.blockhash != "" { w.write_with_tag(18, |w| w.write_string(&**&self.blockhash))?; }
        if self.parent_slot != 0u64 { w.write_with_tag(24, |w| w.write_uint64(*&self.parent_slot))?; }
        for s in &self.transactions { w.write_with_tag(34, |w| w.write_message(s))?; }
        for s in &self.rewards { w.write_with_tag(42, |w| w.write_message(s))?; }
        if let Some(ref s) = self.block_time { w.write_with_tag(50, |w| w.write_message(s))?; }
        if let Some(ref s) = self.block_height { w.write_with_tag(58, |w| w.write_message(s))?; }
        if self.slot != 0u64 { w.write_with_tag(160, |w| w.write_uint64(*&self.slot))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct ConfirmedTransaction<'a> {
    pub transaction: Option<sf::solana::r#type::v1::Transaction<'a>>,
    pub meta: Option<sf::solana::r#type::v1::TransactionStatusMeta<'a>>,
}

impl<'a> MessageRead<'a> for ConfirmedTransaction<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.transaction = Some(r.read_message::<sf::solana::r#type::v1::Transaction>(bytes)?),
                Ok(18) => msg.meta = Some(r.read_message::<sf::solana::r#type::v1::TransactionStatusMeta>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for ConfirmedTransaction<'a> {
    fn get_size(&self) -> usize {
        0
        + self.transaction.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
        + self.meta.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.transaction { w.write_with_tag(10, |w| w.write_message(s))?; }
        if let Some(ref s) = self.meta { w.write_with_tag(18, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Transaction<'a> {
    pub signatures: Vec<Cow<'a, [u8]>>,
    pub message: Option<sf::solana::r#type::v1::Message<'a>>,
}

impl<'a> MessageRead<'a> for Transaction<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.signatures.push(r.read_bytes(bytes).map(Cow::Borrowed)?),
                Ok(18) => msg.message = Some(r.read_message::<sf::solana::r#type::v1::Message>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for Transaction<'a> {
    fn get_size(&self) -> usize {
        0
        + self.signatures.iter().map(|s| 1 + sizeof_len((s).len())).sum::<usize>()
        + self.message.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.signatures { w.write_with_tag(10, |w| w.write_bytes(&**s))?; }
        if let Some(ref s) = self.message { w.write_with_tag(18, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Message<'a> {
    pub header: Option<sf::solana::r#type::v1::MessageHeader>,
    pub account_keys: Vec<Cow<'a, [u8]>>,
    pub recent_blockhash: Cow<'a, [u8]>,
    pub instructions: Vec<sf::solana::r#type::v1::CompiledInstruction<'a>>,
    pub versioned: bool,
    pub address_table_lookups: Vec<sf::solana::r#type::v1::MessageAddressTableLookup<'a>>,
}

impl<'a> MessageRead<'a> for Message<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.header = Some(r.read_message::<sf::solana::r#type::v1::MessageHeader>(bytes)?),
                Ok(18) => msg.account_keys.push(r.read_bytes(bytes).map(Cow::Borrowed)?),
                Ok(26) => msg.recent_blockhash = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(34) => msg.instructions.push(r.read_message::<sf::solana::r#type::v1::CompiledInstruction>(bytes)?),
                Ok(40) => msg.versioned = r.read_bool(bytes)?,
                Ok(50) => msg.address_table_lookups.push(r.read_message::<sf::solana::r#type::v1::MessageAddressTableLookup>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for Message<'a> {
    fn get_size(&self) -> usize {
        0
        + self.header.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
        + self.account_keys.iter().map(|s| 1 + sizeof_len((s).len())).sum::<usize>()
        + if self.recent_blockhash == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.recent_blockhash).len()) }
        + self.instructions.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + if self.versioned == false { 0 } else { 1 + sizeof_varint(*(&self.versioned) as u64) }
        + self.address_table_lookups.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.header { w.write_with_tag(10, |w| w.write_message(s))?; }
        for s in &self.account_keys { w.write_with_tag(18, |w| w.write_bytes(&**s))?; }
        if self.recent_blockhash != Cow::Borrowed(b"") { w.write_with_tag(26, |w| w.write_bytes(&**&self.recent_blockhash))?; }
        for s in &self.instructions { w.write_with_tag(34, |w| w.write_message(s))?; }
        if self.versioned != false { w.write_with_tag(40, |w| w.write_bool(*&self.versioned))?; }
        for s in &self.address_table_lookups { w.write_with_tag(50, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct MessageHeader {
    pub num_required_signatures: u32,
    pub num_readonly_signed_accounts: u32,
    pub num_readonly_unsigned_accounts: u32,
}

impl<'a> MessageRead<'a> for MessageHeader {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.num_required_signatures = r.read_uint32(bytes)?,
                Ok(16) => msg.num_readonly_signed_accounts = r.read_uint32(bytes)?,
                Ok(24) => msg.num_readonly_unsigned_accounts = r.read_uint32(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for MessageHeader {
    fn get_size(&self) -> usize {
        0
        + if self.num_required_signatures == 0u32 { 0 } else { 1 + sizeof_varint(*(&self.num_required_signatures) as u64) }
        + if self.num_readonly_signed_accounts == 0u32 { 0 } else { 1 + sizeof_varint(*(&self.num_readonly_signed_accounts) as u64) }
        + if self.num_readonly_unsigned_accounts == 0u32 { 0 } else { 1 + sizeof_varint(*(&self.num_readonly_unsigned_accounts) as u64) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.num_required_signatures != 0u32 { w.write_with_tag(8, |w| w.write_uint32(*&self.num_required_signatures))?; }
        if self.num_readonly_signed_accounts != 0u32 { w.write_with_tag(16, |w| w.write_uint32(*&self.num_readonly_signed_accounts))?; }
        if self.num_readonly_unsigned_accounts != 0u32 { w.write_with_tag(24, |w| w.write_uint32(*&self.num_readonly_unsigned_accounts))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct MessageAddressTableLookup<'a> {
    pub account_key: Cow<'a, [u8]>,
    pub writable_indexes: Cow<'a, [u8]>,
    pub readonly_indexes: Cow<'a, [u8]>,
}

impl<'a> MessageRead<'a> for MessageAddressTableLookup<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.account_key = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(18) => msg.writable_indexes = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(26) => msg.readonly_indexes = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for MessageAddressTableLookup<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.account_key == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.account_key).len()) }
        + if self.writable_indexes == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.writable_indexes).len()) }
        + if self.readonly_indexes == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.readonly_indexes).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.account_key != Cow::Borrowed(b"") { w.write_with_tag(10, |w| w.write_bytes(&**&self.account_key))?; }
        if self.writable_indexes != Cow::Borrowed(b"") { w.write_with_tag(18, |w| w.write_bytes(&**&self.writable_indexes))?; }
        if self.readonly_indexes != Cow::Borrowed(b"") { w.write_with_tag(26, |w| w.write_bytes(&**&self.readonly_indexes))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct TransactionStatusMeta<'a> {
    pub err: Option<sf::solana::r#type::v1::TransactionError<'a>>,
    pub fee: u64,
    pub pre_balances: Vec<u64>,
    pub post_balances: Vec<u64>,
    pub inner_instructions: Vec<sf::solana::r#type::v1::InnerInstructions<'a>>,
    pub log_messages: Vec<Cow<'a, str>>,
    pub pre_token_balances: Vec<sf::solana::r#type::v1::TokenBalance<'a>>,
    pub post_token_balances: Vec<sf::solana::r#type::v1::TokenBalance<'a>>,
    pub rewards: Vec<sf::solana::r#type::v1::Reward<'a>>,
    pub loaded_writable_addresses: Vec<Cow<'a, [u8]>>,
    pub loaded_readonly_addresses: Vec<Cow<'a, [u8]>>,
    pub return_data: Option<sf::solana::r#type::v1::ReturnData<'a>>,
    pub compute_units_consumed: u64,
    pub cost_units: u64,
}

impl<'a> MessageRead<'a> for TransactionStatusMeta<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.err = Some(r.read_message::<sf::solana::r#type::v1::TransactionError>(bytes)?),
                Ok(16) => msg.fee = r.read_uint64(bytes)?,
                Ok(26) => msg.pre_balances = r.read_packed(bytes, |r, bytes| Ok(r.read_uint64(bytes)?))?,
                Ok(34) => msg.post_balances = r.read_packed(bytes, |r, bytes| Ok(r.read_uint64(bytes)?))?,
                Ok(42) => msg.inner_instructions.push(r.read_message::<sf::solana::r#type::v1::InnerInstructions>(bytes)?),
                Ok(50) => msg.log_messages.push(r.read_string(bytes).map(Cow::Borrowed)?),
                Ok(58) => msg.pre_token_balances.push(r.read_message::<sf::solana::r#type::v1::TokenBalance>(bytes)?),
                Ok(66) => msg.post_token_balances.push(r.read_message::<sf::solana::r#type::v1::TokenBalance>(bytes)?),
                Ok(74) => msg.rewards.push(r.read_message::<sf::solana::r#type::v1::Reward>(bytes)?),
                Ok(98) => msg.loaded_writable_addresses.push(r.read_bytes(bytes).map(Cow::Borrowed)?),
                Ok(106) => msg.loaded_readonly_addresses.push(r.read_bytes(bytes).map(Cow::Borrowed)?),
                Ok(114) => msg.return_data = Some(r.read_message::<sf::solana::r#type::v1::ReturnData>(bytes)?),
                Ok(128) => msg.compute_units_consumed = r.read_uint64(bytes)?,
                Ok(136) => msg.cost_units = r.read_uint64(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for TransactionStatusMeta<'a> {
    fn get_size(&self) -> usize {
        0
        + self.err.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
        + if self.fee == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.fee) as u64) }
        + if self.pre_balances.is_empty() { 0 } else { 1 + sizeof_len(self.pre_balances.iter().map(|s| sizeof_varint(*(s) as u64)).sum::<usize>()) }
        + if self.post_balances.is_empty() { 0 } else { 1 + sizeof_len(self.post_balances.iter().map(|s| sizeof_varint(*(s) as u64)).sum::<usize>()) }
        + self.inner_instructions.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.log_messages.iter().map(|s| 1 + sizeof_len((s).len())).sum::<usize>()
        + self.pre_token_balances.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.post_token_balances.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.rewards.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
        + self.loaded_writable_addresses.iter().map(|s| 1 + sizeof_len((s).len())).sum::<usize>()
        + self.loaded_readonly_addresses.iter().map(|s| 1 + sizeof_len((s).len())).sum::<usize>()
        + self.return_data.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
        + if self.compute_units_consumed == 0u64 { 0 } else { 2 + sizeof_varint(*(&self.compute_units_consumed) as u64) }
        + if self.cost_units == 0u64 { 0 } else { 2 + sizeof_varint(*(&self.cost_units) as u64) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if let Some(ref s) = self.err { w.write_with_tag(10, |w| w.write_message(s))?; }
        if self.fee != 0u64 { w.write_with_tag(16, |w| w.write_uint64(*&self.fee))?; }
        w.write_packed_with_tag(26, &self.pre_balances, |w, m| w.write_uint64(*m), &|m| sizeof_varint(*(m) as u64))?;
        w.write_packed_with_tag(34, &self.post_balances, |w, m| w.write_uint64(*m), &|m| sizeof_varint(*(m) as u64))?;
        for s in &self.inner_instructions { w.write_with_tag(42, |w| w.write_message(s))?; }
        for s in &self.log_messages { w.write_with_tag(50, |w| w.write_string(&**s))?; }
        for s in &self.pre_token_balances { w.write_with_tag(58, |w| w.write_message(s))?; }
        for s in &self.post_token_balances { w.write_with_tag(66, |w| w.write_message(s))?; }
        for s in &self.rewards { w.write_with_tag(74, |w| w.write_message(s))?; }
        for s in &self.loaded_writable_addresses { w.write_with_tag(98, |w| w.write_bytes(&**s))?; }
        for s in &self.loaded_readonly_addresses { w.write_with_tag(106, |w| w.write_bytes(&**s))?; }
        if let Some(ref s) = self.return_data { w.write_with_tag(114, |w| w.write_message(s))?; }
        if self.compute_units_consumed != 0u64 { w.write_with_tag(128, |w| w.write_uint64(*&self.compute_units_consumed))?; }
        if self.cost_units != 0u64 { w.write_with_tag(136, |w| w.write_uint64(*&self.cost_units))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct TransactionError<'a> {
    pub err: Cow<'a, [u8]>,
}

impl<'a> MessageRead<'a> for TransactionError<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.err = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for TransactionError<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.err == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.err).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.err != Cow::Borrowed(b"") { w.write_with_tag(10, |w| w.write_bytes(&**&self.err))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct InnerInstructions<'a> {
    pub index: u32,
    pub instructions: Vec<sf::solana::r#type::v1::InnerInstruction<'a>>,
}

impl<'a> MessageRead<'a> for InnerInstructions<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.index = r.read_uint32(bytes)?,
                Ok(18) => msg.instructions.push(r.read_message::<sf::solana::r#type::v1::InnerInstruction>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for InnerInstructions<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.index == 0u32 { 0 } else { 1 + sizeof_varint(*(&self.index) as u64) }
        + self.instructions.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.index != 0u32 { w.write_with_tag(8, |w| w.write_uint32(*&self.index))?; }
        for s in &self.instructions { w.write_with_tag(18, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct InnerInstruction<'a> {
    pub program_id_index: u32,
    pub accounts: Cow<'a, [u8]>,
    pub data: Cow<'a, [u8]>,
    pub stack_height: u32,
}

impl<'a> MessageRead<'a> for InnerInstruction<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.program_id_index = r.read_uint32(bytes)?,
                Ok(18) => msg.accounts = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(26) => msg.data = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(32) => msg.stack_height = r.read_uint32(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for InnerInstruction<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.program_id_index == 0u32 { 0 } else { 1 + sizeof_varint(*(&self.program_id_index) as u64) }
        + if self.accounts == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.accounts).len()) }
        + if self.data == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.data).len()) }
        + if self.stack_height == 0u32 { 0 } else { 1 + sizeof_varint(*(&self.stack_height) as u64) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.program_id_index != 0u32 { w.write_with_tag(8, |w| w.write_uint32(*&self.program_id_index))?; }
        if self.accounts != Cow::Borrowed(b"") { w.write_with_tag(18, |w| w.write_bytes(&**&self.accounts))?; }
        if self.data != Cow::Borrowed(b"") { w.write_with_tag(26, |w| w.write_bytes(&**&self.data))?; }
        if self.stack_height != 0u32 { w.write_with_tag(32, |w| w.write_uint32(*&self.stack_height))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct CompiledInstruction<'a> {
    pub program_id_index: u32,
    pub accounts: Cow<'a, [u8]>,
    pub data: Cow<'a, [u8]>,
}

impl<'a> MessageRead<'a> for CompiledInstruction<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.program_id_index = r.read_uint32(bytes)?,
                Ok(18) => msg.accounts = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(26) => msg.data = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for CompiledInstruction<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.program_id_index == 0u32 { 0 } else { 1 + sizeof_varint(*(&self.program_id_index) as u64) }
        + if self.accounts == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.accounts).len()) }
        + if self.data == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.data).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.program_id_index != 0u32 { w.write_with_tag(8, |w| w.write_uint32(*&self.program_id_index))?; }
        if self.accounts != Cow::Borrowed(b"") { w.write_with_tag(18, |w| w.write_bytes(&**&self.accounts))?; }
        if self.data != Cow::Borrowed(b"") { w.write_with_tag(26, |w| w.write_bytes(&**&self.data))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct TokenBalance<'a> {
    pub account_index: u32,
    pub mint: Cow<'a, str>,
    pub ui_token_amount: Option<sf::solana::r#type::v1::UiTokenAmount<'a>>,
    pub owner: Cow<'a, str>,
    pub program_id: Cow<'a, str>,
}

impl<'a> MessageRead<'a> for TokenBalance<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.account_index = r.read_uint32(bytes)?,
                Ok(18) => msg.mint = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(26) => msg.ui_token_amount = Some(r.read_message::<sf::solana::r#type::v1::UiTokenAmount>(bytes)?),
                Ok(34) => msg.owner = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(42) => msg.program_id = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for TokenBalance<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.account_index == 0u32 { 0 } else { 1 + sizeof_varint(*(&self.account_index) as u64) }
        + if self.mint == "" { 0 } else { 1 + sizeof_len((&self.mint).len()) }
        + self.ui_token_amount.as_ref().map_or(0, |m| 1 + sizeof_len((m).get_size()))
        + if self.owner == "" { 0 } else { 1 + sizeof_len((&self.owner).len()) }
        + if self.program_id == "" { 0 } else { 1 + sizeof_len((&self.program_id).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.account_index != 0u32 { w.write_with_tag(8, |w| w.write_uint32(*&self.account_index))?; }
        if self.mint != "" { w.write_with_tag(18, |w| w.write_string(&**&self.mint))?; }
        if let Some(ref s) = self.ui_token_amount { w.write_with_tag(26, |w| w.write_message(s))?; }
        if self.owner != "" { w.write_with_tag(34, |w| w.write_string(&**&self.owner))?; }
        if self.program_id != "" { w.write_with_tag(42, |w| w.write_string(&**&self.program_id))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct UiTokenAmount<'a> {
    pub ui_amount: f64,
    pub decimals: u32,
    pub amount: Cow<'a, str>,
    pub ui_amount_string: Cow<'a, str>,
}

impl<'a> MessageRead<'a> for UiTokenAmount<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(9) => msg.ui_amount = r.read_double(bytes)?,
                Ok(16) => msg.decimals = r.read_uint32(bytes)?,
                Ok(26) => msg.amount = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(34) => msg.ui_amount_string = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for UiTokenAmount<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.ui_amount == 0f64 { 0 } else { 1 + 8 }
        + if self.decimals == 0u32 { 0 } else { 1 + sizeof_varint(*(&self.decimals) as u64) }
        + if self.amount == "" { 0 } else { 1 + sizeof_len((&self.amount).len()) }
        + if self.ui_amount_string == "" { 0 } else { 1 + sizeof_len((&self.ui_amount_string).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.ui_amount != 0f64 { w.write_with_tag(9, |w| w.write_double(*&self.ui_amount))?; }
        if self.decimals != 0u32 { w.write_with_tag(16, |w| w.write_uint32(*&self.decimals))?; }
        if self.amount != "" { w.write_with_tag(26, |w| w.write_string(&**&self.amount))?; }
        if self.ui_amount_string != "" { w.write_with_tag(34, |w| w.write_string(&**&self.ui_amount_string))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct ReturnData<'a> {
    pub program_id: Cow<'a, [u8]>,
    pub data: Cow<'a, [u8]>,
}

impl<'a> MessageRead<'a> for ReturnData<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.program_id = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(18) => msg.data = r.read_bytes(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for ReturnData<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.program_id == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.program_id).len()) }
        + if self.data == Cow::Borrowed(b"") { 0 } else { 1 + sizeof_len((&self.data).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.program_id != Cow::Borrowed(b"") { w.write_with_tag(10, |w| w.write_bytes(&**&self.program_id))?; }
        if self.data != Cow::Borrowed(b"") { w.write_with_tag(18, |w| w.write_bytes(&**&self.data))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Reward<'a> {
    pub pubkey: Cow<'a, str>,
    pub lamports: i64,
    pub post_balance: u64,
    pub reward_type: sf::solana::r#type::v1::RewardType,
    pub commission: Cow<'a, str>,
}

impl<'a> MessageRead<'a> for Reward<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.pubkey = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(16) => msg.lamports = r.read_int64(bytes)?,
                Ok(24) => msg.post_balance = r.read_uint64(bytes)?,
                Ok(32) => msg.reward_type = r.read_enum(bytes)?,
                Ok(42) => msg.commission = r.read_string(bytes).map(Cow::Borrowed)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for Reward<'a> {
    fn get_size(&self) -> usize {
        0
        + if self.pubkey == "" { 0 } else { 1 + sizeof_len((&self.pubkey).len()) }
        + if self.lamports == 0i64 { 0 } else { 1 + sizeof_varint(*(&self.lamports) as u64) }
        + if self.post_balance == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.post_balance) as u64) }
        + if self.reward_type == sf::solana::r#type::v1::RewardType::Unspecified { 0 } else { 1 + sizeof_varint(*(&self.reward_type) as u64) }
        + if self.commission == "" { 0 } else { 1 + sizeof_len((&self.commission).len()) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.pubkey != "" { w.write_with_tag(10, |w| w.write_string(&**&self.pubkey))?; }
        if self.lamports != 0i64 { w.write_with_tag(16, |w| w.write_int64(*&self.lamports))?; }
        if self.post_balance != 0u64 { w.write_with_tag(24, |w| w.write_uint64(*&self.post_balance))?; }
        if self.reward_type != sf::solana::r#type::v1::RewardType::Unspecified { w.write_with_tag(32, |w| w.write_enum(*&self.reward_type as i32))?; }
        if self.commission != "" { w.write_with_tag(42, |w| w.write_string(&**&self.commission))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct Rewards<'a> {
    pub rewards: Vec<sf::solana::r#type::v1::Reward<'a>>,
}

impl<'a> MessageRead<'a> for Rewards<'a> {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(10) => msg.rewards.push(r.read_message::<sf::solana::r#type::v1::Reward>(bytes)?),
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl<'a> MessageWrite for Rewards<'a> {
    fn get_size(&self) -> usize {
        0
        + self.rewards.iter().map(|s| 1 + sizeof_len((s).get_size())).sum::<usize>()
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        for s in &self.rewards { w.write_with_tag(10, |w| w.write_message(s))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct UnixTimestamp {
    pub timestamp: i64,
}

impl<'a> MessageRead<'a> for UnixTimestamp {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.timestamp = r.read_int64(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for UnixTimestamp {
    fn get_size(&self) -> usize {
        0
        + if self.timestamp == 0i64 { 0 } else { 1 + sizeof_varint(*(&self.timestamp) as u64) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.timestamp != 0i64 { w.write_with_tag(8, |w| w.write_int64(*&self.timestamp))?; }
        Ok(())
    }
}

#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Debug, Default, PartialEq, Clone)]
pub struct BlockHeight {
    pub block_height: u64,
}

impl<'a> MessageRead<'a> for BlockHeight {
    fn from_reader(r: &mut BytesReader, bytes: &'a [u8]) -> Result<Self> {
        let mut msg = Self::default();
        while !r.is_eof() {
            match r.next_tag(bytes) {
                Ok(8) => msg.block_height = r.read_uint64(bytes)?,
                Ok(t) => { r.read_unknown(bytes, t)?; }
                Err(e) => return Err(e),
            }
        }
        Ok(msg)
    }
}

impl MessageWrite for BlockHeight {
    fn get_size(&self) -> usize {
        0
        + if self.block_height == 0u64 { 0 } else { 1 + sizeof_varint(*(&self.block_height) as u64) }
    }

    fn write_message<W: WriterBackend>(&self, w: &mut Writer<W>) -> Result<()> {
        if self.block_height != 0u64 { w.write_with_tag(8, |w| w.write_uint64(*&self.block_height))?; }
        Ok(())
    }
}

