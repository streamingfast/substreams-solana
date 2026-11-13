//! Reads an Anchor IDL file into a normalized model.
//!
//! Accepts the Anchor 0.30+ format (`metadata.spec` is `0.1.0`) and the legacy
//! format of Anchor 0.29 and earlier, including Shank IDLs. Legacy IDLs are
//! normalized like `anchor_lang_idl::convert::convert_idl`: snake_case
//! instruction, argument, account and field names, and computed
//! [`discriminator`]s. Unknown keys and types are an [`IdlError`] with the JSON
//! path; only `metadata` is read leniently.
//!
//! Legacy event layouts are not added to [`Idl::types`]: a legacy event may
//! share its name with an unrelated type (Jupiter v4 `Swap`).

pub mod discriminator;
mod error;
mod parse;
mod resolve;

use std::path::Path;

pub use error::IdlError;

/// A program's IDL, normalized from either input format.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Idl {
    /// The base58 program address, when the IDL carries one.
    pub address: Option<String>,
    /// The program name, as written (usually snake_case).
    pub name: String,
    /// The program version, when the IDL carries one.
    pub version: Option<String>,
    /// The program description, from `metadata.description`.
    pub description: Option<String>,
    /// Which input format the IDL was read from.
    pub format: IdlFormat,
    /// Which tool produced the IDL.
    pub origin: IdlOrigin,
    pub docs: Vec<String>,
    pub instructions: Vec<Instruction>,
    /// Accounts the program owns.
    pub accounts: Vec<Account>,
    /// Events the program emits.
    pub events: Vec<Event>,
    /// Types referred to by [`IdlType::Defined`]. Includes account layouts, and
    /// event layouts only in the 0.30 format.
    pub types: Vec<TypeDef>,
    pub errors: Vec<ErrorCode>,
    pub constants: Vec<Constant>,
}

/// The input format an IDL was read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdlFormat {
    /// Anchor 0.29 and earlier, without `metadata.spec`.
    Legacy,
    /// Anchor 0.30 and later, with the `metadata.spec` version.
    Spec(String),
}

/// The tool that produced an IDL, from `metadata.origin`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdlOrigin {
    Anchor,
    /// A Shank IDL. Instructions use their `discriminant` value; accounts and
    /// events have no discriminator.
    Shank,
}

/// An instruction the program accepts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    /// The instruction name in snake_case, as the Rust handler is named.
    pub name: String,
    pub docs: Vec<String>,
    /// The bytes the instruction data begins with.
    pub discriminator: Vec<u8>,
    /// The accounts, in order, with composite groups flattened in place.
    pub accounts: Vec<InstructionAccount>,
    /// The instruction arguments, in the order they are serialized.
    pub args: Vec<Field>,
    /// The type the instruction returns, if any.
    pub returns: Option<IdlType>,
    /// Shank's `legacyOptionalAccountsStrategy`: an absent optional account is left out of the
    /// list instead of passed as the program id.
    pub legacy_optional_accounts_strategy: bool,
}

/// One account an instruction takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstructionAccount {
    /// The account name in snake_case.
    pub name: String,
    /// The enclosing composite group names, outermost first.
    pub groups: Vec<String>,
    pub docs: Vec<String>,
    pub writable: bool,
    pub signer: bool,
    /// Shank's `isOptionalSigner`: the account may sign but does not have to.
    pub optional_signer: bool,
    /// Whether the account is optional. Anchor passes the program ID for a
    /// missing optional account.
    pub optional: bool,
    /// The account address, in base58, when it is always the same account.
    pub address: Option<String>,
    /// How the account address is derived, when it is a PDA.
    pub pda: Option<Pda>,
    /// Names of other accounts this account is checked against.
    pub relations: Vec<String>,
}

impl InstructionAccount {
    /// The name prefixed with its group names, such as `pool.token_vault`.
    pub fn qualified_name(&self) -> String {
        let mut name = String::new();
        for group in &self.groups {
            name.push_str(group);
            name.push('.');
        }
        name.push_str(&self.name);
        name
    }
}

/// How a program derived address is found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pda {
    pub seeds: Vec<Seed>,
    /// The program the address is derived from, when it is not this program.
    pub program: Option<Seed>,
}

/// One seed of a program derived address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Seed {
    /// Fixed bytes.
    Const { value: Vec<u8> },
    /// The value of an instruction argument, by path (e.g. `params.amount`).
    Arg { path: String },
    /// An account address or account field, by path (e.g. `pool.mint`).
    /// `account` names the account type when the field is read from its data.
    Account {
        path: String,
        account: Option<String>,
    },
}

/// An account the program owns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub name: String,
    pub docs: Vec<String>,
    /// The bytes the account data begins with. Empty for Shank.
    pub discriminator: Vec<u8>,
    /// The layout of the data after the discriminator.
    pub layout: TypeDef,
}

/// An event the program emits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub name: String,
    /// The bytes the event data begins with. Empty for Shank.
    pub discriminator: Vec<u8>,
    /// The layout of the data after the discriminator.
    pub layout: TypeDef,
}

/// A named field, argument or variant field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    pub name: String,
    pub docs: Vec<String>,
    pub ty: IdlType,
}

/// A type the IDL defines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeDef {
    pub name: String,
    pub docs: Vec<String>,
    /// The generic parameters, in order.
    pub generics: Vec<GenericParam>,
    /// How values of the type are serialized.
    pub serialization: Serialization,
    /// The `#[repr]` of the type, when it has one.
    pub repr: Option<Repr>,
    pub kind: TypeDefKind,
}

/// What a type definition is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeDefKind {
    Struct(Fields),
    /// The variants, serialized as the variant index, then its fields.
    Enum(Vec<EnumVariant>),
    /// Another name for a type.
    Alias(IdlType),
}

/// The fields of a struct or an enum variant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fields {
    /// No fields.
    Unit,
    /// Named fields, in order. Never empty.
    Named(Vec<Field>),
    /// Unnamed fields, in order. Never empty.
    Tuple(Vec<IdlType>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnumVariant {
    pub name: String,
    pub fields: Fields,
}

/// A generic parameter of a type definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericParam {
    /// A type parameter, such as `T`.
    Type { name: String },
    /// A const parameter, such as `N: usize`. `ty` is the Rust type.
    Const { name: String, ty: String },
}

impl GenericParam {
    pub fn name(&self) -> &str {
        match self {
            GenericParam::Type { name } | GenericParam::Const { name, .. } => name,
        }
    }
}

/// How values of a type are serialized.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Serialization {
    #[default]
    Borsh,
    /// The raw memory layout, for zero-copy accounts.
    Bytemuck,
    /// The raw memory layout, for zero-copy accounts that have padding.
    BytemuckUnsafe,
    /// A serialization the IDL only names.
    Custom(String),
}

/// The `#[repr]` of a type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Repr {
    Rust(ReprModifier),
    C(ReprModifier),
    Transparent,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReprModifier {
    pub packed: bool,
    pub align: Option<usize>,
}

/// An error code the program can fail with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorCode {
    pub code: u32,
    pub name: String,
    pub msg: Option<String>,
}

/// A constant the program declares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Constant {
    pub name: String,
    pub docs: Vec<String>,
    pub ty: IdlType,
    /// The value as the IDL writes it, a Rust expression.
    pub value: String,
}

/// The type of a field, argument, alias or constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdlType {
    Bool,
    U8,
    I8,
    U16,
    I16,
    U32,
    I32,
    F32,
    U64,
    I64,
    F64,
    U128,
    I128,
    U256,
    I256,
    /// `Vec<u8>`.
    Bytes,
    String,
    Pubkey,
    /// A borsh `Option`: one tag byte, then the value when the tag is 1.
    Option(Box<IdlType>),
    /// A `COption`, as in SPL Token, whose tag is four bytes instead of one.
    COption(Box<IdlType>),
    Vec(Box<IdlType>),
    /// Shank's `hashMap` / `bTreeMap`: a `u32` length, then the key-value pairs.
    Map(Box<IdlType>, Box<IdlType>),
    /// Shank's `hashSet` / `bTreeSet`: a `u32` length, then the elements.
    Set(Box<IdlType>),
    /// Shank's `tuple`: the elements one after another.
    Tuple(Vec<IdlType>),
    Array(Box<IdlType>, ArrayLen),
    /// A type defined in [`Idl::types`], with its generic arguments.
    Defined {
        name: String,
        generics: Vec<GenericArg>,
    },
    /// A generic type parameter of the enclosing type definition.
    Generic(String),
}

/// The length of an array type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArrayLen {
    Value(usize),
    /// A const generic parameter of the enclosing type definition.
    Generic(String),
}

/// A generic argument given to a defined type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericArg {
    Type(IdlType),
    /// A literal such as `8`, or the name of an enclosing const parameter.
    Const(String),
}

impl Idl {
    /// Reads an IDL from its JSON text.
    pub fn from_json(json: &str) -> Result<Idl, IdlError> {
        let value: serde_json::Value = serde_json::from_str(json).map_err(IdlError::Json)?;
        let (mut idl, sources) = parse::parse(&value)?;
        resolve::resolve(&mut idl, &sources)?;
        Ok(idl)
    }

    /// Reads an IDL from a JSON file.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Idl, IdlError> {
        let path = path.as_ref();
        let json = std::fs::read_to_string(path).map_err(|source| IdlError::Io {
            path: path.to_path_buf(),
            source,
        })?;
        Idl::from_json(&json)
    }

    /// The type definition with the given name.
    pub fn type_def(&self, name: &str) -> Option<&TypeDef> {
        self.types.iter().find(|def| def.name == name)
    }
}
