//! This crate provides the `b58!` macro for converting base58 string literals
//! to a byte array at compile time.
//!
//! # Examples
//! ```
//! # #[macro_use] extern crate substreams_solana_macro;
//! // the macro can be used in const context
//! const DATA: [u8; 3] = b58!("reg");
//! # fn main() {
//! assert_eq!(DATA, [0x02,0x8c,0x6d]);
//! # }
//! ```
extern crate proc_macro;
use proc_macro::{Delimiter, Group, Literal, Punct, Spacing, TokenStream, TokenTree};
use bs58;

/// Strips any outer `Delimiter::None` groups from the input,
/// returning a `TokenStream` consisting of the innermost
/// non-empty-group `TokenTree`.
/// This is used to handle a proc macro being invoked
/// by a `macro_rules!` expansion.
/// See https://github.com/rust-lang/rust/issues/72545 for background
fn ignore_groups(mut input: TokenStream) -> TokenStream {
    let mut tokens = input.clone().into_iter();
    loop {
        if let Some(TokenTree::Group(group)) = tokens.next() {
            if group.delimiter() == Delimiter::None {
                input = group.stream();
                continue;
            }
        }
        return input;
    }
}


/// Macro for converting sequence of string literals containing base58 encoded data
/// into an array of bytes.
///
#[proc_macro]
pub fn b58(input: TokenStream) -> TokenStream {
    for tt in ignore_groups(input) {
        match tt {
            TokenTree::Literal(literal) => {
                let mut input = literal.to_string();

                match input.as_bytes() {
                    [b'"', .., b'"'] => (),
                    _ => panic!("expected string literal, got `{}`", literal),
                };

                input.retain(|c| !r#"""#.contains(c));
                match  bs58::decode(input).into_vec() {
                    Ok(bytes) => {
                        let mut tokens: Vec<TokenTree> = vec![];
                        let mut has_seen_first = false;
                        for v in bytes {
                            if has_seen_first {
                                tokens.push(TokenTree::Punct(Punct::new(',', Spacing::Alone)))
                            } else {
                                has_seen_first = true;
                            }
                            tokens.push(TokenTree::Literal(Literal::u8_suffixed(v)))
                        }
                        let mut foo = TokenStream::new();
                        foo.extend(tokens.into_iter());
                        let out = TokenTree::Group(Group::new(Delimiter::Bracket, foo));
                        return TokenStream::from(out);

                    },
                    Err(e) => {
                        panic!("failed to decode string literal `{}`", e)
                    }
                }
            },
            unexpected => panic!("expected string literal, got `{}`", unexpected),
        };
    }
    panic!("expected a string literal")
}
