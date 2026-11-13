//! Generates Substreams Solana program bindings from Anchor IDL files.
//!
//! Run it from a `build.rs` through [`build::Idlgen`]. [`idl`] reads an IDL
//! (Anchor 0.30+ or legacy) into one model, [`codegen`] turns it into Rust, and
//! [`layout`] computes zero-copy account layouts.
//!
//! ```no_run
//! use substreams_solana_idlgen::idl::Idl;
//!
//! let idl = Idl::from_path("idls/my_program.json").expect("valid IDL");
//! for instruction in &idl.instructions {
//!     println!("{} {:?}", instruction.name, instruction.discriminator);
//! }
//! ```

pub mod build;
pub mod codegen;
pub mod idl;
pub mod layout;
