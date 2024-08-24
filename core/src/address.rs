use crate::base58;

/// A wrapper around a byte array that represents a Solana address.
/// It provides a way to convert the address to a base58 encoded string.
pub struct Address<'a>(pub &'a Vec<u8>);

impl Address<'_> {
    /// Returns the address as a base58 encoded string.
    pub fn to_string(&self) -> String {
        base58::encode(self.0)
    }
}

impl<'a> std::fmt::Debug for Address<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&base58::encode(self.0))
    }
}

impl<'a> std::fmt::Display for Address<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&base58::encode(self.0))
    }
}

impl AsRef<[u8]> for Address<'_> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl PartialEq<Address<'_>> for &Address<'_> {
    fn eq(&self, other: &Address<'_>) -> bool {
        *self.0 == *other.0
    }
}

impl PartialEq<Vec<u8>> for &Address<'_> {
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.0 == other
    }
}

impl PartialEq<Address<'_>> for Vec<u8> {
    fn eq(&self, other: &Address) -> bool {
        self == other.0
    }
}

impl PartialEq<Address<'_>> for &Vec<u8> {
    fn eq(&self, other: &Address) -> bool {
        *self == other.0
    }
}

impl PartialEq<&Address<'_>> for Vec<u8> {
    fn eq(&self, other: &&Address) -> bool {
        self == (*other)
    }
}

impl<T: AsRef<[u8]>> PartialEq<T> for Address<'_> {
    fn eq(&self, other: &T) -> bool {
        self.0 == other.as_ref()
    }
}

impl<const N: usize> PartialEq<[u8; N]> for &Address<'_> {
    fn eq(&self, other: &[u8; N]) -> bool {
        self.0.as_slice() == other
    }
}

impl<const N: usize> PartialEq<Address<'_>> for [u8; N] {
    fn eq(&self, other: &Address) -> bool {
        self == other.0.as_slice()
    }
}

impl<const N: usize> PartialEq<Address<'_>> for &[u8; N] {
    fn eq(&self, other: &Address) -> bool {
        *self == other.0.as_slice()
    }
}

impl<const N: usize> PartialEq<&Address<'_>> for [u8; N] {
    fn eq(&self, other: &&Address) -> bool {
        self == other.0.as_slice()
    }
}

#[cfg(test)]
mod tests {
    use super::Address;

    #[test]
    fn it_address_equality_works() {
        let data: Vec<u8> = vec![1, 2, 3];
        let address = Address(&data);

        assert_eq!(&address, &address);
        assert_eq!(&address, Address(&data));
        assert_eq!(Address(&data), &address);
        assert_eq!(Address(&data), Address(&data));

        assert_eq!(address, vec![1, 2, 3]);
        assert_eq!(&address, vec![1, 2, 3]);
        assert_eq!(address, &vec![1, 2, 3]);
        assert_eq!(&address, &vec![1, 2, 3]);

        assert_eq!(vec![1, 2, 3], address);
        assert_eq!(vec![1, 2, 3], &address);
        assert_eq!(&vec![1, 2, 3], address);
        assert_eq!(&vec![1, 2, 3], &address);

        let fixed: [u8; 3] = [1, 2, 3];

        assert_eq!(&address, &[1, 2, 3]);
        assert_eq!(&address, &fixed);
        assert_eq!(&address, [1u8, 2u8, 3u8]);
        assert_eq!(address, &[1, 2, 3]);
        assert_eq!(address, &fixed);
        assert_eq!(address, [1u8, 2u8, 3u8]);

        assert_eq!(&[1, 2, 3], &address);
        assert_eq!(&fixed, &address);
        assert_eq!([1u8, 2u8, 3u8], &address);
        assert_eq!(&[1, 2, 3], address);
        assert_eq!(&fixed, address);
        assert_eq!([1u8, 2u8, 3u8], address);
    }
}
