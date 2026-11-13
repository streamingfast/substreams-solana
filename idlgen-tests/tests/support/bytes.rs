//! A test-only builder of Borsh bytes, so decoding tests need no encoder.

#[derive(Default)]
pub struct Bytes(Vec<u8>);

impl Bytes {
    pub fn with(prefix: &[u8]) -> Self {
        Self(prefix.to_vec())
    }

    pub fn raw(mut self, bytes: &[u8]) -> Self {
        self.0.extend_from_slice(bytes);
        self
    }

    pub fn u8(self, v: u8) -> Self {
        self.raw(&[v])
    }

    pub fn bool(self, v: bool) -> Self {
        self.u8(v as u8)
    }

    pub fn u16(self, v: u16) -> Self {
        self.raw(&v.to_le_bytes())
    }

    pub fn u32(self, v: u32) -> Self {
        self.raw(&v.to_le_bytes())
    }

    pub fn u64(self, v: u64) -> Self {
        self.raw(&v.to_le_bytes())
    }

    pub fn i64(self, v: i64) -> Self {
        self.raw(&v.to_le_bytes())
    }

    pub fn u128(self, v: u128) -> Self {
        self.raw(&v.to_le_bytes())
    }

    pub fn i128(self, v: i128) -> Self {
        self.raw(&v.to_le_bytes())
    }

    pub fn f64(self, v: f64) -> Self {
        self.raw(&v.to_le_bytes())
    }

    pub fn key(self, key: [u8; 32]) -> Self {
        self.raw(&key)
    }

    /// A `u32` length prefix.
    pub fn len(self, len: usize) -> Self {
        self.u32(len as u32)
    }

    pub fn string(self, s: &str) -> Self {
        self.len(s.len()).raw(s.as_bytes())
    }

    pub fn build(self) -> Vec<u8> {
        self.0
    }
}
