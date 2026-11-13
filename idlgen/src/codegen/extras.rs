//! The program's constants and error codes.

use proc_macro2::{Literal, TokenStream};
use quote::quote;

use super::names::{self, Namespace};
use super::{borsh, docs, error, Generator, Result, Scope};
use crate::idl::{ArrayLen, IdlType};

impl Generator<'_> {
    /// The `constants` module: one typed `pub const` per IDL constant.
    ///
    /// A literal that does not fit its type is an error. An expression such as
    /// `10u64.pow(9)`, or an unsupported type, is kept as source text in
    /// `<NAME>_EXPR: &str`, with a warning.
    pub(crate) fn constants_module(&self) -> Result<TokenStream> {
        let mut namespace = Namespace::new();
        let mut items = Vec::new();
        for constant in &self.idl.constants {
            let path = format!("constants.{}", constant.name);
            let resolved = self.resolve_alias(&constant.ty, &path)?;
            let value = constant.value.trim();
            let kept = match (literal_form(resolved, value), supported(resolved)) {
                (Form::Malformed, _) => {
                    return error(
                        &path,
                        format!("`{value}` is not a valid Rust literal or expression"),
                    )
                }
                (Form::Literal, true) => None,
                (Form::Literal, false) => Some(format!(
                    "constants of type {} are not generated",
                    type_name(resolved)
                )),
                (Form::Expression, _) => {
                    Some("the value is an expression, which is not evaluated".to_string())
                }
            };

            if let Some(reason) = kept {
                let ident =
                    namespace.unique(format!("{}_EXPR", names::sanitize_ident(&constant.name)));
                self.warn(format!(
                    "{path}: {reason}; kept as source text in `{ident}`"
                ));
                let mut doc = constant.docs.clone();
                if !doc.is_empty() {
                    doc.push(String::new());
                }
                doc.push(format!(
                    "The value of `{}` (type {}) as the IDL writes it: {reason}.",
                    constant.name,
                    type_name(&constant.ty)
                ));
                let doc = docs(&doc);
                items.push(quote!(#doc pub const #ident: &str = #value;));
                continue;
            }

            let ident = namespace.unique(names::sanitize_ident(&constant.name));
            let ty = match resolved {
                // Constants are borrowed, as Anchor declares them.
                IdlType::String => quote!(&str),
                IdlType::Bytes => quote!(&[u8]),
                _ => self.rust_type(&constant.ty, Scope::Sibling, &path)?,
            };
            let value = self.constant_value(resolved, value, &path)?;
            let doc = docs(&constant.docs);
            items.push(quote!(#doc pub const #ident: #ty = #value;));
        }
        Ok(quote! {
            /// The constants the program declares (`#[constant]`).
            pub mod constants {
                #![allow(non_upper_case_globals, unused_imports, clippy::all)]
                #(#items)*
            }
        })
    }

    /// The Rust expression of a constant literal `value` of type `ty`.
    pub(crate) fn constant_value(
        &self,
        ty: &IdlType,
        value: &str,
        path: &str,
    ) -> Result<TokenStream> {
        let value = value.trim();
        let invalid = |what: &str| error(path, format!("`{value}` is not {what}"));
        let b = borsh();
        Ok(match ty {
            IdlType::Bool => match value {
                "true" => quote!(true),
                "false" => quote!(false),
                _ => return invalid("a bool"),
            },
            IdlType::U8 | IdlType::U16 | IdlType::U32 | IdlType::U64 | IdlType::U128 => {
                let max = match ty {
                    IdlType::U8 => u8::MAX as u128,
                    IdlType::U16 => u16::MAX as u128,
                    IdlType::U32 => u32::MAX as u128,
                    IdlType::U64 => u64::MAX as u128,
                    _ => u128::MAX,
                };
                match parse_int(value) {
                    Some(Int::Positive(n)) if n <= max => {
                        let n = Literal::u128_unsuffixed(n);
                        quote!(#n)
                    }
                    Some(_) => {
                        return error(
                            path,
                            format!("`{value}` is out of range for {}", type_name(ty)),
                        )
                    }
                    None => return invalid("an integer literal"),
                }
            }
            IdlType::I8 | IdlType::I16 | IdlType::I32 | IdlType::I64 | IdlType::I128 => {
                let (min, max): (i128, i128) = match ty {
                    IdlType::I8 => (i8::MIN.into(), i8::MAX.into()),
                    IdlType::I16 => (i16::MIN.into(), i16::MAX.into()),
                    IdlType::I32 => (i32::MIN.into(), i32::MAX.into()),
                    IdlType::I64 => (i64::MIN.into(), i64::MAX.into()),
                    _ => (i128::MIN, i128::MAX),
                };
                match parse_int(value).and_then(Int::into_i128) {
                    Some(n) if (min..=max).contains(&n) => signed_literal(n),
                    Some(_) => {
                        return error(
                            path,
                            format!("`{value}` is out of range for {}", type_name(ty)),
                        )
                    }
                    None if parse_int(value).is_some() => {
                        return error(
                            path,
                            format!("`{value}` is out of range for {}", type_name(ty)),
                        )
                    }
                    None => return invalid("an integer literal"),
                }
            }
            IdlType::U256 => match parse_int(value) {
                Some(Int::Positive(n)) => {
                    let n = Literal::u128_unsuffixed(n);
                    quote!(#b::U256::from_u128(#n))
                }
                Some(Int::Negative(_)) => return error(path, format!("`{value}` is negative")),
                None => return invalid("an integer literal that fits in 128 bits"),
            },
            IdlType::I256 => match parse_int(value).and_then(Int::into_i128) {
                Some(n) => {
                    let n = signed_literal(n);
                    quote!(#b::I256::from_i128(#n))
                }
                None => return invalid("an integer literal that fits in 128 bits"),
            },
            IdlType::F32 | IdlType::F64 => {
                let cleaned = value.replace('_', "");
                let text = strip_suffix(&cleaned, &["f32", "f64"]);
                match text.parse::<f64>() {
                    Ok(v)
                        if v.is_finite()
                            && (matches!(ty, IdlType::F64) || (v as f32).is_finite()) =>
                    {
                        if matches!(ty, IdlType::F32) {
                            float_literal(Literal::f32_unsuffixed(v.abs() as f32), v < 0.0)
                        } else {
                            float_literal(Literal::f64_unsuffixed(v.abs()), v < 0.0)
                        }
                    }
                    _ => return invalid("a finite float literal in range"),
                }
            }
            IdlType::String => match syn::parse_str::<syn::LitStr>(value) {
                Ok(lit) => quote!(#lit),
                Err(_) => return invalid("a string literal"),
            },
            IdlType::Bytes => {
                let bytes = if let Ok(lit) = syn::parse_str::<syn::LitByteStr>(value) {
                    lit.value()
                } else {
                    let Some(items) = split_array(value) else {
                        return invalid("a byte array or a byte string literal");
                    };
                    let mut bytes = Vec::with_capacity(items.len());
                    for item in items {
                        match parse_int(item) {
                            Some(Int::Positive(n)) if n <= 255 => bytes.push(n as u8),
                            _ => return error(path, format!("`{item}` is not a byte")),
                        }
                    }
                    bytes
                };
                let bytes = bytes.iter().map(|b| Literal::u8_unsuffixed(*b));
                quote!(&[#(#bytes),*])
            }
            IdlType::Pubkey => {
                let text = value
                    .strip_prefix("pubkey!(")
                    .and_then(|v| v.strip_suffix(')'))
                    .unwrap_or(value)
                    .trim()
                    .trim_matches('"');
                match bs58::decode(text).into_vec() {
                    Ok(bytes) if bytes.len() == 32 => {
                        let bytes = bytes.iter().map(|b| Literal::u8_unsuffixed(*b));
                        quote!(#b::Pubkey::new([#(#bytes),*]))
                    }
                    _ => return invalid("a base58 32-byte address"),
                }
            }
            IdlType::Array(element, ArrayLen::Value(len)) => {
                let Some(items) = split_array(value) else {
                    return invalid("an array literal");
                };
                if items.len() != *len {
                    return error(
                        path,
                        format!("`{value}` has {} elements, the type has {len}", items.len()),
                    );
                }
                let element = self.resolve_alias(element, path)?;
                let items = items
                    .iter()
                    .map(|item| self.constant_value(element, item, path))
                    .collect::<Result<Vec<_>>>()?;
                quote!([#(#items),*])
            }
            other => {
                return error(
                    path,
                    format!("constants of type {} are not supported", type_name(other)),
                )
            }
        })
    }

    /// The `ProgramError` enum of the program's error codes.
    pub(crate) fn program_error(&self) -> TokenStream {
        let mut namespace = Namespace::new();
        let errors = &self.idl.errors;
        let idents: Vec<_> = errors.iter().map(|e| namespace.pascal(&e.name)).collect();
        let variants = errors.iter().zip(&idents).map(|(e, ident)| {
            let doc = match &e.msg {
                Some(msg) => docs(&[format!("{}: {msg}", e.code)]),
                None => docs(&[e.code.to_string()]),
            };
            quote!(#doc #ident)
        });

        let (code, name, msg) = if errors.is_empty() {
            (
                quote!(match *self {}),
                quote!(match *self {}),
                quote!(match *self {}),
            )
        } else {
            let codes = errors.iter().map(|e| e.code);
            let names = errors.iter().map(|e| e.name.as_str());
            let msgs = errors.iter().map(|e| match &e.msg {
                Some(msg) => quote!(::core::option::Option::Some(#msg)),
                None => quote!(::core::option::Option::None),
            });
            (
                quote!(match self { #(Self::#idents => #codes,)* }),
                quote!(match self { #(Self::#idents => #names,)* }),
                quote!(match self { #(Self::#idents => #msgs,)* }),
            )
        };
        // A code listed twice maps back to its first variant.
        let mut seen = std::collections::HashSet::new();
        let from_code = errors
            .iter()
            .zip(&idents)
            .filter(|(e, _)| seen.insert(e.code))
            .map(|(e, ident)| {
                let code = e.code;
                quote!(#code => ::core::option::Option::Some(Self::#ident),)
            })
            .collect::<Vec<_>>();
        let from_code = if from_code.is_empty() {
            quote! {
                let _ = code;
                ::core::option::Option::None
            }
        } else {
            quote! {
                match code {
                    #(#from_code)*
                    _ => ::core::option::Option::None,
                }
            }
        };

        quote! {
            /// The error codes the program defines, as reported in `InstructionError::Custom(code)`.
            ///
            /// Anchor programs number their errors from 6000, below which are Anchor's own
            /// framework errors; native (Shank) programs choose their own codes.
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
            pub enum ProgramError {
                #(#variants,)*
            }

            impl ProgramError {
                /// The error with the custom error `code`, [None] when the program defines none.
                pub fn from_code(code: u32) -> ::core::option::Option<Self> {
                    #from_code
                }

                /// The custom error code.
                pub fn code(&self) -> u32 {
                    #code
                }

                /// The error name, as in the IDL.
                pub fn name(&self) -> &'static str {
                    #name
                }

                /// The error message, when the IDL has one.
                pub fn msg(&self) -> ::core::option::Option<&'static str> {
                    #msg
                }
            }

            impl ::core::fmt::Display for ProgramError {
                fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
                    match self.msg() {
                        ::core::option::Option::Some(msg) => {
                            ::core::write!(f, "{} ({}): {}", self.name(), self.code(), msg)
                        }
                        ::core::option::Option::None => {
                            ::core::write!(f, "{} ({})", self.name(), self.code())
                        }
                    }
                }
            }

            impl ::std::error::Error for ProgramError {}
        }
    }
}

/// How a constant value is written.
#[derive(Debug, PartialEq)]
enum Form {
    /// A literal: checked strictly against the type.
    Literal,
    /// Any other valid Rust expression.
    Expression,
    /// Not valid Rust.
    Malformed,
}

/// Whether `value` is a literal, an expression, or neither. A bare base58
/// address counts as a literal of a `pubkey` constant.
fn literal_form(ty: &IdlType, value: &str) -> Form {
    if matches!(ty, IdlType::Pubkey)
        && bs58::decode(value.trim_matches('"'))
            .into_vec()
            .is_ok_and(|b| b.len() == 32)
    {
        return Form::Literal;
    }
    match syn::parse_str::<syn::Expr>(value) {
        Ok(expr) if is_literal(&expr) => Form::Literal,
        Ok(_) => Form::Expression,
        Err(_) => Form::Malformed,
    }
}

fn is_literal(expr: &syn::Expr) -> bool {
    match expr {
        syn::Expr::Lit(_) => true,
        syn::Expr::Unary(unary) => {
            matches!(unary.op, syn::UnOp::Neg(_)) && matches!(&*unary.expr, syn::Expr::Lit(_))
        }
        syn::Expr::Array(array) => array.elems.iter().all(is_literal),
        syn::Expr::Macro(mac) => mac.mac.path.is_ident("pubkey"),
        syn::Expr::Group(group) => is_literal(&group.expr),
        _ => false,
    }
}

/// Whether a typed constant can be generated for `ty`.
fn supported(ty: &IdlType) -> bool {
    match ty {
        IdlType::Option(_)
        | IdlType::COption(_)
        | IdlType::Vec(_)
        | IdlType::Map(..)
        | IdlType::Set(_)
        | IdlType::Tuple(_)
        | IdlType::Generic(_) => false,
        IdlType::Defined { .. } => false,
        IdlType::Array(element, ArrayLen::Value(_)) => supported(element),
        IdlType::Array(_, ArrayLen::Generic(_)) => false,
        _ => true,
    }
}

enum Int {
    Positive(u128),
    /// The magnitude of a negative value.
    Negative(u128),
}

impl Int {
    fn into_i128(self) -> Option<i128> {
        match self {
            Int::Positive(n) => i128::try_from(n).ok(),
            Int::Negative(n) if n == i128::MIN.unsigned_abs() => Some(i128::MIN),
            Int::Negative(n) => i128::try_from(n).ok().map(|n| -n),
        }
    }
}

const INT_SUFFIXES: &[&str] = &[
    "u128", "i128", "usize", "isize", "u64", "i64", "u32", "i32", "u16", "i16", "u8", "i8",
];

fn strip_suffix<'s>(text: &'s str, suffixes: &[&str]) -> &'s str {
    suffixes
        .iter()
        .find_map(|suffix| text.strip_suffix(suffix))
        .unwrap_or(text)
}

/// Parses an integer literal: decimal, `0x`, `0o` or `0b`, with optional `_`
/// separators, type suffix and leading `-`.
fn parse_int(value: &str) -> Option<Int> {
    let text = value.trim().replace('_', "");
    let (negative, text) = match text.strip_prefix('-') {
        Some(rest) => (true, rest.trim_start()),
        None => (false, text.as_str()),
    };
    // Suffixes start with `u` or `i`, which are not hex digits.
    let text = strip_suffix(text, INT_SUFFIXES);
    let (radix, digits) = match text.get(..2) {
        Some("0x") | Some("0X") => (16, &text[2..]),
        Some("0o") => (8, &text[2..]),
        Some("0b") => (2, &text[2..]),
        _ => (10, text),
    };
    if digits.is_empty() || !digits.chars().all(|c| c.is_digit(radix)) {
        return None;
    }
    let n = u128::from_str_radix(digits, radix).ok()?;
    Some(if negative {
        Int::Negative(n)
    } else {
        Int::Positive(n)
    })
}

fn signed_literal(n: i128) -> TokenStream {
    let magnitude = Literal::u128_unsuffixed(n.unsigned_abs());
    if n < 0 {
        quote!(-#magnitude)
    } else {
        quote!(#magnitude)
    }
}

fn float_literal(magnitude: Literal, negative: bool) -> TokenStream {
    if negative {
        quote!(-#magnitude)
    } else {
        quote!(#magnitude)
    }
}

/// The top-level items of an array literal `[a, [b, c], d]`.
fn split_array(value: &str) -> Option<Vec<&str>> {
    let inner = value.trim().strip_prefix('[')?.strip_suffix(']')?.trim();
    if inner.is_empty() {
        return Some(Vec::new());
    }
    let mut items = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (i, c) in inner.char_indices() {
        match c {
            '[' => depth += 1,
            ']' => depth = depth.checked_sub(1)?,
            ',' if depth == 0 => {
                items.push(inner[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    let last = inner[start..].trim();
    if !last.is_empty() {
        items.push(last);
    }
    Some(items)
}

fn type_name(ty: &IdlType) -> String {
    match ty {
        IdlType::Bool => "bool".into(),
        IdlType::U8 => "u8".into(),
        IdlType::I8 => "i8".into(),
        IdlType::U16 => "u16".into(),
        IdlType::I16 => "i16".into(),
        IdlType::U32 => "u32".into(),
        IdlType::I32 => "i32".into(),
        IdlType::F32 => "f32".into(),
        IdlType::U64 => "u64".into(),
        IdlType::I64 => "i64".into(),
        IdlType::F64 => "f64".into(),
        IdlType::U128 => "u128".into(),
        IdlType::I128 => "i128".into(),
        IdlType::U256 => "u256".into(),
        IdlType::I256 => "i256".into(),
        IdlType::Bytes => "bytes".into(),
        IdlType::String => "string".into(),
        IdlType::Pubkey => "pubkey".into(),
        IdlType::Option(_) => "option".into(),
        IdlType::COption(_) => "coption".into(),
        IdlType::Vec(_) => "vec".into(),
        IdlType::Map(..) => "map".into(),
        IdlType::Set(_) => "set".into(),
        IdlType::Tuple(_) => "tuple".into(),
        IdlType::Array(..) => "array".into(),
        IdlType::Defined { name, .. } => format!("`{name}`"),
        IdlType::Generic(name) => format!("generic `{name}`"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idl::Idl;

    fn value(ty: IdlType, value: &str) -> std::result::Result<String, String> {
        let idl = Idl::from_json(
            r#"{"address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
                "metadata": {"name": "t", "version": "0.1.0", "spec": "0.1.0"},
                "instructions": []}"#,
        )
        .unwrap();
        let generator = Generator::new(&idl).unwrap();
        generator
            .constant_value(&ty, value, "constants.X")
            .map(|tokens| tokens.to_string())
            .map_err(|err| err.to_string())
    }

    #[test]
    fn integers() {
        assert_eq!(value(IdlType::U64, "1_000_000_000").unwrap(), "1000000000");
        assert_eq!(value(IdlType::U8, "0xff").unwrap(), "255");
        assert_eq!(value(IdlType::U16, "42u16").unwrap(), "42");
        assert_eq!(value(IdlType::I32, "-7").unwrap(), "- 7");
        assert_eq!(
            value(IdlType::I128, &i128::MIN.to_string()).unwrap(),
            "- 170141183460469231731687303715884105728"
        );
        assert_eq!(
            value(IdlType::U128, &u128::MAX.to_string()).unwrap(),
            u128::MAX.to_string()
        );
        assert!(value(IdlType::U8, "256")
            .unwrap_err()
            .contains("out of range for u8"));
        assert!(value(IdlType::U8, "-1")
            .unwrap_err()
            .contains("out of range"));
        assert!(value(IdlType::I8, "-129")
            .unwrap_err()
            .contains("out of range for i8"));
        assert!(value(IdlType::U64, "10 * 1000")
            .unwrap_err()
            .starts_with("constants.X: `10 * 1000` is not an integer literal"));
        assert!(value(IdlType::U256, "5")
            .unwrap()
            .contains("U256 :: from_u128 (5)"));
    }

    #[test]
    fn other_types() {
        assert_eq!(value(IdlType::Bool, "true").unwrap(), "true");
        assert_eq!(value(IdlType::F64, "-1.5").unwrap(), "- 1.5");
        assert!(value(IdlType::F32, "1e60").is_err());
        assert_eq!(value(IdlType::String, r#""seed""#).unwrap(), r#""seed""#);
        assert!(value(IdlType::String, "seed").is_err());
        assert_eq!(value(IdlType::Bytes, "[97, 117]").unwrap(), "& [97 , 117]");
        assert_eq!(value(IdlType::Bytes, r#"b"au""#).unwrap(), "& [97 , 117]");
        assert!(value(IdlType::Bytes, "[256]")
            .unwrap_err()
            .contains("not a byte"));
        assert!(
            value(IdlType::Pubkey, "\"11111111111111111111111111111111\"")
                .unwrap()
                .contains("Pubkey :: new")
        );
        let array = IdlType::Array(Box::new(IdlType::U8), ArrayLen::Value(2));
        assert_eq!(value(array.clone(), "[1, 2]").unwrap(), "[1 , 2]");
        assert!(value(array, "[1]").unwrap_err().contains("has 1 elements"));
        assert!(value(IdlType::Vec(Box::new(IdlType::U8)), "[]")
            .unwrap_err()
            .contains("not supported"));
    }
}
