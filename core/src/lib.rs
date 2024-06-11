use std::{collections::HashMap, ops::Deref};

use pb::sf::solana::r#type::v1::{Block, CompiledInstruction, InnerInstruction};

use crate::pb::sf::solana::r#type::v1::ConfirmedTransaction;

pub mod pb;

/// Helpers to deal with block sources.
pub mod block_view;

// Instruction trait to be implemented by all instructions. The trait enables you to wokr on
// a generic instruction type instead of working with either [CompiledInstruction] or [InnerInstruction]
// model.
pub trait Instruction {
    fn program_id_index(&self) -> u32;
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
    pub fn resolved_accounts(&self) -> Vec<&Vec<u8>> {
        let meta = self.meta.as_ref().unwrap();
        let mut accounts = vec![];

        self.transaction
            .as_ref()
            .unwrap()
            .message
            .as_ref()
            .unwrap()
            .account_keys
            .iter()
            .for_each(|addr| {
                accounts.push(addr);
            });
        meta.loaded_writable_addresses.iter().for_each(|addr| {
            accounts.push(addr);
        });
        meta.loaded_readonly_addresses.iter().for_each(|addr| {
            accounts.push(addr);
        });

        return accounts;
    }

    pub fn resolved_accounts_as_strings(&self) -> Vec<String> {
        self.resolved_accounts()
            .iter()
            .map(|addr| bs58::encode(addr).into_string())
            .collect()
    }
}
