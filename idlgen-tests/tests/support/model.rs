//! Random valid bytes for any IDL type, built from the IDL model independently of the generated
//! code: Borsh values, and the raw memory layout of zero-copy structs.

use std::collections::HashMap;

use substreams_solana_idlgen::idl::{
    ArrayLen, Fields, GenericArg, GenericParam, Idl, IdlType, TypeDef, TypeDefKind,
};
use substreams_solana_idlgen::layout;

/// xorshift64*, so the test needs no extra crate and is deterministic.
pub struct Rng(pub u64);

impl Rng {
    pub fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }

    pub fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }

    pub fn bytes(&mut self, n: usize, out: &mut Vec<u8>) {
        out.extend((0..n).map(|_| self.next() as u8));
    }
}

/// Generic arguments in scope: type parameters and const parameters.
#[derive(Default, Clone)]
pub struct Env {
    types: HashMap<String, IdlType>,
    consts: HashMap<String, usize>,
}

impl Env {
    pub fn subst(&self, ty: &IdlType) -> IdlType {
        match ty {
            IdlType::Generic(name) => self.types[name].clone(),
            IdlType::Option(inner) => IdlType::Option(Box::new(self.subst(inner))),
            IdlType::COption(inner) => IdlType::COption(Box::new(self.subst(inner))),
            IdlType::Vec(inner) => IdlType::Vec(Box::new(self.subst(inner))),
            IdlType::Set(inner) => IdlType::Set(Box::new(self.subst(inner))),
            IdlType::Map(key, value) => {
                IdlType::Map(Box::new(self.subst(key)), Box::new(self.subst(value)))
            }
            IdlType::Tuple(items) => IdlType::Tuple(items.iter().map(|t| self.subst(t)).collect()),
            IdlType::Array(inner, len) => {
                let len = match len {
                    ArrayLen::Generic(name) => ArrayLen::Value(self.consts[name]),
                    len => len.clone(),
                };
                IdlType::Array(Box::new(self.subst(inner)), len)
            }
            IdlType::Defined { name, generics } => IdlType::Defined {
                name: name.clone(),
                generics: generics
                    .iter()
                    .map(|arg| match arg {
                        GenericArg::Type(ty) => GenericArg::Type(self.subst(ty)),
                        GenericArg::Const(value) => GenericArg::Const(
                            self.consts
                                .get(value)
                                .map(usize::to_string)
                                .unwrap_or_else(|| value.clone()),
                        ),
                    })
                    .collect(),
            },
            ty => ty.clone(),
        }
    }
}

/// Writes a random valid Borsh value of `ty`.
pub fn borsh_value(idl: &Idl, ty: &IdlType, rng: &mut Rng, depth: usize, out: &mut Vec<u8>) {
    let deep = depth > 6;
    match ty {
        IdlType::Bool => out.push(rng.below(2) as u8),
        IdlType::U8 | IdlType::I8 => rng.bytes(1, out),
        IdlType::U16 | IdlType::I16 => rng.bytes(2, out),
        IdlType::U32 | IdlType::I32 => rng.bytes(4, out),
        IdlType::U64 | IdlType::I64 => rng.bytes(8, out),
        IdlType::U128 | IdlType::I128 => rng.bytes(16, out),
        IdlType::U256 | IdlType::I256 | IdlType::Pubkey => rng.bytes(32, out),
        IdlType::F32 => out.extend((rng.next() as i32 as f32 / 7.0).to_le_bytes()),
        IdlType::F64 => out.extend((rng.next() as i64 as f64 / 7.0).to_le_bytes()),
        IdlType::Bytes => {
            let len = rng.below(5) as usize;
            out.extend((len as u32).to_le_bytes());
            rng.bytes(len, out);
        }
        IdlType::String => {
            let len = rng.below(5) as usize;
            out.extend((len as u32).to_le_bytes());
            out.extend((0..len).map(|_| b'a' + rng.below(26) as u8));
        }
        IdlType::Option(inner) => {
            if deep || rng.below(2) == 0 {
                out.push(0);
            } else {
                out.push(1);
                borsh_value(idl, inner, rng, depth + 1, out);
            }
        }
        IdlType::COption(inner) => {
            if deep || rng.below(2) == 0 {
                out.extend(0u32.to_le_bytes());
            } else {
                out.extend(1u32.to_le_bytes());
                borsh_value(idl, inner, rng, depth + 1, out);
            }
        }
        IdlType::Map(key, value) => {
            let pair = IdlType::Tuple(vec![(**key).clone(), (**value).clone()]);
            borsh_value(idl, &IdlType::Vec(Box::new(pair)), rng, depth, out)
        }
        IdlType::Tuple(items) => {
            for item in items {
                borsh_value(idl, item, rng, depth + 1, out);
            }
        }
        IdlType::Vec(inner) | IdlType::Set(inner) => {
            let len = if deep { 0 } else { rng.below(4) as usize };
            let mut items = Vec::new();
            let mut written = 0;
            for _ in 0..len {
                let before = items.len();
                borsh_value(idl, inner, rng, depth + 1, &mut items);
                if items.len() == before {
                    // Borsh rejects vectors of zero-sized elements.
                    break;
                }
                written += 1;
            }
            out.extend((written as u32).to_le_bytes());
            out.extend(items);
        }
        IdlType::Array(inner, ArrayLen::Value(len)) => {
            for _ in 0..*len {
                borsh_value(idl, inner, rng, depth + 1, out);
            }
        }
        IdlType::Array(_, ArrayLen::Generic(name)) | IdlType::Generic(name) => {
            panic!("unsubstituted generic `{name}`")
        }
        IdlType::Defined { name, generics } => {
            let def = idl.type_def(name).expect("defined type");
            let mut env = Env::default();
            for (param, arg) in def.generics.iter().zip(generics) {
                match (param, arg) {
                    (GenericParam::Type { name }, GenericArg::Type(ty)) => {
                        env.types.insert(name.clone(), ty.clone());
                    }
                    (GenericParam::Const { name, .. }, GenericArg::Const(value)) => {
                        env.consts
                            .insert(name.clone(), value.parse().expect("const value"));
                    }
                    _ => panic!("generic argument kind mismatch in `{name}`"),
                }
            }
            type_def_value(idl, def, &env, rng, depth + 1, out);
        }
    }
}

pub fn type_def_value(
    idl: &Idl,
    def: &TypeDef,
    env: &Env,
    rng: &mut Rng,
    depth: usize,
    out: &mut Vec<u8>,
) {
    match &def.kind {
        TypeDefKind::Alias(ty) => borsh_value(idl, &env.subst(ty), rng, depth, out),
        TypeDefKind::Struct(fields) => fields_value(idl, fields, env, rng, depth, out),
        TypeDefKind::Enum(variants) => {
            let index = rng.below(variants.len() as u64) as usize;
            out.push(index as u8);
            fields_value(idl, &variants[index].fields, env, rng, depth, out);
        }
    }
}

pub fn fields_value(
    idl: &Idl,
    fields: &Fields,
    env: &Env,
    rng: &mut Rng,
    depth: usize,
    out: &mut Vec<u8>,
) {
    match fields {
        Fields::Unit => {}
        Fields::Named(fields) => {
            for field in fields {
                borsh_value(idl, &env.subst(&field.ty), rng, depth, out);
            }
        }
        Fields::Tuple(types) => {
            for ty in types {
                borsh_value(idl, &env.subst(ty), rng, depth, out);
            }
        }
    }
}

/// Writes a random raw memory layout of the zero-copy struct `def`, padding
/// zeroed.
pub fn zero_copy_value(idl: &Idl, def: &TypeDef, rng: &mut Rng, out: &mut Vec<u8>) {
    let layout = layout::struct_layout(idl, def).expect("zero-copy layout");
    let start = out.len();
    out.resize(start + layout.size, 0);
    let types: Vec<&IdlType> = match &def.kind {
        TypeDefKind::Struct(Fields::Named(fields)) => fields.iter().map(|f| &f.ty).collect(),
        TypeDefKind::Struct(Fields::Tuple(types)) => types.iter().collect(),
        _ => Vec::new(),
    };
    for (field, ty) in layout.fields.iter().zip(types) {
        let mut bytes = Vec::new();
        zero_copy_field(idl, ty, rng, &mut bytes);
        assert_eq!(bytes.len(), field.size, "{}.{}", def.name, field.name);
        out[start + field.offset..start + field.offset + field.size].copy_from_slice(&bytes);
    }
}

pub fn zero_copy_field(idl: &Idl, ty: &IdlType, rng: &mut Rng, out: &mut Vec<u8>) {
    match ty {
        IdlType::Array(inner, ArrayLen::Value(len)) => {
            for _ in 0..*len {
                zero_copy_field(idl, inner, rng, out);
            }
        }
        IdlType::Defined { name, .. } => {
            let def = idl.type_def(name).expect("defined type");
            match &def.kind {
                TypeDefKind::Alias(target) => zero_copy_field(idl, target, rng, out),
                _ => zero_copy_value(idl, def, rng, out),
            }
        }
        // Primitives: the raw bytes are the Borsh bytes.
        ty => borsh_value(idl, ty, rng, 0, out),
    }
}

pub fn keys(rng: &mut Rng, n: usize) -> Vec<[u8; 32]> {
    (0..n)
        .map(|_| {
            let mut key = Vec::new();
            rng.bytes(32, &mut key);
            key.try_into().unwrap()
        })
        .collect()
}
