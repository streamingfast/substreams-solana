//! Reads every sample IDL in `tests/fixtures/idls` and spot-checks the model.

use substreams_solana_idlgen::idl::{
    discriminator, ArrayLen, EnumVariant, Fields, Idl, IdlFormat, IdlOrigin, IdlType, Repr,
    ReprModifier, Serialization, TypeDefKind,
};

fn fixture(name: &str) -> Idl {
    let path = format!(
        "{}/tests/fixtures/idls/{name}.json",
        env!("CARGO_MANIFEST_DIR")
    );
    Idl::from_path(&path).unwrap_or_else(|err| panic!("reading {path}: {err}"))
}

fn defined(name: &str) -> IdlType {
    IdlType::Defined {
        name: name.to_string(),
        generics: Vec::new(),
    }
}

fn arg<'a>(idl: &'a Idl, instruction: &str, arg: &str) -> &'a IdlType {
    let ix = idl
        .instructions
        .iter()
        .find(|ix| ix.name == instruction)
        .unwrap_or_else(|| panic!("no instruction {instruction}"));
    &ix.args
        .iter()
        .find(|a| a.name == arg)
        .unwrap_or_else(|| panic!("no arg {arg} in {instruction}"))
        .ty
}

fn named_field<'a>(fields: &'a Fields, name: &str) -> &'a IdlType {
    match fields {
        Fields::Named(fields) => &fields.iter().find(|f| f.name == name).expect("field").ty,
        other => panic!("expected named fields, got {other:?}"),
    }
}

fn struct_field<'a>(idl: &'a Idl, ty: &str, field: &str) -> &'a IdlType {
    match &idl
        .type_def(ty)
        .unwrap_or_else(|| panic!("no type {ty}"))
        .kind
    {
        TypeDefKind::Struct(fields) => named_field(fields, field),
        other => panic!("{ty} is not a struct: {other:?}"),
    }
}

const ALL: &[&str] = &[
    "jupiter_v4_swap",
    "meteora_bonding_curve",
    "mpl_token_metadata",
    "pumpfun",
    "raydium_amm",
    "stake_interface",
];

#[test]
fn every_fixture_reads() {
    for name in ALL {
        fixture(name);
    }
}

/// (fixture, format is 0.30, instructions, accounts, events, types, errors, constants)
#[allow(clippy::type_complexity)]
const COUNTS: &[(&str, bool, usize, usize, usize, usize, usize, usize)] = &[
    ("jupiter_v4_swap", false, 34, 0, 2, 8, 10, 0),
    ("meteora_bonding_curve", true, 22, 9, 17, 46, 36, 0),
    ("mpl_token_metadata", false, 5, 2, 0, 19, 5, 0),
    ("pumpfun", false, 6, 3, 4, 3, 9, 0),
    ("raydium_amm", false, 16, 3, 5, 12, 57, 0),
    ("stake_interface", false, 18, 0, 0, 13, 0, 0),
];

#[test]
fn counts() {
    for &(name, spec, instructions, accounts, events, types, errors, constants) in COUNTS {
        let idl = fixture(name);
        let format = if spec {
            IdlFormat::Spec("0.1.0".into())
        } else {
            IdlFormat::Legacy
        };
        assert_eq!(idl.format, format, "{name} format");
        assert_eq!(idl.instructions.len(), instructions, "{name} instructions");
        assert_eq!(idl.accounts.len(), accounts, "{name} accounts");
        assert_eq!(idl.events.len(), events, "{name} events");
        assert_eq!(idl.types.len(), types, "{name} types");
        assert_eq!(idl.errors.len(), errors, "{name} errors");
        assert_eq!(idl.constants.len(), constants, "{name} constants");
        assert!(idl.address.is_some(), "{name} address");
    }
}

/// The 0.30 fixtures write their discriminators out. Computing them from the
/// names must give the same bytes.
#[test]
fn computed_discriminators_match_spec_fixtures() {
    for &(name, spec, ..) in COUNTS {
        if !spec {
            continue;
        }
        let idl = fixture(name);
        for ix in &idl.instructions {
            assert_eq!(
                ix.discriminator,
                discriminator::instruction(&ix.name),
                "{name} {}",
                ix.name
            );
        }
        for account in &idl.accounts {
            assert_eq!(
                account.discriminator,
                discriminator::account(&account.name),
                "{name} {}",
                account.name
            );
        }
        for event in &idl.events {
            assert_eq!(
                event.discriminator,
                discriminator::event(&event.name),
                "{name} {}",
                event.name
            );
        }
    }
}

#[test]
fn spec_account_and_event_layouts_are_their_types() {
    for &(name, spec, ..) in COUNTS {
        if !spec {
            continue;
        }
        let idl = fixture(name);
        for account in &idl.accounts {
            assert_eq!(
                Some(&account.layout),
                idl.type_def(&account.name),
                "{name} {}",
                account.name
            );
        }
        for event in &idl.events {
            assert_eq!(
                Some(&event.layout),
                idl.type_def(&event.name),
                "{name} {}",
                event.name
            );
        }
    }
}

#[test]
fn pumpfun_legacy() {
    let idl = fixture("pumpfun");
    assert_eq!(
        idl.address.as_deref(),
        Some("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P")
    );
    assert_eq!(idl.origin, IdlOrigin::Anchor);

    let buy = idl.instructions.iter().find(|ix| ix.name == "buy").unwrap();
    // As seen on chain.
    assert_eq!(buy.discriminator, [102, 6, 61, 18, 1, 218, 235, 234]);
    assert_eq!(
        buy.args.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
        ["amount", "max_sol_cost"]
    );
    let user = buy.accounts.iter().find(|a| a.name == "user").unwrap();
    assert!(user.writable && user.signer && !user.optional);

    let set_params = idl
        .instructions
        .iter()
        .find(|ix| ix.name == "set_params")
        .unwrap();
    assert_eq!(
        set_params.discriminator,
        discriminator::instruction("set_params")
    );

    let curve = idl
        .accounts
        .iter()
        .find(|a| a.name == "BondingCurve")
        .unwrap();
    assert_eq!(curve.discriminator, [23, 183, 248, 55, 96, 216, 172, 96]);
    assert_eq!(
        named_field(struct_fields(&curve.layout.kind), "virtual_token_reserves"),
        &IdlType::U64
    );
    assert_eq!(Some(&curve.layout), idl.type_def("BondingCurve"));

    let trade = idl.events.iter().find(|e| e.name == "TradeEvent").unwrap();
    assert_eq!(trade.discriminator, [189, 219, 127, 211, 78, 230, 97, 238]);
    assert_eq!(
        named_field(struct_fields(&trade.layout.kind), "is_buy"),
        &IdlType::Bool
    );
    assert_eq!(
        named_field(struct_fields(&trade.layout.kind), "mint"),
        &IdlType::Pubkey
    );
    // Legacy events are not added to the types.
    assert!(idl.type_def("TradeEvent").is_none());
}

fn struct_fields(kind: &TypeDefKind) -> &Fields {
    match kind {
        TypeDefKind::Struct(fields) => fields,
        other => panic!("expected a struct, got {other:?}"),
    }
}

#[test]
fn jupiter_legacy() {
    let idl = fixture("jupiter_v4_swap");

    // A composite account group, flattened in place.
    let serum = idl
        .instructions
        .iter()
        .find(|ix| ix.name == "serum_swap")
        .unwrap();
    let market = serum
        .accounts
        .iter()
        .find(|a| a.name == "open_orders")
        .unwrap();
    assert_eq!(market.groups, ["market"]);
    assert_eq!(market.qualified_name(), "market.open_orders");
    assert_eq!(serum.accounts[0].qualified_name(), "market.market");

    // An event and an unrelated enum share the name `Swap`.
    let event = idl.events.iter().find(|e| e.name == "Swap").unwrap();
    assert_eq!(
        named_field(struct_fields(&event.layout.kind), "input_mint"),
        &IdlType::Pubkey
    );
    let TypeDefKind::Enum(variants) = &idl.type_def("Swap").unwrap().kind else {
        panic!("Swap is not an enum");
    };
    assert_eq!(
        variants[0],
        EnumVariant {
            name: "Saber".into(),
            fields: Fields::Unit
        }
    );
    let crema = variants.iter().find(|v| v.name == "Crema").unwrap();
    assert_eq!(named_field(&crema.fields, "a_to_b"), &IdlType::Bool);

    // Data-carrying variants.
    let TypeDefKind::Enum(variants) = &idl.type_def("SwapLeg").unwrap().kind else {
        panic!("SwapLeg is not an enum");
    };
    assert_eq!(
        named_field(&variants[0].fields, "swap_legs"),
        &IdlType::Vec(Box::new(defined("SwapLegDeeper")))
    );
}

#[test]
fn raydium_shank() {
    let idl = fixture("raydium_amm");
    assert_eq!(idl.origin, IdlOrigin::Shank);
    for (i, ix) in idl.instructions.iter().enumerate() {
        assert_eq!(ix.discriminator, [i as u8], "{}", ix.name);
    }
    assert_eq!(idl.instructions[9].name, "swap_base_in");
    // `amm_target_orders` may be left out of both swaps, see the fixtures README.
    for name in ["swap_base_in", "swap_base_out"] {
        let swap = idl.instructions.iter().find(|ix| ix.name == name).unwrap();
        assert_eq!(swap.accounts[4].name, "amm_target_orders", "{}", swap.name);
        assert!(swap.accounts[4].optional, "{}", swap.name);
    }
    assert!(idl.accounts.iter().all(|a| a.discriminator.is_empty()));
    assert!(idl.events.iter().all(|e| e.discriminator.is_empty()));

    assert_eq!(
        arg(&idl, "set_params", "new_pubkey"),
        &IdlType::Option(Box::new(IdlType::Pubkey))
    );
    assert_eq!(
        arg(&idl, "set_params", "fees"),
        &IdlType::Option(Box::new(defined("Fees")))
    );
    assert_eq!(
        struct_field(&idl, "TargetOrders", "buy_orders"),
        &IdlType::Array(Box::new(defined("TargetOrder")), ArrayLen::Value(50))
    );
}

#[test]
fn meteora_bonding_curve_spec() {
    let idl = fixture("meteora_bonding_curve");
    let claim = idl
        .instructions
        .iter()
        .find(|ix| ix.name == "claim_creator_trading_fee")
        .unwrap();
    assert_eq!(claim.discriminator, [82, 220, 250, 189, 3, 85, 107, 45]);
    let authority = claim
        .accounts
        .iter()
        .find(|a| a.name == "pool_authority")
        .unwrap();
    assert_eq!(
        authority.address.as_deref(),
        Some("FhVo3mqL8PW5pH5U2CN4XE33DokiyZnUwuGpH2hmHLuM")
    );
    let event_authority = claim
        .accounts
        .iter()
        .find(|a| a.name == "event_authority")
        .unwrap();
    assert_eq!(
        event_authority.pda.as_ref().unwrap().seeds,
        [substreams_solana_idlgen::idl::Seed::Const {
            value: b"__event_authority".to_vec()
        }]
    );

    let pool = idl.type_def("PoolConfig").unwrap();
    assert_eq!(pool.serialization, Serialization::Bytemuck);
    assert_eq!(pool.repr, Some(Repr::C(ReprModifier::default())));
    assert_eq!(
        struct_field(&idl, "ConfigParameters", "token_supply"),
        &IdlType::Option(Box::new(defined("TokenSupplyParams")))
    );
}

#[test]
fn packed_zero_copy_spec() {
    let idl = Idl::from_json(
        r#"{
        "address": "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
        "metadata": { "name": "demo", "version": "0.1.0", "spec": "0.1.0" },
        "instructions": [],
        "types": [{
            "name": "Bank",
            "serialization": "bytemuckunsafe",
            "repr": { "kind": "rust", "packed": true },
            "type": { "kind": "struct", "fields": [{ "name": "flag", "type": "u8" }] }
        }]
    }"#,
    )
    .unwrap();
    let bank = idl.type_def("Bank").unwrap();
    assert_eq!(bank.serialization, Serialization::BytemuckUnsafe);
    assert_eq!(
        bank.repr,
        Some(Repr::Rust(ReprModifier {
            packed: true,
            align: None
        }))
    );
}
