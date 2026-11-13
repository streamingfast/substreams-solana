//! Rust identifiers from IDL names.
//!
//! Types and variants are UpperCamelCase, fields snake_case. Keywords become raw
//! identifiers, except `self`, `Self`, `super` and `crate`, which get a trailing
//! `_`. Clashes within a namespace get a numeric suffix (`Swap`, `Swap2`).

use std::collections::HashSet;

use heck::{ToSnakeCase, ToUpperCamelCase};
use proc_macro2::{Ident, Span};

const KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "else", "enum", "extern", "false", "fn", "for", "if",
    "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "static",
    "struct", "trait", "true", "type", "unsafe", "use", "where", "while", "async", "await", "dyn",
    "abstract", "become", "box", "do", "final", "macro", "override", "priv", "typeof", "unsized",
    "virtual", "yield", "try", "gen",
];

/// Keywords that cannot be raw identifiers.
const NOT_RAW: &[&str] = &["self", "Self", "super", "crate", "_"];

/// Replaces characters that cannot appear in an identifier and prefixes a
/// leading digit.
fn sanitize(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() {
        out.push('_');
    }
    if out.starts_with(|c: char| c.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

/// `name` with characters an identifier cannot hold replaced, case kept.
pub fn sanitize_ident(name: &str) -> String {
    sanitize(name)
}

/// `name` as an UpperCamelCase string.
pub fn pascal(name: &str) -> String {
    let name = sanitize(name);
    let already = name.starts_with(|c: char| c.is_ascii_uppercase()) && !name.contains('_');
    if already {
        name
    } else {
        let converted = name.to_upper_camel_case();
        if converted.is_empty() || converted.starts_with(|c: char| c.is_ascii_digit()) {
            format!("T{converted}")
        } else {
            converted
        }
    }
}

/// `name` as a snake_case string.
pub fn snake(name: &str) -> String {
    let name = sanitize(name);
    if name.chars().any(|c| c.is_ascii_uppercase()) {
        let converted = name.to_snake_case();
        if converted.is_empty() || converted.starts_with(|c: char| c.is_ascii_digit()) {
            format!("_{converted}")
        } else {
            converted
        }
    } else {
        name
    }
}

/// An identifier for `name`, which must already be sanitized and cased.
pub fn ident(name: &str) -> Ident {
    if NOT_RAW.contains(&name) {
        Ident::new(&format!("{name}_"), Span::call_site())
    } else if KEYWORDS.contains(&name) {
        Ident::new_raw(name, Span::call_site())
    } else {
        Ident::new(name, Span::call_site())
    }
}

/// Hands out unique identifiers within one Rust namespace.
#[derive(Default)]
pub struct Namespace {
    taken: HashSet<String>,
}

impl Namespace {
    pub fn new() -> Self {
        Self::default()
    }

    /// Marks `name` as used, so that later names avoid it.
    pub fn reserve(&mut self, name: &str) {
        self.taken.insert(name.to_string());
    }

    /// A unique identifier for `cased`, a cased name, with a numeric suffix if
    /// it is already taken.
    pub fn unique(&mut self, cased: String) -> Ident {
        let ident_text = |s: &str| ident(s).to_string();
        let mut candidate = cased.clone();
        let mut n = 2;
        while self.taken.contains(&ident_text(&candidate)) {
            candidate = format!("{cased}{n}");
            n += 1;
        }
        let id = ident(&candidate);
        self.taken.insert(id.to_string());
        id
    }

    pub fn pascal(&mut self, name: &str) -> Ident {
        self.unique(pascal(name))
    }

    pub fn snake(&mut self, name: &str) -> Ident {
        self.unique(snake(name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn casing() {
        assert_eq!(pascal("Swap"), "Swap");
        assert_eq!(pascal("USDC"), "USDC");
        assert_eq!(pascal("swap_base_in"), "SwapBaseIn");
        assert_eq!(pascal("swapBaseIn"), "SwapBaseIn");
        assert_eq!(pascal("1x"), "T1x");
        assert_eq!(snake("amountIn"), "amount_in");
        assert_eq!(snake("_padding"), "_padding");
        assert_eq!(snake("pool.vault"), "pool_vault");
        assert_eq!(snake("2nd"), "_2nd");
    }

    #[test]
    fn keywords_and_clashes() {
        assert_eq!(ident("type").to_string(), "r#type");
        assert_eq!(ident("self").to_string(), "self_");
        assert_eq!(ident("Self").to_string(), "Self_");
        let mut ns = Namespace::new();
        assert_eq!(ns.pascal("swap").to_string(), "Swap");
        assert_eq!(ns.pascal("Swap").to_string(), "Swap2");
        assert_eq!(ns.pascal("swap").to_string(), "Swap3");
        let mut ns = Namespace::new();
        ns.reserve("accounts");
        assert_eq!(ns.snake("accounts").to_string(), "accounts2");
        assert_eq!(ns.snake("self").to_string(), "self_");
    }
}
