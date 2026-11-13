//! Generation errors name the IDL item at fault.

use substreams_solana_idlgen::build::Idlgen;

fn idl(address: Option<&str>, types: &str) -> String {
    let address = address
        .map(|a| format!(r#""address": "{a}","#))
        .unwrap_or_default();
    format!(
        r#"{{
        {address}
        "metadata": {{ "name": "demo", "version": "0.1.0", "spec": "0.1.0" }},
        "instructions": [],
        "types": {types}
    }}"#
    )
}

fn generate(json: &str) -> Result<String, String> {
    Idlgen::from_bytes("demo", json.as_bytes())
        .unwrap()
        .generate()
        .map(|generated| generated.code().to_string())
        .map_err(|err| format!("{err:#}"))
}

const ADDRESS: &str = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";

#[test]
fn zero_copy_type_with_a_vec_is_rejected() {
    let json = idl(
        Some(ADDRESS),
        r#"[{
            "name": "State",
            "serialization": "bytemuck",
            "repr": { "kind": "c" },
            "type": { "kind": "struct", "fields": [
                { "name": "count", "type": "u64" },
                { "name": "items", "type": { "vec": "u8" } }
            ] }
        }]"#,
    );
    let err = generate(&json).unwrap_err();
    assert!(err.contains("types.State.items"), "{err}");
    assert!(err.contains("variable-size"), "{err}");
}

#[test]
fn zero_copy_type_without_repr_is_rejected() {
    let json = idl(
        Some(ADDRESS),
        r#"[{
            "name": "State",
            "serialization": "bytemuck",
            "type": { "kind": "struct", "fields": [{ "name": "count", "type": "u64" }] }
        }]"#,
    );
    let err = generate(&json).unwrap_err();
    assert!(
        err.contains("types.State") && err.contains("unspecified"),
        "{err}"
    );
}

#[test]
fn missing_program_address_needs_program_id() {
    let json = idl(None, "[]");
    let err = generate(&json).unwrap_err();
    assert!(err.contains("Idlgen::program_id"), "{err}");

    let code = Idlgen::from_bytes("demo", json.as_bytes())
        .unwrap()
        .program_id(ADDRESS)
        .generate()
        .unwrap();
    assert!(code.code().contains(ADDRESS));
    assert!(code.code().starts_with("// @generated\n"));
}

#[test]
fn custom_serialization_is_rejected() {
    let json = idl(
        Some(ADDRESS),
        r#"[{
            "name": "State",
            "serialization": { "custom": "mine" },
            "type": { "kind": "struct", "fields": [{ "name": "count", "type": "u64" }] }
        }]"#,
    );
    let err = generate(&json).unwrap_err();
    assert!(err.contains("types.State"), "{err}");
}

fn with_constants(constants: &str) -> String {
    format!(
        r#"{{
        "address": "{ADDRESS}",
        "metadata": {{ "name": "demo", "version": "0.1.0", "spec": "0.1.0" }},
        "instructions": [],
        "constants": {constants}
    }}"#
    )
}

#[test]
fn constant_expressions_are_kept_as_source_text() {
    let json = with_constants(
        r#"[
            { "name": "SCALE", "type": "u64", "value": "10u64.pow(9)" },
            { "name": "DOUBLE", "type": "u64", "value": "MAX * 2" },
            { "name": "MAX", "type": "u64", "value": "100" }
        ]"#,
    );
    let generated = Idlgen::from_bytes("demo", json.as_bytes())
        .unwrap()
        .generate()
        .unwrap();
    let code = generated.code();
    assert!(
        code.contains(r#"pub const SCALE_EXPR: &str = "10u64.pow(9)";"#),
        "{code}"
    );
    assert!(
        code.contains(r#"pub const DOUBLE_EXPR: &str = "MAX * 2";"#),
        "{code}"
    );
    assert!(code.contains("pub const MAX: u64 = 100;"), "{code}");
    assert!(!code.contains("pub const SCALE:"), "{code}");
    let warnings = generated.warnings();
    assert_eq!(warnings.len(), 2, "{warnings:?}");
    assert!(
        warnings[0].contains("constants.SCALE") && warnings[0].contains("SCALE_EXPR"),
        "{warnings:?}"
    );
}

#[test]
fn wrong_constant_literals_are_errors() {
    let cases = [
        (
            r#"{ "name": "BIG", "type": "u8", "value": "256" }"#,
            "out of range for u8",
        ),
        (
            r#"{ "name": "NEG", "type": "u64", "value": "-1" }"#,
            "out of range for u64",
        ),
        (
            r#"{ "name": "BAD", "type": "u64", "value": "12abc" }"#,
            "not an integer literal",
        ),
        (
            r#"{ "name": "CUT", "type": "u64", "value": "1 +" }"#,
            "not a valid Rust literal",
        ),
        (
            r#"{ "name": "STR", "type": "u64", "value": "\"seed\"" }"#,
            "not an integer literal",
        ),
        (
            r#"{ "name": "ARR", "type": { "array": ["u8", 3] }, "value": "[1, 2]" }"#,
            "has 2 elements, the type has 3",
        ),
    ];
    for (constant, message) in cases {
        let json = with_constants(&format!("[{constant}]"));
        let err = generate(&json).unwrap_err();
        assert!(err.contains("constants."), "{err}");
        assert!(err.contains(message), "{constant}: {err}");
    }
}

#[test]
fn duplicate_discriminators_are_rejected() {
    let json = format!(
        r#"{{
        "address": "{ADDRESS}",
        "metadata": {{ "name": "demo", "version": "0.1.0", "spec": "0.1.0" }},
        "instructions": [
            {{ "name": "a", "discriminator": [1, 2], "accounts": [], "args": [] }},
            {{ "name": "b", "discriminator": [1, 2], "accounts": [], "args": [] }}
        ]
    }}"#
    );
    let err = generate(&json).unwrap_err();
    assert!(
        err.contains("instructions.b") && err.contains("same discriminator as `a`"),
        "{err}"
    );
}
