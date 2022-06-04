#[path = "sf.solana.type.v1.rs"]
mod sf_solana_type_v1;

pub mod sol {
    pub mod v1 {
        pub use crate::pb::sf_solana_type_v1::*;
    }
}
