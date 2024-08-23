//! # Substreams Solana Library
//!
//! This library provides the Substreams Solana generated Protobuf bindings. As well as a bunch of helpers that
//! exists to make it easier to work with the generated Protobuf bindings.
//!
//! Below you will find information about the most important types, helpers and traits that are provided by this
//! library.
//!
//! ### Block, Transaction and Instruction Views & Helpers
//!
//! We provide the following helpers to deal with Solana block and transaction:
//! - [Block::transactions][substreams_solana_core::pb::sf::solana::type::v1::Block::transactions]
//!   returns an iterator over successful transactions of a block. The iterator element is a reference to
//!   [ConfirmedTransaction][substreams_solana_core::pb::sf::solana::type::v1::ConfirmedTransaction].
//!
//! - [Block::transactions_owned][substreams_solana_core::pb::sf::solana::type::v1::Block::transactions_owned]
//!   returns an owned iterator over successful transactions of a block. The iterator element is a owned
//!   [ConfirmedTransaction][substreams_solana_core::pb::sf::solana::type::v1::ConfirmedTransaction].
//!
//! - [Transaction::id][substreams_solana_core::pb::sf::solana::type::v1::Transaction::id]
//!   returns the transaction's id (first signature) as a base58 encoded string.
//!
//! - [Transaction::hash][substreams_solana_core::pb::sf::solana::type::v1::Transaction::hash]
//!   returns the transaction's hash (first signature) as byte array.
//!
//! - [ConfirmedTransaction::id][substreams_solana_core::pb::sf::solana::type::v1::ConfirmedTransaction::id]
//!   returns the transaction's id (first signature) as a base58 encoded string.
//!
//! - [ConfirmedTransaction::hash][substreams_solana_core::pb::sf::solana::type::v1::ConfirmedTransaction::hash]
//!   returns the transaction's hash (first signature) as byte array.
//!
//! - [ConfirmedTransaction::all_instructions][substreams_solana_core::pb::sf::solana::type::v1::ConfirmedTransaction::all_instructions]
//!   returns an iterator over all instructions, including inner instructions, of a transaction. The iterator
//!   element is a view over the instruction, has resolved accounts and provides access to the transaction, top level
//!   instruction and inner instructions of the transaction.  It make it much easier to walk the whole instruction tree of a
//!   transaction. Refer to the method documentation for more information about it.
pub use substreams_solana_core::{base58, block_view, pb, Instruction};
pub use substreams_solana_macro::b58;
