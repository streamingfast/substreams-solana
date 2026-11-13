//! Constructs the sample IDLs do not cover, the two formats normalizing to the
//! same model, and errors naming their JSON path.

use substreams_solana_idlgen::idl::{
    ArrayLen, EnumVariant, Field, Fields, GenericArg, GenericParam, Idl, IdlError, IdlType,
    TypeDefKind,
};

const ADDRESS: &str = "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB";

fn read(json: &str) -> Idl {
    Idl::from_json(json).unwrap_or_else(|err| panic!("{err}"))
}

fn defined(name: &str) -> IdlType {
    IdlType::Defined {
        name: name.to_string(),
        generics: Vec::new(),
    }
}

fn field(name: &str, ty: IdlType) -> Field {
    Field {
        name: name.to_string(),
        docs: Vec::new(),
        ty,
    }
}

fn legacy() -> String {
    format!(
        r#"{{
        "version": "0.1.0",
        "name": "demo",
        "metadata": {{ "address": "{ADDRESS}" }},
        "instructions": [{{
            "name": "swapBaseIn",
            "accounts": [
                {{ "name": "userAuthority", "isMut": false, "isSigner": true }},
                {{ "name": "pool", "accounts": [
                    {{ "name": "poolState", "isMut": true, "isSigner": false }},
                    {{ "name": "rewards", "isMut": false, "isSigner": false, "isOptional": true }}
                ] }}
            ],
            "args": [
                {{ "name": "amountIn", "type": "u64" }},
                {{ "name": "route", "type": {{ "vec": {{ "option": {{ "defined": "Leg" }} }} }} }},
                {{ "name": "owner", "type": {{ "option": {{ "array": ["u8", 32] }} }} }},
                {{ "name": "wrapped", "type": {{ "definedWithTypeArgs": {{ "name": "Wrapper", "args": [
                    {{ "type": "u64" }}, {{ "value": "4" }}
                ] }} }} }}
            ]
        }}],
        "accounts": [{{
            "name": "PoolState",
            "type": {{ "kind": "struct", "fields": [
                {{ "name": "authority", "type": "publicKey" }},
                {{ "name": "totalAmount", "type": "u128" }}
            ] }}
        }}],
        "events": [{{
            "name": "Swapped",
            "fields": [{{ "name": "amountIn", "type": "u64", "index": false }}]
        }}],
        "types": [
            {{ "name": "Leg", "type": {{ "kind": "enum", "variants": [
                {{ "name": "Empty" }},
                {{ "name": "Named", "fields": [{{ "name": "minOut", "type": "u64" }}] }},
                {{ "name": "Tuple", "fields": ["u8", {{ "defined": "Inner" }}] }}
            ] }} }},
            {{ "name": "Inner", "type": {{ "kind": "struct", "fields": [{{ "name": "x", "type": "i16" }}] }} }},
            {{ "name": "Wrapper", "generics": ["T", "N"], "type": {{ "kind": "struct", "fields": [
                {{ "name": "items", "type": {{ "genericLenArray": [{{ "generic": "T" }}, "N"] }} }},
                {{ "name": "last", "type": {{ "option": {{ "generic": "T" }} }} }}
            ] }} }}
        ],
        "errors": [{{ "code": 6000, "name": "Slippage", "msg": "too much" }}],
        "constants": [{{ "name": "SEED", "type": "string", "value": "\"pool\"" }}]
    }}"#
    )
}

fn spec() -> String {
    format!(
        r#"{{
        "address": "{ADDRESS}",
        "metadata": {{ "name": "demo", "version": "0.1.0", "spec": "0.1.0" }},
        "instructions": [{{
            "name": "swap_base_in",
            "discriminator": [42, 236, 72, 162, 242, 24, 39, 84],
            "accounts": [
                {{ "name": "user_authority", "signer": true }},
                {{ "name": "pool", "accounts": [
                    {{ "name": "pool_state", "writable": true }},
                    {{ "name": "rewards", "optional": true }}
                ] }}
            ],
            "args": [
                {{ "name": "amount_in", "type": "u64" }},
                {{ "name": "route", "type": {{ "vec": {{ "option": {{ "defined": {{ "name": "Leg" }} }} }} }} }},
                {{ "name": "owner", "type": {{ "option": {{ "array": ["u8", 32] }} }} }},
                {{ "name": "wrapped", "type": {{ "defined": {{ "name": "Wrapper", "generics": [
                    {{ "kind": "type", "type": "u64" }}, {{ "kind": "const", "value": "4" }}
                ] }} }} }}
            ]
        }}],
        "accounts": [{{ "name": "PoolState", "discriminator": [247, 237, 227, 245, 215, 195, 222, 70] }}],
        "events": [{{ "name": "Swapped", "discriminator": [217, 52, 52, 83, 147, 135, 96, 109] }}],
        "types": [
            {{ "name": "Leg", "type": {{ "kind": "enum", "variants": [
                {{ "name": "Empty" }},
                {{ "name": "Named", "fields": [{{ "name": "min_out", "type": "u64" }}] }},
                {{ "name": "Tuple", "fields": ["u8", {{ "defined": {{ "name": "Inner" }} }}] }}
            ] }} }},
            {{ "name": "Inner", "type": {{ "kind": "struct", "fields": [{{ "name": "x", "type": "i16" }}] }} }},
            {{ "name": "Wrapper",
               "generics": [{{ "kind": "type", "name": "T" }}, {{ "kind": "const", "name": "N", "type": "usize" }}],
               "type": {{ "kind": "struct", "fields": [
                {{ "name": "items", "type": {{ "array": [{{ "generic": "T" }}, {{ "generic": "N" }}] }} }},
                {{ "name": "last", "type": {{ "option": {{ "generic": "T" }} }} }}
            ] }} }},
            {{ "name": "PoolState", "type": {{ "kind": "struct", "fields": [
                {{ "name": "authority", "type": "pubkey" }},
                {{ "name": "total_amount", "type": "u128" }}
            ] }} }},
            {{ "name": "Swapped", "type": {{ "kind": "struct", "fields": [{{ "name": "amount_in", "type": "u64" }}] }} }}
        ],
        "errors": [{{ "code": 6000, "name": "Slippage", "msg": "too much" }}],
        "constants": [{{ "name": "SEED", "type": "string", "value": "\"pool\"" }}]
    }}"#
    )
}

#[test]
fn legacy_and_spec_normalize_to_the_same_model() {
    let legacy = read(&legacy());
    let spec = read(&spec());

    assert_eq!(legacy.address, spec.address);
    assert_eq!(legacy.name, spec.name);
    assert_eq!(legacy.version, spec.version);
    assert_eq!(legacy.instructions, spec.instructions);
    assert_eq!(legacy.accounts, spec.accounts);
    assert_eq!(legacy.events, spec.events);
    assert_eq!(legacy.errors, spec.errors);
    assert_eq!(legacy.constants, spec.constants);
    for name in ["Leg", "Inner", "Wrapper", "PoolState"] {
        assert_eq!(legacy.type_def(name), spec.type_def(name), "{name}");
    }
    // The only difference: a legacy event layout is not among the types.
    assert!(legacy.type_def("Swapped").is_none());
    assert_eq!(spec.type_def("Swapped"), Some(&spec.events[0].layout));
}

#[test]
fn instruction_model() {
    let idl = read(&spec());
    let ix = &idl.instructions[0];

    let names: Vec<_> = ix.accounts.iter().map(|a| a.qualified_name()).collect();
    assert_eq!(names, ["user_authority", "pool.pool_state", "pool.rewards"]);
    assert!(ix.accounts[0].signer && !ix.accounts[0].writable);
    assert!(ix.accounts[1].writable && !ix.accounts[1].optional);
    assert!(ix.accounts[2].optional);

    assert_eq!(
        ix.args[1].ty,
        IdlType::Vec(Box::new(IdlType::Option(Box::new(defined("Leg")))))
    );
    assert_eq!(
        ix.args[2].ty,
        IdlType::Option(Box::new(IdlType::Array(
            Box::new(IdlType::U8),
            ArrayLen::Value(32)
        )))
    );
    assert_eq!(
        ix.args[3].ty,
        IdlType::Defined {
            name: "Wrapper".into(),
            generics: vec![
                GenericArg::Type(IdlType::U64),
                GenericArg::Const("4".into())
            ],
        }
    );
}

#[test]
fn enums_and_generics() {
    let idl = read(&legacy());

    let TypeDefKind::Enum(variants) = &idl.type_def("Leg").unwrap().kind else {
        panic!("Leg is not an enum");
    };
    assert_eq!(
        variants,
        &[
            EnumVariant {
                name: "Empty".into(),
                fields: Fields::Unit
            },
            EnumVariant {
                name: "Named".into(),
                fields: Fields::Named(vec![field("min_out", IdlType::U64)])
            },
            EnumVariant {
                name: "Tuple".into(),
                fields: Fields::Tuple(vec![IdlType::U8, defined("Inner")])
            },
        ]
    );

    // Legacy generics are told apart by use: `N` is an array length.
    let wrapper = idl.type_def("Wrapper").unwrap();
    assert_eq!(
        wrapper.generics,
        [
            GenericParam::Type { name: "T".into() },
            GenericParam::Const {
                name: "N".into(),
                ty: "usize".into()
            },
        ]
    );
    assert_eq!(
        wrapper.kind,
        TypeDefKind::Struct(Fields::Named(vec![
            field(
                "items",
                IdlType::Array(
                    Box::new(IdlType::Generic("T".into())),
                    ArrayLen::Generic("N".into())
                )
            ),
            field(
                "last",
                IdlType::Option(Box::new(IdlType::Generic("T".into())))
            ),
        ]))
    );
}

/// A minimal 0.30 IDL with the given types and instruction arguments.
fn spec_with(types: &str, args: &str) -> String {
    format!(
        r#"{{
        "address": "{ADDRESS}",
        "metadata": {{ "name": "demo", "version": "0.1.0", "spec": "0.1.0" }},
        "instructions": [{{ "name": "go", "discriminator": [1], "accounts": [], "args": [{args}] }}],
        "types": [{types}]
    }}"#
    )
}

#[test]
fn deep_nesting() {
    let idl = read(&spec_with(
        r#"{ "name": "Cell", "type": { "kind": "struct", "fields": [
            { "name": "grid", "type": { "array": [{ "array": ["u16", 4] }, 8] } },
            { "name": "nested", "type": { "vec": { "option": { "vec": { "option": { "array": ["pubkey", 2] } } } } } },
            { "name": "c", "type": { "coption": "pubkey" } }
        ] } },
        { "name": "Pair", "generics": [{ "kind": "type", "name": "A" }, { "kind": "type", "name": "B" }],
          "type": { "kind": "struct", "fields": [{ "generic": "A" }, { "generic": "B" }] } },
        { "name": "Id", "type": { "kind": "type", "alias": { "array": ["u8", 16] } } },
        { "name": "Unit", "type": { "kind": "struct" } }"#,
        r#"{ "name": "p", "type": { "option": { "defined": { "name": "Pair", "generics": [
            { "kind": "type", "type": { "vec": { "defined": "Cell" } } },
            { "kind": "type", "type": { "defined": { "name": "Pair", "generics": [
                { "kind": "type", "type": "u8" }, { "kind": "type", "type": { "defined": "Id" } }
            ] } } }
        ] } } } }"#,
    ));

    let u8_array = |len| IdlType::Array(Box::new(IdlType::U8), ArrayLen::Value(len));
    let TypeDefKind::Struct(Fields::Named(fields)) = &idl.type_def("Cell").unwrap().kind else {
        panic!("Cell is not a struct with named fields");
    };
    assert_eq!(
        fields[0].ty,
        IdlType::Array(
            Box::new(IdlType::Array(Box::new(IdlType::U16), ArrayLen::Value(4))),
            ArrayLen::Value(8)
        )
    );
    assert_eq!(
        fields[1].ty,
        IdlType::Vec(Box::new(IdlType::Option(Box::new(IdlType::Vec(Box::new(
            IdlType::Option(Box::new(IdlType::Array(
                Box::new(IdlType::Pubkey),
                ArrayLen::Value(2)
            )))
        ))))))
    );
    assert_eq!(fields[2].ty, IdlType::COption(Box::new(IdlType::Pubkey)));

    assert_eq!(
        idl.type_def("Pair").unwrap().kind,
        TypeDefKind::Struct(Fields::Tuple(vec![
            IdlType::Generic("A".into()),
            IdlType::Generic("B".into())
        ]))
    );
    assert_eq!(
        idl.type_def("Id").unwrap().kind,
        TypeDefKind::Alias(u8_array(16))
    );
    assert_eq!(
        idl.type_def("Unit").unwrap().kind,
        TypeDefKind::Struct(Fields::Unit)
    );

    assert_eq!(
        idl.instructions[0].args[0].ty,
        IdlType::Option(Box::new(IdlType::Defined {
            name: "Pair".into(),
            generics: vec![
                GenericArg::Type(IdlType::Vec(Box::new(defined("Cell")))),
                GenericArg::Type(IdlType::Defined {
                    name: "Pair".into(),
                    generics: vec![
                        GenericArg::Type(IdlType::U8),
                        GenericArg::Type(defined("Id"))
                    ],
                }),
            ],
        }))
    );
}

fn error(json: &str) -> (String, String) {
    match Idl::from_json(json) {
        Err(IdlError::Invalid { path, message }) => (path, message),
        Err(other) => panic!("expected an invalid IDL error, got {other}"),
        Ok(_) => panic!("expected an error"),
    }
}

#[test]
fn errors_name_the_json_path() {
    let (path, message) = error(&spec_with(
        "",
        r#"{ "name": "a", "type": "u8" }, { "name": "b", "type": "u512" }"#,
    ));
    assert_eq!(path, "instructions[0].args[1].type");
    assert!(message.contains("u512"), "{message}");

    let (path, _) = error(&spec_with(
        "",
        r#"{ "name": "a", "type": { "vec": { "option": { "linkedList": ["u8"] } } } }"#,
    ));
    assert_eq!(path, "instructions[0].args[0].type.vec.option");

    let (path, message) = error(&spec_with(
        "",
        r#"{ "name": "a", "type": { "defined": { "name": "Missing" } } }"#,
    ));
    assert_eq!(path, "instructions[0].args[0].type");
    assert!(message.contains("undefined type `Missing`"), "{message}");

    let (path, _) = error(&spec_with(
        "",
        r#"{ "name": "a", "type": "u8", "extra": true }"#,
    ));
    assert_eq!(path, "instructions[0].args[0].extra");

    let (path, message) = error(&spec_with(
        r#"{ "name": "S", "type": { "kind": "struct", "fields": [{ "name": "x", "type": { "generic": "T" } }] } }"#,
        "",
    ));
    assert_eq!(path, "types[0].type.fields[0].type");
    assert!(message.contains("`T`"), "{message}");

    let (path, message) = error(&spec_with(
        r#"{ "name": "G", "generics": [{ "kind": "type", "name": "T" }], "type": { "kind": "struct", "fields": [{ "generic": "T" }] } }"#,
        r#"{ "name": "a", "type": { "defined": { "name": "G" } } }"#,
    ));
    assert_eq!(path, "instructions[0].args[0].type");
    assert!(
        message.contains("takes 1 generic arguments, 0 given"),
        "{message}"
    );

    let (path, _) = error(&spec_with(
        r#"{ "name": "E", "type": { "kind": "enum", "variants": [{ "name": "V", "fields": [{ "name": "x", "type": "u8" }, "u8"] }] } }"#,
        "",
    ));
    assert_eq!(path, "types[0].type.variants[0].fields[1]");

    let (path, _) = error(&spec_with(
        r#"{ "name": "S", "serialization": "zerocopy", "type": { "kind": "struct" } }"#,
        "",
    ));
    assert_eq!(path, "types[0].serialization");

    let (path, _) = error(&spec().replace("[42, 236, 72", "[420, 236, 72"));
    assert_eq!(path, "instructions[0].discriminator[0]");

    let (path, _) = error(&spec().replace(r#""spec": "0.1.0""#, r#""spec": "0.2.0""#));
    assert_eq!(path, "metadata.spec");

    let (path, message) = error(&spec().replace(
        r#""name": "Swapped", "discriminator""#,
        r#""name": "Gone", "discriminator""#,
    ));
    assert_eq!(path, "events[0]");
    assert!(message.contains("`Gone`"), "{message}");

    let (path, _) =
        error(&legacy().replace(r#""isMut": false, "isSigner": true"#, r#""isSigner": true"#));
    assert_eq!(path, "instructions[0].accounts[0].isMut");

    let (path, _) = error(&legacy().replace(r#""index": false"#, r#""index": false, "docs": []"#));
    assert_eq!(path, "events[0].fields[0].docs");

    let (path, message) = error(&legacy().replace(r#"["T", "N"]"#, r#"["T", "N", "M"]"#));
    assert_eq!(path, "types[2].generics[2]");
    assert!(message.contains("`M`"), "{message}");
}

fn shank(instruction: &str) -> String {
    format!(
        r#"{{"version": "0.1.0", "name": "s",
        "metadata": {{ "origin": "shank", "address": "{ADDRESS}" }},
        "instructions": [{instruction}],
        "types": []}}"#
    )
}

#[test]
fn shank_account_flags_and_desc() {
    let idl = read(&shank(
        r#"{"name": "doIt", "discriminant": {"type": "u8", "value": 1},
            "legacyOptionalAccountsStrategy": true, "args": [],
            "accounts": [
                {"name": "a", "isMut": true, "isSigner": true, "isOptionalSigner": true,
                 "desc": "The authority"},
                {"name": "b", "isMut": false, "isSigner": false, "isOptional": true}
            ]}"#,
    ));
    let ix = &idl.instructions[0];
    assert!(ix.legacy_optional_accounts_strategy);
    assert!(ix.accounts[0].optional_signer);
    assert_eq!(ix.accounts[0].docs, ["The authority"]);
    assert!(ix.accounts[1].optional && !ix.accounts[1].optional_signer);
}

#[test]
fn shank_maps_tuples_and_builtin_aliases() {
    let idl = read(&shank(
        r#"{"name": "doIt", "discriminant": {"type": "u8", "value": 1}, "accounts": [],
            "args": [
                {"name": "m", "type": {"hashMap": ["string", "u64"]}},
                {"name": "s", "type": {"bTreeSet": "u8"}},
                {"name": "p", "type": {"tuple": ["u8", "u64"]}},
                {"name": "t", "type": {"defined": "UnixTimestamp"}},
                {"name": "e", "type": {"defined": "Epoch"}},
                {"name": "f", "type": {"defined": "f64"}}
            ]}"#,
    ));
    let types: Vec<&IdlType> = idl.instructions[0].args.iter().map(|a| &a.ty).collect();
    assert_eq!(
        types,
        [
            &IdlType::Map(Box::new(IdlType::String), Box::new(IdlType::U64)),
            &IdlType::Set(Box::new(IdlType::U8)),
            &IdlType::Tuple(vec![IdlType::U8, IdlType::U64]),
            &IdlType::I64,
            &IdlType::U64,
            &IdlType::F64,
        ]
    );
}

#[test]
fn codama_idls_are_rejected() {
    let err = Idl::from_json(r#"{"kind": "rootNode", "standard": "codama", "version": "1.0.0"}"#)
        .unwrap_err()
        .to_string();
    assert!(err.contains("Codama IDLs are not supported"), "{err}");
}
