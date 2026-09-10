use crate::{address::Address, pb::sf::solana::r#type::v1 as pb, Instruction};
use std::collections::HashMap;

impl pb::Block {
    /// Iterates over successful transactions in given block.
    pub fn transactions(&self) -> impl Iterator<Item = &pb::ConfirmedTransaction> {
        self.transactions
            .iter()
            .filter(|trx: &&pb::ConfirmedTransaction| -> bool { trx.is_successful() })
    }

    /// Iterates over successful transactions in given block and take ownership.
    pub fn transactions_owned(self) -> impl Iterator<Item = pb::ConfirmedTransaction> {
        self.transactions
            .into_iter()
            .filter(pb::ConfirmedTransaction::is_successful)
    }

    /// Iterates over compiled instructions of the block. Refer to [pb::ConfirmedTransaction::compiled_instructions]
    /// for details about the iteration.
    pub fn compiled_instructions<'a>(&'a self) -> impl Iterator<Item = InstructionView<'a>> + 'a {
        self.transactions()
            .map(|trx| trx.compiled_instructions())
            .flatten()
    }

    /// Iterates over all instructions, including inner instructions, of the block. Refer to
    /// [pb::ConfirmedTransaction::walk_instructions] for details about the iteration.
    pub fn walk_instructions(&self) -> impl Iterator<Item = InstructionView<'_>> {
        self.transactions()
            .map(|trx| trx.walk_instructions())
            .flatten()
    }
}

/// A view over an instruction when iterating over a transaction.
pub struct InstructionView<'a> {
    instruction: Box<dyn Instruction + 'a>,
    trx: &'a pb::ConfirmedTransaction,
    compiled_instruction: &'a pb::CompiledInstruction,

    // Available only if the current instruction's view is a compiled instruction.
    // Used to iterate over inner instructions of the compiled instruction, if
    // desired.
    compiled_index: Option<usize>,
}

static EMPTY_INNER_INSTRUCTIONS: Vec<pb::InnerInstruction> = Vec::new();

impl<'a> InstructionView<'a> {
    /// Returns the resolved program id defined by this instruction as an [Address]
    /// type which is a wrapper around a byte array and provide [Address::to_string]
    /// method to get the base58 encoded string:
    ///
    /// ```no_run
    /// # let instruction_view: substreams_solana_core::block_view::InstructionView = unimplemented!();
    /// let program_id = instruction_view.program_id().to_string();
    /// ```
    pub fn program_id(&self) -> Address<'a> {
        // &self.resolved_program_id
        self.trx
            .account_at(self.instruction.program_id_index() as u8)
    }

    /// Returns the resolved accounts defined by this instruction. You can
    /// easily get the base58 encoded string of the addresses:
    ///
    /// ```no_run
    /// # use substreams_solana_core::address::Address;
    /// # let instruction_view: substreams_solana_core::block_view::InstructionView = unimplemented!();
    /// let accounts = instruction_view.accounts().iter().map(Address::to_string).collect::<Vec<_>>();
    /// ```
    pub fn accounts(&self) -> Vec<Address<'a>> {
        // &self.resolved_accounts
        self.instruction
            .accounts()
            .iter()
            .map(|index| self.trx.account_at(*index))
            .collect()
    }

    pub fn data(&self) -> &Vec<u8> {
        self.instruction.data()
    }

    /// Returns the stack height of the instruction or zero if instruction does not have
    /// a stack height field which appeared in Solana v1.14.6 and activated around
    /// block 200M on Mainnet. So if you deal with block heights before that, you can
    /// think of using `maybe_stack_height` method to determine if the field is present
    /// or not
    pub fn stack_height(&self) -> u32 {
        self.maybe_stack_height().unwrap_or(0)
    }

    /// Returns the stack height of the instruction if it's present or None if the
    /// See [Self::stack_height] for details when the stack height field was introduced.
    pub fn maybe_stack_height(&self) -> Option<u32> {
        self.instruction.stack_height()
    }

    /// The inner instruction at index `at` of the compiled instruction that holds this instruction.
    /// It's the direct children of [Self::compiled_instruction]. This method will return
    /// [None] if the current instruction is not a compiled instruction, e.g. [Self::is_root()] == false.
    /// or if the inner instruction at the given index does not exist.
    ///
    /// If you are **not** in a compiled instruction and would still like to get a specific
    /// inner instruction, open an issue and we will consider adding a method to do that.
    pub fn inner_instruction(&'a self, at: usize) -> Option<InstructionView<'a>> {
        match self.compiled_index {
            None => None,
            Some(index) => self
                .meta()
                .inner_instructions
                .iter()
                .find(|i: &&pb::InnerInstructions| i.index == index as u32)
                .and_then(|i| i.instructions.get(at))
                .map(|instruction| InstructionView {
                    instruction: Box::new(instruction),
                    trx: self.trx,
                    compiled_instruction: self.compiled_instruction,
                    compiled_index: None,
                }),
        }
    }

    /// The inner instructions of the compiled instruction that holds this instruction.
    /// It's the direct children of [Self::compiled_instruction]. This method will return
    /// an empty iterator if the current instruction is not a compiled instruction, e.g.
    /// [Self::is_root()] == false.
    ///
    /// If you are **not** in a compiled instruction and would still like to iterate over
    /// inner instructions, open an issue and we will consider adding a method to do that.
    pub fn inner_instructions(&'a self) -> impl Iterator<Item = InstructionView<'a>> + 'a {
        let inner = match self.compiled_index {
            None => &EMPTY_INNER_INSTRUCTIONS,
            Some(index) => self
                .meta()
                .inner_instructions
                .iter()
                .find(|i| i.index == index as u32)
                .map(|i| &i.instructions)
                .unwrap_or_else(|| &EMPTY_INNER_INSTRUCTIONS),
        };

        inner.iter().map(move |inner_instruction| InstructionView {
            instruction: Box::new(inner_instruction),
            trx: self.trx,
            compiled_instruction: self.compiled_instruction,
            compiled_index: None,
        })
    }
    /// Returns true if the instruction your are iterating over is a compiled instruction,
    /// e.g. a root instruction of a transaction or false if the view represents an
    /// inner instruction.
    pub fn is_root(&self) -> bool {
        self.compiled_index.is_some()
    }

    /// The compiled instruction within which this instruction was originally found.
    /// Could be the same as the current [InstructionView] instance that you are
    /// currently viewing if [Self::is_root] is `true`.
    ///
    /// If you are iterating over inner instructions, this method will return the
    /// compiled instruction that holds the inner instructions.
    pub fn compiled_instruction(&self) -> InstructionView<'a> {
        InstructionView {
            instruction: Box::new(self.compiled_instruction),
            trx: self.trx,
            compiled_instruction: self.compiled_instruction,
            compiled_index: self.compiled_index,
        }
    }

    /// The transactions's message that holds this instruction.
    pub fn message(&self) -> &'a pb::Message {
        &self.transaction().message
    }

    /// The transactions's meta that holds this instruction
    pub fn meta(&self) -> &'a pb::TransactionStatusMeta {
        &self.trx.meta
    }

    /// The transaction that holds this instruction, for easy access to the message
    /// and other related transaction data.
    pub fn transaction(&self) -> &'a pb::Transaction {
        &self.trx.transaction
    }

    /// The confirmed transaction that holds this instruction, for easy access to the message
    /// and other related transaction data.
    pub fn confirmed_transaction(&self) -> &'a pb::ConfirmedTransaction {
        self.trx
    }
}

impl pb::ConfirmedTransaction {
    /// Iterates over compiled instructions for the given transaction. You receive an [InstructionView]
    /// for each compiled level instruction in the transaction. The [InstructionView] provides convenient
    /// access to the resolved program id and accounts instead of the raw account indices.
    ///
    /// If you then need to iterate over inner instructions of a compiled instruction, you can use:
    ///
    /// ```no_run
    /// # use substreams_solana_core::block_view::InstructionView;
    /// # let trx = substreams_solana_core::pb::sf::solana::r#type::v1::ConfirmedTransaction::default();
    /// for view in trx.compiled_instructions() {
    ///    for inner_instruction_view in view.inner_instructions() {
    ///       // Do something with the inner instruction
    ///   }
    /// }
    /// ```
    pub fn compiled_instructions<'a>(&'a self) -> impl Iterator<Item = InstructionView<'a>> + 'a {
        let mut inner_instructions_by_parent = HashMap::new();
        if let Some(meta) = self.meta.as_option() {
            for inner_instructions in meta.inner_instructions.iter() {
                inner_instructions_by_parent.insert(inner_instructions.index, inner_instructions);
            }
        }

        self.transaction
            .as_option()
            .into_iter()
            .flat_map(|trx| {
                trx.message
                    .as_option()
                    .into_iter()
                    .flat_map(|m| m.instructions.iter().enumerate())
            })
            .map(move |(i, inst)| InstructionView {
                instruction: Box::new(inst),
                trx: self,
                compiled_instruction: inst,
                compiled_index: Some(i),
            })
    }

    /// Iterates over all instructions, including inner instructions, of the transaction. The iteration
    /// starts with the first compiled instruction and then goes through all its inner instructions, if any.
    /// Then it moves to the next compiled instruction and so on recursively.
    ///
    /// You receive a [InstructionView] for each instruction visited in the transaction. The [InstructionView]
    /// provides convenient access to the resolved [InstructionView::program_id] and [InstructionView::accounts]
    /// instead of the raw program id index & account indices.
    pub fn walk_instructions<'a>(&'a self) -> impl Iterator<Item = InstructionView<'a>> + 'a {
        let trx = &self.transaction;

        let mut inner_instructions_by_parent = HashMap::new();
        if let Some(meta) = self.meta.as_option() {
            for inner_instructions in meta.inner_instructions.iter() {
                inner_instructions_by_parent.insert(inner_instructions.index, inner_instructions);
            }
        }

        AllInstructionIterator {
            confirmed_transaction: self,
            message: &trx.message,
            inner_instructions_by_parent,
            top_level_instruction_index: 0,
            inner_instruction_index: None,
        }
    }

    /// Returns true if this [ConfirmedTransaction] was successful, e.g. its meta.err is None
    pub fn is_successful(&self) -> bool {
        self.meta
            .as_option()
            .map(|m| m.err.is_unset())
            .unwrap_or(false)
    }

    pub fn meta(&self) -> Option<&pb::ConfirmedTransaction> {
        if self.meta.is_unset() || self.meta.meta().is_none() {
            return None;
        }

        return Some(self);
    }
}

struct AllInstructionIterator<'a> {
    confirmed_transaction: &'a pb::ConfirmedTransaction,
    message: &'a pb::Message,
    inner_instructions_by_parent: HashMap<u32, &'a pb::InnerInstructions>,
    top_level_instruction_index: usize,
    inner_instruction_index: Option<usize>,
}

impl<'a> Iterator for AllInstructionIterator<'a> {
    type Item = InstructionView<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.top_level_instruction_index >= self.message.instructions.len() {
            return None;
        }

        let top_level_instruction = &self.message.instructions[self.top_level_instruction_index];
        match self.inner_instruction_index {
            None => {
                self.inner_instruction_index = Some(0);
                return Some(InstructionView {
                    instruction: Box::new(top_level_instruction),
                    trx: self.confirmed_transaction,
                    compiled_instruction: top_level_instruction,
                    compiled_index: Some(self.top_level_instruction_index),
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
                        return Some(InstructionView {
                            instruction: Box::new(inner_instruction),
                            trx: self.confirmed_transaction,
                            compiled_instruction: top_level_instruction,
                            compiled_index: None,
                        });
                    }
                }
            }
        }
    }
}

impl pb::TransactionStatusMeta {
    pub fn meta(&self) -> Option<&pb::TransactionStatusMeta> {
        if self.err.is_set() || self.inner_instructions.is_empty() {
            return None;
        }
        return Some(self);
    }
}

/// Block accessors over buffa's lazy views, mirroring the owned `pb::Block`
/// methods above.
///
/// A lazy view borrows the wire buffer and decodes deferred fields on access,
/// so every accessor here is fallible where the owned equivalent is not, and
/// the resolved-address types hold `&[u8]` rather than `&Vec<u8>`.
pub mod lazy {
    use crate::base58;
    use crate::pb::sf::solana::r#type::v1::__buffa::lazy_view::{
        BlockLazyView, ConfirmedTransactionLazyView, MessageLazyView,
    };
    use buffa::DecodeError;

    /// A resolved Solana address borrowed from the wire buffer.
    pub struct AddressRef<'a>(pub &'a [u8]);

    impl AddressRef<'_> {
        pub fn to_string(&self) -> String {
            base58::encode(&self.0)
        }
    }

    impl std::fmt::Debug for AddressRef<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&base58::encode(&self.0))
        }
    }

    impl std::fmt::Display for AddressRef<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(&base58::encode(&self.0))
        }
    }

    impl AsRef<[u8]> for AddressRef<'_> {
        fn as_ref(&self) -> &[u8] {
            self.0
        }
    }

    /// One instruction reached through a lazy transaction, carrying the account
    /// table needed to resolve its indices.
    pub struct LazyInstructionView<'a> {
        pub program_id_index: u32,
        pub accounts: &'a [u8],
        pub data: &'a [u8],
        pub stack_height: Option<u32>,
        resolved: &'a [&'a [u8]],
    }

    impl<'a> LazyInstructionView<'a> {
        /// The resolved program id, or an all-zero address if the index falls
        /// outside the transaction's account table.
        pub fn program_id(&self) -> AddressRef<'a> {
            self.account_at(self.program_id_index as u8)
        }

        /// The resolved accounts this instruction names.
        pub fn accounts(&self) -> Vec<AddressRef<'a>> {
            self.accounts
                .iter()
                .map(|index| self.account_at(*index))
                .collect()
        }

        pub fn data(&self) -> &'a [u8] {
            self.data
        }

        pub fn stack_height(&self) -> u32 {
            self.stack_height.unwrap_or(0)
        }

        pub fn maybe_stack_height(&self) -> Option<u32> {
            self.stack_height
        }

        fn account_at(&self, index: u8) -> AddressRef<'a> {
            static EMPTY: &[u8] = &[];
            AddressRef(
                self.resolved
                    .get(index as usize)
                    .copied()
                    .unwrap_or(EMPTY),
            )
        }
    }

    /// A lazy transaction with its account table resolved once.
    ///
    /// A lazy view re-decodes a deferred field on every access, so the account
    /// table and inner instructions are read up front and held here rather than
    /// re-read per instruction.
    pub struct LazyTransaction<'a> {
        resolved: Vec<&'a [u8]>,
        message: Option<MessageLazyView<'a>>,
        inner: Vec<(u32, Vec<LazyInner<'a>>)>,
    }

    struct LazyInner<'a> {
        program_id_index: u32,
        accounts: &'a [u8],
        data: &'a [u8],
        stack_height: Option<u32>,
    }

    impl<'a> LazyTransaction<'a> {
        /// Resolves a transaction's account table and inner instructions.
        pub fn new(trx: &ConfirmedTransactionLazyView<'a>) -> Result<Self, DecodeError> {
            let mut resolved: Vec<&'a [u8]> = Vec::new();
            let mut message = None;

            if let Some(transaction) = trx.transaction.get()? {
                if let Some(msg) = transaction.message.get()? {
                    for key in msg.account_keys.iter() {
                        resolved.push(key);
                    }
                    message = Some(msg);
                }
            }

            let mut inner = Vec::new();
            if let Some(meta) = trx.meta.get()? {
                for addr in meta.loaded_writable_addresses.iter() {
                    resolved.push(addr);
                }
                for addr in meta.loaded_readonly_addresses.iter() {
                    resolved.push(addr);
                }

                for group in meta.inner_instructions.iter() {
                    let group = group?;
                    let mut instructions = Vec::new();
                    for instruction in group.instructions.iter() {
                        let instruction = instruction?;
                        instructions.push(LazyInner {
                            program_id_index: instruction.program_id_index,
                            accounts: instruction.accounts,
                            data: instruction.data,
                            stack_height: instruction.stack_height,
                        });
                    }
                    inner.push((group.index, instructions));
                }
            }

            Ok(Self {
                resolved,
                message,
                inner,
            })
        }

        /// The resolved account table: message account keys, then the meta's
        /// loaded writable and readonly addresses, in that order.
        pub fn resolved_accounts(&self) -> &[&'a [u8]] {
            &self.resolved
        }

        pub fn account_at(&self, index: u8) -> AddressRef<'a> {
            static EMPTY: &[u8] = &[];
            AddressRef(self.resolved.get(index as usize).copied().unwrap_or(EMPTY))
        }

        /// Every instruction, compiled ones followed by their inner
        /// instructions, matching the owned `walk_instructions` order.
        pub fn walk_instructions(
            &'a self,
        ) -> Result<impl Iterator<Item = LazyInstructionView<'a>> + 'a, DecodeError> {
            let mut out: Vec<LazyInstructionView<'a>> = Vec::new();

            if let Some(message) = self.message.as_ref() {
                for (i, instruction) in message.instructions.iter().enumerate() {
                    let instruction = instruction?;
                    out.push(LazyInstructionView {
                        program_id_index: instruction.program_id_index,
                        accounts: instruction.accounts,
                        data: instruction.data,
                        stack_height: None,
                        resolved: &self.resolved,
                    });

                    if let Some((_, instructions)) =
                        self.inner.iter().find(|(index, _)| *index == i as u32)
                    {
                        for inner in instructions {
                            out.push(LazyInstructionView {
                                program_id_index: inner.program_id_index,
                                accounts: inner.accounts,
                                data: inner.data,
                                stack_height: inner.stack_height,
                                resolved: &self.resolved,
                            });
                        }
                    }
                }
            }

            Ok(out.into_iter())
        }

        /// The compiled (top-level) instructions only.
        pub fn compiled_instructions(
            &'a self,
        ) -> Result<impl Iterator<Item = LazyInstructionView<'a>> + 'a, DecodeError> {
            let mut out: Vec<LazyInstructionView<'a>> = Vec::new();

            if let Some(message) = self.message.as_ref() {
                for instruction in message.instructions.iter() {
                    let instruction = instruction?;
                    out.push(LazyInstructionView {
                        program_id_index: instruction.program_id_index,
                        accounts: instruction.accounts,
                        data: instruction.data,
                        stack_height: None,
                        resolved: &self.resolved,
                    });
                }
            }

            Ok(out.into_iter())
        }
    }

    impl<'a> BlockLazyView<'a> {
        /// Successful transactions, skipping any that fail to decode.
        pub fn transactions(
            &self,
        ) -> impl Iterator<Item = ConfirmedTransactionLazyView<'a>> + '_ {
            self.transactions
                .iter()
                .filter_map(|trx| trx.ok())
                .filter(|trx| is_successful(trx))
        }

        /// Successful transactions, surfacing decode errors instead of skipping.
        pub fn try_transactions(
            &self,
        ) -> impl Iterator<Item = Result<ConfirmedTransactionLazyView<'a>, DecodeError>> + '_
        {
            self.transactions
                .iter()
                .filter(|trx| trx.as_ref().map(is_successful).unwrap_or(true))
        }
    }

    fn is_successful(trx: &ConfirmedTransactionLazyView<'_>) -> bool {
        trx.meta
            .get()
            .ok()
            .flatten()
            .map(|meta| meta.err.is_unset())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;
    use std::vec;

    use crate::{block_view::InstructionView, pb::sf::solana::r#type::v1 as pb, Instruction};
    use paste::paste;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_is_successful_with_no_error() {
        let trx = pb::ConfirmedTransaction {
            transaction: pb::Transaction {
                signatures: vec![vec![1, 2, 3]],
                message: pb::Message::default().into(),
            }
            .into(),
            meta: pb::TransactionStatusMeta {
                err: buffa::MessageField::none(),
                ..Default::default()
            }
            .into(),
        };

        assert_eq!(true, trx.is_successful());
    }

    #[test]
    fn test_is_successful_with_error() {
        let trx = pb::ConfirmedTransaction {
            transaction: pb::Transaction {
                signatures: vec![vec![1, 2, 3]],
                message: pb::Message::default().into(),
            }
            .into(),
            meta: pb::TransactionStatusMeta {
                err: pb::TransactionError {
                    ..Default::default()
                }
                .into(),
                ..Default::default()
            }
            .into(),
        };

        assert_eq!(false, trx.is_successful());
    }

    #[test]
    fn test_is_successful_with_no_meta() {
        let trx = pb::ConfirmedTransaction {
            transaction: pb::Transaction {
                signatures: vec![vec![1, 2, 3]],
                message: pb::Message::default().into(),
            }
            .into(),
            meta: buffa::MessageField::none(),
        };

        assert_eq!(false, trx.is_successful());
    }

    #[test]
    fn it_iterates_over_successful_transaction() {
        let block = pb::Block {
            transactions: vec![
                pb::ConfirmedTransaction {
                    transaction: pb::Transaction {
                        signatures: vec![vec![1, 2, 3]],
                        message: buffa::MessageField::none(),
                    }
                    .into(),
                    meta: pb::TransactionStatusMeta {
                        err: pb::TransactionError {
                            ..Default::default()
                        }
                        .into(),
                        ..Default::default()
                    }
                    .into(),
                },
                pb::ConfirmedTransaction {
                    transaction: pb::Transaction {
                        signatures: vec![vec![4, 5, 6]],
                        message: buffa::MessageField::none(),
                    }
                    .into(),
                    meta: pb::TransactionStatusMeta {
                        err: buffa::MessageField::none(),
                        ..Default::default()
                    }
                    .into(),
                },
                pb::ConfirmedTransaction {
                    transaction: pb::Transaction {
                        signatures: vec![vec![7, 8, 9]],
                        message: buffa::MessageField::none(),
                    }
                    .into(),
                    meta: buffa::MessageField::none(),
                },
            ],
            ..Default::default()
        };

        let mut iter = block.transactions();
        assert_eq!(
            Some(&pb::ConfirmedTransaction {
                transaction: pb::Transaction {
                    signatures: vec![vec![4, 5, 6]],
                    message: buffa::MessageField::none()
                }
                .into(),
                meta: pb::TransactionStatusMeta {
                    err: buffa::MessageField::none(),
                    ..Default::default()
                }
                .into()
            }),
            iter.next()
        );

        // ... and then None once it's over.
        assert_eq!(None, iter.next());
    }

    macro_rules! walk_instructions_test_case {
        ( $name:ident, $trx:expr, $expected:expr ) => {
            paste! {
                #[test]
                fn [<it_instructions_ $name>]() {
                    let trx = $trx;
                    let instructions = trx
                        .walk_instructions()
                        .map(Into::<ComparableInstructionView>::into)
                        .collect::<Vec<_>>();
                    assert_eq!($expected, instructions);
                }
            }
        };
    }

    walk_instructions_test_case!(
        empty_trx,
        pb::ConfirmedTransaction {
            transaction: pb::Transaction {
                signatures: vec![vec![1, 2, 3]],
                message: pb::Message {
                    account_keys: vec![hex("00"), hex("01"), hex("02")],
                    ..Default::default()
                }
                .into(),
            }
            .into(),
            meta: pb::TransactionStatusMeta {
                err: pb::TransactionError {
                    ..Default::default()
                }
                .into(),
                ..Default::default()
            }
            .into(),
        },
        Vec::<ComparableInstructionView>::new()
    );

    walk_instructions_test_case!(
        single_top_level_instruction,
        pb::ConfirmedTransaction {
            transaction: pb::Transaction {
                signatures: vec![vec![1, 2, 3]],
                message: pb::Message {
                    account_keys: vec![hex("a0"), hex("a1"), hex("a2")],
                    instructions: vec![pb::CompiledInstruction {
                        program_id_index: 1,
                        accounts: vec![0, 1],
                        data: vec![1, 2, 3],
                    }],
                    ..Default::default()
                }
                .into(),
            }
            .into(),
            meta: pb::TransactionStatusMeta {
                ..Default::default()
            }
            .into(),
        },
        vec![ComparableInstructionView {
            program_id: str("a1"),
            accounts: vec![str("a0"), str("a1")],
            data: str("010203"),
            stack_height: 0,
            instruction_id: 1,
            compiled_instruction_id: 1,
        }]
    );

    walk_instructions_test_case!(
        multiple_top_level_instruction,
        pb::ConfirmedTransaction {
            transaction: pb::Transaction {
                signatures: vec![vec![1, 2, 3]],
                message: pb::Message {
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
                }
                .into(),
            }
            .into(),
            meta: pb::TransactionStatusMeta {
                ..Default::default()
            }
            .into(),
        },
        vec![
            ComparableInstructionView {
                program_id: str("a1"),
                accounts: vec![str("a0"), str("a1")],
                data: str("010203"),
                stack_height: 0,
                instruction_id: 1,
                compiled_instruction_id: 1,
            },
            ComparableInstructionView {
                program_id: str("a2"),
                accounts: vec![str("a1"), str("a2")],
                data: str("060708"),
                stack_height: 0,
                instruction_id: 2,
                compiled_instruction_id: 2,
            }
        ]
    );

    walk_instructions_test_case!(
        full_deep_nested_instructions,
        FULL_TRX.clone(),
        vec![
            ComparableInstructionView {
                program_id: str("a1"),
                accounts: vec![str("a0"), str("a1")],
                data: str("010203"),
                stack_height: 0,
                instruction_id: 1,
                compiled_instruction_id: 1,
            },
            ComparableInstructionView {
                program_id: str("a4"),
                accounts: vec![str("a0"), str("a1")],
                data: str("040506"),
                stack_height: 1,
                instruction_id: 4,
                compiled_instruction_id: 1,
            },
            ComparableInstructionView {
                program_id: str("a2"),
                accounts: vec![str("a1"), str("a2")],
                data: str("060708"),
                stack_height: 0,
                instruction_id: 2,
                compiled_instruction_id: 2,
            },
            ComparableInstructionView {
                program_id: str("a3"),
                accounts: vec![str("a2")],
                data: str("090a0b"),
                stack_height: 0,
                instruction_id: 3,
                compiled_instruction_id: 3,
            },
            ComparableInstructionView {
                program_id: str("a5"),
                accounts: vec![str("a0"), str("a1")],
                data: str("0a0b0c"),
                stack_height: 1,
                instruction_id: 5,
                compiled_instruction_id: 3,
            },
            ComparableInstructionView {
                program_id: str("a6"),
                accounts: vec![str("a1"), str("a2")],
                data: str("0d0e0f"),
                stack_height: 2,
                instruction_id: 6,
                compiled_instruction_id: 3,
            },
        ]
    );

    macro_rules! compiled_instructions_test_case {
        ( $name:ident, $trx:expr, $expected:expr ) => {
            paste! {
                #[test]
                fn [<it_compiled_instructions_ $name>]() {
                    let trx = $trx;
                    let instructions = trx
                        .compiled_instructions()
                        .map(Into::<ComparableInstructionView>::into)
                        .collect::<Vec<_>>();
                    assert_eq!($expected, instructions);
                }
            }
        };
    }

    compiled_instructions_test_case!(
        empty_trx,
        pb::ConfirmedTransaction {
            transaction: pb::Transaction {
                signatures: vec![vec![1, 2, 3]],
                message: pb::Message {
                    account_keys: vec![hex("00"), hex("01"), hex("02")],
                    ..Default::default()
                }
                .into(),
            }
            .into(),
            meta: pb::TransactionStatusMeta {
                err: pb::TransactionError {
                    ..Default::default()
                }
                .into(),
                ..Default::default()
            }
            .into(),
        },
        Vec::<ComparableInstructionView>::new()
    );

    compiled_instructions_test_case!(
        full_deep_nested_instructions,
        FULL_TRX.clone(),
        vec![
            ComparableInstructionView {
                program_id: str("a1"),
                accounts: vec![str("a0"), str("a1")],
                data: str("010203"),
                stack_height: 0,
                instruction_id: 1,
                compiled_instruction_id: 1,
            },
            ComparableInstructionView {
                program_id: str("a2"),
                accounts: vec![str("a1"), str("a2")],
                data: str("060708"),
                stack_height: 0,
                instruction_id: 2,
                compiled_instruction_id: 2,
            },
            ComparableInstructionView {
                program_id: str("a3"),
                accounts: vec![str("a2")],
                data: str("090a0b"),
                stack_height: 0,
                instruction_id: 3,
                compiled_instruction_id: 3,
            },
        ]
    );

    #[test]
    pub fn compiled_instruction_inner_instruction() {
        let trx = FULL_TRX.clone();

        let view = trx.compiled_instructions().nth(2).unwrap();

        assert_eq!(
            ComparableInstructionView {
                program_id: str("a5"),
                accounts: vec![str("a0"), str("a1")],
                data: str("0a0b0c"),
                stack_height: 1,
                instruction_id: 5,
                compiled_instruction_id: 3,
            },
            view.inner_instruction(0).unwrap().into()
        );

        assert_eq!(
            ComparableInstructionView {
                program_id: str("a6"),
                accounts: vec![str("a1"), str("a2")],
                data: str("0d0e0f"),
                stack_height: 2,
                instruction_id: 6,
                compiled_instruction_id: 3,
            },
            view.inner_instruction(1).unwrap().into()
        );

        assert_eq!(true, view.inner_instruction(2).is_none());
    }

    #[derive(Debug, PartialEq)]
    struct ComparableInstructionView {
        program_id: String,
        accounts: Vec<String>,
        data: String,
        stack_height: u32,
        // Program ID index of the instruction via the trait Instruction access
        instruction_id: u32,
        // Program ID index of the compiled instruction
        compiled_instruction_id: u32,
    }

    impl From<InstructionView<'_>> for ComparableInstructionView {
        fn from(view: InstructionView) -> Self {
            ComparableInstructionView {
                program_id: hex::encode(view.program_id()),
                accounts: view.accounts().iter().map(hex::encode).collect(),
                data: hex::encode(view.data()),
                stack_height: view.stack_height(),
                instruction_id: view.instruction.program_id_index(),
                compiled_instruction_id: view.compiled_instruction.program_id_index,
            }
        }
    }

    fn str(s: &str) -> String {
        s.to_string()
    }

    fn hex(s: &str) -> Vec<u8> {
        ::hex::decode(s).unwrap()
    }

    pub(super) static FULL_TRX: LazyLock<pb::ConfirmedTransaction> =
        LazyLock::new(|| pb::ConfirmedTransaction {
            transaction: pb::Transaction {
                signatures: vec![vec![1, 2, 3]],
                message: pb::Message {
                    account_keys: vec![
                        hex("a0"),
                        hex("a1"),
                        hex("a2"),
                        hex("a3"),
                        hex("a4"),
                        hex("a5"),
                        hex("a6"),
                    ],
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
                        },
                    ],
                    ..Default::default()
                }
                .into(),
            }
            .into(),
            meta: pb::TransactionStatusMeta {
                inner_instructions: vec![
                    pb::InnerInstructions {
                        index: 0,
                        instructions: vec![pb::InnerInstruction {
                            program_id_index: 4,
                            accounts: vec![0, 1],
                            data: vec![4, 5, 6],
                            stack_height: Some(1),
                        }],
                    },
                    pb::InnerInstructions {
                        index: 2,
                        instructions: vec![
                            pb::InnerInstruction {
                                program_id_index: 5,
                                accounts: vec![0, 1],
                                data: vec![10, 11, 12],
                                stack_height: Some(1),
                            },
                            pb::InnerInstruction {
                                program_id_index: 6,
                                accounts: vec![1, 2],
                                data: vec![13, 14, 15],
                                stack_height: Some(2),
                            },
                        ],
                    },
                ],
                ..Default::default()
            }
            .into(),
        });
}

#[cfg(test)]
mod lazy_parity_tests {
    use super::lazy::LazyTransaction;
    use super::tests::FULL_TRX;
    use crate::pb::sf::solana::r#type::v1 as pb;
    use crate::pb::sf::solana::r#type::v1::__buffa::lazy_view::ConfirmedTransactionLazyView;
    use buffa::view::LazyMessageView;
    use buffa::Message;
    use pretty_assertions::assert_eq;

    /// The lazy walk must visit the same instructions, in the same order, and
    /// resolve the same program ids as the owned walk.
    #[test]
    fn lazy_walk_instructions_matches_owned() {
        let owned: &pb::ConfirmedTransaction = &FULL_TRX;
        let bytes = owned.encode_to_vec();
        let view = ConfirmedTransactionLazyView::decode_lazy(&bytes).expect("valid transaction");

        let expected: Vec<(String, Vec<u8>, u32)> = owned
            .walk_instructions()
            .map(|inst| {
                (
                    inst.program_id().to_string(),
                    inst.data().clone(),
                    inst.stack_height(),
                )
            })
            .collect();

        let trx = LazyTransaction::new(&view).expect("resolves");
        let actual: Vec<(String, Vec<u8>, u32)> = trx
            .walk_instructions()
            .expect("walks")
            .map(|inst| {
                (
                    inst.program_id().to_string(),
                    inst.data().to_vec(),
                    inst.stack_height(),
                )
            })
            .collect();

        assert_eq!(expected, actual);
    }

    #[test]
    fn lazy_resolved_accounts_matches_owned() {
        let owned: &pb::ConfirmedTransaction = &FULL_TRX;
        let bytes = owned.encode_to_vec();
        let view = ConfirmedTransactionLazyView::decode_lazy(&bytes).expect("valid transaction");

        let expected: Vec<&Vec<u8>> = owned.resolved_accounts();
        let trx = LazyTransaction::new(&view).expect("resolves");
        let actual = trx.resolved_accounts();

        assert_eq!(expected.len(), actual.len());
        for (want, got) in expected.iter().zip(actual.iter()) {
            assert_eq!(want.as_slice(), *got);
        }
    }
}
