//! The program-level items: `PROGRAM_ID`, the `instructions`, `accounts` and
//! `events` modules, and the dispatch enums.

use proc_macro2::{Ident, Literal, TokenStream};
use quote::quote;

use super::names::{self, Namespace};
use super::{borsh, bytes_literal, docs, error, result_ty, Generator, Options, Result, Scope};
use crate::idl::{Instruction, InstructionAccount, Serialization, TypeDef};

/// One item of a dispatch enum.
struct Dispatch<'a> {
    ident: Ident,
    name: &'a str,
    discriminator: &'a [u8],
}

fn idl_path() -> TokenStream {
    quote!(::substreams_solana::idl)
}

/// Sorts the items so that a longer discriminator is tried before one of its
/// prefixes.
fn dispatch_order(items: &mut [Dispatch<'_>]) {
    items.sort_by_key(|item| std::cmp::Reverse(item.discriminator.len()));
}

impl<'a> Generator<'a> {
    pub(crate) fn program(&self, options: &Options) -> Result<TokenStream> {
        let idl = self.idl;
        unique_discriminators(
            "instructions",
            idl.instructions
                .iter()
                .map(|ix| (ix.name.as_str(), ix.discriminator.as_slice())),
        )?;
        unique_discriminators(
            "accounts",
            idl.accounts
                .iter()
                .map(|a| (a.name.as_str(), a.discriminator.as_slice())),
        )?;
        unique_discriminators(
            "events",
            idl.events
                .iter()
                .map(|e| (e.name.as_str(), e.discriminator.as_slice())),
        )?;
        let program_id = self.program_id(options)?;
        let types = self.types_module()?;
        let (instructions, instruction_items) = self.instructions_module()?;
        let (accounts, account_items, skipped_accounts) = self.accounts_module()?;
        let (events, event_items, skipped_events) = self.events_module()?;
        let instruction_enum = self.instruction_enum(instruction_items);
        let account_enum = self.account_enum(account_items, &skipped_accounts);
        let event_enum = self.event_enum(event_items, &skipped_events);
        let constants = self.constants_module()?;
        let program_error = self.program_error();

        Ok(quote! {
            #program_id
            #instruction_enum
            #account_enum
            #event_enum
            #program_error
            #constants
            #types
            #instructions
            #accounts
            #events
        })
    }

    fn program_id(&self, options: &Options) -> Result<TokenStream> {
        let Some(address) = options.program_id.as_ref().or(self.idl.address.as_ref()) else {
            return error(
                "address",
                format!(
                    "the IDL of `{}` has no program address; provide one with `Idlgen::program_id`",
                    self.idl.name
                ),
            );
        };
        let bytes = bs58::decode(address)
            .into_vec()
            .ok()
            .filter(|b| b.len() == 32);
        let Some(bytes) = bytes else {
            return error(
                "address",
                format!("`{address}` is not a base58 32-byte program address"),
            );
        };
        let b = borsh();
        let bytes = bytes.iter().map(|b| Literal::u8_unsuffixed(*b));
        let doc = docs(&[format!(
            "Address of the `{}` program, `{address}`.",
            self.idl.name
        )]);
        Ok(quote! {
            #doc
            pub const PROGRAM_ID: #b::Pubkey = #b::Pubkey::new([#(#bytes),*]);
        })
    }

    fn instructions_module(&self) -> Result<(TokenStream, Vec<Dispatch<'a>>)> {
        let mut namespace = Namespace::new();
        let idents: Vec<Ident> = self
            .idl
            .instructions
            .iter()
            .map(|ix| namespace.pascal(&ix.name))
            .collect();
        let accounts_idents: Vec<Ident> = self
            .idl
            .instructions
            .iter()
            .map(|ix| namespace.unique(format!("{}Accounts", names::pascal(&ix.name))))
            .collect();

        let mut items = Vec::new();
        let mut dispatch = Vec::new();
        for ((ix, ident), accounts_ident) in self
            .idl
            .instructions
            .iter()
            .zip(idents)
            .zip(accounts_idents)
        {
            items.push(self.instruction(ix, &ident, &accounts_ident)?);
            dispatch.push(Dispatch {
                ident,
                name: &ix.name,
                discriminator: &ix.discriminator,
            });
        }
        Ok((
            quote! {
                /// The instructions the program accepts.
                pub mod instructions {
                    #![allow(non_camel_case_types, non_snake_case, unused_imports, unused_variables, clippy::all)]
                    #(#items)*
                }
            },
            dispatch,
        ))
    }

    fn instruction(
        &self,
        ix: &Instruction,
        ident: &Ident,
        accounts_ident: &Ident,
    ) -> Result<TokenStream> {
        let b = borsh();
        let idl = idl_path();
        let path = format!("instructions.{}", ix.name);
        let ok = result_ty(quote!(Self));

        let mut fields = Namespace::new();
        fields.reserve("accounts");
        fields.reserve("account_keys");
        let mut arg_decls = Vec::new();
        let mut arg_idents = Vec::new();
        for arg in &ix.args {
            let arg_ident = fields.snake(&arg.name);
            let ty = self.rust_type(
                &arg.ty,
                Scope::Sibling,
                &format!("{path}.args.{}", arg.name),
            )?;
            let doc = docs(&arg.docs);
            arg_decls.push(quote!(#doc pub #arg_ident: #ty));
            arg_idents.push(arg_ident);
        }

        let mut account_fields = Namespace::new();
        let mut account_decls = Vec::new();
        let mut account_decodes = Vec::new();
        let mut optional_flags = Vec::new();
        for (index, account) in ix.accounts.iter().enumerate() {
            let index = Literal::usize_unsuffixed(index);
            let account_ident = account_fields.snake(&account.qualified_name().replace('.', "_"));
            let doc = docs(&account_docs(account));
            optional_flags.push(account.optional);
            if account.optional {
                account_decls
                    .push(quote!(#doc pub #account_ident: ::core::option::Option<#b::Pubkey>));
                account_decodes.push(quote!(#account_ident: named.optional(#index)?));
            } else {
                account_decls.push(quote!(#doc pub #account_ident: #b::Pubkey));
                account_decodes.push(quote!(#account_ident: named.required(#index)?));
            }
        }

        let mut ix_docs = ix.docs.clone();
        if let Some(returns) = &ix.returns {
            let returns = self.rust_type(returns, Scope::Sibling, &format!("{path}.returns"))?;
            ix_docs.push(String::new());
            ix_docs.push(format!(
                "Returns `{}`.",
                returns.to_string().replace(' ', "")
            ));
        }
        let doc = docs(&ix_docs);
        let discriminator = bytes_literal(&ix.discriminator);
        let accounts_doc = docs(&[
            format!("The accounts of [{ident}], in IDL order."),
            "An optional account is [None] when given as the program id, or left out of a shorter \
             list (later accounts then shift down)."
                .to_string(),
        ]);

        Ok(quote! {
            #doc
            #[derive(Debug, Clone, PartialEq)]
            pub struct #ident {
                #(#arg_decls,)*
                /// The named accounts, or why they cannot be named (too few accounts, or an
                /// ambiguous optional omission). The arguments decode either way.
                pub accounts: ::core::result::Result<#accounts_ident, #b::DecodeError>,
                /// Every account address the instruction names, in order.
                pub account_keys: ::std::vec::Vec<#b::Pubkey>,
            }

            #accounts_doc
            #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
            pub struct #accounts_ident {
                #(#account_decls,)*
            }

            impl #accounts_ident {
                /// Whether each listed account is optional, in IDL order.
                pub const OPTIONAL: &'static [bool] = &[#(#optional_flags),*];

                /// Names the accounts of `ix`, ignoring extra accounts.
                pub fn decode<I: #idl::InstructionInput + ?Sized>(ix: &I) -> #ok {
                    let named = #idl::NamedAccounts::resolve(ix, Self::OPTIONAL)?;
                    ::core::result::Result::Ok(Self { #(#account_decodes,)* })
                }
            }

            impl #idl::IdlInstruction for #ident {
                const PROGRAM_ID: #b::Pubkey = super::PROGRAM_ID;
                const DISCRIMINATOR: &'static [u8] = #discriminator;

                fn decode_instruction<I: #idl::InstructionInput + ?Sized>(
                    r: &mut #b::BorshReader<'_>,
                    ix: &I,
                ) -> #ok {
                    ::core::result::Result::Ok(Self {
                        #(#arg_idents: r.read()?,)*
                        accounts: #accounts_ident::decode(ix),
                        account_keys: #idl::InstructionInput::account_keys(ix)?,
                    })
                }
            }
        })
    }

    /// The `accounts` module, the dispatchable accounts and the names of the
    /// accounts left out of dispatch.
    fn accounts_module(&self) -> Result<(TokenStream, Vec<Dispatch<'a>>, Vec<&'a str>)> {
        let b = borsh();
        let idl = idl_path();
        let mut namespace = Namespace::new();
        let mut items = Vec::new();
        let mut dispatch = Vec::new();
        let mut skipped = Vec::new();
        for account in &self.idl.accounts {
            let ident = namespace.pascal(&account.name);
            let path = format!("accounts.{}", account.name);
            let zero_copy = account.layout.serialization != Serialization::Borsh;
            items.push(self.layout_item(&account.layout, &ident, &path, zero_copy)?);

            let discriminator = bytes_literal(&account.discriminator);
            let decode = if zero_copy {
                quote! {
                    /// Decodes the discriminator followed by the raw memory layout.
                    fn decode_account(data: &[u8]) -> ::core::result::Result<Self, #b::DecodeError> {
                        let mut r = #b::BorshReader::new(data);
                        r.expect_discriminator(<Self as #idl::IdlAccount>::DISCRIMINATOR)?;
                        Self::decode_zero_copy(r.remaining())
                    }
                }
            } else {
                quote!()
            };
            items.push(quote! {
                impl #idl::IdlAccount for #ident {
                    const DISCRIMINATOR: &'static [u8] = #discriminator;
                    #decode
                }
            });

            if account.discriminator.is_empty() {
                skipped.push(account.name.as_str());
            } else {
                dispatch.push(Dispatch {
                    ident,
                    name: &account.name,
                    discriminator: &account.discriminator,
                });
            }
        }
        Ok((
            quote! {
                /// The accounts the program owns.
                pub mod accounts {
                    #![allow(non_camel_case_types, non_snake_case, unused_imports, unused_variables, clippy::all)]
                    #(#items)*
                }
            },
            dispatch,
            skipped,
        ))
    }

    /// The `events` module, the dispatchable events and the names of the
    /// events left out of dispatch.
    fn events_module(&self) -> Result<(TokenStream, Vec<Dispatch<'a>>, Vec<&'a str>)> {
        let b = borsh();
        let idl = idl_path();
        let mut namespace = Namespace::new();
        let mut items = Vec::new();
        let mut dispatch = Vec::new();
        let mut skipped = Vec::new();
        for event in &self.idl.events {
            let ident = namespace.pascal(&event.name);
            let path = format!("events.{}", event.name);
            items.push(self.layout_item(&event.layout, &ident, &path, false)?);
            let discriminator = bytes_literal(&event.discriminator);
            items.push(quote! {
                impl #idl::IdlEvent for #ident {
                    const PROGRAM_ID: #b::Pubkey = super::PROGRAM_ID;
                    const DISCRIMINATOR: &'static [u8] = #discriminator;
                }
            });

            if event.discriminator.is_empty() {
                skipped.push(event.name.as_str());
            } else {
                dispatch.push(Dispatch {
                    ident,
                    name: &event.name,
                    discriminator: &event.discriminator,
                });
            }
        }
        Ok((
            quote! {
                /// The events the program emits.
                pub mod events {
                    #![allow(non_camel_case_types, non_snake_case, unused_imports, unused_variables, clippy::all)]
                    #(#items)*
                }
            },
            dispatch,
            skipped,
        ))
    }

    /// The type of an account or event: a re-export from `types` when it is
    /// declared there, a new definition otherwise (legacy events).
    fn layout_item(
        &self,
        layout: &TypeDef,
        ident: &Ident,
        path: &str,
        zero_copy: bool,
    ) -> Result<TokenStream> {
        if self.idl.type_def(&layout.name) == Some(layout) {
            let type_ident = self.type_ident(&layout.name);
            return Ok(if type_ident == ident {
                quote!(pub use super::types::#type_ident;)
            } else {
                quote!(pub use super::types::#type_ident as #ident;)
            });
        }
        if zero_copy {
            return error(path, "a zero-copy layout must be declared in `types`");
        }
        self.type_def_items(layout, ident, Scope::Sibling, path, false)
    }

    fn instruction_enum(&self, mut items: Vec<Dispatch<'_>>) -> TokenStream {
        let idl = idl_path();
        let ok = result_ty(quote!(Self));
        let variants: Vec<TokenStream> = items
            .iter()
            .map(|item| {
                let ident = &item.ident;
                quote!(#ident(instructions::#ident))
            })
            .collect();
        let names = name_arms(&items);
        let b = borsh();
        let (account_keys, accounts_error) = if items.is_empty() {
            (quote!(match *self {}), quote!(match *self {}))
        } else {
            let idents: Vec<&Ident> = items.iter().map(|item| &item.ident).collect();
            (
                quote!(match self { #(Self::#idents(value) => &value.account_keys,)* }),
                quote!(match self { #(Self::#idents(value) => value.accounts.as_ref().err(),)* }),
            )
        };
        dispatch_order(&mut items);
        let decodes = items.iter().map(|item| {
            let ident = &item.ident;
            quote! {
                if data.starts_with(<instructions::#ident as #idl::IdlInstruction>::DISCRIMINATOR) {
                    return ::core::option::Option::Some(
                        <instructions::#ident as #idl::IdlInstruction>::decode(ix).map(Self::#ident),
                    );
                }
            }
        });

        quote! {
            /// Any instruction of the program.
            #[derive(Debug, Clone, PartialEq)]
            #[allow(clippy::large_enum_variant)]
            pub enum Instruction {
                #(#variants,)*
            }

            impl Instruction {
                /// Decodes `ix` if it targets [PROGRAM_ID] with a known discriminator, [None]
                /// otherwise.
                pub fn match_and_decode<I: #idl::InstructionInput + ?Sized>(
                    ix: &I,
                ) -> ::core::option::Option<#ok> {
                    if PROGRAM_ID != #idl::InstructionInput::program_id(ix) {
                        return ::core::option::Option::None;
                    }
                    Self::decode(ix)
                }

                /// Decodes `ix` by discriminator, [None] when none matches. Does not check the
                /// program id, so it also decodes the program deployed at another address.
                pub fn decode<I: #idl::InstructionInput + ?Sized>(
                    ix: &I,
                ) -> ::core::option::Option<#ok> {
                    let data = #idl::InstructionInput::data(ix);
                    #(#decodes)*
                    let _ = data;
                    ::core::option::Option::None
                }

                /// The instruction name, as in the IDL.
                pub fn name(&self) -> &'static str {
                    #names
                }

                /// Every account address the instruction names, in order.
                pub fn account_keys(&self) -> &[#b::Pubkey] {
                    #account_keys
                }

                /// Why the accounts could not be named, [None] when they were.
                pub fn accounts_error(&self) -> ::core::option::Option<&#b::DecodeError> {
                    #accounts_error
                }
            }
        }
    }

    fn account_enum(&self, mut items: Vec<Dispatch<'_>>, skipped: &[&str]) -> TokenStream {
        let idl = idl_path();
        let ok = result_ty(quote!(Self));
        let variants: Vec<TokenStream> = items
            .iter()
            .map(|item| {
                let ident = &item.ident;
                quote!(#ident(accounts::#ident))
            })
            .collect();
        let names = name_arms(&items);
        dispatch_order(&mut items);
        let decodes = items.iter().map(|item| {
            let ident = &item.ident;
            quote! {
                if data.starts_with(<accounts::#ident as #idl::IdlAccount>::DISCRIMINATOR) {
                    return ::core::option::Option::Some(
                        <accounts::#ident as #idl::IdlAccount>::decode_account(data).map(Self::#ident),
                    );
                }
            }
        });
        let mut doc = vec!["Any account of the program.".to_string()];
        skipped_doc(&mut doc, "accounts", skipped);
        let doc = docs(&doc);

        quote! {
            #doc
            #[derive(Debug, Clone, PartialEq)]
            #[allow(clippy::large_enum_variant)]
            pub enum Account {
                #(#variants,)*
            }

            impl Account {
                /// Decodes account data by discriminator, [None] when none matches. Does not
                /// check the account owner.
                pub fn decode(data: &[u8]) -> ::core::option::Option<#ok> {
                    #(#decodes)*
                    let _ = data;
                    ::core::option::Option::None
                }

                /// The account type name, as in the IDL.
                pub fn name(&self) -> &'static str {
                    #names
                }
            }
        }
    }

    fn event_enum(&self, mut items: Vec<Dispatch<'_>>, skipped: &[&str]) -> TokenStream {
        let idl = idl_path();
        let ok = result_ty(quote!(Self));
        let variants: Vec<TokenStream> = items
            .iter()
            .map(|item| {
                let ident = &item.ident;
                quote!(#ident(events::#ident))
            })
            .collect();
        let names = name_arms(&items);
        dispatch_order(&mut items);
        let decodes = items.iter().map(|item| {
            let ident = &item.ident;
            quote! {
                if data.starts_with(<events::#ident as #idl::IdlEvent>::DISCRIMINATOR) {
                    return ::core::option::Option::Some(
                        <events::#ident as #idl::IdlEvent>::decode_log_data(data).map(Self::#ident),
                    );
                }
            }
        });
        let mut doc = vec!["Any event of the program.".to_string()];
        skipped_doc(&mut doc, "events", skipped);
        let doc = docs(&doc);

        quote! {
            #doc
            #[derive(Debug, Clone, PartialEq)]
            #[allow(clippy::large_enum_variant)]
            pub enum Event {
                #(#variants,)*
            }

            impl Event {
                /// Decodes a `Program data:` payload by discriminator, [None] when none matches.
                pub fn decode_log_data(data: &[u8]) -> ::core::option::Option<#ok> {
                    #(#decodes)*
                    let _ = data;
                    ::core::option::Option::None
                }

                /// Decodes self-CPI event instruction data (`emit_cpi!`), [None] when it is not
                /// one or no event matches. Does not check the program id.
                pub fn decode_cpi_data(data: &[u8]) -> ::core::option::Option<#ok> {
                    Self::decode_log_data(data.strip_prefix(#idl::EVENT_IX_TAG_LE.as_slice())?)
                }

                /// Decodes `ix` if it is a self-CPI event instruction of [PROGRAM_ID], [None]
                /// otherwise.
                pub fn match_and_decode_cpi<I: #idl::InstructionInput + ?Sized>(
                    ix: &I,
                ) -> ::core::option::Option<#ok> {
                    if PROGRAM_ID != #idl::InstructionInput::program_id(ix) {
                        return ::core::option::Option::None;
                    }
                    Self::decode_cpi_data(#idl::InstructionInput::data(ix))
                }

                /// The events [PROGRAM_ID] logged in a transaction's log messages, see
                /// [program_data](::substreams_solana::idl::program_data).
                pub fn decode_logs<L>(logs: L) -> impl ::core::iter::Iterator<Item = #ok>
                where
                    L: ::core::iter::IntoIterator,
                    L::Item: ::core::convert::AsRef<str>,
                {
                    #idl::program_data(logs, PROGRAM_ID.as_bytes())
                        .filter_map(|data| Self::decode_log_data(&data))
                }

                /// The event name, as in the IDL.
                pub fn name(&self) -> &'static str {
                    #names
                }
            }
        }
    }
}

fn skipped_doc(doc: &mut Vec<String>, kind: &str, skipped: &[&str]) {
    if skipped.is_empty() {
        return;
    }
    let list = skipped
        .iter()
        .map(|name| format!("`{name}`"))
        .collect::<Vec<_>>()
        .join(", ");
    doc.push(String::new());
    doc.push(format!(
        "Not dispatched, as they have no discriminator: {kind} {list}. Decode them through \
         their own type."
    ));
}

/// The body of `name()` of a dispatch enum.
fn name_arms(items: &[Dispatch<'_>]) -> TokenStream {
    if items.is_empty() {
        return quote!(match *self {});
    }
    let idents: Vec<&Ident> = items.iter().map(|item| &item.ident).collect();
    let names = items.iter().map(|item| item.name);
    quote!(match self { #(Self::#idents(_) => #names,)* })
}

fn account_docs(account: &InstructionAccount) -> Vec<String> {
    let mut flags = Vec::new();
    if account.writable {
        flags.push("writable");
    }
    if account.signer {
        flags.push("signer");
    }
    if account.optional {
        flags.push("optional");
    }
    let mut doc = account.docs.clone();
    let mut summary = format!("`{}`", account.qualified_name());
    if !flags.is_empty() {
        summary.push_str(&format!(" ({})", flags.join(", ")));
    }
    if let Some(address) = &account.address {
        summary.push_str(&format!(", always `{address}`"));
    }
    summary.push('.');
    if !doc.is_empty() {
        doc.push(String::new());
    }
    doc.push(summary);
    doc
}

/// Fails when two items share a discriminator, which dispatch could not tell apart. Empty
/// discriminators (Shank accounts and events) are never dispatched and are skipped.
fn unique_discriminators<'a>(
    kind: &str,
    items: impl Iterator<Item = (&'a str, &'a [u8])>,
) -> Result<()> {
    let mut seen: Vec<(&str, &[u8])> = Vec::new();
    for (name, discriminator) in items.filter(|(_, d)| !d.is_empty()) {
        if let Some((other, _)) = seen.iter().find(|(_, d)| *d == discriminator) {
            return error(
                format!("{kind}.{name}"),
                format!("has the same discriminator as `{other}`"),
            );
        }
        seen.push((name, discriminator));
    }
    Ok(())
}
