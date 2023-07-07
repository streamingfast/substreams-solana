use crate::{pb::sf::solana::r#type::v1 as pb};

impl pb::Block {
    /// Iterates over successful transactions in given block.
    pub fn transactions(&self) -> impl Iterator<Item = &pb::ConfirmedTransaction> {
        self.transactions.iter().filter(|trx| -> bool {
            if let Some(meta) = &trx.meta {
                return meta.err.is_none()
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
        self.transaction
            .iter()
            .flat_map(|trx| trx.message.iter().flat_map(|m| m.instructions.iter().map(|inst| InstructionView::<'a> {
                message: m,
                transaction: trx,
                instruction: inst,
            })))
    }

    pub fn meta(&self) -> Option<&pb::ConfirmedTransaction> {
        if self.meta.is_none() || self.meta.as_ref().unwrap().meta().is_none() {
            return None
        }

        return Some(self)
    }
}

impl pb::TransactionStatusMeta {
    pub fn meta(&self) -> Option<&pb::TransactionStatusMeta> {
        if self.err.is_some() || self.inner_instructions_none {
            return None
        }
        return Some(self)
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


impl InstructionView<'_>{
    pub fn account_at(&self,account_index: usize) -> &Vec<u8>{
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
    use crate::{pb::sf::solana::r#type::v1 as pb};

    #[test]
    fn it_iterates_over_successful_transaction() {
        let block = pb::Block{
            transactions: vec![
                pb::ConfirmedTransaction{
                    transaction: Some(pb::Transaction{
                        signatures: vec![vec![1,2,3]],
                        message: None
                    }),
                    meta: Some(pb::TransactionStatusMeta{
                        err: Some(pb::TransactionError{
                            ..Default::default()
                        }),
                        ..Default::default()
                    })
                },
                pb::ConfirmedTransaction{
                    transaction: Some(pb::Transaction{
                        signatures: vec![vec![4,5,6]],
                        message: None
                    }),
                    meta: Some(pb::TransactionStatusMeta{
                        err: None,
                        ..Default::default()
                    })
                },
                pb::ConfirmedTransaction{
                    transaction: Some(pb::Transaction{
                        signatures: vec![vec![7,8,9]],
                        message: None
                    }),
                    meta: None
                },
            ],
            ..Default::default()
        };

        let mut iter = block.transactions();
        assert_eq!(Some(&pb::ConfirmedTransaction{
            transaction: Some(pb::Transaction{
                signatures: vec![vec![4,5,6]],
                message: None
            }),
            meta: Some(pb::TransactionStatusMeta{
                err: None,
                ..Default::default()
            })
        }), iter.next());

        // ... and then None once it's over.
        assert_eq!(None, iter.next());
    }

}
