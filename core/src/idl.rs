//! Traits implemented by code generated from Anchor IDLs, and helpers to extract events from logs.
//!
//! The traits are prefixed ([IdlInstruction], [IdlAccount], [IdlEvent]) to avoid clashing with
//! [Instruction][crate::Instruction]. They decode from any [InstructionInput].

use crate::block_view::{lazy::LazyInstructionView, InstructionView};
use crate::borsh::{BorshDecode, BorshReader, DecodeError, Pubkey};

/// Prefix of an Anchor self-CPI event (`emit_cpi!`), followed by the event discriminator and data.
/// Same as `anchor_lang::event::EVENT_IX_TAG_LE` (first 8 bytes of `sha256("anchor:event")`).
pub const EVENT_IX_TAG_LE: [u8; 8] = [0xe4, 0x45, 0xa5, 0x2e, 0x51, 0xcb, 0x9a, 0x1d];

/// A read-only view of one instruction with resolved program id and accounts.
///
/// Separate from [crate::Instruction], which exposes unresolved account indexes. The block views
/// ([InstructionView], [LazyInstructionView]) panic on an index outside the transaction's account
/// table, like the rest of the block model; [RawInstruction] never panics.
pub trait InstructionInput {
    /// The resolved program id.
    fn program_id(&self) -> &[u8];

    /// The raw instruction data.
    fn data(&self) -> &[u8];

    /// How many accounts the instruction names.
    fn accounts_len(&self) -> usize;

    /// The resolved address of the `index`-th account, [None] past [Self::accounts_len].
    fn account(&self, index: usize) -> Option<&[u8]>;

    /// The `index`-th account as a [Pubkey].
    fn account_key(&self, index: usize) -> Result<Pubkey, DecodeError> {
        let account = self.account(index).ok_or(DecodeError::MissingAccount {
            index,
            len: self.accounts_len(),
        })?;
        <[u8; 32]>::try_from(account)
            .map(Pubkey)
            .map_err(|_| DecodeError::InvalidAccount {
                index,
                len: account.len(),
            })
    }

    /// Every account address the instruction names, in order.
    fn account_keys(&self) -> Result<Vec<Pubkey>, DecodeError> {
        (0..self.accounts_len())
            .map(|index| self.account_key(index))
            .collect()
    }
}

impl<T: InstructionInput + ?Sized> InstructionInput for &T {
    fn program_id(&self) -> &[u8] {
        (**self).program_id()
    }

    fn data(&self) -> &[u8] {
        (**self).data()
    }

    fn accounts_len(&self) -> usize {
        (**self).accounts_len()
    }

    fn account(&self, index: usize) -> Option<&[u8]> {
        (**self).account(index)
    }
}

impl InstructionInput for InstructionView<'_> {
    fn program_id(&self) -> &[u8] {
        let index = self.raw_instruction().program_id_index() as u8;
        self.confirmed_transaction().account_at(index).0
    }

    fn data(&self) -> &[u8] {
        self.raw_instruction().data()
    }

    fn accounts_len(&self) -> usize {
        self.raw_instruction().accounts().len()
    }

    fn account(&self, index: usize) -> Option<&[u8]> {
        let account_index = *self.raw_instruction().accounts().get(index)?;
        Some(self.confirmed_transaction().account_at(account_index).0)
    }
}

impl InstructionInput for LazyInstructionView<'_> {
    fn program_id(&self) -> &[u8] {
        self.resolved_account(self.program_id_index as u8)
    }

    fn data(&self) -> &[u8] {
        self.data
    }

    fn accounts_len(&self) -> usize {
        self.accounts.len()
    }

    fn account(&self, index: usize) -> Option<&[u8]> {
        Some(self.resolved_account(*self.accounts.get(index)?))
    }
}

/// An instruction built from already-resolved parts, for sources without a block view (RPC, tests).
#[derive(Debug, Clone, Copy)]
pub struct RawInstruction<'a> {
    pub program_id: &'a [u8],
    pub data: &'a [u8],
    pub accounts: &'a [&'a [u8]],
}

impl InstructionInput for RawInstruction<'_> {
    fn program_id(&self) -> &[u8] {
        self.program_id
    }

    fn data(&self) -> &[u8] {
        self.data
    }

    fn accounts_len(&self) -> usize {
        self.accounts.len()
    }

    fn account(&self, index: usize) -> Option<&[u8]> {
        self.accounts.get(index).copied()
    }
}

/// The accounts of an instruction matched to its IDL account slots.
///
/// With at least as many accounts as listed, slots are positional and an optional slot holding
/// the program id is omitted (Anchor's and Metaplex's placeholder). With fewer, the omitted
/// optional accounts are left out and later accounts shift down, as Shank programs read them.
#[derive(Debug, Clone, Copy)]
pub struct NamedAccounts<'a, I: ?Sized> {
    ix: &'a I,
    optional: &'a [bool],
    /// Whether every optional account is left out of the list.
    shifted: bool,
}

impl<'a, I: InstructionInput + ?Sized> NamedAccounts<'a, I> {
    /// Matches the accounts of `ix` to the slots of a layout, `optional[i]` telling whether slot
    /// `i` is optional. Fails with [DecodeError::MissingAccount] when too few accounts remain even
    /// without the optional ones, [DecodeError::AmbiguousAccounts] when only some are left out.
    pub fn resolve(ix: &'a I, optional: &'a [bool]) -> Result<Self, DecodeError> {
        let len = ix.accounts_len();
        let listed = optional.len();
        let shifted = len < listed;
        if shifted {
            let optional_count = optional.iter().filter(|&&o| o).count();
            let missing = listed - len;
            if missing > optional_count {
                return Err(DecodeError::MissingAccount { index: len, len });
            }
            if missing < optional_count {
                return Err(DecodeError::AmbiguousAccounts {
                    len,
                    listed,
                    optional: optional_count,
                });
            }
        }
        Ok(Self {
            ix,
            optional,
            shifted,
        })
    }

    /// The account of a required slot.
    pub fn required(&self, slot: usize) -> Result<Pubkey, DecodeError> {
        let index = match (self.shifted, self.optional.get(..slot)) {
            (true, Some(before)) => slot - before.iter().filter(|&&o| o).count(),
            _ => slot,
        };
        self.ix.account_key(index)
    }

    /// The account of an optional slot, [None] when omitted.
    pub fn optional(&self, slot: usize) -> Result<Option<Pubkey>, DecodeError> {
        if self.shifted || slot >= self.ix.accounts_len() {
            return Ok(None);
        }
        let key = self.ix.account_key(slot)?;
        Ok((key != self.ix.program_id()).then_some(key))
    }
}

/// An instruction of a program described by an IDL.
///
/// ```ignore
/// for ix in trx.walk_instructions() {
///     if let Some(Ok(swap)) = Swap::match_and_decode(&ix) {
///         // ...
///     }
/// }
/// ```
pub trait IdlInstruction: Sized {
    /// The program the instruction belongs to.
    const PROGRAM_ID: Pubkey;

    /// Leading bytes of the instruction data identifying it, usually 8.
    const DISCRIMINATOR: &'static [u8];

    /// Decodes the arguments from `args` (after the discriminator) and the accounts from `ix`.
    fn decode_instruction<I: InstructionInput + ?Sized>(
        args: &mut BorshReader<'_>,
        ix: &I,
    ) -> Result<Self, DecodeError>;

    /// Whether `ix` targets [Self::PROGRAM_ID] and its data starts with [Self::DISCRIMINATOR].
    fn match_instruction<I: InstructionInput + ?Sized>(ix: &I) -> bool {
        Self::PROGRAM_ID == ix.program_id() && ix.data().starts_with(Self::DISCRIMINATOR)
    }

    /// Decodes `ix`, checking the discriminator but not the program id.
    fn decode<I: InstructionInput + ?Sized>(ix: &I) -> Result<Self, DecodeError> {
        let mut r = BorshReader::new(ix.data());
        r.expect_discriminator(Self::DISCRIMINATOR)?;
        Self::decode_instruction(&mut r, ix)
    }

    /// [Self::decode] if [Self::match_instruction], [None] otherwise.
    fn match_and_decode<I: InstructionInput + ?Sized>(ix: &I) -> Option<Result<Self, DecodeError>> {
        Self::match_instruction(ix).then(|| Self::decode(ix))
    }
}

/// An account type described by an IDL. The [BorshDecode] impl decodes the fields that follow
/// the discriminator.
pub trait IdlAccount: BorshDecode {
    /// Leading bytes of the account data identifying the type.
    const DISCRIMINATOR: &'static [u8];

    /// Whether `data` starts with [Self::DISCRIMINATOR]. Does not check the account owner.
    fn match_account(data: &[u8]) -> bool {
        data.starts_with(Self::DISCRIMINATOR)
    }

    /// Decodes raw account data, discriminator included.
    fn decode_account(data: &[u8]) -> Result<Self, DecodeError> {
        decode_prefixed(data, &[], Self::DISCRIMINATOR)
    }

    fn match_and_decode(data: &[u8]) -> Option<Result<Self, DecodeError>> {
        Self::match_account(data).then(|| Self::decode_account(data))
    }
}

/// An event described by an IDL, emitted in `Program data:` logs (`emit!`) or as a self-CPI
/// (`emit_cpi!`). The [BorshDecode] impl decodes the fields after the discriminator.
pub trait IdlEvent: BorshDecode {
    /// The program emitting the event.
    const PROGRAM_ID: Pubkey;

    /// Leading bytes of the event payload identifying the type.
    const DISCRIMINATOR: &'static [u8];

    /// Whether a decoded `Program data:` payload holds this event.
    fn match_log_data(data: &[u8]) -> bool {
        data.starts_with(Self::DISCRIMINATOR)
    }

    /// Decodes a `Program data:` payload, discriminator included.
    fn decode_log_data(data: &[u8]) -> Result<Self, DecodeError> {
        decode_prefixed(data, &[], Self::DISCRIMINATOR)
    }

    fn match_and_decode_log_data(data: &[u8]) -> Option<Result<Self, DecodeError>> {
        Self::match_log_data(data).then(|| Self::decode_log_data(data))
    }

    /// Every occurrence of this event logged by [Self::PROGRAM_ID]. See [program_data].
    fn decode_logs<L>(logs: L) -> impl Iterator<Item = Result<Self, DecodeError>>
    where
        L: IntoIterator,
        L::Item: AsRef<str>,
    {
        program_data(logs, Self::PROGRAM_ID.as_bytes())
            .filter_map(|data| Self::match_and_decode_log_data(&data))
    }

    /// Whether `ix` is the self-CPI emitting this event.
    fn match_cpi<I: InstructionInput + ?Sized>(ix: &I) -> bool {
        Self::PROGRAM_ID == ix.program_id()
            && ix
                .data()
                .strip_prefix(EVENT_IX_TAG_LE.as_slice())
                .is_some_and(|rest| rest.starts_with(Self::DISCRIMINATOR))
    }

    /// Decodes a self-CPI event instruction, checking both prefixes but not the program id.
    fn decode_cpi<I: InstructionInput + ?Sized>(ix: &I) -> Result<Self, DecodeError> {
        decode_prefixed(ix.data(), &EVENT_IX_TAG_LE, Self::DISCRIMINATOR)
    }

    fn match_and_decode_cpi<I: InstructionInput + ?Sized>(
        ix: &I,
    ) -> Option<Result<Self, DecodeError>> {
        Self::match_cpi(ix).then(|| Self::decode_cpi(ix))
    }
}

fn decode_prefixed<T: BorshDecode>(
    data: &[u8],
    tag: &'static [u8],
    discriminator: &'static [u8],
) -> Result<T, DecodeError> {
    let mut r = BorshReader::new(data);
    r.expect_discriminator(tag)?;
    r.expect_discriminator(discriminator)?;
    T::decode(&mut r)
}

/// The decoded payloads of the `Program data:` lines that `program_id` itself logged, in order.
///
/// `invoke [n]` / `success` / `failed` lines are tracked as a call stack so CPI logs go to the
/// callee. Stops at `Log truncated`; invalid base64 lines are skipped.
pub fn program_data<L>(logs: L, program_id: &[u8]) -> ProgramData<L::IntoIter>
where
    L: IntoIterator,
    L::Item: AsRef<str>,
{
    let mut id = [0u8; 44];
    // Ids that are not 32-byte addresses do not fit and never match.
    let id_len = bs58::encode(program_id)
        .into(&mut id[..])
        .ok()
        .filter(|&n| n > 0);
    ProgramData {
        logs: logs.into_iter(),
        program_id: id,
        program_id_len: id_len,
        stack: Vec::new(),
    }
}

/// Iterator returned by [program_data].
pub struct ProgramData<I> {
    logs: I,
    /// The base58 program id, encoded once.
    program_id: [u8; 44],
    program_id_len: Option<usize>,
    /// One entry per active invocation, `true` when it is the tracked program.
    stack: Vec<bool>,
}

impl<I> Iterator for ProgramData<I>
where
    I: Iterator,
    I::Item: AsRef<str>,
{
    type Item = Vec<u8>;

    fn next(&mut self) -> Option<Vec<u8>> {
        for log in self.logs.by_ref() {
            let line = log.as_ref();

            if let Some(payload) = line.strip_prefix("Program data: ") {
                if self.stack.last() == Some(&true) {
                    if let Some(data) = base64_decode_fields(payload) {
                        return Some(data);
                    }
                }
                continue;
            }

            if line == "Log truncated" {
                self.stack.clear();
                // Drain so later calls keep returning None.
                self.logs.by_ref().for_each(drop);
                return None;
            }

            let Some((id, tail)) = line
                .strip_prefix("Program ")
                .and_then(|rest| rest.split_once(' '))
            else {
                continue;
            };
            // `Program log:`, `Program return:` and friends; base58 has no ':'.
            if id.contains(':') {
                continue;
            }

            if let Some(depth) = tail
                .strip_prefix("invoke [")
                .and_then(|d| d.strip_suffix(']'))
                .and_then(|d| d.parse::<usize>().ok())
            {
                self.stack.truncate(depth.saturating_sub(1));
                let tracked = self
                    .program_id_len
                    .is_some_and(|len| id.as_bytes() == &self.program_id[..len]);
                self.stack.push(tracked);
            } else if tail == "success" || tail.starts_with("failed") {
                self.stack.pop();
            }
        }
        None
    }
}

/// Decodes the space-separated base64 fields of a `Program data:` line (padding optional) into
/// their concatenation. [None] on invalid input.
fn base64_decode_fields(payload: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(payload.len() / 4 * 3 + 2);
    for field in payload.split(' ').filter(|f| !f.is_empty()) {
        if !base64_decode_into(field.as_bytes(), &mut out) {
            return None;
        }
    }
    Some(out)
}

fn base64_decode_into(input: &[u8], out: &mut Vec<u8>) -> bool {
    fn value(c: u8) -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32)
    }

    let data = match input {
        [rest @ .., b'=', b'='] if input.len() % 4 == 0 => rest,
        [rest @ .., b'='] if input.len() % 4 == 0 => rest,
        _ => input,
    };
    if data.len() % 4 == 1 {
        return false;
    }

    let mut chunks = data.chunks_exact(4);
    for chunk in chunks.by_ref() {
        let mut n = 0u32;
        for &c in chunk {
            match value(c) {
                Some(v) => n = (n << 6) | v,
                None => return false,
            }
        }
        out.extend_from_slice(&n.to_be_bytes()[1..]);
    }

    let rest = chunks.remainder();
    if rest.is_empty() {
        return true;
    }
    let mut n = 0u32;
    for &c in rest {
        match value(c) {
            Some(v) => n = (n << 6) | v,
            None => return false,
        }
    }
    // 2 chars carry 1 byte, 3 chars carry 2 bytes; left-align the bits.
    n <<= 6 * (4 - rest.len() as u32);
    let bytes = n.to_be_bytes();
    out.extend_from_slice(&bytes[1..rest.len()]);
    true
}

#[cfg(test)]
mod tests;
