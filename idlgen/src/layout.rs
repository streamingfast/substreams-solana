//! Memory layout of zero-copy types (IDL serialization `bytemuck` or
//! `bytemuckunsafe`), stored on chain as raw struct memory after the discriminator.
//!
//! - `repr(C)`: fields in order, each aligned; size rounded up to the struct alignment.
//! - `align(n)`: raises the struct alignment to at least `n`.
//! - `packed`: no padding, alignment 1; rustc never reorders packed fields.
//! - `repr(transparent)`: the layout of its single field.
//!
//! Sizes and alignments are those of Solana's SBF target. Plain `repr(Rust)`,
//! enums, `u256`/`i256`, generics and variable-size types are rejected.

use std::fmt;

use crate::idl::{ArrayLen, Fields, Idl, IdlType, Repr, TypeDef, TypeDefKind};

/// Alignment of `u128` and `i128` on Solana's SBF target, where hosts may use 16.
/// Source: `solana_program_entrypoint::BPF_ALIGN_OF_U128`.
pub const U128_ALIGN: usize = 8;

/// Size and alignment of a type, in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    pub size: usize,
    pub align: usize,
}

/// Layout of a struct and the offset of each field, in declaration order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructLayout {
    pub size: usize,
    pub align: usize,
    pub fields: Vec<FieldLayout>,
}

/// Where a field sits in its struct.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldLayout {
    /// The field name, or its index for a tuple struct.
    pub name: String,
    pub offset: usize,
    pub size: usize,
}

/// Why a type has no zero-copy layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutError {
    /// The type and field where the problem is, such as `Bank.config.rates`.
    pub path: String,
    pub message: String,
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.path, self.message)
    }
}

impl std::error::Error for LayoutError {}

fn err<T>(path: &str, message: impl Into<String>) -> Result<T, LayoutError> {
    Err(LayoutError {
        path: path.to_string(),
        message: message.into(),
    })
}

fn round_up(n: usize, align: usize) -> usize {
    n.div_ceil(align) * align
}

/// The layout of a struct type definition.
pub fn struct_layout(idl: &Idl, def: &TypeDef) -> Result<StructLayout, LayoutError> {
    Computer {
        idl,
        visiting: Vec::new(),
    }
    .struct_layout(def, &def.name)
}

struct Computer<'a> {
    idl: &'a Idl,
    /// Type names being laid out, to reject a type that contains itself.
    visiting: Vec<&'a str>,
}

impl<'a> Computer<'a> {
    fn struct_layout(&mut self, def: &'a TypeDef, path: &str) -> Result<StructLayout, LayoutError> {
        if !def.generics.is_empty() {
            return err(
                path,
                "generic types have no fixed zero-copy layout in the IDL",
            );
        }
        let fields = match &def.kind {
            TypeDefKind::Struct(fields) => fields,
            TypeDefKind::Enum(_) => {
                return err(
                    path,
                    "enums have no zero-copy layout: the IDL does not record the size of the discriminant",
                )
            }
            TypeDefKind::Alias(_) => {
                return err(path, "a type alias has no layout of its own")
            }
        };
        let field_types: Vec<(String, &IdlType)> = match fields {
            Fields::Unit => Vec::new(),
            Fields::Named(fields) => fields.iter().map(|f| (f.name.clone(), &f.ty)).collect(),
            Fields::Tuple(types) => types
                .iter()
                .enumerate()
                .map(|(i, ty)| (i.to_string(), ty))
                .collect(),
        };

        let (packed, min_align) = match &def.repr {
            Some(Repr::C(modifier)) => (modifier.packed, modifier.align),
            // rustc does not reorder the fields of a packed struct.
            Some(Repr::Rust(modifier)) if modifier.packed => (true, modifier.align),
            Some(Repr::Transparent) => {
                if field_types.len() != 1 {
                    return err(path, "repr(transparent) needs exactly one field");
                }
                let (name, ty) = &field_types[0];
                let layout = self.type_layout(ty, &format!("{path}.{name}"))?;
                return Ok(StructLayout {
                    size: layout.size,
                    align: layout.align,
                    fields: vec![FieldLayout {
                        name: name.clone(),
                        offset: 0,
                        size: layout.size,
                    }],
                });
            }
            Some(Repr::Rust(_)) | None => {
                return err(
                    path,
                    "the type has no #[repr(C)], #[repr(packed)] or #[repr(transparent)], so its memory layout is unspecified",
                )
            }
        };
        if packed && min_align.is_some() {
            return err(path, "a struct cannot be both packed and aligned");
        }
        if let Some(align) = min_align {
            if !align.is_power_of_two() {
                return err(path, format!("alignment {align} is not a power of two"));
            }
        }

        let mut offset = 0usize;
        let mut align = 1usize;
        let mut out = Vec::with_capacity(field_types.len());
        for (name, ty) in field_types {
            let layout = self.type_layout(ty, &format!("{path}.{name}"))?;
            let field_align = if packed { 1 } else { layout.align };
            offset = round_up(offset, field_align);
            out.push(FieldLayout {
                name,
                offset,
                size: layout.size,
            });
            offset += layout.size;
            align = align.max(field_align);
        }
        if let Some(min) = min_align {
            align = align.max(min);
        }
        Ok(StructLayout {
            size: round_up(offset, align),
            align,
            fields: out,
        })
    }

    fn type_layout(&mut self, ty: &'a IdlType, path: &str) -> Result<Layout, LayoutError> {
        let primitive = |size: usize| Ok(Layout { size, align: size });
        match ty {
            IdlType::Bool | IdlType::U8 | IdlType::I8 => primitive(1),
            IdlType::U16 | IdlType::I16 => primitive(2),
            IdlType::U32 | IdlType::I32 | IdlType::F32 => primitive(4),
            IdlType::U64 | IdlType::I64 | IdlType::F64 => primitive(8),
            IdlType::U128 | IdlType::I128 => Ok(Layout {
                size: 16,
                align: U128_ALIGN,
            }),
            IdlType::Pubkey => Ok(Layout { size: 32, align: 1 }),
            IdlType::Array(element, len) => {
                let len = match len {
                    ArrayLen::Value(len) => *len,
                    ArrayLen::Generic(name) => {
                        return err(path, format!("array length is the generic parameter `{name}`"))
                    }
                };
                let element = self.type_layout(element, path)?;
                let size = element.size.checked_mul(len).ok_or_else(|| LayoutError {
                    path: path.to_string(),
                    message: "array size overflows usize".to_string(),
                })?;
                Ok(Layout {
                    size,
                    align: element.align,
                })
            }
            IdlType::Defined { name, generics } => {
                if !generics.is_empty() {
                    return err(
                        path,
                        format!("generic type `{name}` has no fixed zero-copy layout in the IDL"),
                    );
                }
                let Some(def) = self.idl.type_def(name) else {
                    return err(path, format!("unknown type `{name}`"));
                };
                if let TypeDefKind::Alias(target) = &def.kind {
                    if !def.generics.is_empty() {
                        return err(path, format!("generic alias `{name}` has no fixed layout"));
                    }
                    return self.with_type(name, path, |c| c.type_layout(target, path));
                }
                self.with_type(name, path, |c| {
                    let layout = c.struct_layout(def, &format!("{path} ({name})"))?;
                    Ok(Layout {
                        size: layout.size,
                        align: layout.align,
                    })
                })
            }
            IdlType::U256 | IdlType::I256 => err(
                path,
                "u256/i256 have no zero-copy layout: their alignment depends on the program's Rust type",
            ),
            IdlType::Tuple(_) => err(path, "Rust tuples have no defined memory layout"),
            IdlType::Bytes
            | IdlType::String
            | IdlType::Vec(_)
            | IdlType::Map(..)
            | IdlType::Set(_)
            | IdlType::Option(_)
            | IdlType::COption(_) => err(
                path,
                "variable-size types (bytes, string, vec, option, coption) cannot be part of a zero-copy layout",
            ),
            IdlType::Generic(name) => err(path, format!("generic parameter `{name}` has no fixed layout")),
        }
    }

    fn with_type<T>(
        &mut self,
        name: &'a str,
        path: &str,
        f: impl FnOnce(&mut Self) -> Result<T, LayoutError>,
    ) -> Result<T, LayoutError> {
        if self.visiting.contains(&name) {
            return err(path, format!("type `{name}` contains itself"));
        }
        self.visiting.push(name);
        let result = f(self);
        self.visiting.pop();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::idl::{Field, ReprModifier, Serialization};

    fn field(name: &str, ty: IdlType) -> Field {
        Field {
            name: name.to_string(),
            docs: Vec::new(),
            ty,
        }
    }

    fn def(name: &str, repr: Option<Repr>, fields: Vec<Field>) -> TypeDef {
        TypeDef {
            name: name.to_string(),
            docs: Vec::new(),
            generics: Vec::new(),
            serialization: Serialization::Bytemuck,
            repr,
            kind: TypeDefKind::Struct(Fields::Named(fields)),
        }
    }

    fn c() -> Option<Repr> {
        Some(Repr::C(ReprModifier::default()))
    }

    fn idl(types: Vec<TypeDef>) -> Idl {
        Idl {
            address: None,
            name: "test".to_string(),
            version: None,
            description: None,
            format: crate::idl::IdlFormat::Spec("0.1.0".to_string()),
            origin: crate::idl::IdlOrigin::Anchor,
            docs: Vec::new(),
            instructions: Vec::new(),
            accounts: Vec::new(),
            events: Vec::new(),
            types,
            errors: Vec::new(),
            constants: Vec::new(),
        }
    }

    fn defined(name: &str) -> IdlType {
        IdlType::Defined {
            name: name.to_string(),
            generics: Vec::new(),
        }
    }

    fn offsets(layout: &StructLayout) -> Vec<(&str, usize, usize)> {
        layout
            .fields
            .iter()
            .map(|f| (f.name.as_str(), f.offset, f.size))
            .collect()
    }

    #[test]
    fn repr_c_mixed_sizes() {
        // struct { a: u8, b: u64, c: u16, d: u128, e: bool }
        // a@0, pad 7, b@8, c@16, pad 6, d@24 (u128 aligned to 8), e@40, tail pad 7 -> 48.
        let mixed = def(
            "Mixed",
            c(),
            vec![
                field("a", IdlType::U8),
                field("b", IdlType::U64),
                field("c", IdlType::U16),
                field("d", IdlType::U128),
                field("e", IdlType::Bool),
            ],
        );
        let layout = struct_layout(&idl(vec![mixed.clone()]), &mixed).unwrap();
        assert_eq!(
            offsets(&layout),
            vec![
                ("a", 0, 1),
                ("b", 8, 8),
                ("c", 16, 2),
                ("d", 24, 16),
                ("e", 40, 1)
            ]
        );
        assert_eq!((layout.size, layout.align), (48, 8));
    }

    #[test]
    fn repr_c_small_alignment_and_pubkey() {
        // struct { flag: u8, key: Pubkey, n: u32, x: u16 }: flag@0, key@1, pad 3, n@36, x@40,
        // tail pad 2 -> 44, align 4.
        let s = def(
            "S",
            c(),
            vec![
                field("flag", IdlType::U8),
                field("key", IdlType::Pubkey),
                field("n", IdlType::U32),
                field("x", IdlType::U16),
            ],
        );
        let layout = struct_layout(&idl(vec![s.clone()]), &s).unwrap();
        assert_eq!(
            offsets(&layout),
            vec![("flag", 0, 1), ("key", 1, 32), ("n", 36, 4), ("x", 40, 2)]
        );
        assert_eq!((layout.size, layout.align), (44, 4));
    }

    #[test]
    fn packed_has_no_padding() {
        let modifier = ReprModifier {
            packed: true,
            align: None,
        };
        for repr in [Repr::Rust(modifier.clone()), Repr::C(modifier)] {
            let p = def(
                "P",
                Some(repr),
                vec![
                    field("a", IdlType::U8),
                    field("b", IdlType::U64),
                    field("c", IdlType::U16),
                ],
            );
            let layout = struct_layout(&idl(vec![p.clone()]), &p).unwrap();
            assert_eq!(
                offsets(&layout),
                vec![("a", 0, 1), ("b", 1, 8), ("c", 9, 2)]
            );
            assert_eq!((layout.size, layout.align), (11, 1));
        }
    }

    #[test]
    fn nested_and_arrays() {
        // Inner { a: u64, b: u8 } -> size 16, align 8.
        // Outer { x: u8, inner: Inner, list: [Inner; 3], tail: [u16; 3] }
        // x@0, inner@8, list@24 (48 bytes), tail@72 (6 bytes) -> 78, tail pad -> 80.
        let inner = def(
            "Inner",
            c(),
            vec![field("a", IdlType::U64), field("b", IdlType::U8)],
        );
        let outer = def(
            "Outer",
            c(),
            vec![
                field("x", IdlType::U8),
                field("inner", defined("Inner")),
                field(
                    "list",
                    IdlType::Array(Box::new(defined("Inner")), ArrayLen::Value(3)),
                ),
                field(
                    "tail",
                    IdlType::Array(Box::new(IdlType::U16), ArrayLen::Value(3)),
                ),
            ],
        );
        let idl = idl(vec![inner.clone(), outer.clone()]);
        assert_eq!(struct_layout(&idl, &inner).unwrap().size, 16);
        let layout = struct_layout(&idl, &outer).unwrap();
        assert_eq!(
            offsets(&layout),
            vec![
                ("x", 0, 1),
                ("inner", 8, 16),
                ("list", 24, 48),
                ("tail", 72, 6)
            ]
        );
        assert_eq!((layout.size, layout.align), (80, 8));
    }

    #[test]
    fn packed_nested_in_c_has_alignment_one() {
        let packed = def(
            "Packed",
            Some(Repr::Rust(ReprModifier {
                packed: true,
                align: None,
            })),
            vec![field("a", IdlType::U8), field("b", IdlType::U32)],
        );
        let outer = def(
            "Outer",
            c(),
            vec![
                field("x", IdlType::U8),
                field("p", defined("Packed")),
                field("y", IdlType::U16),
            ],
        );
        let idl = idl(vec![packed, outer.clone()]);
        let layout = struct_layout(&idl, &outer).unwrap();
        assert_eq!(
            offsets(&layout),
            vec![("x", 0, 1), ("p", 1, 5), ("y", 6, 2)]
        );
        assert_eq!((layout.size, layout.align), (8, 2));
    }

    #[test]
    fn align_modifier() {
        let a = def(
            "A",
            Some(Repr::C(ReprModifier {
                packed: false,
                align: Some(16),
            })),
            vec![field("a", IdlType::U32), field("b", IdlType::U8)],
        );
        let layout = struct_layout(&idl(vec![a.clone()]), &a).unwrap();
        assert_eq!((layout.size, layout.align), (16, 16));
    }

    #[test]
    fn rejects_what_has_no_fixed_layout() {
        let cases = [
            (IdlType::Vec(Box::new(IdlType::U8)), "variable-size"),
            (IdlType::String, "variable-size"),
            (IdlType::Option(Box::new(IdlType::U8)), "variable-size"),
            (IdlType::U256, "u256"),
        ];
        for (ty, message) in cases {
            let s = def("S", c(), vec![field("ok", IdlType::U8), field("bad", ty)]);
            let error = struct_layout(&idl(vec![s.clone()]), &s).unwrap_err();
            assert_eq!(error.path, "S.bad");
            assert!(error.message.contains(message), "{error}");
        }

        let unspecified = def("R", None, vec![field("a", IdlType::U8)]);
        let error = struct_layout(&idl(vec![unspecified.clone()]), &unspecified).unwrap_err();
        assert!(error.message.contains("unspecified"), "{error}");

        let looped = def(
            "L",
            c(),
            vec![field(
                "me",
                IdlType::Array(Box::new(defined("L")), ArrayLen::Value(1)),
            )],
        );
        let error = struct_layout(&idl(vec![looped.clone()]), &looped).unwrap_err();
        assert!(error.message.contains("contains itself"), "{error}");
    }
}
