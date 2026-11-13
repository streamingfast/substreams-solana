//! A dependency-free [Borsh](https://borsh.io) runtime used by code generated from Anchor IDLs.
//!
//! Decoding never panics: bad input returns a [DecodeError] with the offset. Trailing bytes are
//! ignored, as in Anchor; use [decode_exact] or [BorshReader::finish] to reject them. NaN floats
//! are rejected on decode.

use std::fmt;

/// Error returned when bytes cannot be decoded into the requested type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecodeError {
    /// The input ended before `needed` more bytes could be read at `offset`.
    UnexpectedEof {
        offset: usize,
        needed: usize,
        remaining: usize,
    },
    /// A `bool` byte was neither 0 nor 1.
    InvalidBool { offset: usize, value: u8 },
    /// An `Option` tag was neither 0 nor 1.
    InvalidOptionTag { offset: usize, tag: u8 },
    /// A [COption] tag was neither 0 nor 1.
    InvalidCOptionTag { offset: usize, tag: u32 },
    /// An enum variant index does not exist on `type_name`.
    InvalidEnumVariant {
        offset: usize,
        type_name: &'static str,
        variant: u8,
    },
    /// A string was not valid UTF-8.
    InvalidUtf8 { offset: usize },
    /// A float decoded to NaN, which Borsh forbids.
    InvalidFloat { offset: usize },
    /// A length-prefixed collection of zero-sized elements.
    ZeroSizedElements { offset: usize },
    /// The data does not start with the expected discriminator. `found` holds the leading bytes
    /// of the data, at most as many as `expected`.
    UnknownDiscriminator {
        expected: &'static [u8],
        found: Vec<u8>,
    },
    /// The instruction names fewer accounts than the decoded layout requires.
    MissingAccount { index: usize, len: usize },
    /// An account resolved to something other than a 32-byte address.
    InvalidAccount { index: usize, len: usize },
    /// The instruction lists `len` of `listed` accounts and the omitted optional ones cannot be
    /// told apart. See [NamedAccounts][crate::idl::NamedAccounts].
    AmbiguousAccounts {
        len: usize,
        listed: usize,
        optional: usize,
    },
    /// Bytes remained after decoding a value that must consume all of its input.
    TrailingBytes { offset: usize, remaining: usize },
    /// Values nest deeper than [MAX_DEPTH] at `offset`.
    DepthLimit { offset: usize },
    /// A free-form error for decoders written by hand.
    Custom(&'static str),
}

impl fmt::Display for DecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use DecodeError::*;
        match self {
            UnexpectedEof {
                offset,
                needed,
                remaining,
            } => write!(
                f,
                "expected {needed} bytes at offset {offset}, only {remaining} remaining"
            ),
            InvalidBool { offset, value } => {
                write!(f, "invalid bool value {value} at offset {offset}")
            }
            InvalidOptionTag { offset, tag } => {
                write!(f, "invalid option tag {tag} at offset {offset}")
            }
            InvalidCOptionTag { offset, tag } => {
                write!(f, "invalid coption tag {tag} at offset {offset}")
            }
            InvalidEnumVariant {
                offset,
                type_name,
                variant,
            } => write!(
                f,
                "invalid variant {variant} for enum {type_name} at offset {offset}"
            ),
            InvalidUtf8 { offset } => write!(f, "invalid utf-8 string at offset {offset}"),
            InvalidFloat { offset } => write!(f, "NaN float at offset {offset}"),
            ZeroSizedElements { offset } => {
                write!(f, "collection of zero-sized elements at offset {offset}")
            }
            UnknownDiscriminator { expected, found } => write!(
                f,
                "unknown discriminator {}, expected {}",
                Hex(found),
                Hex(expected)
            ),
            MissingAccount { index, len } => write!(
                f,
                "missing account at index {index}, instruction has {len} accounts"
            ),
            InvalidAccount { index, len } => {
                write!(f, "account at index {index} is {len} bytes, expected 32")
            }
            AmbiguousAccounts {
                len,
                listed,
                optional,
            } => write!(
                f,
                "instruction has {len} of {listed} listed accounts: which of the {optional} \
                 optional accounts are left out is ambiguous"
            ),
            TrailingBytes { offset, remaining } => {
                write!(f, "{remaining} trailing bytes at offset {offset}")
            }
            DepthLimit { offset } => {
                write!(
                    f,
                    "values nest deeper than {MAX_DEPTH} levels at offset {offset}"
                )
            }
            Custom(msg) => f.write_str(msg),
        }
    }
}

impl std::error::Error for DecodeError {}

struct Hex<'a>(&'a [u8]);

impl fmt::Display for Hex<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for b in self.0 {
            write!(f, "{b:02x}")?;
        }
        Ok(())
    }
}

/// A cursor over borrowed Borsh-encoded bytes.
#[derive(Debug, Clone)]
pub struct BorshReader<'a> {
    data: &'a [u8],
    offset: usize,
    depth: usize,
}

/// How deep `Option`, `Box`, `Vec` and `COption` values may nest, as in borsh-rs.
pub const MAX_DEPTH: usize = 128;

/// Upper bound, in bytes, of the capacity reserved from a length prefix, as in borsh-rs.
const MAX_PREALLOCATION: usize = 4096;

impl<'a> BorshReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self {
            data,
            offset: 0,
            depth: 0,
        }
    }

    /// Runs `f` one nesting level deeper, failing with [DecodeError::DepthLimit] past
    /// [MAX_DEPTH].
    pub fn nested<T>(
        &mut self,
        f: impl FnOnce(&mut Self) -> Result<T, DecodeError>,
    ) -> Result<T, DecodeError> {
        if self.depth >= MAX_DEPTH {
            return Err(DecodeError::DepthLimit {
                offset: self.offset,
            });
        }
        self.depth += 1;
        let result = f(self);
        self.depth -= 1;
        result
    }

    /// Position of the cursor from the start of the input.
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// The bytes not read yet.
    pub fn remaining(&self) -> &'a [u8] {
        // `offset` never exceeds `data.len()`, see `read_bytes`.
        &self.data[self.offset..]
    }

    /// Whether every byte has been read.
    pub fn is_empty(&self) -> bool {
        self.offset >= self.data.len()
    }

    /// Reads a `T` at the cursor.
    pub fn read<T: BorshDecode>(&mut self) -> Result<T, DecodeError> {
        T::decode(self)
    }

    /// Borrows the next `n` bytes and advances past them.
    pub fn read_bytes(&mut self, n: usize) -> Result<&'a [u8], DecodeError> {
        let remaining = self.data.len() - self.offset;
        if n > remaining {
            return Err(DecodeError::UnexpectedEof {
                offset: self.offset,
                needed: n,
                remaining,
            });
        }
        let out = &self.data[self.offset..self.offset + n];
        self.offset += n;
        Ok(out)
    }

    /// Borrows the next `N` bytes as a fixed-size array.
    pub fn read_array<const N: usize>(&mut self) -> Result<&'a [u8; N], DecodeError> {
        let bytes = self.read_bytes(N)?;
        // Cannot fail, `read_bytes` returned exactly N bytes.
        bytes
            .try_into()
            .map_err(|_| DecodeError::Custom("array length"))
    }

    /// Advances past `n` bytes.
    pub fn skip(&mut self, n: usize) -> Result<(), DecodeError> {
        self.read_bytes(n).map(|_| ())
    }

    /// Reads a `u32` length prefix.
    pub fn read_len(&mut self) -> Result<usize, DecodeError> {
        Ok(u32::from_le_bytes(*self.read_array::<4>()?) as usize)
    }

    /// Borrows a `u32`-length-prefixed byte string (the IDL `bytes` type) without copying.
    pub fn read_byte_slice(&mut self) -> Result<&'a [u8], DecodeError> {
        let len = self.read_len()?;
        self.read_bytes(len)
    }

    /// Borrows a `u32`-length-prefixed UTF-8 string without copying.
    pub fn read_str(&mut self) -> Result<&'a str, DecodeError> {
        let len = self.read_len()?;
        let offset = self.offset;
        let bytes = self.read_bytes(len)?;
        std::str::from_utf8(bytes).map_err(|_| DecodeError::InvalidUtf8 { offset })
    }

    /// Reads the one-byte variant index of an enum. Pair with
    /// [DecodeError::invalid_variant] for unknown indices.
    pub fn read_variant(&mut self) -> Result<u8, DecodeError> {
        self.read::<u8>()
    }

    /// Consumes `discriminator` from the cursor, or fails with
    /// [DecodeError::UnknownDiscriminator] without advancing.
    pub fn expect_discriminator(
        &mut self,
        discriminator: &'static [u8],
    ) -> Result<(), DecodeError> {
        let rest = self.remaining();
        if rest.starts_with(discriminator) {
            self.offset += discriminator.len();
            return Ok(());
        }
        let found = &rest[..rest.len().min(discriminator.len())];
        Err(DecodeError::UnknownDiscriminator {
            expected: discriminator,
            found: found.to_vec(),
        })
    }

    /// Fails with [DecodeError::TrailingBytes] if any byte is left unread.
    pub fn finish(&self) -> Result<(), DecodeError> {
        if self.is_empty() {
            Ok(())
        } else {
            Err(DecodeError::TrailingBytes {
                offset: self.offset,
                remaining: self.data.len() - self.offset,
            })
        }
    }
}

impl DecodeError {
    /// Builds [DecodeError::InvalidEnumVariant] for a variant index just read by
    /// [BorshReader::read_variant].
    pub fn invalid_variant(r: &BorshReader<'_>, type_name: &'static str, variant: u8) -> Self {
        DecodeError::InvalidEnumVariant {
            offset: r.offset().saturating_sub(1),
            type_name,
            variant,
        }
    }
}

/// A type that can be read from Borsh-encoded bytes.
pub trait BorshDecode: Sized {
    fn decode(r: &mut BorshReader<'_>) -> Result<Self, DecodeError>;

    /// Decodes `len` consecutive values. Overridden for `u8` to copy in one go; not meant to be
    /// called directly.
    #[doc(hidden)]
    fn decode_vec(r: &mut BorshReader<'_>, len: usize) -> Result<Vec<Self>, DecodeError> {
        let size = std::mem::size_of::<Self>().max(1);
        let capacity = len
            .min(r.remaining().len() / size)
            .min(MAX_PREALLOCATION / size);
        let mut out = Vec::with_capacity(capacity);
        r.nested(|r| {
            for _ in 0..len {
                let before = r.offset();
                out.push(Self::decode(r)?);
                if r.offset() == before {
                    return Err(DecodeError::ZeroSizedElements { offset: before });
                }
            }
            Ok(())
        })?;
        Ok(out)
    }

    /// Decodes `N` consecutive values. Overridden for `u8` to avoid the intermediate `Vec`; not
    /// meant to be called directly.
    #[doc(hidden)]
    fn decode_array<const N: usize>(r: &mut BorshReader<'_>) -> Result<[Self; N], DecodeError> {
        try_array(|| Self::decode(r))
    }
}

/// Builds `[T; N]` from `N` calls to `f`, without an intermediate `Vec`, stopping at the first
/// error.
pub fn try_array<T, const N: usize>(
    mut f: impl FnMut() -> Result<T, DecodeError>,
) -> Result<[T; N], DecodeError> {
    let mut error = None;
    let items: [Option<T>; N] = std::array::from_fn(|_| {
        if error.is_some() {
            return None;
        }
        f().map_err(|e| error = Some(e)).ok()
    });
    match error {
        Some(error) => Err(error),
        // Without an error, every item is `Some`.
        None => Ok(items.map(|item| item.unwrap_or_else(|| unreachable!()))),
    }
}

/// Decodes a `T` from the start of `data`, ignoring trailing bytes (Anchor semantics).
pub fn decode<T: BorshDecode>(data: &[u8]) -> Result<T, DecodeError> {
    T::decode(&mut BorshReader::new(data))
}

/// Decodes a `T` that must span all of `data`.
pub fn decode_exact<T: BorshDecode>(data: &[u8]) -> Result<T, DecodeError> {
    let mut r = BorshReader::new(data);
    let value = T::decode(&mut r)?;
    r.finish()?;
    Ok(value)
}

macro_rules! impl_int {
    ($($ty:ty),*) => {$(
        impl BorshDecode for $ty {
            #[inline]
            fn decode(r: &mut BorshReader<'_>) -> Result<Self, DecodeError> {
                Ok(<$ty>::from_le_bytes(*r.read_array()?))
            }
        }

    )*};
}

impl_int!(u16, u32, u64, u128, i8, i16, i32, i64, i128);

impl BorshDecode for u8 {
    #[inline]
    fn decode(r: &mut BorshReader<'_>) -> Result<Self, DecodeError> {
        Ok(r.read_array::<1>()?[0])
    }

    fn decode_vec(r: &mut BorshReader<'_>, len: usize) -> Result<Vec<Self>, DecodeError> {
        r.read_bytes(len).map(<[u8]>::to_vec)
    }

    fn decode_array<const N: usize>(r: &mut BorshReader<'_>) -> Result<[Self; N], DecodeError> {
        r.read_array::<N>().copied()
    }
}

macro_rules! impl_float {
    ($($ty:ty),*) => {$(
        impl BorshDecode for $ty {
            fn decode(r: &mut BorshReader<'_>) -> Result<Self, DecodeError> {
                let offset = r.offset();
                let value = <$ty>::from_le_bytes(*r.read_array()?);
                if value.is_nan() {
                    return Err(DecodeError::InvalidFloat { offset });
                }
                Ok(value)
            }
        }

    )*};
}

impl_float!(f32, f64);

impl BorshDecode for bool {
    fn decode(r: &mut BorshReader<'_>) -> Result<Self, DecodeError> {
        let offset = r.offset();
        match u8::decode(r)? {
            0 => Ok(false),
            1 => Ok(true),
            value => Err(DecodeError::InvalidBool { offset, value }),
        }
    }
}

impl BorshDecode for String {
    fn decode(r: &mut BorshReader<'_>) -> Result<Self, DecodeError> {
        r.read_str().map(str::to_owned)
    }
}

impl<T: BorshDecode> BorshDecode for Vec<T> {
    fn decode(r: &mut BorshReader<'_>) -> Result<Self, DecodeError> {
        let len = r.read_len()?;
        T::decode_vec(r, len)
    }
}

impl<T: BorshDecode, const N: usize> BorshDecode for [T; N] {
    fn decode(r: &mut BorshReader<'_>) -> Result<Self, DecodeError> {
        T::decode_array::<N>(r)
    }
}

impl<T: BorshDecode> BorshDecode for Option<T> {
    fn decode(r: &mut BorshReader<'_>) -> Result<Self, DecodeError> {
        let offset = r.offset();
        match u8::decode(r)? {
            0 => Ok(None),
            1 => r.nested(T::decode).map(Some),
            tag => Err(DecodeError::InvalidOptionTag { offset, tag }),
        }
    }
}

impl<T: BorshDecode> BorshDecode for Box<T> {
    fn decode(r: &mut BorshReader<'_>) -> Result<Self, DecodeError> {
        r.nested(T::decode).map(Box::new)
    }
}

macro_rules! impl_tuple {
    ($(($($name:ident $idx:tt),+)),*) => {$(
        impl<$($name: BorshDecode),+> BorshDecode for ($($name,)+) {
            fn decode(r: &mut BorshReader<'_>) -> Result<Self, DecodeError> {
                Ok(($($name::decode(r)?,)+))
            }
        }

    )*};
}

impl_tuple!(
    (A 0),
    (A 0, B 1),
    (A 0, B 1, C 2),
    (A 0, B 1, C 2, D 3),
    (A 0, B 1, C 2, D 3, E 4),
    (A 0, B 1, C 2, D 3, E 4, F 5)
);

/// An owned 32-byte Solana address, usable in `const` context.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Pubkey(pub [u8; 32]);

impl Pubkey {
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    pub const fn to_bytes(self) -> [u8; 32] {
        self.0
    }

    /// Base58 encoding of the address.
    pub fn to_base58(&self) -> String {
        crate::base58::encode(self.0)
    }
}

impl fmt::Display for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_base58())
    }
}

impl fmt::Debug for Pubkey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Pubkey({})", self.to_base58())
    }
}

/// Error parsing a [Pubkey] from base58 text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsePubkeyError;

impl fmt::Display for ParsePubkeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid base58 pubkey")
    }
}

impl std::error::Error for ParsePubkeyError {}

impl std::str::FromStr for Pubkey {
    type Err = ParsePubkeyError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut out = [0u8; 32];
        match bs58::decode(s).into(&mut out) {
            Ok(32) => Ok(Pubkey(out)),
            _ => Err(ParsePubkeyError),
        }
    }
}

impl From<[u8; 32]> for Pubkey {
    fn from(bytes: [u8; 32]) -> Self {
        Pubkey(bytes)
    }
}

impl From<Pubkey> for [u8; 32] {
    fn from(key: Pubkey) -> Self {
        key.0
    }
}

impl TryFrom<&[u8]> for Pubkey {
    type Error = DecodeError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        <[u8; 32]>::try_from(bytes)
            .map(Pubkey)
            .map_err(|_| DecodeError::UnexpectedEof {
                offset: 0,
                needed: 32,
                remaining: bytes.len(),
            })
    }
}

impl AsRef<[u8]> for Pubkey {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl PartialEq<[u8]> for Pubkey {
    fn eq(&self, other: &[u8]) -> bool {
        self.0.as_slice() == other
    }
}

impl PartialEq<&[u8]> for Pubkey {
    fn eq(&self, other: &&[u8]) -> bool {
        self.0.as_slice() == *other
    }
}

impl BorshDecode for Pubkey {
    fn decode(r: &mut BorshReader<'_>) -> Result<Self, DecodeError> {
        r.read_array::<32>().map(|b| Pubkey(*b))
    }
}

/// The IDL `coption` type: a 4-byte little-endian tag, followed by the value only when `Some`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum COption<T> {
    #[default]
    None,
    Some(T),
}

impl<T> COption<T> {
    pub fn as_ref(&self) -> COption<&T> {
        match self {
            COption::None => COption::None,
            COption::Some(v) => COption::Some(v),
        }
    }

    pub fn is_some(&self) -> bool {
        matches!(self, COption::Some(_))
    }

    pub fn is_none(&self) -> bool {
        !self.is_some()
    }

    pub fn into_option(self) -> Option<T> {
        self.into()
    }
}

impl<T> From<Option<T>> for COption<T> {
    fn from(value: Option<T>) -> Self {
        match value {
            None => COption::None,
            Some(v) => COption::Some(v),
        }
    }
}

impl<T> From<COption<T>> for Option<T> {
    fn from(value: COption<T>) -> Self {
        match value {
            COption::None => None,
            COption::Some(v) => Some(v),
        }
    }
}

impl<T: BorshDecode> BorshDecode for COption<T> {
    fn decode(r: &mut BorshReader<'_>) -> Result<Self, DecodeError> {
        let offset = r.offset();
        match u32::decode(r)? {
            0 => Ok(COption::None),
            1 => r.nested(T::decode).map(COption::Some),
            tag => Err(DecodeError::InvalidCOptionTag { offset, tag }),
        }
    }
}

macro_rules! impl_wide_int {
    ($(#[$doc:meta] $name:ident, $from:ident: $small:ty),*) => {$(
        #[$doc]
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Default)]
        pub struct $name(pub [u8; 32]);

        impl $name {
            pub const ZERO: Self = Self([0; 32]);

            pub const fn from_le_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }

            pub const fn to_le_bytes(self) -> [u8; 32] {
                self.0
            }
        }

        impl From<$small> for $name {
            fn from(value: $small) -> Self {
                Self::$from(value)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}({})", stringify!($name), self)
            }
        }

        impl BorshDecode for $name {
            fn decode(r: &mut BorshReader<'_>) -> Result<Self, DecodeError> {
                r.read_array::<32>().map(|b| Self(*b))
            }
        }

    )*};
}

impl_wide_int!(
    /// The IDL `u256` type: 32 bytes, little-endian, unsigned.
    U256, from_u128: u128,
    /// The IDL `i256` type: 32 bytes, little-endian, two's complement.
    I256, from_i128: i128
);

impl U256 {
    pub const fn from_u128(value: u128) -> Self {
        let mut out = [0u8; 32];
        let le = value.to_le_bytes();
        let mut i = 0;
        while i < 16 {
            out[i] = le[i];
            i += 1;
        }
        Self(out)
    }

    /// The value as a `u128`, or `None` if it does not fit.
    pub fn to_u128(&self) -> Option<u128> {
        if self.0[16..].iter().any(|b| *b != 0) {
            return None;
        }
        Some(u128::from_le_bytes(self.0[..16].try_into().ok()?))
    }
}

impl I256 {
    pub const fn from_i128(value: i128) -> Self {
        let fill = if value < 0 { 0xff } else { 0 };
        let mut out = [fill; 32];
        let le = value.to_le_bytes();
        let mut i = 0;
        while i < 16 {
            out[i] = le[i];
            i += 1;
        }
        Self(out)
    }

    pub fn is_negative(&self) -> bool {
        self.0[31] & 0x80 != 0
    }

    /// The value as an `i128`, or `None` if it does not fit.
    pub fn to_i128(&self) -> Option<i128> {
        let low = i128::from_le_bytes(self.0[..16].try_into().ok()?);
        let fill = if low < 0 { 0xff } else { 0 };
        self.0[16..].iter().all(|b| *b == fill).then_some(low)
    }

    /// Absolute value as unsigned magnitude; `I256::MIN` maps to `2^255`.
    fn unsigned_abs(&self) -> U256 {
        if !self.is_negative() {
            return U256(self.0);
        }
        // Two's complement negation: invert and add one.
        let mut out = [0u8; 32];
        let mut carry = 1u16;
        for (o, b) in out.iter_mut().zip(self.0.iter()) {
            let v = (!*b) as u16 + carry;
            *o = v as u8;
            carry = v >> 8;
        }
        U256(out)
    }
}

impl fmt::Display for U256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Repeated division by 10 over little-endian u64 limbs.
        let mut limbs = [0u64; 4];
        for (i, limb) in limbs.iter_mut().enumerate() {
            let mut b = [0u8; 8];
            b.copy_from_slice(&self.0[i * 8..i * 8 + 8]);
            *limb = u64::from_le_bytes(b);
        }
        let mut digits = [0u8; 78];
        let mut n = 0;
        loop {
            let mut rem = 0u128;
            for limb in limbs.iter_mut().rev() {
                let cur = (rem << 64) | *limb as u128;
                *limb = (cur / 10) as u64;
                rem = cur % 10;
            }
            digits[n] = b'0' + rem as u8;
            n += 1;
            if limbs.iter().all(|l| *l == 0) {
                break;
            }
        }
        digits[..n].reverse();
        // Only ASCII digits were written.
        f.pad_integral(true, "", std::str::from_utf8(&digits[..n]).unwrap_or("?"))
    }
}

impl fmt::Display for I256 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let abs = self.unsigned_abs().to_string();
        f.pad_integral(!self.is_negative(), "", &abs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Debug;

    fn decodes<T: BorshDecode + PartialEq + Debug>(value: T, bytes: &[u8]) {
        assert_eq!(decode_exact::<T>(bytes).unwrap(), value);
        // Every strict prefix must fail cleanly.
        for cut in 0..bytes.len() {
            assert!(
                decode::<T>(&bytes[..cut]).is_err(),
                "prefix of {cut} bytes decoded for {value:?}"
            );
        }
    }

    #[test]
    fn primitives() {
        decodes(true, &[1]);
        decodes(false, &[0]);
        decodes(0xabu8, &[0xab]);
        decodes(0x0102u16, &[2, 1]);
        decodes(0x01020304u32, &[4, 3, 2, 1]);
        decodes(0x0102030405060708u64, &[8, 7, 6, 5, 4, 3, 2, 1]);
        decodes(1u128, &{
            let mut b = [0u8; 16];
            b[0] = 1;
            b
        });
        decodes(-1i8, &[0xff]);
        decodes(-2i16, &[0xfe, 0xff]);
        decodes(-1i32, &[0xff; 4]);
        decodes(i64::MIN, &[0, 0, 0, 0, 0, 0, 0, 0x80]);
        decodes(-1i128, &[0xff; 16]);
        decodes(1.5f32, &1.5f32.to_le_bytes());
        decodes(-2.25f64, &(-2.25f64).to_le_bytes());
    }

    #[test]
    fn strings_and_collections() {
        decodes(String::from("hé"), &[3, 0, 0, 0, b'h', 0xc3, 0xa9]);
        decodes(String::new(), &[0, 0, 0, 0]);
        decodes(vec![1u16, 2], &[2, 0, 0, 0, 1, 0, 2, 0]);
        decodes(vec![7u8, 8, 9], &[3, 0, 0, 0, 7, 8, 9]);
        decodes(Vec::<u64>::new(), &[0, 0, 0, 0]);
        decodes([1u8, 2, 3], &[1, 2, 3]);
        decodes([1u32, 2], &[1, 0, 0, 0, 2, 0, 0, 0]);
        decodes(Some(5u8), &[1, 5]);
        decodes(None::<u8>, &[0]);
        decodes(Box::new(3u16), &[3, 0]);
        decodes((1u8, true, String::from("a")), &[1, 1, 1, 0, 0, 0, b'a']);
        decodes(vec![Some(vec![1u8])], &[1, 0, 0, 0, 1, 1, 0, 0, 0, 1]);
        decodes(COption::Some(2u8), &[1, 0, 0, 0, 2]);
        decodes(COption::<u8>::None, &[0, 0, 0, 0]);
        decodes(Pubkey([9; 32]), &[9; 32]);
    }

    #[test]
    fn wide_ints() {
        let mut one = [0u8; 32];
        one[0] = 1;
        decodes(U256::from(1u128), &one);
        decodes(I256::from(-1i128), &[0xff; 32]);

        assert_eq!(U256::from(12345u128).to_string(), "12345");
        assert_eq!(U256::ZERO.to_string(), "0");
        assert_eq!(
            U256([0xff; 32]).to_string(),
            "115792089237316195423570985008687907853269984665640564039457584007913129639935"
        );
        assert_eq!(I256::from(-42i128).to_string(), "-42");
        assert_eq!(I256::from(i128::MIN).to_i128(), Some(i128::MIN));
        assert_eq!(I256::from(-42i128).to_i128(), Some(-42));
        assert_eq!(U256([0xff; 32]).to_u128(), None);
        let mut min = [0u8; 32];
        min[31] = 0x80;
        assert_eq!(
            I256(min).to_string(),
            "-57896044618658097711785492504343953926634992332820282019728792003956564819968"
        );
        assert_eq!(format!("{:?}", U256::from(7u128)), "U256(7)");
    }

    #[test]
    fn pubkey_text() {
        let key: Pubkey = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
            .parse()
            .unwrap();
        assert_eq!(
            key.to_string(),
            "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        );
        assert!("abc".parse::<Pubkey>().is_err());
        assert!("not base58 0OIl".parse::<Pubkey>().is_err());
        assert!(Pubkey::try_from(&[1u8; 31][..]).is_err());
    }

    #[test]
    fn invalid_values() {
        assert_eq!(
            decode::<bool>(&[2]),
            Err(DecodeError::InvalidBool {
                offset: 0,
                value: 2
            })
        );
        assert_eq!(
            decode::<Option<u8>>(&[2, 0]),
            Err(DecodeError::InvalidOptionTag { offset: 0, tag: 2 })
        );
        assert_eq!(
            decode::<COption<u8>>(&[0, 1, 0, 0, 0]),
            Err(DecodeError::InvalidCOptionTag {
                offset: 0,
                tag: 256
            })
        );
        assert_eq!(
            decode::<String>(&[2, 0, 0, 0, 0xc3, 0x28]),
            Err(DecodeError::InvalidUtf8 { offset: 4 })
        );
        assert_eq!(
            decode::<f32>(&f32::NAN.to_le_bytes()),
            Err(DecodeError::InvalidFloat { offset: 0 })
        );
        assert_eq!(
            decode::<(u8, f64)>(&[0, 0, 0, 0, 0, 0, 0, 0xf8, 0x7f]),
            Err(DecodeError::InvalidFloat { offset: 1 })
        );
        assert_eq!(
            decode::<u32>(&[1, 2]),
            Err(DecodeError::UnexpectedEof {
                offset: 0,
                needed: 4,
                remaining: 2
            })
        );
        assert_eq!(
            decode_exact::<u8>(&[1, 2]),
            Err(DecodeError::TrailingBytes {
                offset: 1,
                remaining: 1
            })
        );
        // Trailing bytes are ignored by default, like Anchor.
        assert_eq!(decode::<u8>(&[1, 2]), Ok(1));
    }

    #[test]
    fn nesting_depth_is_limited() {
        #[derive(Debug)]
        #[allow(dead_code)]
        struct Nest(Option<Box<Nest>>);
        impl BorshDecode for Nest {
            fn decode(r: &mut BorshReader<'_>) -> Result<Self, DecodeError> {
                r.read().map(Nest)
            }
        }
        // Each level is an `Option` and a `Box`: two nesting levels.
        let nested = |levels: usize| {
            let mut data = vec![1u8; levels];
            data.push(0);
            data
        };
        assert!(decode::<Nest>(&nested(MAX_DEPTH / 2 - 1)).is_ok());
        assert!(matches!(
            decode::<Nest>(&nested(MAX_DEPTH)),
            Err(DecodeError::DepthLimit { .. })
        ));
    }

    #[test]
    fn huge_length_prefix_does_not_allocate() {
        // Claims u32::MAX elements with no payload behind it.
        let data = [0xff, 0xff, 0xff, 0xff];
        assert!(decode::<Vec<u64>>(&data).is_err());
        assert!(decode::<Vec<u8>>(&data).is_err());
        assert!(decode::<String>(&data).is_err());
    }

    #[derive(Debug, PartialEq)]
    struct Empty;

    impl BorshDecode for Empty {
        fn decode(_: &mut BorshReader<'_>) -> Result<Self, DecodeError> {
            Ok(Empty)
        }
    }

    #[test]
    fn zero_sized_vec_elements_are_rejected() {
        assert_eq!(
            decode::<Vec<Empty>>(&[0xff, 0xff, 0xff, 0xff]),
            Err(DecodeError::ZeroSizedElements { offset: 4 })
        );
        assert_eq!(decode::<Vec<Empty>>(&[0, 0, 0, 0]), Ok(vec![]));
    }

    #[test]
    fn reader_borrows() {
        let data = [3, 0, 0, 0, b'a', b'b', b'c', 2, 0, 0, 0, 1, 2, 9];
        let mut r = BorshReader::new(&data);
        assert_eq!(r.read_str().unwrap(), "abc");
        assert_eq!(r.read_byte_slice().unwrap(), &[1, 2]);
        assert_eq!(r.remaining(), &[9]);
        assert!(!r.is_empty());
        assert!(r.finish().is_err());
        r.skip(1).unwrap();
        assert!(r.is_empty());
        assert!(r.finish().is_ok());
        assert!(r.skip(1).is_err());
    }

    #[test]
    fn discriminator() {
        const D: &[u8] = &[1, 2, 3];
        let mut r = BorshReader::new(&[1, 2, 3, 4]);
        r.expect_discriminator(D).unwrap();
        assert_eq!(r.offset(), 3);

        let mut r = BorshReader::new(&[1, 2]);
        assert_eq!(
            r.expect_discriminator(D),
            Err(DecodeError::UnknownDiscriminator {
                expected: D,
                found: vec![1, 2]
            })
        );
        assert_eq!(r.offset(), 0);
        let err = BorshReader::new(&[9, 9, 9, 9])
            .expect_discriminator(D)
            .unwrap_err();
        assert_eq!(
            err.to_string(),
            "unknown discriminator 090909, expected 010203"
        );
    }

    /// Deterministic pseudo-random bytes, so fuzz-style tests need no extra crate.
    fn garbage(seed: u64, len: usize) -> Vec<u8> {
        let mut state = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (0..len)
            .map(|_| {
                state ^= state << 13;
                state ^= state >> 7;
                state ^= state << 17;
                state as u8
            })
            .collect()
    }

    #[test]
    fn garbage_never_panics() {
        type Nested = (
            Vec<Option<String>>,
            [COption<Pubkey>; 2],
            Box<(bool, f64, I256)>,
            Vec<Vec<u8>>,
        );
        for seed in 0..5000u64 {
            let data = garbage(seed, (seed % 97) as usize);
            let _ = decode::<Nested>(&data);
            let _ = decode::<Vec<(u16, String)>>(&data);
            let _ = decode::<Option<[u64; 3]>>(&data);
            let _ = decode::<Vec<bool>>(&data);
        }
    }
}
