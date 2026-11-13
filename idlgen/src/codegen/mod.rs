//! Generates Rust bindings for a program from its [`Idl`].
//!
//! The output is one module body per program, depending only on
//! `substreams-solana`. It holds `PROGRAM_ID`, the `types`, `instructions`,
//! `accounts`, `events` and `constants` modules, the `Instruction`, `Account`
//! and `Event` dispatch enums, and `ProgramError`. Zero-copy types get their raw
//! memory layout, see [`crate::layout`].
//!
//! Generated code names outside items by absolute path, so IDL types named
//! `Option`, `Pubkey` or `Result` cannot shadow them.

mod extras;
mod names;
mod program;

use std::cell::RefCell;
use std::collections::{BTreeSet, HashMap};
use std::fmt;

use proc_macro2::{Ident, Literal, TokenStream};
use quote::{format_ident, quote};

use crate::idl::{
    ArrayLen, Fields, GenericArg, GenericParam, Idl, IdlType, Serialization, TypeDef, TypeDefKind,
};
use crate::layout;

/// Settings that the IDL does not carry.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// The program address in base58. Overrides the IDL address; required when
    /// the IDL has none.
    pub program_id: Option<String>,
}

/// Why bindings could not be generated.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodegenError {
    /// The IDL item at fault, such as `types.Bank.config`.
    pub path: String,
    pub message: String,
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            write!(f, "{}", self.message)
        } else {
            write!(f, "{}: {}", self.path, self.message)
        }
    }
}

impl std::error::Error for CodegenError {}

pub(crate) type Result<T> = std::result::Result<T, CodegenError>;

pub(crate) fn error<T>(path: impl Into<String>, message: impl Into<String>) -> Result<T> {
    Err(CodegenError {
        path: path.into(),
        message: message.into(),
    })
}

/// Generated bindings and the warnings about items not generated as typed code.
#[derive(Debug, Clone)]
pub struct Output {
    pub tokens: TokenStream,
    /// Each starts with the IDL item, such as `constants.SCALE: ...`.
    pub warnings: Vec<String>,
}

/// Generates the bindings of `idl`.
pub fn generate(idl: &Idl, options: &Options) -> Result<Output> {
    let generator = Generator::new(idl)?;
    let tokens = generator.program(options)?;
    Ok(Output {
        tokens,
        warnings: generator.warnings.into_inner(),
    })
}

/// The module generated code lives in, which decides the path to `types`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Scope {
    /// Inside the `types` module.
    Types,
    /// Inside a sibling module (`instructions`, `accounts`, `events`).
    Sibling,
}

pub(crate) struct Generator<'a> {
    idl: &'a Idl,
    /// The Rust name of each type definition, by IDL name.
    type_idents: HashMap<&'a str, Ident>,
    /// Types that need a zero-copy reader and writer.
    zero_copy: BTreeSet<&'a str>,
    warnings: RefCell<Vec<String>>,
}

fn borsh() -> TokenStream {
    quote!(::substreams_solana::borsh)
}

fn result_ty(ok: TokenStream) -> TokenStream {
    let b = borsh();
    quote!(::core::result::Result<#ok, #b::DecodeError>)
}

fn push_field_types<'t>(fields: &'t Fields, out: &mut Vec<&'t IdlType>) {
    match fields {
        Fields::Unit => {}
        Fields::Named(fields) => out.extend(fields.iter().map(|f| &f.ty)),
        Fields::Tuple(types) => out.extend(types),
    }
}

/// `#[doc]` attributes for `lines`, prefixed with a space to print as `/// line`.
pub(crate) fn docs<S: AsRef<str>>(lines: &[S]) -> TokenStream {
    let lines = lines.iter().map(|line| {
        let line = line.as_ref();
        if line.is_empty() || line.starts_with(' ') {
            line.to_string()
        } else {
            format!(" {line}")
        }
    });
    quote!(#( #[doc = #lines] )*)
}

fn bytes_literal(bytes: &[u8]) -> TokenStream {
    let bytes = bytes.iter().map(|b| Literal::u8_unsuffixed(*b));
    quote!(&[#(#bytes),*])
}

impl<'a> Generator<'a> {
    fn new(idl: &'a Idl) -> Result<Self> {
        let mut namespace = names::Namespace::new();
        let type_idents = idl
            .types
            .iter()
            .map(|def| (def.name.as_str(), namespace.pascal(&def.name)))
            .collect();

        let mut generator = Generator {
            idl,
            type_idents,
            zero_copy: BTreeSet::new(),
            warnings: RefCell::new(Vec::new()),
        };
        for def in &idl.types {
            match &def.serialization {
                Serialization::Borsh => {}
                Serialization::Bytemuck | Serialization::BytemuckUnsafe => {
                    if !matches!(def.kind, TypeDefKind::Alias(_)) {
                        generator.mark_zero_copy(def)?;
                    }
                }
                Serialization::Custom(name) => {
                    return error(
                        format!("types.{}", def.name),
                        format!("serialization `{name}` is not supported"),
                    )
                }
            }
        }
        Ok(generator)
    }

    /// Checks that `def` has a zero-copy layout and marks it and its nested types.
    fn mark_zero_copy(&mut self, def: &'a TypeDef) -> Result<()> {
        if let Err(err) = layout::struct_layout(self.idl, def) {
            return error(format!("types.{}", err.path), err.message);
        }
        let mut pending = vec![def];
        while let Some(def) = pending.pop() {
            if !self.zero_copy.insert(&def.name) {
                continue;
            }
            let mut types: Vec<&IdlType> = Vec::new();
            match &def.kind {
                TypeDefKind::Struct(Fields::Named(fields)) => {
                    types.extend(fields.iter().map(|f| &f.ty))
                }
                TypeDefKind::Struct(Fields::Tuple(fields)) => types.extend(fields),
                _ => {}
            }
            while let Some(ty) = types.pop() {
                match ty {
                    IdlType::Array(element, _) => types.push(element),
                    IdlType::Defined { name, .. } => {
                        let nested = self.type_def(name, "")?;
                        match &nested.kind {
                            TypeDefKind::Alias(target) => types.push(target),
                            _ => pending.push(nested),
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok(())
    }

    pub(crate) fn warn(&self, warning: String) {
        self.warnings.borrow_mut().push(warning);
    }

    pub(crate) fn type_def(&self, name: &str, path: &str) -> Result<&'a TypeDef> {
        match self.idl.type_def(name) {
            Some(def) => Ok(def),
            None => error(path, format!("unknown type `{name}`")),
        }
    }

    pub(crate) fn type_ident(&self, name: &str) -> &Ident {
        &self.type_idents[name]
    }

    /// The path to the IDL type `name` from `scope`.
    pub(crate) fn type_path(&self, name: &str, scope: Scope, path: &str) -> Result<TokenStream> {
        let Some(ident) = self.type_idents.get(name) else {
            return error(path, format!("unknown type `{name}`"));
        };
        Ok(match scope {
            Scope::Types => quote!(self::#ident),
            Scope::Sibling => quote!(super::types::#ident),
        })
    }

    /// The Rust type of an IDL type.
    pub(crate) fn rust_type(&self, ty: &IdlType, scope: Scope, path: &str) -> Result<TokenStream> {
        self.field_type(ty, scope, path, None)
    }

    /// The Rust type of a field of `owner`. A reference back to `owner` not behind
    /// a `Vec` is boxed: the IDL drops the `Box`, and Borsh encodes `Box<T>` as `T`.
    fn field_type(
        &self,
        ty: &IdlType,
        scope: Scope,
        path: &str,
        owner: Option<&str>,
    ) -> Result<TokenStream> {
        let rust_type = |ty: &IdlType| self.field_type(ty, scope, path, owner);
        let b = borsh();
        Ok(match ty {
            IdlType::Bool => quote!(bool),
            IdlType::U8 => quote!(u8),
            IdlType::I8 => quote!(i8),
            IdlType::U16 => quote!(u16),
            IdlType::I16 => quote!(i16),
            IdlType::U32 => quote!(u32),
            IdlType::I32 => quote!(i32),
            IdlType::F32 => quote!(f32),
            IdlType::U64 => quote!(u64),
            IdlType::I64 => quote!(i64),
            IdlType::F64 => quote!(f64),
            IdlType::U128 => quote!(u128),
            IdlType::I128 => quote!(i128),
            IdlType::U256 => quote!(#b::U256),
            IdlType::I256 => quote!(#b::I256),
            IdlType::Bytes => quote!(::std::vec::Vec<u8>),
            IdlType::String => quote!(::std::string::String),
            IdlType::Pubkey => quote!(#b::Pubkey),
            IdlType::Option(inner) => {
                let inner = rust_type(inner)?;
                quote!(::core::option::Option<#inner>)
            }
            IdlType::COption(inner) => {
                let inner = rust_type(inner)?;
                quote!(#b::COption<#inner>)
            }
            IdlType::Vec(inner) | IdlType::Set(inner) => {
                let inner = rust_type(inner)?;
                quote!(::std::vec::Vec<#inner>)
            }
            // Kept as pairs, in the encoded order: lossless and without `Hash`/`Ord` bounds.
            IdlType::Map(key, value) => {
                let key = rust_type(key)?;
                let value = rust_type(value)?;
                quote!(::std::vec::Vec<(#key, #value)>)
            }
            IdlType::Tuple(items) => {
                let items = items.iter().map(rust_type).collect::<Result<Vec<_>>>()?;
                quote!((#(#items,)*))
            }
            IdlType::Array(inner, len) => {
                let inner = rust_type(inner)?;
                let len = match len {
                    ArrayLen::Value(n) => {
                        let n = Literal::usize_unsuffixed(*n);
                        quote!(#n)
                    }
                    ArrayLen::Generic(name) => {
                        let ident = names::ident(name);
                        quote!(#ident)
                    }
                };
                quote!([#inner; #len])
            }
            IdlType::Defined { name, generics } => {
                let mut base = self.type_path(name, scope, path)?;
                let recursive = owner.is_some_and(|owner| self.reaches(name, owner));
                if !generics.is_empty() {
                    let args = generics
                        .iter()
                        .map(|arg| self.generic_arg(arg, scope, path, owner))
                        .collect::<Result<Vec<_>>>()?;
                    base = quote!(#base<#(#args),*>);
                }
                if recursive {
                    quote!(::std::boxed::Box<#base>)
                } else {
                    base
                }
            }
            IdlType::Generic(name) => {
                let ident = names::ident(name);
                quote!(#ident)
            }
        })
    }

    /// Whether the type `from` holds a `target` without a `Vec` in between.
    fn reaches(&self, from: &str, target: &str) -> bool {
        let mut seen: Vec<&str> = Vec::new();
        let mut pending = vec![from];
        while let Some(name) = pending.pop() {
            if name == target {
                return true;
            }
            if seen.contains(&name) {
                continue;
            }
            seen.push(name);
            let Some(def) = self.idl.type_def(name) else {
                continue;
            };
            let mut types: Vec<&IdlType> = Vec::new();
            match &def.kind {
                TypeDefKind::Alias(ty) => types.push(ty),
                TypeDefKind::Struct(fields) => push_field_types(fields, &mut types),
                TypeDefKind::Enum(variants) => {
                    for variant in variants {
                        push_field_types(&variant.fields, &mut types);
                    }
                }
            }
            while let Some(ty) = types.pop() {
                match ty {
                    IdlType::Option(inner) | IdlType::COption(inner) | IdlType::Array(inner, _) => {
                        types.push(inner)
                    }
                    IdlType::Tuple(items) => types.extend(items),
                    IdlType::Defined { name, generics } => {
                        pending.push(name);
                        for arg in generics {
                            if let GenericArg::Type(ty) = arg {
                                types.push(ty);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        false
    }

    fn generic_arg(
        &self,
        arg: &GenericArg,
        scope: Scope,
        path: &str,
        owner: Option<&str>,
    ) -> Result<TokenStream> {
        match arg {
            GenericArg::Type(ty) => self.field_type(ty, scope, path, owner),
            GenericArg::Const(value) => {
                let value = value.trim();
                if let Ok(n) = value.parse::<u128>() {
                    let n = Literal::u128_unsuffixed(n);
                    Ok(quote!(#n))
                } else if let Ok(n) = value.parse::<i128>() {
                    let n = Literal::i128_unsuffixed(n);
                    Ok(quote!({ #n }))
                } else if value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    let ident = names::ident(value);
                    Ok(quote!(#ident))
                } else {
                    error(
                        path,
                        format!("unsupported const generic argument `{value}`"),
                    )
                }
            }
        }
    }

    /// The generic parameters of a definition, with `bound` on each type
    /// parameter, and the matching arguments.
    fn generics(
        &self,
        def: &TypeDef,
        bound: Option<TokenStream>,
        path: &str,
    ) -> Result<(TokenStream, TokenStream)> {
        if def.generics.is_empty() {
            return Ok((quote!(), quote!()));
        }
        let mut params = Vec::new();
        let mut args = Vec::new();
        for param in &def.generics {
            let ident = names::ident(param.name());
            match param {
                GenericParam::Type { .. } => match &bound {
                    Some(bound) => params.push(quote!(#ident: #bound)),
                    None => params.push(quote!(#ident)),
                },
                GenericParam::Const { ty, .. } => {
                    let ty: syn::Type = syn::parse_str(ty).map_err(|_| CodegenError {
                        path: path.to_string(),
                        message: format!("invalid const generic type `{ty}`"),
                    })?;
                    params.push(quote!(const #ident: #ty));
                }
            }
            args.push(quote!(#ident));
        }
        Ok((quote!(<#(#params),*>), quote!(<#(#args),*>)))
    }

    /// The `types` module.
    fn types_module(&self) -> Result<TokenStream> {
        let mut items = Vec::new();
        for def in &self.idl.types {
            let ident = self.type_ident(&def.name).clone();
            let zero_copy = self.zero_copy.contains(def.name.as_str());
            items.push(self.type_def_items(
                def,
                &ident,
                Scope::Types,
                &format!("types.{}", def.name),
                zero_copy,
            )?);
        }
        Ok(quote! {
            /// The types the program defines.
            pub mod types {
                #![allow(non_camel_case_types, non_snake_case, unused_imports, unused_variables, clippy::all)]
                #(#items)*
            }
        })
    }

    /// A type definition with its Borsh impls and, if `zero_copy`, its zero-copy impl.
    pub(crate) fn type_def_items(
        &self,
        def: &TypeDef,
        ident: &Ident,
        scope: Scope,
        path: &str,
        zero_copy: bool,
    ) -> Result<TokenStream> {
        let b = borsh();
        let doc = docs(&def.docs);
        let (decl_generics, args) = self.generics(def, None, path)?;
        let (decode_generics, _) = self.generics(def, Some(quote!(#b::BorshDecode)), path)?;
        let ok = result_ty(quote!(Self));

        let (decl, decode) = match &def.kind {
            TypeDefKind::Alias(target) => {
                let target = self.rust_type(target, scope, path)?;
                return Ok(quote! {
                    #doc
                    pub type #ident #decl_generics = #target;
                });
            }
            TypeDefKind::Struct(fields) => {
                let (body, decode) = self.struct_parts(fields, Some(&def.name), scope, path)?;
                let decl = match fields {
                    Fields::Named(_) => quote!(pub struct #ident #decl_generics #body),
                    _ => quote!(pub struct #ident #decl_generics #body;),
                };
                (decl, decode)
            }
            TypeDefKind::Enum(variants) => {
                if variants.len() > 256 {
                    return error(path, "an enum has at most 256 variants in Borsh");
                }
                let mut namespace = names::Namespace::new();
                let mut decl_variants = Vec::new();
                let mut decode_arms = Vec::new();
                for (index, variant) in variants.iter().enumerate() {
                    let index = index as u8;
                    let variant_path = format!("{path}.{}", variant.name);
                    let v = namespace.pascal(&variant.name);
                    let (body, decode) = self.variant_parts(
                        &v,
                        &variant.fields,
                        Some(&def.name),
                        scope,
                        &variant_path,
                    )?;
                    decl_variants.push(quote!(#v #body));
                    decode_arms.push(quote!(#index => #decode));
                }
                let name = &def.name;
                let decl = quote!(pub enum #ident #decl_generics { #(#decl_variants),* });
                let decode = quote! {
                    let variant = r.read_variant()?;
                    match variant {
                        #(#decode_arms,)*
                        _ => ::core::result::Result::Err(#b::DecodeError::invalid_variant(r, #name, variant)),
                    }
                };
                (decl, decode)
            }
        };

        let zero_copy = if zero_copy {
            self.zero_copy_impl(def, ident, scope, path)?
        } else {
            quote!()
        };

        Ok(quote! {
            #doc
            #[derive(Debug, Clone, PartialEq)]
            #decl

            impl #decode_generics #b::BorshDecode for #ident #args {
                fn decode(r: &mut #b::BorshReader<'_>) -> #ok {
                    #decode
                }
            }

            #zero_copy
        })
    }

    /// Declaration body and decode expression of a struct.
    fn struct_parts(
        &self,
        fields: &Fields,
        owner: Option<&str>,
        scope: Scope,
        path: &str,
    ) -> Result<(TokenStream, TokenStream)> {
        Ok(match fields {
            Fields::Unit => (quote!(), quote!(::core::result::Result::Ok(Self))),
            Fields::Named(fields) => {
                let mut namespace = names::Namespace::new();
                let mut decl = Vec::new();
                let mut idents = Vec::new();
                for field in fields {
                    let ident = namespace.snake(&field.name);
                    let ty = self.field_type(
                        &field.ty,
                        scope,
                        &format!("{path}.{}", field.name),
                        owner,
                    )?;
                    let doc = docs(&field.docs);
                    decl.push(quote!(#doc pub #ident: #ty));
                    idents.push(ident);
                }
                (
                    quote!({ #(#decl,)* }),
                    quote!(::core::result::Result::Ok(Self { #(#idents: r.read()?,)* })),
                )
            }
            Fields::Tuple(types) => {
                let types = types
                    .iter()
                    .enumerate()
                    .map(|(i, ty)| self.field_type(ty, scope, &format!("{path}.{i}"), owner))
                    .collect::<Result<Vec<_>>>()?;
                let reads = types.iter().map(|_| quote!(r.read()?));
                (
                    quote!((#(pub #types),*)),
                    quote!(::core::result::Result::Ok(Self(#(#reads),*))),
                )
            }
        })
    }

    /// Declaration body and decode expression of an enum variant.
    fn variant_parts(
        &self,
        v: &Ident,
        fields: &Fields,
        owner: Option<&str>,
        scope: Scope,
        path: &str,
    ) -> Result<(TokenStream, TokenStream)> {
        Ok(match fields {
            Fields::Unit => (quote!(), quote!(::core::result::Result::Ok(Self::#v))),
            Fields::Named(fields) => {
                let mut namespace = names::Namespace::new();
                let mut decl = Vec::new();
                let mut idents = Vec::new();
                for field in fields {
                    let ident = namespace.snake(&field.name);
                    let ty = self.field_type(
                        &field.ty,
                        scope,
                        &format!("{path}.{}", field.name),
                        owner,
                    )?;
                    let doc = docs(&field.docs);
                    decl.push(quote!(#doc #ident: #ty));
                    idents.push(ident);
                }
                (
                    quote!({ #(#decl,)* }),
                    quote!(::core::result::Result::Ok(Self::#v { #(#idents: r.read()?,)* })),
                )
            }
            Fields::Tuple(types) => {
                let types = types
                    .iter()
                    .enumerate()
                    .map(|(i, ty)| self.field_type(ty, scope, &format!("{path}.{i}"), owner))
                    .collect::<Result<Vec<_>>>()?;
                let reads = types.iter().map(|_| quote!(r.read()?));
                (
                    quote!((#(#types),*)),
                    quote!(::core::result::Result::Ok(Self::#v(#(#reads),*))),
                )
            }
        })
    }

    /// `SIZE`, `ALIGN` and the raw-layout reader of a zero-copy struct.
    fn zero_copy_impl(
        &self,
        def: &TypeDef,
        ident: &Ident,
        scope: Scope,
        path: &str,
    ) -> Result<TokenStream> {
        let b = borsh();
        let ok = result_ty(quote!(Self));
        let layout = match layout::struct_layout(self.idl, def) {
            Ok(layout) => layout,
            Err(err) => return error(format!("types.{}", err.path), err.message),
        };
        let types: Vec<&IdlType> = match &def.kind {
            TypeDefKind::Struct(Fields::Named(fields)) => fields.iter().map(|f| &f.ty).collect(),
            TypeDefKind::Struct(Fields::Tuple(types)) => types.iter().collect(),
            _ => Vec::new(),
        };

        let mut cursor = 0usize;
        let mut reads = Vec::new();
        let mut bindings = Vec::new();
        for (i, (field, ty)) in layout.fields.iter().zip(&types).enumerate() {
            let field_path = format!("{path}.{}", field.name);
            let pad = Literal::usize_unsuffixed(field.offset - cursor);
            let has_pad = field.offset > cursor;
            let binding = format_ident!("__f{}", i);
            let read = self.zero_copy_read(ty, scope, &field_path)?;
            if has_pad {
                reads.push(quote!(r.skip(#pad)?;));
            }
            reads.push(quote!(let #binding = #read;));
            bindings.push(binding);
            cursor = field.offset + field.size;
        }
        let has_tail = layout.size > cursor;
        let tail = Literal::usize_unsuffixed(layout.size - cursor);
        if has_tail {
            reads.push(quote!(r.skip(#tail)?;));
        }
        let construct = match &def.kind {
            TypeDefKind::Struct(Fields::Named(_)) => {
                let idents = self.field_idents(def);
                quote!(Self { #(#idents: #bindings),* })
            }
            TypeDefKind::Struct(Fields::Tuple(_)) => quote!(Self(#(#bindings),*)),
            _ => quote!(Self),
        };
        let size = Literal::usize_unsuffixed(layout.size);
        let align = Literal::usize_unsuffixed(layout.align);
        let size_doc = docs(&[format!(
            "Size in bytes of the zero-copy memory layout of `{}`, discriminator excluded.",
            def.name
        )]);

        Ok(quote! {
            impl #ident {
                #size_doc
                pub const SIZE: usize = #size;
                /// Alignment of the zero-copy memory layout on Solana's SBF target.
                pub const ALIGN: usize = #align;

                /// Reads the raw memory layout, padding included, at the cursor.
                pub fn zero_copy_read(r: &mut #b::BorshReader<'_>) -> #ok {
                    #(#reads)*
                    ::core::result::Result::Ok(#construct)
                }

                /// Decodes the raw memory layout from the start of `data`, ignoring trailing bytes.
                pub fn decode_zero_copy(data: &[u8]) -> #ok {
                    if data.len() < Self::SIZE {
                        return ::core::result::Result::Err(#b::DecodeError::UnexpectedEof {
                            offset: 0,
                            needed: Self::SIZE,
                            remaining: data.len(),
                        });
                    }
                    Self::zero_copy_read(&mut #b::BorshReader::new(data))
                }

            }
        })
    }

    /// The field identifiers of a struct with named fields, as declared.
    fn field_idents(&self, def: &TypeDef) -> Vec<Ident> {
        let mut namespace = names::Namespace::new();
        match &def.kind {
            TypeDefKind::Struct(Fields::Named(fields)) => {
                fields.iter().map(|f| namespace.snake(&f.name)).collect()
            }
            _ => Vec::new(),
        }
    }

    /// Follows aliases to the type they name.
    fn resolve_alias<'t>(&'t self, ty: &'t IdlType, path: &str) -> Result<&'t IdlType> {
        let mut ty = ty;
        for _ in 0..64 {
            match ty {
                IdlType::Defined { name, .. } => match &self.type_def(name, path)?.kind {
                    TypeDefKind::Alias(target) => ty = target,
                    _ => return Ok(ty),
                },
                _ => return Ok(ty),
            }
        }
        error(path, "alias chain too deep")
    }

    /// Whether the raw memory bytes of `ty` decode as its Borsh encoding.
    fn same_as_borsh(&self, ty: &IdlType, path: &str) -> Result<bool> {
        Ok(match self.resolve_alias(ty, path)? {
            IdlType::Bool
            | IdlType::U8
            | IdlType::I8
            | IdlType::U16
            | IdlType::I16
            | IdlType::U32
            | IdlType::I32
            | IdlType::U64
            | IdlType::I64
            | IdlType::U128
            | IdlType::I128
            | IdlType::Pubkey => true,
            IdlType::Array(element, _) => self.same_as_borsh(element, path)?,
            _ => false,
        })
    }

    /// An expression reading `ty` from its raw memory layout at `r`.
    fn zero_copy_read(&self, ty: &IdlType, scope: Scope, path: &str) -> Result<TokenStream> {
        if self.same_as_borsh(ty, path)? {
            return Ok(quote!(r.read()?));
        }
        let b = borsh();
        Ok(match self.resolve_alias(ty, path)? {
            // Raw memory may hold NaN, which Borsh decoding rejects.
            IdlType::F32 => quote!(f32::from_le_bytes(*r.read_array::<4>()?)),
            IdlType::F64 => quote!(f64::from_le_bytes(*r.read_array::<8>()?)),
            IdlType::Defined { name, .. } => {
                let path = self.type_path(name, scope, path)?;
                quote!(#path::zero_copy_read(r)?)
            }
            IdlType::Array(element, ArrayLen::Value(len)) => {
                let len = Literal::usize_unsuffixed(*len);
                let element = self.zero_copy_read(element, scope, path)?;
                quote!(#b::try_array::<_, #len>(|| ::core::result::Result::Ok(#element))?)
            }
            _ => return error(path, "type has no zero-copy layout"),
        })
    }
}
