use std::collections::HashMap;

use crate::{pb::sf::solana::r#type::v1 as pb, Instruction};

impl pb::Block {
    /// Iterates over successful transactions in given block.
    pub fn transactions(&self) -> impl Iterator<Item = &pb::ConfirmedTransaction> {
        self.transactions.iter().filter(|trx| -> bool {
            if let Some(meta) = &trx.meta {
                return meta.err.is_none();
            }
            false
        })
    }

    /// Iterates over successful transactions in given block and take ownership.
    pub fn transactions_owned(self) -> impl Iterator<Item = pb::ConfirmedTransaction> {
        self.transactions.into_iter().filter(|trx| -> bool {
            if let Some(meta) = &trx.meta {
                return meta.err.is_none();
            }
            false
        })
    }

    /// Iterates over instructions of successful transactions in given block.
    pub fn instructions(&self) -> impl Iterator<Item = InstructionView> {
        self.transactions().map(|trx| trx.instructions()).flatten()
    }
}

impl pb::ConfirmedTransaction {
    /// Iterates over instructions for the given transaction.
    pub fn instructions<'a>(&'a self) -> impl Iterator<Item = InstructionView> {
        self.transaction.iter().flat_map(|trx| {
            trx.message.iter().flat_map(|m| {
                m.instructions.iter().map(|inst| InstructionView::<'a> {
                    message: m,
                    transaction: trx,
                    instruction: inst,
                })
            })
        })
    }

    pub fn all_instructions<'a>(&'a self) -> impl Iterator<Item = impl Instruction + 'a> + 'a {
        let trx = self.transaction.as_ref().unwrap();

        let mut inner_instructions_by_parent = HashMap::new();
        if let Some(meta) = self.meta.as_ref() {
            for inner_instructions in meta.inner_instructions.iter() {
                inner_instructions_by_parent.insert(inner_instructions.index, inner_instructions);
            }
        }

        AllInstructionIterator {
            transaction: self.transaction.as_ref().unwrap(),
            message: trx.message.as_ref().unwrap(),
            inner_instructions_by_parent,
            top_level_instruction_index: 0,
            inner_instruction_index: None,
        }
    }

    pub fn meta(&self) -> Option<&pb::ConfirmedTransaction> {
        if self.meta.is_none() || self.meta.as_ref().unwrap().meta().is_none() {
            return None;
        }

        return Some(self);
    }
}

struct AllInstructionIterator<'a> {
    transaction: &'a pb::Transaction,
    message: &'a pb::Message,
    inner_instructions_by_parent: HashMap<u32, &'a pb::InnerInstructions>,
    top_level_instruction_index: usize,
    inner_instruction_index: Option<usize>,
}

impl<'a> Iterator for AllInstructionIterator<'a> {
    type Item = Box<dyn Instruction + 'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.top_level_instruction_index
            >= self
                .transaction
                .message
                .as_ref()
                .unwrap()
                .instructions
                .len()
        {
            return None;
        }

        let top_level_instruction = &self.message.instructions[self.top_level_instruction_index];
        match self.inner_instruction_index {
            None => {
                self.inner_instruction_index = Some(0);
                return Some(Box::new(top_level_instruction));
            }
            Some(inner_instruction_index) => {
                let inner_instructions = self
                    .inner_instructions_by_parent
                    .get(&(self.top_level_instruction_index as u32));
                if let Some(inner_instructions) = inner_instructions {
                    if inner_instruction_index >= inner_instructions.instructions.len() {
                        self.inner_instruction_index = None;
                        self.top_level_instruction_index += 1;
                        return self.next();
                    }

                    let inner_instruction =
                        &inner_instructions.instructions[inner_instruction_index];
                    self.inner_instruction_index = Some(inner_instruction_index + 1);
                    return Some(Box::new(inner_instruction));
                }

                self.inner_instruction_index = None;
                self.top_level_instruction_index += 1;
                return Some(Box::new(top_level_instruction));
            }
        }
    }
}

impl pb::TransactionStatusMeta {
    pub fn meta(&self) -> Option<&pb::TransactionStatusMeta> {
        if self.err.is_some() || self.inner_instructions_none {
            return None;
        }
        return Some(self);
    }
}

#[derive(Copy, Clone)]
pub struct InstructionView<'a> {
    pub message: &'a pb::Message,
    pub transaction: &'a pb::Transaction,
    pub instruction: &'a pb::CompiledInstruction,
}

impl AsRef<pb::CompiledInstruction> for InstructionView<'_> {
    fn as_ref(&self) -> &pb::CompiledInstruction {
        self.instruction
    }
}

impl InstructionView<'_> {
    pub fn account_at(&self, account_index: usize) -> &Vec<u8> {
        &self.message.account_keys[self.instruction.accounts[account_index] as usize]
    }

    pub fn get_program_id(&self) -> &Vec<u8> {
        self.account_at(self.instruction.program_id_index as usize)
    }

    pub fn is_from_program_id(&self, program_addr: impl AsRef<[u8]>) -> bool {
        self.get_program_id().as_slice() == program_addr.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use crate::pb::sf::solana::r#type::v1 as pb;

    #[test]
    fn it_iterates_over_successful_transaction() {
        let block = pb::Block {
            transactions: vec![
                pb::ConfirmedTransaction {
                    transaction: Some(pb::Transaction {
                        signatures: vec![vec![1, 2, 3]],
                        message: None,
                    }),
                    meta: Some(pb::TransactionStatusMeta {
                        err: Some(pb::TransactionError {
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                },
                pb::ConfirmedTransaction {
                    transaction: Some(pb::Transaction {
                        signatures: vec![vec![4, 5, 6]],
                        message: None,
                    }),
                    meta: Some(pb::TransactionStatusMeta {
                        err: None,
                        ..Default::default()
                    }),
                },
                pb::ConfirmedTransaction {
                    transaction: Some(pb::Transaction {
                        signatures: vec![vec![7, 8, 9]],
                        message: None,
                    }),
                    meta: None,
                },
            ],
            ..Default::default()
        };

        let mut iter = block.transactions();
        assert_eq!(
            Some(&pb::ConfirmedTransaction {
                transaction: Some(pb::Transaction {
                    signatures: vec![vec![4, 5, 6]],
                    message: None
                }),
                meta: Some(pb::TransactionStatusMeta {
                    err: None,
                    ..Default::default()
                })
            }),
            iter.next()
        );

        // ... and then None once it's over.
        assert_eq!(None, iter.next());
    }
}
