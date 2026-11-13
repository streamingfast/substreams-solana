//! Reads the IDL JSON into the model, for both input formats.
//!
//! Every reader takes the JSON path of its value so errors can name it.
//! Unknown keys are errors.

use serde_json::{Map, Value};

use super::discriminator::{self, legacy_snake_case};
use super::*;

type Result<T> = std::result::Result<T, IdlError>;
type Object = Map<String, Value>;

/// The input format, and for the legacy format, whether it comes from Shank.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Format {
    Spec,
    Legacy,
    Shank,
}

impl Format {
    fn is_legacy(self) -> bool {
        matches!(self, Format::Legacy | Format::Shank)
    }

    /// Legacy names are camelCase. Anchor's converter turns them to snake_case.
    fn field_name(self, name: String) -> String {
        if self.is_legacy() {
            legacy_snake_case(&name)
        } else {
            name
        }
    }
}

/// The only 0.30+ spec version that exists.
const SPEC_VERSION: &str = "0.1.0";

/// Where each type definition was read from, for `resolve` error paths.
pub(super) struct Sources {
    pub types: Vec<TypeDefSource>,
    pub accounts: Vec<TypeDefSource>,
    pub events: Vec<TypeDefSource>,
}

/// Where a type definition was read from.
#[derive(Clone)]
pub(super) struct TypeDefSource {
    pub path: String,
    pub shape: Shape,
}

/// The JSON shape a type definition was read from.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Shape {
    /// A 0.30 type definition.
    Spec,
    /// A legacy type or account definition.
    Legacy,
    /// A legacy event, whose fields are inline.
    LegacyEvent,
}

fn sources(key: &str, count: usize, shape: Shape) -> impl Iterator<Item = TypeDefSource> + '_ {
    (0..count).map(move |i| TypeDefSource {
        path: index(key, i),
        shape,
    })
}

pub(super) fn parse(value: &Value) -> Result<(Idl, Sources)> {
    let root = object(value, "")?;
    if root.get("standard").and_then(Value::as_str) == Some("codama") {
        return Err(IdlError::invalid(
            "standard",
            "Codama IDLs are not supported, use the program's Anchor or Shank IDL",
        ));
    }
    let metadata = match root.get("metadata") {
        None => None,
        Some(value) => Some(object(value, "metadata")?),
    };
    let spec = match metadata.and_then(|m| m.get("spec")) {
        None => None,
        Some(value) => Some(string(value, "metadata.spec")?),
    };
    let origin = match metadata.and_then(|m| m.get("origin")) {
        None => IdlOrigin::Anchor,
        Some(value) => match string(value, "metadata.origin")?.as_str() {
            "anchor" => IdlOrigin::Anchor,
            "shank" => IdlOrigin::Shank,
            other => {
                return Err(IdlError::invalid(
                    "metadata.origin",
                    format!("unsupported IDL origin `{other}`"),
                ))
            }
        },
    };

    match spec {
        Some(spec) if spec != SPEC_VERSION => Err(IdlError::invalid(
            "metadata.spec",
            format!("unsupported IDL spec `{spec}`, only `{SPEC_VERSION}` is supported"),
        )),
        Some(spec) if origin == IdlOrigin::Shank => Err(IdlError::invalid(
            "metadata.spec",
            format!("a Shank IDL with spec `{spec}` is not supported"),
        )),
        Some(spec) => parse_spec(root, metadata.expect("spec is read from metadata"), spec),
        None => {
            let format = match origin {
                IdlOrigin::Anchor => Format::Legacy,
                IdlOrigin::Shank => Format::Shank,
            };
            parse_legacy(root, metadata, format, origin)
        }
    }
}

fn parse_spec(root: &Object, metadata: &Object, spec: String) -> Result<(Idl, Sources)> {
    check_keys(
        root,
        "",
        &[
            "address",
            "metadata",
            "docs",
            "instructions",
            "accounts",
            "events",
            "errors",
            "types",
            "constants",
        ],
    )?;
    let format = Format::Spec;

    let address = match root.get("address") {
        None => None,
        Some(value) => Some(address(value, "address")?),
    };
    let name = string(required(metadata, "metadata", "name")?, "metadata.name")?;
    let version = optional_string(metadata, "metadata", "version")?;
    let description = optional_string(metadata, "metadata", "description")?;

    let instructions = list(root, "", "instructions", true, |value, path| {
        parse_instruction(value, &path, format)
    })?;
    let types = list(root, "", "types", false, |value, path| {
        parse_type_def(value, &path, format)
    })?;
    let type_sources: Vec<TypeDefSource> = sources("types", types.len(), Shape::Spec).collect();

    // Accounts and events name their layout, a type of the same name.
    let layout = |name: &str, path: &str, what: &str| -> Result<(TypeDef, TypeDefSource)> {
        let i = types
            .iter()
            .position(|def| def.name == name)
            .ok_or_else(|| {
                IdlError::invalid(
                    path,
                    format!("{what} `{name}` has no type definition of the same name"),
                )
            })?;
        Ok((types[i].clone(), type_sources[i].clone()))
    };
    let mut account_sources = Vec::new();
    let accounts = list(root, "", "accounts", false, |value, path| {
        let obj = object(value, &path)?;
        check_keys(obj, &path, &["name", "docs", "discriminator"])?;
        let name = required_string(obj, &path, "name")?;
        let (layout, source) = layout(&name, &path, "account")?;
        account_sources.push(source);
        Ok(Account {
            name,
            docs: docs(obj, &path)?,
            discriminator: discriminator_bytes(
                required(obj, &path, "discriminator")?,
                &join(&path, "discriminator"),
            )?,
            layout,
        })
    })?;
    let mut event_sources = Vec::new();
    let events = list(root, "", "events", false, |value, path| {
        let obj = object(value, &path)?;
        check_keys(obj, &path, &["name", "discriminator"])?;
        let name = required_string(obj, &path, "name")?;
        let (layout, source) = layout(&name, &path, "event")?;
        event_sources.push(source);
        Ok(Event {
            name,
            discriminator: discriminator_bytes(
                required(obj, &path, "discriminator")?,
                &join(&path, "discriminator"),
            )?,
            layout,
        })
    })?;

    let sources = Sources {
        types: type_sources,
        accounts: account_sources,
        events: event_sources,
    };
    let idl = Idl {
        address,
        name,
        version,
        description,
        format: IdlFormat::Spec(spec),
        origin: IdlOrigin::Anchor,
        docs: docs(root, "")?,
        instructions,
        accounts,
        events,
        types,
        errors: list(root, "", "errors", false, parse_error_code)?,
        constants: list(root, "", "constants", false, |value, path| {
            parse_constant(value, &path, format)
        })?,
    };
    Ok((idl, sources))
}

fn parse_legacy(
    root: &Object,
    metadata: Option<&Object>,
    format: Format,
    origin: IdlOrigin,
) -> Result<(Idl, Sources)> {
    check_keys(
        root,
        "",
        &[
            "version",
            "name",
            "docs",
            "constants",
            "instructions",
            "accounts",
            "types",
            "events",
            "errors",
            "metadata",
        ],
    )?;

    let address = match metadata.and_then(|m| m.get("address")) {
        None => None,
        Some(value) => Some(address(value, "metadata.address")?),
    };
    let description = match metadata {
        None => None,
        Some(metadata) => optional_string(metadata, "metadata", "description")?,
    };

    let instructions = list(root, "", "instructions", true, |value, path| {
        parse_instruction(value, &path, format)
    })?;
    let mut types = list(root, "", "types", false, |value, path| {
        parse_type_def(value, &path, format)
    })?;
    let mut type_sources: Vec<TypeDefSource> =
        sources("types", types.len(), Shape::Legacy).collect();

    // Legacy account layouts are inline; also add them to the types, as
    // Anchor's converter does, since other types may refer to them.
    let account_defs = list(root, "", "accounts", false, |value, path| {
        parse_type_def(value, &path, format)
    })?;
    let mut accounts = Vec::with_capacity(account_defs.len());
    let account_sources: Vec<TypeDefSource> =
        self::sources("accounts", account_defs.len(), Shape::Legacy).collect();
    for (def, source) in account_defs.into_iter().zip(&account_sources) {
        match types.iter().find(|existing| existing.name == def.name) {
            Some(existing) if *existing == def => {}
            Some(_) => {
                return Err(IdlError::invalid(
                    &source.path,
                    format!(
                        "account `{}` differs from the type of the same name",
                        def.name
                    ),
                ))
            }
            None => {
                types.push(def.clone());
                type_sources.push(source.clone());
            }
        }
        accounts.push(Account {
            name: def.name.clone(),
            docs: def.docs.clone(),
            discriminator: match format {
                Format::Shank => Vec::new(),
                _ => discriminator::account(&def.name).to_vec(),
            },
            layout: def,
        });
    }

    // Legacy event layouts are not added to the types: an event may share its
    // name with an unrelated type.
    let event_defs = list(root, "", "events", false, |value, path| {
        parse_legacy_event(value, &path, format)
    })?;
    let event_sources = self::sources("events", event_defs.len(), Shape::LegacyEvent).collect();
    let events = event_defs
        .into_iter()
        .map(|def| Event {
            name: def.name.clone(),
            discriminator: match format {
                Format::Shank => Vec::new(),
                _ => discriminator::event(&def.name).to_vec(),
            },
            layout: def,
        })
        .collect();

    let sources = Sources {
        types: type_sources,
        accounts: account_sources,
        events: event_sources,
    };
    let idl = Idl {
        address,
        name: required_string(root, "", "name")?,
        version: optional_string(root, "", "version")?,
        description,
        format: IdlFormat::Legacy,
        origin,
        docs: docs(root, "")?,
        instructions,
        accounts,
        events,
        types,
        errors: list(root, "", "errors", false, parse_error_code)?,
        constants: list(root, "", "constants", false, |value, path| {
            parse_constant(value, &path, format)
        })?,
    };
    Ok((idl, sources))
}

fn parse_instruction(value: &Value, path: &str, format: Format) -> Result<Instruction> {
    let obj = object(value, path)?;
    let keys: &[&str] = match format {
        Format::Spec => &[
            "name",
            "docs",
            "discriminator",
            "accounts",
            "args",
            "returns",
        ],
        Format::Legacy => &["name", "docs", "accounts", "args", "returns"],
        Format::Shank => &[
            "name",
            "docs",
            "accounts",
            "args",
            "returns",
            "discriminant",
            "legacyOptionalAccountsStrategy",
        ],
    };
    check_keys(obj, path, keys)?;

    let name = format.field_name(required_string(obj, path, "name")?);
    let discriminator = match format {
        Format::Spec => discriminator_bytes(
            required(obj, path, "discriminator")?,
            &join(path, "discriminator"),
        )?,
        Format::Legacy => discriminator::instruction(&name).to_vec(),
        Format::Shank => shank_discriminant(
            required(obj, path, "discriminant")?,
            &join(path, "discriminant"),
        )?,
    };

    let mut accounts = Vec::new();
    let accounts_path = join(path, "accounts");
    for (i, item) in array(required(obj, path, "accounts")?, &accounts_path)?
        .iter()
        .enumerate()
    {
        parse_account_item(item, &index(&accounts_path, i), format, &[], &mut accounts)?;
    }

    Ok(Instruction {
        name,
        docs: docs(obj, path)?,
        discriminator,
        accounts,
        args: list(obj, path, "args", true, |value, path| {
            parse_field(value, &path, format)
        })?,
        returns: match obj.get("returns") {
            None | Some(Value::Null) => None,
            Some(value) => Some(parse_type(value, &join(path, "returns"))?),
        },
        legacy_optional_accounts_strategy: optional_bool(
            obj,
            path,
            "legacyOptionalAccountsStrategy",
        )?,
    })
}

/// Reads one entry of an instruction's accounts. A composite group is
/// flattened into `out` in place, its accounts carrying the group names.
fn parse_account_item(
    value: &Value,
    path: &str,
    format: Format,
    groups: &[String],
    out: &mut Vec<InstructionAccount>,
) -> Result<()> {
    let obj = object(value, path)?;
    let name = format.field_name(required_string(obj, path, "name")?);

    if obj.contains_key("accounts") {
        check_keys(obj, path, &["name", "accounts"])?;
        let mut inner_groups = groups.to_vec();
        inner_groups.push(name);
        let accounts_path = join(path, "accounts");
        for (i, item) in array(required(obj, path, "accounts")?, &accounts_path)?
            .iter()
            .enumerate()
        {
            parse_account_item(item, &index(&accounts_path, i), format, &inner_groups, out)?;
        }
        return Ok(());
    }

    let mut docs = docs(obj, path)?;
    let (writable, signer, optional_signer, optional, address) = if format.is_legacy() {
        check_keys(
            obj,
            path,
            &[
                "name",
                "docs",
                "desc",
                "isMut",
                "isSigner",
                "isOptionalSigner",
                "isOptional",
                "pda",
                "relations",
            ],
        )?;
        // Shank writes a one-line description as `desc`.
        if let Some(desc) = optional_string(obj, path, "desc")? {
            docs.push(desc);
        }
        (
            boolean(required(obj, path, "isMut")?, &join(path, "isMut"))?,
            boolean(required(obj, path, "isSigner")?, &join(path, "isSigner"))?,
            optional_bool(obj, path, "isOptionalSigner")?,
            optional_bool(obj, path, "isOptional")?,
            None,
        )
    } else {
        check_keys(
            obj,
            path,
            &[
                "name",
                "docs",
                "writable",
                "signer",
                "optional",
                "address",
                "pda",
                "relations",
            ],
        )?;
        (
            optional_bool(obj, path, "writable")?,
            optional_bool(obj, path, "signer")?,
            false,
            optional_bool(obj, path, "optional")?,
            match obj.get("address") {
                None => None,
                Some(value) => Some(self::address(value, &join(path, "address"))?),
            },
        )
    };

    out.push(InstructionAccount {
        name,
        groups: groups.to_vec(),
        docs,
        writable,
        signer,
        optional_signer,
        optional,
        address,
        pda: match obj.get("pda") {
            None => None,
            Some(value) => Some(parse_pda(value, &join(path, "pda"), format)?),
        },
        relations: match obj.get("relations") {
            None => Vec::new(),
            Some(value) => strings(value, &join(path, "relations"))?,
        },
    });
    Ok(())
}

fn parse_pda(value: &Value, path: &str, format: Format) -> Result<Pda> {
    let obj = object(value, path)?;
    let program_key = if format.is_legacy() {
        "programId"
    } else {
        "program"
    };
    check_keys(obj, path, &["seeds", program_key])?;
    Ok(Pda {
        seeds: list(obj, path, "seeds", true, |value, path| {
            parse_seed(value, &path, format)
        })?,
        program: match obj.get(program_key) {
            None => None,
            Some(value) => Some(parse_seed(value, &join(path, program_key), format)?),
        },
    })
}

fn parse_seed(value: &Value, path: &str, format: Format) -> Result<Seed> {
    let obj = object(value, path)?;
    let kind = required_string(obj, path, "kind")?;
    let legacy = format.is_legacy();
    match kind.as_str() {
        "const" => {
            check_keys(
                obj,
                path,
                if legacy {
                    &["kind", "type", "value"]
                } else {
                    &["kind", "value"]
                },
            )?;
            let value_path = join(path, "value");
            let value = required(obj, path, "value")?;
            let bytes = if legacy {
                // A legacy const seed value is written in its `type`; only
                // types with unambiguous bytes are accepted.
                let ty = parse_type(required(obj, path, "type")?, &join(path, "type"))?;
                match (&ty, value) {
                    (IdlType::String, Value::String(s)) => s.as_bytes().to_vec(),
                    (IdlType::Bytes, Value::Array(_)) => byte_array(value, &value_path)?,
                    (IdlType::Array(inner, ArrayLen::Value(len)), Value::Array(items))
                        if **inner == IdlType::U8 && items.len() == *len =>
                    {
                        byte_array(value, &value_path)?
                    }
                    _ => {
                        return Err(IdlError::invalid(
                            path,
                            format!("unsupported legacy const seed of type {ty:?}"),
                        ))
                    }
                }
            } else {
                byte_array(value, &value_path)?
            };
            Ok(Seed::Const { value: bytes })
        }
        "arg" => {
            check_keys(
                obj,
                path,
                if legacy {
                    &["kind", "type", "path"]
                } else {
                    &["kind", "path"]
                },
            )?;
            if legacy {
                parse_type(required(obj, path, "type")?, &join(path, "type"))?;
            }
            Ok(Seed::Arg {
                path: required_string(obj, path, "path")?,
            })
        }
        "account" => {
            check_keys(
                obj,
                path,
                if legacy {
                    &["kind", "type", "path", "account"]
                } else {
                    &["kind", "path", "account"]
                },
            )?;
            if legacy {
                parse_type(required(obj, path, "type")?, &join(path, "type"))?;
            }
            Ok(Seed::Account {
                path: required_string(obj, path, "path")?,
                account: optional_string(obj, path, "account")?,
            })
        }
        other => Err(IdlError::invalid(
            join(path, "kind"),
            format!("unknown seed kind `{other}`"),
        )),
    }
}

fn parse_type_def(value: &Value, path: &str, format: Format) -> Result<TypeDef> {
    let obj = object(value, path)?;
    if format.is_legacy() {
        check_keys(obj, path, &["name", "docs", "generics", "type"])?;
    } else {
        check_keys(
            obj,
            path,
            &["name", "docs", "serialization", "repr", "generics", "type"],
        )?;
    }

    let name = required_string(obj, path, "name")?;
    let ty_path = join(path, "type");
    let ty = object(required(obj, path, "type")?, &ty_path)?;
    let kind = required_string(ty, &ty_path, "kind")?;
    let kind = match kind.as_str() {
        "struct" => {
            // The 0.30 format leaves `fields` out for a unit struct.
            if format.is_legacy() {
                check_keys(ty, &ty_path, &["kind", "fields"])?;
                required(ty, &ty_path, "fields")?;
            } else {
                check_keys(ty, &ty_path, &["kind", "fields"])?;
            }
            TypeDefKind::Struct(parse_fields(
                ty.get("fields"),
                &join(&ty_path, "fields"),
                format,
            )?)
        }
        "enum" => {
            check_keys(ty, &ty_path, &["kind", "variants"])?;
            TypeDefKind::Enum(list(ty, &ty_path, "variants", true, |value, path| {
                let variant = object(value, &path)?;
                check_keys(variant, &path, &["name", "fields"])?;
                Ok(EnumVariant {
                    name: required_string(variant, &path, "name")?,
                    fields: parse_fields(variant.get("fields"), &join(&path, "fields"), format)?,
                })
            })?)
        }
        "type" if !format.is_legacy() => {
            check_keys(ty, &ty_path, &["kind", "alias"])?;
            TypeDefKind::Alias(parse_type(
                required(ty, &ty_path, "alias")?,
                &join(&ty_path, "alias"),
            )?)
        }
        "alias" if format.is_legacy() => {
            check_keys(ty, &ty_path, &["kind", "value"])?;
            TypeDefKind::Alias(parse_type(
                required(ty, &ty_path, "value")?,
                &join(&ty_path, "value"),
            )?)
        }
        other => {
            return Err(IdlError::invalid(
                join(&ty_path, "kind"),
                format!("unknown type definition kind `{other}`"),
            ))
        }
    };

    let generics_path = join(path, "generics");
    let generics = match obj.get("generics") {
        None => Vec::new(),
        Some(value) if format.is_legacy() => {
            // Legacy generics are bare names; `resolve` infers type vs const
            // from usage.
            strings(value, &generics_path)?
                .into_iter()
                .map(|name| GenericParam::Type { name })
                .collect()
        }
        Some(value) => array(value, &generics_path)?
            .iter()
            .enumerate()
            .map(|(i, value)| parse_generic_param(value, &index(&generics_path, i)))
            .collect::<Result<_>>()?,
    };

    Ok(TypeDef {
        name,
        docs: docs(obj, path)?,
        generics,
        serialization: match obj.get("serialization") {
            None => Serialization::Borsh,
            Some(value) => parse_serialization(value, &join(path, "serialization"))?,
        },
        repr: match obj.get("repr") {
            None => None,
            Some(value) => Some(parse_repr(value, &join(path, "repr"))?),
        },
        kind,
    })
}

/// Reads a legacy event, whose fields are inline, into a struct definition.
fn parse_legacy_event(value: &Value, path: &str, format: Format) -> Result<TypeDef> {
    let obj = object(value, path)?;
    check_keys(obj, path, &["name", "fields"])?;
    let fields = list(obj, path, "fields", true, |value, path| {
        let field = object(value, &path)?;
        check_keys(field, &path, &["name", "type", "index"])?;
        // `index` has no effect on the layout, see the module documentation.
        boolean(required(field, &path, "index")?, &join(&path, "index"))?;
        Ok(Field {
            name: format.field_name(required_string(field, &path, "name")?),
            docs: Vec::new(),
            ty: parse_type(required(field, &path, "type")?, &join(&path, "type"))?,
        })
    })?;

    Ok(TypeDef {
        name: required_string(obj, path, "name")?,
        docs: Vec::new(),
        generics: Vec::new(),
        serialization: Serialization::Borsh,
        repr: None,
        kind: TypeDefKind::Struct(if fields.is_empty() {
            Fields::Unit
        } else {
            Fields::Named(fields)
        }),
    })
}

/// Reads struct or variant fields: a list of named fields, a list of types, or
/// nothing.
fn parse_fields(value: Option<&Value>, path: &str, format: Format) -> Result<Fields> {
    let items = match value {
        None => return Ok(Fields::Unit),
        Some(value) => array(value, path)?,
    };
    let Some(first) = items.first() else {
        return Ok(Fields::Unit);
    };

    // A named field is an object with a `name`. No type is written that way.
    let is_named = |value: &Value| {
        value
            .as_object()
            .is_some_and(|obj| obj.contains_key("name"))
    };
    if is_named(first) {
        let fields = items
            .iter()
            .enumerate()
            .map(|(i, value)| {
                let path = index(path, i);
                if !is_named(value) {
                    return Err(IdlError::invalid(
                        &path,
                        "expected a named field like the ones before it",
                    ));
                }
                parse_field(value, &path, format)
            })
            .collect::<Result<_>>()?;
        Ok(Fields::Named(fields))
    } else {
        let types = items
            .iter()
            .enumerate()
            .map(|(i, value)| {
                let path = index(path, i);
                if is_named(value) {
                    return Err(IdlError::invalid(
                        &path,
                        "expected a type like the ones before it",
                    ));
                }
                parse_type(value, &path)
            })
            .collect::<Result<_>>()?;
        Ok(Fields::Tuple(types))
    }
}

fn parse_field(value: &Value, path: &str, format: Format) -> Result<Field> {
    let obj = object(value, path)?;
    check_keys(obj, path, &["name", "docs", "type"])?;
    Ok(Field {
        name: format.field_name(required_string(obj, path, "name")?),
        docs: docs(obj, path)?,
        ty: parse_type(required(obj, path, "type")?, &join(path, "type"))?,
    })
}

/// Reads a type in either format. The two vocabularies do not overlap, so both
/// are accepted everywhere.
pub(super) fn parse_type(value: &Value, path: &str) -> Result<IdlType> {
    let obj = match value {
        Value::String(name) => {
            return Ok(match name.as_str() {
                "bool" => IdlType::Bool,
                "u8" => IdlType::U8,
                "i8" => IdlType::I8,
                "u16" => IdlType::U16,
                "i16" => IdlType::I16,
                "u32" => IdlType::U32,
                "i32" => IdlType::I32,
                "f32" => IdlType::F32,
                "u64" => IdlType::U64,
                "i64" => IdlType::I64,
                "f64" => IdlType::F64,
                "u128" => IdlType::U128,
                "i128" => IdlType::I128,
                "u256" => IdlType::U256,
                "i256" => IdlType::I256,
                "bytes" => IdlType::Bytes,
                "string" => IdlType::String,
                "pubkey" | "publicKey" => IdlType::Pubkey,
                other => return Err(IdlError::invalid(path, format!("unknown type `{other}`"))),
            })
        }
        Value::Object(obj) => obj,
        _ => {
            return Err(IdlError::invalid(
                path,
                "expected a type, a string or an object",
            ))
        }
    };

    let mut entries = obj.iter();
    let (key, inner) = match (entries.next(), entries.next()) {
        (Some(entry), None) => entry,
        _ => {
            let keys: Vec<&str> = obj.keys().map(String::as_str).collect();
            return Err(IdlError::invalid(
                path,
                format!("expected a type object with exactly one key, found {keys:?}"),
            ));
        }
    };
    let inner_path = join(path, key);

    Ok(match key.as_str() {
        "option" => IdlType::Option(Box::new(parse_type(inner, &inner_path)?)),
        "coption" => IdlType::COption(Box::new(parse_type(inner, &inner_path)?)),
        "vec" => IdlType::Vec(Box::new(parse_type(inner, &inner_path)?)),
        "hashSet" | "bTreeSet" => IdlType::Set(Box::new(parse_type(inner, &inner_path)?)),
        "hashMap" | "bTreeMap" => {
            let pair = array(inner, &inner_path)?;
            if pair.len() != 2 {
                return Err(IdlError::invalid(&inner_path, "expected [key, value]"));
            }
            IdlType::Map(
                Box::new(parse_type(&pair[0], &index(&inner_path, 0))?),
                Box::new(parse_type(&pair[1], &index(&inner_path, 1))?),
            )
        }
        "tuple" => IdlType::Tuple(
            array(inner, &inner_path)?
                .iter()
                .enumerate()
                .map(|(i, item)| parse_type(item, &index(&inner_path, i)))
                .collect::<Result<_>>()?,
        ),
        "array" | "genericLenArray" => {
            let pair = array(inner, &inner_path)?;
            if pair.len() != 2 {
                return Err(IdlError::invalid(&inner_path, "expected [type, length]"));
            }
            let element = parse_type(&pair[0], &index(&inner_path, 0))?;
            let len_path = index(&inner_path, 1);
            let len = match (key.as_str(), &pair[1]) {
                ("array", Value::Number(n)) => ArrayLen::Value(
                    n.as_u64()
                        .and_then(|n| usize::try_from(n).ok())
                        .ok_or_else(|| {
                            IdlError::invalid(&len_path, "array length is not a valid usize")
                        })?,
                ),
                // 0.30: `{"generic": "N"}`.
                ("array", Value::Object(len)) if len.len() == 1 && len.contains_key("generic") => {
                    ArrayLen::Generic(string(&len["generic"], &join(&len_path, "generic"))?)
                }
                // Legacy: `{"genericLenArray": [T, "N"]}`.
                ("genericLenArray", Value::String(name)) => ArrayLen::Generic(name.clone()),
                _ => return Err(IdlError::invalid(&len_path, "unsupported array length")),
            };
            IdlType::Array(Box::new(element), len)
        }
        "defined" => match inner {
            // Legacy: `{"defined": "Name"}`.
            Value::String(name) => IdlType::Defined {
                name: name.clone(),
                generics: Vec::new(),
            },
            // 0.30: `{"defined": {"name": "Name", "generics": [...]}}`.
            Value::Object(defined) => {
                check_keys(defined, &inner_path, &["name", "generics"])?;
                IdlType::Defined {
                    name: required_string(defined, &inner_path, "name")?,
                    generics: list(defined, &inner_path, "generics", false, |value, path| {
                        parse_generic_arg(value, &path)
                    })?,
                }
            }
            _ => {
                return Err(IdlError::invalid(
                    &inner_path,
                    "expected a type name or an object",
                ))
            }
        },
        // Legacy: `{"definedWithTypeArgs": {"name": "Name", "args": [...]}}`.
        "definedWithTypeArgs" => {
            let defined = object(inner, &inner_path)?;
            check_keys(defined, &inner_path, &["name", "args"])?;
            IdlType::Defined {
                name: required_string(defined, &inner_path, "name")?,
                generics: list(defined, &inner_path, "args", true, |value, path| {
                    parse_legacy_generic_arg(value, &path)
                })?,
            }
        }
        "generic" => IdlType::Generic(string(inner, &inner_path)?),
        other => return Err(IdlError::invalid(path, format!("unknown type `{other}`"))),
    })
}

/// 0.30: `{"kind": "type", "type": T}` or `{"kind": "const", "value": "8"}`.
fn parse_generic_arg(value: &Value, path: &str) -> Result<GenericArg> {
    let obj = object(value, path)?;
    match required_string(obj, path, "kind")?.as_str() {
        "type" => {
            check_keys(obj, path, &["kind", "type"])?;
            Ok(GenericArg::Type(parse_type(
                required(obj, path, "type")?,
                &join(path, "type"),
            )?))
        }
        "const" => {
            check_keys(obj, path, &["kind", "value"])?;
            Ok(GenericArg::Const(required_string(obj, path, "value")?))
        }
        other => Err(IdlError::invalid(
            join(path, "kind"),
            format!("unknown generic argument kind `{other}`"),
        )),
    }
}

/// Legacy: `{"type": T}`, `{"value": "8"}` or `{"generic": "T"}`. A generic is
/// read as a type; `resolve` turns it into a const when declared as one.
fn parse_legacy_generic_arg(value: &Value, path: &str) -> Result<GenericArg> {
    let obj = object(value, path)?;
    let mut entries = obj.iter();
    match (entries.next(), entries.next()) {
        (Some((key, inner)), None) => {
            let inner_path = join(path, key);
            match key.as_str() {
                "type" => Ok(GenericArg::Type(parse_type(inner, &inner_path)?)),
                "value" => Ok(GenericArg::Const(string(inner, &inner_path)?)),
                "generic" => Ok(GenericArg::Type(IdlType::Generic(string(
                    inner,
                    &inner_path,
                )?))),
                other => Err(IdlError::invalid(
                    path,
                    format!("unknown generic argument `{other}`"),
                )),
            }
        }
        _ => Err(IdlError::invalid(
            path,
            "expected a generic argument object with exactly one key",
        )),
    }
}

/// 0.30: `{"kind": "type", "name": "T"}` or `{"kind": "const", "name": "N",
/// "type": "usize"}`.
fn parse_generic_param(value: &Value, path: &str) -> Result<GenericParam> {
    let obj = object(value, path)?;
    match required_string(obj, path, "kind")?.as_str() {
        "type" => {
            check_keys(obj, path, &["kind", "name"])?;
            Ok(GenericParam::Type {
                name: required_string(obj, path, "name")?,
            })
        }
        "const" => {
            check_keys(obj, path, &["kind", "name", "type"])?;
            Ok(GenericParam::Const {
                name: required_string(obj, path, "name")?,
                ty: required_string(obj, path, "type")?,
            })
        }
        other => Err(IdlError::invalid(
            join(path, "kind"),
            format!("unknown generic parameter kind `{other}`"),
        )),
    }
}

fn parse_serialization(value: &Value, path: &str) -> Result<Serialization> {
    match value {
        Value::String(s) => match s.as_str() {
            "borsh" => Ok(Serialization::Borsh),
            "bytemuck" => Ok(Serialization::Bytemuck),
            "bytemuckunsafe" => Ok(Serialization::BytemuckUnsafe),
            other => Err(IdlError::invalid(
                path,
                format!("unknown serialization `{other}`"),
            )),
        },
        Value::Object(obj) => {
            check_keys(obj, path, &["custom"])?;
            Ok(Serialization::Custom(required_string(obj, path, "custom")?))
        }
        _ => Err(IdlError::invalid(
            path,
            "expected a serialization name or `{\"custom\": ...}`",
        )),
    }
}

fn parse_repr(value: &Value, path: &str) -> Result<Repr> {
    let obj = object(value, path)?;
    let kind = required_string(obj, path, "kind")?;
    let modifier = |obj: &Object| -> Result<ReprModifier> {
        check_keys(obj, path, &["kind", "packed", "align"])?;
        Ok(ReprModifier {
            packed: optional_bool(obj, path, "packed")?,
            align: match obj.get("align") {
                None => None,
                Some(value) => Some(usize_value(value, &join(path, "align"))?),
            },
        })
    };
    match kind.as_str() {
        "rust" => Ok(Repr::Rust(modifier(obj)?)),
        "c" => Ok(Repr::C(modifier(obj)?)),
        "transparent" => {
            check_keys(obj, path, &["kind"])?;
            Ok(Repr::Transparent)
        }
        other => Err(IdlError::invalid(
            join(path, "kind"),
            format!("unknown repr `{other}`"),
        )),
    }
}

fn parse_error_code(value: &Value, path: String) -> Result<ErrorCode> {
    let obj = object(value, &path)?;
    check_keys(obj, &path, &["code", "name", "msg"])?;
    let code_path = join(&path, "code");
    let code = required(obj, &path, "code")?
        .as_u64()
        .and_then(|code| u32::try_from(code).ok())
        .ok_or_else(|| {
            IdlError::invalid(
                &code_path,
                "expected an error code between 0 and 4294967295",
            )
        })?;
    Ok(ErrorCode {
        code,
        name: required_string(obj, &path, "name")?,
        msg: optional_string(obj, &path, "msg")?,
    })
}

fn parse_constant(value: &Value, path: &str, format: Format) -> Result<Constant> {
    let obj = object(value, path)?;
    if format.is_legacy() {
        check_keys(obj, path, &["name", "type", "value"])?;
    } else {
        check_keys(obj, path, &["name", "docs", "type", "value"])?;
    }
    Ok(Constant {
        name: required_string(obj, path, "name")?,
        docs: docs(obj, path)?,
        ty: parse_type(required(obj, path, "type")?, &join(path, "type"))?,
        value: required_string(obj, path, "value")?,
    })
}

/// A Shank instruction's `{"type": "u8", "value": 3}`: the value serialized in
/// that type, little-endian as borsh writes it.
fn shank_discriminant(value: &Value, path: &str) -> Result<Vec<u8>> {
    let obj = object(value, path)?;
    check_keys(obj, path, &["type", "value"])?;
    let ty = parse_type(required(obj, path, "type")?, &join(path, "type"))?;
    let value_path = join(path, "value");
    let n = required(obj, path, "value")?
        .as_u64()
        .ok_or_else(|| IdlError::invalid(&value_path, "expected an unsigned integer"))?;
    let width = match ty {
        IdlType::U8 => 1,
        IdlType::U16 => 2,
        IdlType::U32 => 4,
        IdlType::U64 => 8,
        other => {
            return Err(IdlError::invalid(
                join(path, "type"),
                format!("unsupported discriminant type {other:?}"),
            ))
        }
    };
    if width < 8 && n >> (width * 8) != 0 {
        return Err(IdlError::invalid(
            &value_path,
            format!("{n} does not fit the discriminant type"),
        ));
    }
    Ok(n.to_le_bytes()[..width].to_vec())
}

fn discriminator_bytes(value: &Value, path: &str) -> Result<Vec<u8>> {
    let bytes = byte_array(value, path)?;
    if bytes.is_empty() {
        return Err(IdlError::invalid(path, "discriminator is empty"));
    }
    Ok(bytes)
}

fn address(value: &Value, path: &str) -> Result<String> {
    let address = string(value, path)?;
    match bs58::decode(&address).into_vec() {
        Ok(bytes) if bytes.len() == 32 => Ok(address),
        _ => Err(IdlError::invalid(
            path,
            format!("`{address}` is not a base58 address of 32 bytes"),
        )),
    }
}

// JSON helpers. Each takes the path of the value it reads.

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{path}.{key}")
    }
}

fn index(path: &str, i: usize) -> String {
    format!("{path}[{i}]")
}

fn check_keys(obj: &Object, path: &str, allowed: &[&str]) -> Result<()> {
    match obj.keys().find(|key| !allowed.contains(&key.as_str())) {
        None => Ok(()),
        Some(key) => Err(IdlError::invalid(join(path, key), "unknown key")),
    }
}

fn object<'a>(value: &'a Value, path: &str) -> Result<&'a Object> {
    value
        .as_object()
        .ok_or_else(|| IdlError::invalid(path, "expected an object"))
}

fn array<'a>(value: &'a Value, path: &str) -> Result<&'a Vec<Value>> {
    value
        .as_array()
        .ok_or_else(|| IdlError::invalid(path, "expected an array"))
}

fn string(value: &Value, path: &str) -> Result<String> {
    value
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| IdlError::invalid(path, "expected a string"))
}

fn boolean(value: &Value, path: &str) -> Result<bool> {
    value
        .as_bool()
        .ok_or_else(|| IdlError::invalid(path, "expected a boolean"))
}

fn usize_value(value: &Value, path: &str) -> Result<usize> {
    value
        .as_u64()
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| IdlError::invalid(path, "expected an unsigned integer"))
}

fn strings(value: &Value, path: &str) -> Result<Vec<String>> {
    array(value, path)?
        .iter()
        .enumerate()
        .map(|(i, value)| string(value, &index(path, i)))
        .collect()
}

fn byte_array(value: &Value, path: &str) -> Result<Vec<u8>> {
    array(value, path)?
        .iter()
        .enumerate()
        .map(|(i, value)| {
            value
                .as_u64()
                .and_then(|n| u8::try_from(n).ok())
                .ok_or_else(|| IdlError::invalid(index(path, i), "expected a byte, 0 to 255"))
        })
        .collect()
}

fn required<'a>(obj: &'a Object, path: &str, key: &str) -> Result<&'a Value> {
    obj.get(key)
        .ok_or_else(|| IdlError::invalid(join(path, key), "missing"))
}

fn required_string(obj: &Object, path: &str, key: &str) -> Result<String> {
    string(required(obj, path, key)?, &join(path, key))
}

fn optional_string(obj: &Object, path: &str, key: &str) -> Result<Option<String>> {
    match obj.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => string(value, &join(path, key)).map(Some),
    }
}

fn optional_bool(obj: &Object, path: &str, key: &str) -> Result<bool> {
    match obj.get(key) {
        None => Ok(false),
        Some(value) => boolean(value, &join(path, key)),
    }
}

fn docs(obj: &Object, path: &str) -> Result<Vec<String>> {
    match obj.get("docs") {
        None => Ok(Vec::new()),
        Some(value) => strings(value, &join(path, "docs")),
    }
}

/// Reads the array under `key` with `read`, which gets each item and its path.
/// A missing optional array reads as empty.
fn list<T>(
    obj: &Object,
    path: &str,
    key: &str,
    required_key: bool,
    mut read: impl FnMut(&Value, String) -> Result<T>,
) -> Result<Vec<T>> {
    let list_path = join(path, key);
    let items = match obj.get(key) {
        None if !required_key => return Ok(Vec::new()),
        None => return Err(IdlError::invalid(list_path, "missing")),
        Some(value) => array(value, &list_path)?,
    };
    items
        .iter()
        .enumerate()
        .map(|(i, value)| read(value, index(&list_path, i)))
        .collect()
}
