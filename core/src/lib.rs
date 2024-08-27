use std::ops::Deref;

use address::Address;
use pb::sf::solana::r#type::v1::{CompiledInstruction, InnerInstruction, Transaction};

use crate::pb::sf::solana::r#type::v1::ConfirmedTransaction;

pub mod address;
pub mod pb;

/// Helpers to deal with block sources.
pub mod block_view;

/// Helpers to deal with base58 encoding and decoding.
pub mod base58 {
    /// Base58 encoding helper using [bs58] crate internally. This method
    /// exists for having a simpler API to encode to base58 [String] type, particularly
    /// useful when mapping over a collection of byte arrays where you can use `.map(base58::encode)`
    ///
    /// Advanced use case(s) like encode to [`Vec<u8>`] or to an existing buffer
    /// can use `bs58::encode` directly.
    pub fn encode<T: AsRef<[u8]>>(data: T) -> String {
        bs58::encode(data.as_ref()).into_string()
    }

    /// Base58 decoding helper using [bs58] crate internally. This method
    /// exists for having a simpler API to decoder from [`AsRef<str>`] (so &[str],
    /// [String] and mostly any string implementation) to [`Vec<u8>`].
    ///
    /// Advanced use case(s) like decode to an existing buffer can use `bs58::decode`
    /// directly.
    pub fn decode<T: AsRef<str>>(data: T) -> Result<Vec<u8>, bs58::decode::Error> {
        bs58::decode(data.as_ref()).into_vec()
    }
}

/// Instruction trait to be implemented by all instructions. The trait enables you to work on
/// a generic instruction type instead of working with either [CompiledInstruction] or [InnerInstruction]
/// model.
pub trait Instruction {
    /// Returns the index of the program id in the transaction message's account keys.
    fn program_id_index(&self) -> u32;

    /// Returns the indices of the accounts that are specified for this instruction. Those are
    /// not the resolved addresses but the indices of the accounts in the transaction message.
    fn accounts(&self) -> &Vec<u8>;
    fn data(&self) -> &Vec<u8>;
    fn stack_height(&self) -> Option<u32>;
}

impl<'a> Instruction for Box<dyn Instruction + 'a> {
    fn program_id_index(&self) -> u32 {
        self.deref().program_id_index()
    }

    fn accounts(&self) -> &Vec<u8> {
        self.deref().accounts()
    }

    fn data(&self) -> &Vec<u8> {
        self.deref().data()
    }

    fn stack_height(&self) -> Option<u32> {
        self.deref().stack_height()
    }
}

impl<'a> Instruction for &Box<dyn Instruction + 'a> {
    fn program_id_index(&self) -> u32 {
        (*self).deref().program_id_index()
    }

    fn accounts(&self) -> &Vec<u8> {
        (*self).deref().accounts()
    }

    fn data(&self) -> &Vec<u8> {
        (*self).deref().data()
    }

    fn stack_height(&self) -> Option<u32> {
        (*self).deref().stack_height()
    }
}

impl Instruction for CompiledInstruction {
    fn program_id_index(&self) -> u32 {
        self.program_id_index
    }

    fn accounts(&self) -> &Vec<u8> {
        &self.accounts
    }

    fn data(&self) -> &Vec<u8> {
        &self.data
    }

    fn stack_height(&self) -> Option<u32> {
        Some(0)
    }
}

impl Instruction for &CompiledInstruction {
    fn program_id_index(&self) -> u32 {
        self.program_id_index
    }

    fn accounts(&self) -> &Vec<u8> {
        &self.accounts
    }

    fn data(&self) -> &Vec<u8> {
        &self.data
    }

    fn stack_height(&self) -> Option<u32> {
        Some(0)
    }
}

impl Instruction for InnerInstruction {
    fn program_id_index(&self) -> u32 {
        self.program_id_index
    }

    fn accounts(&self) -> &Vec<u8> {
        &self.accounts
    }

    fn data(&self) -> &Vec<u8> {
        &self.data
    }

    fn stack_height(&self) -> Option<u32> {
        self.stack_height
    }
}

impl Instruction for &InnerInstruction {
    fn program_id_index(&self) -> u32 {
        self.program_id_index
    }

    fn accounts(&self) -> &Vec<u8> {
        &self.accounts
    }

    fn data(&self) -> &Vec<u8> {
        &self.data
    }

    fn stack_height(&self) -> Option<u32> {
        self.stack_height
    }
}

impl ConfirmedTransaction {
    /// Returns the transaction id as a base58 string. Use [Self::hash] method to get the
    /// transaction's hash as a byte array if it's what you are after
    ///
    /// This is a simpler helper over `self.transaction.as_ref().unwrap().id()`.
    pub fn id(&self) -> String {
        self.transaction.as_ref().unwrap().id()
    }

    /// Returns the transaction's hash as a byte array. Use [Self::id] method to get the
    /// transaction's id as a base58 string if it's what you are after.
    ///
    /// This is a simpler helper over `self.transaction.as_ref().unwrap().hash()`.
    pub fn hash(&self) -> &[u8] {
        self.transaction.as_ref().unwrap().hash()
    }

    /// Returns the resolved accounts for the transaction. The resolved accounts are the
    /// accounts that are used in the transaction message and the accounts that are loaded
    /// by the transaction's meta.
    ///
    /// This returns each account as a reference to a byte array. If you need to convert them to
    /// a string, you can use:
    ///
    /// ```no_run
    /// # use substreams_solana_core::base58;
    /// # let trx = substreams_solana_core::pb::sf::solana::r#type::v1::ConfirmedTransaction::default();
    /// let accounts: Vec<_> = trx.resolved_accounts().iter().map(base58::encode).collect();
    /// ```
    pub fn resolved_accounts(&self) -> Vec<&Vec<u8>> {
        let meta = self.meta.as_ref().unwrap();
        let message = self.transaction.as_ref().unwrap().message.as_ref().unwrap();

        let mut accounts = vec![];
        accounts.extend(message.account_keys.iter());
        accounts.extend(meta.loaded_writable_addresses.iter());
        accounts.extend(meta.loaded_readonly_addresses.iter());

        accounts
    }

    /// Returns the account at the given index. The index is the index of the account in the
    /// transaction message's account keys/meta loaded writable/readonly addresses. If the
    /// index is out of bounds, the method panics.
    pub fn account_at<'a>(&'a self, index: u8) -> Address<'a> {
        let mut i: usize = index as usize;

        let account_keys = &self
            .transaction
            .as_ref()
            .unwrap()
            .message
            .as_ref()
            .unwrap()
            .account_keys;

        if i < account_keys.len() {
            return Address(&account_keys[i]);
        }

        let meta = self.meta.as_ref().unwrap();

        i = i - account_keys.len();
        if i < meta.loaded_writable_addresses.len() {
            return Address(&meta.loaded_writable_addresses[i]);
        }

        i = i - meta.loaded_writable_addresses.len();
        if i < meta.loaded_readonly_addresses.len() {
            return Address(&meta.loaded_readonly_addresses[i]);
        }

        panic!("Account index {} out of bounds", index);
    }
}

impl Transaction {
    /// Returns the transaction id as a base58 string. Use [Self::hash] method to get the
    /// transaction's hash as a byte array if it's what you are after
    pub fn id(&self) -> String {
        bs58::encode(self.hash()).into_string()
    }

    /// Returns the transaction's hash as a byte array. Use [Self::id] method to get the
    /// transaction's id as a base58 string if it's what you are after.
    pub fn hash(&self) -> &[u8] {
        &self.signatures[0]
    }
}

#[cfg(test)]
mod tests {
    use crate::pb::sf::solana::r#type::v1 as pb;
    use pretty_assertions::assert_eq;

    #[test]
    fn it_resolves_account_correctly() {
        let trx = pb::ConfirmedTransaction {
            transaction: Some(pb::Transaction {
                signatures: vec![vec![1, 2, 3]],
                message: Some(pb::Message {
                    account_keys: vec![bytes("a0"), bytes("a1"), bytes("a2")],
                    ..Default::default()
                }),
            }),
            meta: Some(pb::TransactionStatusMeta {
                loaded_writable_addresses: vec![bytes("a3"), bytes("a4")],
                loaded_readonly_addresses: vec![bytes("a5"), bytes("a6")],
                ..Default::default()
            }),
        };

        assert_eq!(
            vec![
                bytes("a0"),
                bytes("a1"),
                bytes("a2"),
                bytes("a3"),
                bytes("a4"),
                bytes("a5"),
                bytes("a6")
            ],
            trx.resolved_accounts()
                .into_iter()
                .cloned()
                .collect::<Vec<_>>()
        );

        assert_eq!(bytes("a0"), trx.account_at(0));
        assert_eq!(bytes("a1"), trx.account_at(1));
        assert_eq!(bytes("a2"), trx.account_at(2));
        assert_eq!(bytes("a3"), trx.account_at(3));
        assert_eq!(bytes("a4"), trx.account_at(4));
        assert_eq!(bytes("a5"), trx.account_at(5));
        assert_eq!(bytes("a6"), trx.account_at(6));
    }

    fn bytes(s: &str) -> Vec<u8> {
        ::hex::decode(s).unwrap()
    }
}
