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

/// A view over an instruction when iterating over a transaction.
pub struct InstructionView2<'a> {
    /// The actual iterated instruction.
    pub instruction: Box<dyn Instruction + 'a>,
    /// The transaction that holds this instruction, for easy access to the message
    /// and other related transaction data.
    pub trx: &'a pb::ConfirmedTransaction,
    // FIXME: I tried to make this a &Vec<&'a Vec<u8>>, the thing is that the
    // & on &Vec<...> and the inner one &'a Vec<u8> are different lifetimes.
    // The former reference is bound to the Iterator, while the latter is bound
    // to the ConfirmedTransaction that holds this instruction.
    //
    // The thing is that this fails because the iterator has a shorter lifetime.
    // If we could attach a resolved accounts field, e.g. an owned `Vec<Vec<u8>>`,
    // to ConfirmTransaction, the compiler would be happy since we could
    // specify `&'a Vec<&'a Vec<u8>>` as both would be owned by the same struct.
    /// The resolved accounts for this specific instruction, it's the same as
    /// `instruction.accounts()` but each element is the resolved address and not
    /// the index of the account in the transaction message.
    pub resolved_accounts: Vec<&'a Vec<u8>>,
    /// The top level instruction that holds this instruction, could be the same
    /// as the instruction itself if it's a top level instruction.
    pub top_instruction: &'a pb::CompiledInstruction,
    // The inner instructions of the top level instruction that holds this instruction.
    pub top_inner_instructions: &'a Vec<pb::InnerInstruction>,
}

impl<'a> InstructionView2<'a> {
    pub fn program_id_index(&self) -> u32 {
        self.instruction.program_id_index()
    }

    pub fn accounts(&self) -> &Vec<&Vec<u8>> {
        &self.resolved_accounts
    }

    pub fn data(&self) -> &Vec<u8> {
        self.instruction.data()
    }

    pub fn stack_height(&self) -> Option<u32> {
        self.instruction.stack_height()
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

    /// Iterates over all instructions, including inner instructions, of the transaction. The iteration
    /// starts with the first top level instruction and then goes through all inner instructions, if any,
    /// of this top level instruction. Then it moves to the next top level instruction and so on recursively.
    pub fn all_instructions<'a>(&'a self) -> impl Iterator<Item = InstructionView2<'a>> + 'a {
        let trx = self.transaction.as_ref().unwrap();

        let mut inner_instructions_by_parent = HashMap::new();
        if let Some(meta) = self.meta.as_ref() {
            for inner_instructions in meta.inner_instructions.iter() {
                inner_instructions_by_parent.insert(inner_instructions.index, inner_instructions);
            }
        }

        AllInstructionIterator {
            confirmed_transaction: self,
            resolved_accounts: self.resolved_accounts(),
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

static EMPTY_INNER_INSTRUCTIONS: Vec<pb::InnerInstruction> = Vec::new();

struct AllInstructionIterator<'a> {
    confirmed_transaction: &'a pb::ConfirmedTransaction,
    message: &'a pb::Message,
    inner_instructions_by_parent: HashMap<u32, &'a pb::InnerInstructions>,
    top_level_instruction_index: usize,
    inner_instruction_index: Option<usize>,
    resolved_accounts: Vec<&'a Vec<u8>>,
}

impl<'a> Iterator for AllInstructionIterator<'a> {
    type Item = InstructionView2<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.top_level_instruction_index >= self.message.instructions.len() {
            return None;
        }

        let top_level_instruction = &self.message.instructions[self.top_level_instruction_index];
        match self.inner_instruction_index {
            None => {
                let inner_instructions = self
                    .inner_instructions_by_parent
                    .get(&(self.top_level_instruction_index as u32));

                self.inner_instruction_index = Some(0);
                return Some(InstructionView2 {
                    instruction: Box::new(top_level_instruction),
                    trx: self.confirmed_transaction,
                    resolved_accounts: top_level_instruction
                        .accounts
                        .iter()
                        .map(|index| self.resolved_accounts[*index as usize])
                        .collect(),
                    top_instruction: top_level_instruction,
                    top_inner_instructions: inner_instructions
                        .map(|i| &i.instructions)
                        .unwrap_or(&EMPTY_INNER_INSTRUCTIONS),
                });
            }
            Some(inner_instruction_index) => {
                let inner_instructions = self
                    .inner_instructions_by_parent
                    .get(&(self.top_level_instruction_index as u32));

                match inner_instructions {
                    None => {
                        self.inner_instruction_index = None;
                        self.top_level_instruction_index += 1;
                        return self.next();
                    }
                    Some(inner_instructions) => {
                        let inner_instruction_count = inner_instructions.instructions.len();
                        if inner_instruction_index >= inner_instruction_count {
                            self.inner_instruction_index = None;
                            self.top_level_instruction_index += 1;
                            return self.next();
                        }

                        let inner_instruction =
                            &inner_instructions.instructions[inner_instruction_index];
                        self.inner_instruction_index = Some(inner_instruction_index + 1);
                        return Some(InstructionView2 {
                            instruction: Box::new(inner_instruction),
                            resolved_accounts: inner_instruction
                                .accounts
                                .iter()
                                .map(|index| self.resolved_accounts[*index as usize])
                                .collect(),
                            trx: self.confirmed_transaction,
                            top_instruction: top_level_instruction,
                            top_inner_instructions: &inner_instructions.instructions,
                        });
                    }
                }
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
    use std::vec;

    use crate::{block_view::InstructionView2, pb::sf::solana::r#type::v1 as pb};
    use paste::paste;
    use pretty_assertions::assert_eq;

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

    macro_rules! all_instructions_test_case {
        ( $name:ident, $trx:expr, $expected:expr ) => {
            paste! {
                #[test]
                fn [<it_all_instructions_ $name>]() {
                    let trx = $trx;
                    let instructions = trx
                        .all_instructions()
                        .map(Into::<ComparableInstructionView>::into)
                        .collect::<Vec<_>>();
                    assert_eq!($expected, instructions);
                }
            }
        };
    }

    all_instructions_test_case!(
        empty_trx,
        pb::ConfirmedTransaction {
            transaction: Some(pb::Transaction {
                signatures: vec![vec![1, 2, 3]],
                message: Some(pb::Message {
                    account_keys: vec![hex("00"), hex("01"), hex("02")],
                    ..Default::default()
                }),
            }),
            meta: Some(pb::TransactionStatusMeta {
                err: Some(pb::TransactionError {
                    ..Default::default()
                }),
                ..Default::default()
            }),
        },
        Vec::<ComparableInstructionView>::new()
    );

    all_instructions_test_case!(
        single_top_level_instruction,
        pb::ConfirmedTransaction {
            transaction: Some(pb::Transaction {
                signatures: vec![vec![1, 2, 3]],
                message: Some(pb::Message {
                    account_keys: vec![hex("a0"), hex("a1"), hex("a2")],
                    instructions: vec![pb::CompiledInstruction {
                        program_id_index: 1,
                        accounts: vec![0, 1],
                        data: vec![1, 2, 3],
                    }],
                    ..Default::default()
                }),
            }),
            meta: Some(pb::TransactionStatusMeta {
                ..Default::default()
            }),
        },
        vec![ComparableInstructionView {
            program_id_index: 1,
            accounts: vec![str("a0"), str("a1")],
            data: str("010203"),
            stack_height: Some(0),
            top_instruction_id: 1,
        }]
    );

    all_instructions_test_case!(
        multiple_top_level_instruction,
        pb::ConfirmedTransaction {
            transaction: Some(pb::Transaction {
                signatures: vec![vec![1, 2, 3]],
                message: Some(pb::Message {
                    account_keys: vec![hex("a0"), hex("a1"), hex("a2")],
                    instructions: vec![
                        pb::CompiledInstruction {
                            program_id_index: 1,
                            accounts: vec![0, 1],
                            data: vec![1, 2, 3],
                        },
                        pb::CompiledInstruction {
                            program_id_index: 2,
                            accounts: vec![1, 2],
                            data: vec![6, 7, 8],
                        }
                    ],
                    ..Default::default()
                }),
            }),
            meta: Some(pb::TransactionStatusMeta {
                ..Default::default()
            }),
        },
        vec![
            ComparableInstructionView {
                program_id_index: 1,
                accounts: vec![str("a0"), str("a1")],
                data: str("010203"),
                stack_height: Some(0),
                top_instruction_id: 1,
            },
            ComparableInstructionView {
                program_id_index: 2,
                accounts: vec![str("a1"), str("a2")],
                data: str("060708"),
                stack_height: Some(0),
                top_instruction_id: 2,
            }
        ]
    );

    all_instructions_test_case!(
        full_deep_nested_instructions,
        pb::ConfirmedTransaction {
            transaction: Some(pb::Transaction {
                signatures: vec![vec![1, 2, 3]],
                message: Some(pb::Message {
                    account_keys: vec![hex("a0"), hex("a1"), hex("a2")],
                    instructions: vec![
                        pb::CompiledInstruction {
                            program_id_index: 1,
                            accounts: vec![0, 1],
                            data: vec![1, 2, 3],
                        },
                        pb::CompiledInstruction {
                            program_id_index: 2,
                            accounts: vec![1, 2],
                            data: vec![6, 7, 8],
                        },
                        pb::CompiledInstruction {
                            program_id_index: 3,
                            accounts: vec![2],
                            data: vec![9, 10, 11],
                        }
                    ],
                    ..Default::default()
                }),
            }),
            meta: Some(pb::TransactionStatusMeta {
                inner_instructions: vec![
                    pb::InnerInstructions {
                        index: 0,
                        instructions: vec![pb::InnerInstruction {
                            program_id_index: 4,
                            accounts: vec![0, 1],
                            data: vec![4, 5, 6],
                            stack_height: Some(1),
                        },],
                    },
                    pb::InnerInstructions {
                        index: 2,
                        instructions: vec![
                            pb::InnerInstruction {
                                program_id_index: 6,
                                accounts: vec![0, 1],
                                data: vec![10, 11, 12],
                                stack_height: Some(1),
                            },
                            pb::InnerInstruction {
                                program_id_index: 7,
                                accounts: vec![1, 2],
                                data: vec![13, 14, 15],
                                stack_height: Some(2),
                            }
                        ],
                    }
                ],
                ..Default::default()
            }),
        },
        vec![
            ComparableInstructionView {
                program_id_index: 1,
                accounts: vec![str("a0"), str("a1")],
                data: str("010203"),
                stack_height: Some(0),
                top_instruction_id: 1,
            },
            ComparableInstructionView {
                program_id_index: 4,
                accounts: vec![str("a0"), str("a1")],
                data: str("040506"),
                stack_height: Some(1),
                top_instruction_id: 1,
            },
            ComparableInstructionView {
                program_id_index: 2,
                accounts: vec![str("a1"), str("a2")],
                data: str("060708"),
                stack_height: Some(0),
                top_instruction_id: 2,
            },
            ComparableInstructionView {
                program_id_index: 3,
                accounts: vec![str("a2")],
                data: str("090a0b"),
                stack_height: Some(0),
                top_instruction_id: 3,
            },
            ComparableInstructionView {
                program_id_index: 6,
                accounts: vec![str("a0"), str("a1")],
                data: str("0a0b0c"),
                stack_height: Some(1),
                top_instruction_id: 3,
            },
            ComparableInstructionView {
                program_id_index: 7,
                accounts: vec![str("a1"), str("a2")],
                data: str("0d0e0f"),
                stack_height: Some(2),
                top_instruction_id: 3,
            },
        ]
    );

    #[derive(Debug, PartialEq)]
    struct ComparableInstructionView {
        program_id_index: u32,
        accounts: Vec<String>,
        data: String,
        stack_height: Option<u32>,
        // Program ID index of the top level instruction
        top_instruction_id: u32,
    }

    impl From<InstructionView2<'_>> for ComparableInstructionView {
        fn from(view: InstructionView2) -> Self {
            ComparableInstructionView {
                program_id_index: view.program_id_index(),
                accounts: view
                    .resolved_accounts
                    .iter()
                    .map(|b| hex::encode(b))
                    .collect(),
                data: hex::encode(view.data()),
                stack_height: view.stack_height(),
                top_instruction_id: view.top_instruction.program_id_index,
            }
        }
    }

    fn str(s: &str) -> String {
        s.to_string()
    }

    fn hex(s: &str) -> Vec<u8> {
        ::hex::decode(s).unwrap()
    }
}
