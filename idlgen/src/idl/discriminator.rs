//! Computes Anchor discriminators: the first 8 bytes of
//! `sha256("<namespace>:<name>")`, with namespace `global` (snake_case
//! instruction handler), `account` or `event` (Rust struct name).
//!
//! Only legacy IDLs need this; the 0.30 format writes discriminators out.

use heck::ToSnakeCase;
use sha2::{Digest, Sha256};

/// The length of an Anchor discriminator.
pub const LEN: usize = 8;

/// The first 8 bytes of `sha256("<namespace>:<name>")`.
pub fn sighash(namespace: &str, name: &str) -> [u8; LEN] {
    let mut hasher = Sha256::new();
    hasher.update(namespace.as_bytes());
    hasher.update(b":");
    hasher.update(name.as_bytes());

    let mut out = [0u8; LEN];
    out.copy_from_slice(&hasher.finalize()[..LEN]);
    out
}

/// The discriminator of the instruction whose snake_case handler is `name`.
pub fn instruction(name: &str) -> [u8; LEN] {
    sighash("global", name)
}

/// The discriminator of the account whose Rust struct is `name`.
pub fn account(name: &str) -> [u8; LEN] {
    sighash("account", name)
}

/// The discriminator of the event whose Rust struct is `name`.
pub fn event(name: &str) -> [u8; LEN] {
    sighash("event", name)
}

/// Turns a legacy camelCase name into snake_case as Anchor's converter does
/// (`swapV2` becomes `swap_v2`, `initialize2` stays `initialize2`).
pub fn legacy_snake_case(name: &str) -> String {
    name.to_snake_case()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_instruction_discriminators() {
        // pump.fun `buy` and `sell`, as seen on chain.
        assert_eq!(instruction("buy"), [102, 6, 61, 18, 1, 218, 235, 234]);
        assert_eq!(instruction("sell"), [51, 230, 133, 164, 1, 127, 131, 173]);
        // Orca Whirlpool `swap`.
        assert_eq!(
            instruction("swap"),
            [248, 198, 158, 145, 225, 117, 135, 200]
        );
    }

    #[test]
    fn known_account_and_event_discriminators() {
        // pump.fun `BondingCurve` account and `TradeEvent` event.
        assert_eq!(
            account("BondingCurve"),
            [23, 183, 248, 55, 96, 216, 172, 96]
        );
        assert_eq!(event("TradeEvent"), [189, 219, 127, 211, 78, 230, 97, 238]);
    }

    #[test]
    fn legacy_names() {
        assert_eq!(legacy_snake_case("swapBaseIn"), "swap_base_in");
        assert_eq!(legacy_snake_case("initialize2"), "initialize2");
        assert_eq!(legacy_snake_case("swapV2"), "swap_v2");
        assert_eq!(
            legacy_snake_case("openPositionWithMetadata"),
            "open_position_with_metadata"
        );
        assert_eq!(legacy_snake_case("already_snake"), "already_snake");
    }
}
