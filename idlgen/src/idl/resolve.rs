//! Checks references within a parsed IDL: unique names, existing defined types
//! with matching generic arguments, and declared generic parameters. Also
//! infers the kind of legacy generic parameters.

use std::collections::{HashMap, HashSet};

use super::parse::{Shape, Sources, TypeDefSource};
use super::*;

type Result<T> = std::result::Result<T, IdlError>;

pub(super) fn resolve(idl: &mut Idl, sources: &Sources) -> Result<()> {
    debug_assert_eq!(idl.types.len(), sources.types.len());

    unique(
        idl.types.iter().map(|def| def.name.as_str()),
        |i| sources.types[i].path.clone(),
        "type",
    )?;
    unique(
        idl.instructions.iter().map(|ix| ix.name.as_str()),
        |i| format!("instructions[{i}]"),
        "instruction",
    )?;
    unique(
        idl.accounts.iter().map(|acc| acc.name.as_str()),
        |i| format!("accounts[{i}]"),
        "account",
    )?;
    unique(
        idl.events.iter().map(|ev| ev.name.as_str()),
        |i| format!("events[{i}]"),
        "event",
    )?;
    for (i, ix) in idl.instructions.iter().enumerate() {
        unique(
            ix.accounts.iter().map(InstructionAccount::qualified_name),
            |_| format!("instructions[{i}].accounts"),
            "instruction account",
        )?;
    }

    if idl.format == IdlFormat::Legacy {
        for (def, source) in layouts(idl, sources) {
            infer_legacy_generics(def, &source.path)?;
        }
    }

    let signatures: HashMap<String, Vec<GenericParam>> = idl
        .types
        .iter()
        .map(|def| (def.name.clone(), def.generics.clone()))
        .collect();
    let checker = Checker {
        signatures: &signatures,
    };

    for (i, ix) in idl.instructions.iter_mut().enumerate() {
        for (j, arg) in ix.args.iter_mut().enumerate() {
            checker.check(
                &mut arg.ty,
                &[],
                &format!("instructions[{i}].args[{j}].type"),
            )?;
        }
        if let Some(returns) = &mut ix.returns {
            checker.check(returns, &[], &format!("instructions[{i}].returns"))?;
        }
    }
    for (i, constant) in idl.constants.iter_mut().enumerate() {
        checker.check(&mut constant.ty, &[], &format!("constants[{i}].type"))?;
    }
    for (def, source) in layouts(idl, sources) {
        let scope = def.generics.clone();
        for_each_type(def, source, |ty, path| checker.check(ty, &scope, &path))?;
    }

    Ok(())
}

/// Every type definition with its source: the types, then the account and
/// event layouts.
fn layouts<'a>(
    idl: &'a mut Idl,
    sources: &'a Sources,
) -> impl Iterator<Item = (&'a mut TypeDef, &'a TypeDefSource)> {
    idl.types
        .iter_mut()
        .zip(&sources.types)
        .chain(
            idl.accounts
                .iter_mut()
                .map(|acc| &mut acc.layout)
                .zip(&sources.accounts),
        )
        .chain(
            idl.events
                .iter_mut()
                .map(|ev| &mut ev.layout)
                .zip(&sources.events),
        )
}

/// Fails on the first name that repeats, naming the path of the repeat.
fn unique<S: AsRef<str>>(
    names: impl Iterator<Item = S>,
    path: impl Fn(usize) -> String,
    what: &str,
) -> Result<()> {
    let mut seen = HashSet::new();
    for (i, name) in names.enumerate() {
        if !seen.insert(name.as_ref().to_string()) {
            return Err(IdlError::invalid(
                path(i),
                format!("{what} `{}` is defined twice", name.as_ref()),
            ));
        }
    }
    Ok(())
}

/// Calls `f` on every type a definition is made of, with its JSON path.
fn for_each_type(
    def: &mut TypeDef,
    source: &TypeDefSource,
    mut f: impl FnMut(&mut IdlType, String) -> Result<()>,
) -> Result<()> {
    let base = &source.path;
    let fields = |fields: &mut Fields,
                  prefix: String,
                  f: &mut dyn FnMut(&mut IdlType, String) -> Result<()>| {
        match fields {
            Fields::Unit => Ok(()),
            Fields::Named(named) => named
                .iter_mut()
                .enumerate()
                .try_for_each(|(j, field)| f(&mut field.ty, format!("{prefix}[{j}].type"))),
            Fields::Tuple(types) => types
                .iter_mut()
                .enumerate()
                .try_for_each(|(j, ty)| f(ty, format!("{prefix}[{j}]"))),
        }
    };

    match &mut def.kind {
        TypeDefKind::Struct(struct_fields) => {
            let prefix = match source.shape {
                Shape::LegacyEvent => format!("{base}.fields"),
                Shape::Spec | Shape::Legacy => format!("{base}.type.fields"),
            };
            fields(struct_fields, prefix, &mut f)
        }
        TypeDefKind::Enum(variants) => {
            variants
                .iter_mut()
                .enumerate()
                .try_for_each(|(v, variant)| {
                    fields(
                        &mut variant.fields,
                        format!("{base}.type.variants[{v}].fields"),
                        &mut f,
                    )
                })
        }
        TypeDefKind::Alias(ty) => {
            let key = match source.shape {
                Shape::Spec => "alias",
                Shape::Legacy | Shape::LegacyEvent => "value",
            };
            f(ty, format!("{base}.type.{key}"))
        }
    }
}

/// A legacy generic used as an array length becomes a `usize` const parameter,
/// otherwise a type parameter. One used both ways, or neither, is an error.
fn infer_legacy_generics(def: &mut TypeDef, path: &str) -> Result<()> {
    if def.generics.is_empty() {
        return Ok(());
    }

    let mut as_type = HashSet::new();
    let mut as_len = HashSet::new();
    let mut visit = |ty: &IdlType| collect_generic_uses(ty, &mut as_type, &mut as_len);
    match &def.kind {
        TypeDefKind::Struct(fields) => visit_fields(fields, &mut visit),
        TypeDefKind::Enum(variants) => variants
            .iter()
            .for_each(|v| visit_fields(&v.fields, &mut visit)),
        TypeDefKind::Alias(ty) => visit(ty),
    }

    for (k, param) in def.generics.iter_mut().enumerate() {
        let name = param.name().to_string();
        *param = match (as_type.contains(&name), as_len.contains(&name)) {
            (true, false) => GenericParam::Type { name },
            (false, true) => GenericParam::Const { name, ty: "usize".to_string() },
            (used_as_type, _) => {
                return Err(IdlError::invalid(
                    format!("{path}.generics[{k}]"),
                    format!(
                        "cannot tell whether legacy generic parameter `{name}` is a type or a const parameter: \
                         it is used {}",
                        if used_as_type { "both as a type and as an array length" } else { "neither as a type nor as an array length" }
                    ),
                ))
            }
        };
    }
    Ok(())
}

fn visit_fields(fields: &Fields, visit: &mut impl FnMut(&IdlType)) {
    match fields {
        Fields::Unit => {}
        Fields::Named(named) => named.iter().for_each(|field| visit(&field.ty)),
        Fields::Tuple(types) => types.iter().for_each(visit),
    }
}

/// Collects the generic names a type uses as a type and as an array length.
/// A generic passed on as an argument to another defined type is neither.
fn collect_generic_uses(ty: &IdlType, as_type: &mut HashSet<String>, as_len: &mut HashSet<String>) {
    match ty {
        IdlType::Option(inner)
        | IdlType::COption(inner)
        | IdlType::Vec(inner)
        | IdlType::Set(inner) => collect_generic_uses(inner, as_type, as_len),
        IdlType::Map(key, value) => {
            collect_generic_uses(key, as_type, as_len);
            collect_generic_uses(value, as_type, as_len);
        }
        IdlType::Tuple(items) => {
            for item in items {
                collect_generic_uses(item, as_type, as_len);
            }
        }
        IdlType::Array(inner, len) => {
            if let ArrayLen::Generic(name) = len {
                as_len.insert(name.clone());
            }
            collect_generic_uses(inner, as_type, as_len);
        }
        IdlType::Generic(name) => {
            as_type.insert(name.clone());
        }
        IdlType::Defined { generics, .. } => {
            for arg in generics {
                match arg {
                    GenericArg::Type(IdlType::Generic(_)) | GenericArg::Const(_) => {}
                    GenericArg::Type(ty) => collect_generic_uses(ty, as_type, as_len),
                }
            }
        }
        _ => {}
    }
}

struct Checker<'a> {
    signatures: &'a HashMap<String, Vec<GenericParam>>,
}

impl Checker<'_> {
    /// Checks one type against the generic parameters in scope. A generic
    /// argument naming a const parameter in scope becomes a const argument.
    fn check(&self, ty: &mut IdlType, scope: &[GenericParam], path: &str) -> Result<()> {
        match ty {
            IdlType::Option(inner)
            | IdlType::COption(inner)
            | IdlType::Vec(inner)
            | IdlType::Set(inner) => self.check(inner, scope, path),
            IdlType::Map(key, value) => {
                self.check(key, scope, path)?;
                self.check(value, scope, path)
            }
            IdlType::Tuple(items) => items
                .iter_mut()
                .try_for_each(|item| self.check(item, scope, path)),
            IdlType::Defined { name, generics }
                if generics.is_empty() && !self.signatures.contains_key(name.as_str()) =>
            {
                match builtin_alias(name) {
                    Some(builtin) => {
                        *ty = builtin;
                        Ok(())
                    }
                    None => Err(IdlError::invalid(path, format!("undefined type `{name}`"))),
                }
            }
            IdlType::Array(inner, len) => {
                if let ArrayLen::Generic(name) = len {
                    if !is_const_param(scope, name) {
                        return Err(IdlError::invalid(
                            path,
                            format!("array length `{name}` is not a const generic parameter of the enclosing type"),
                        ));
                    }
                }
                self.check(inner, scope, path)
            }
            IdlType::Generic(name) => {
                if scope
                    .iter()
                    .any(|p| matches!(p, GenericParam::Type { name: n } if n == name))
                {
                    Ok(())
                } else {
                    Err(IdlError::invalid(
                        path,
                        format!("`{name}` is not a generic type parameter of the enclosing type"),
                    ))
                }
            }
            IdlType::Defined { name, generics } => {
                let params = self
                    .signatures
                    .get(name.as_str())
                    .ok_or_else(|| IdlError::invalid(path, format!("undefined type `{name}`")))?;
                if params.len() != generics.len() {
                    return Err(IdlError::invalid(
                        path,
                        format!(
                            "type `{name}` takes {} generic arguments, {} given",
                            params.len(),
                            generics.len()
                        ),
                    ));
                }
                for (arg, param) in generics.iter_mut().zip(params) {
                    if let GenericArg::Type(IdlType::Generic(generic)) = arg {
                        if is_const_param(scope, generic) {
                            *arg = GenericArg::Const(std::mem::take(generic));
                        }
                    }
                    match (arg, param) {
                        (GenericArg::Type(ty), GenericParam::Type { .. }) => self.check(ty, scope, path)?,
                        (GenericArg::Const(value), GenericParam::Const { .. }) => {
                            if !is_const_param(scope, value) && !is_literal(value) {
                                return Err(IdlError::invalid(
                                    path,
                                    format!(
                                        "const argument `{value}` of type `{name}` is neither a literal nor a const \
                                         generic parameter of the enclosing type"
                                    ),
                                ));
                            }
                        }
                        (_, param) => {
                            return Err(IdlError::invalid(
                                path,
                                format!(
                                    "generic argument for `{}` of type `{name}` is not of the kind the parameter declares",
                                    param.name()
                                ),
                            ))
                        }
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }
}

fn is_const_param(scope: &[GenericParam], name: &str) -> bool {
    scope
        .iter()
        .any(|p| matches!(p, GenericParam::Const { name: n, .. } if n == name))
}

/// A const generic argument written as a literal: an integer, possibly with
/// `_` separators, or a boolean.
fn is_literal(value: &str) -> bool {
    let digits = value.strip_prefix('-').unwrap_or(value).replace('_', "");
    value == "true"
        || value == "false"
        || (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit()))
}

/// A type Shank IDLs name as `defined` without defining it: a primitive, or a Solana clock alias.
fn builtin_alias(name: &str) -> Option<IdlType> {
    match name {
        "UnixTimestamp" => Some(IdlType::I64),
        "Epoch" | "Slot" => Some(IdlType::U64),
        _ => super::parse::parse_type(&serde_json::Value::String(name.to_string()), "").ok(),
    }
}
