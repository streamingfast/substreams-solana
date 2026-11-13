//! Re-encodes every instruction, account and event of every fixture IDL, decoded from bytes built
//! from the IDL model, back to the same bytes.

mod support;

use substreams_solana::borsh::{DecodeError, Pubkey};
use substreams_solana::idl::{RawInstruction, EVENT_IX_TAG_LE};
use substreams_solana_idlgen::idl::{Idl, Serialization};
use substreams_solana_idlgen_tests::generated::*;
use support::model::{borsh_value, keys, type_def_value, zero_copy_value, Env, Rng};

type Decoded = Option<Result<(&'static str, Vec<u8>), DecodeError>>;

struct Program {
    name: &'static str,
    idl: Idl,
    program_id: Pubkey,
    instruction: fn(&RawInstruction<'_>) -> Decoded,
    account: fn(&[u8]) -> Decoded,
    event: fn(&[u8]) -> Decoded,
    event_cpi: fn(&RawInstruction<'_>) -> Decoded,
}

macro_rules! programs {
    ($($module:ident => $path:literal,)*) => {
        fn programs() -> Vec<Program> {
            vec![$(Program {
                name: stringify!($module),
                idl: Idl::from_json(include_str!($path)).expect("valid fixture"),
                program_id: $module::PROGRAM_ID,
                instruction: |ix| {
                    $module::Instruction::decode(ix).map(|r| r.map(|v| (v.name(), v.encode_data())))
                },
                account: |data| {
                    $module::Account::decode(data).map(|r| r.map(|v| (v.name(), v.encode_account())))
                },
                event: |data| {
                    $module::Event::decode_log_data(data)
                        .map(|r| r.map(|v| (v.name(), v.encode_log_data())))
                },
                event_cpi: |ix| {
                    $module::Event::match_and_decode_cpi(ix)
                        .map(|r| r.map(|v| (v.name(), v.encode_log_data())))
                },
            },)*]
        }
    };
}

programs! {
    edge_cases => "../idls/edge_cases.json",
    jupiter_v4_swap => "../../idlgen/tests/fixtures/idls/jupiter_v4_swap.json",
    meteora_bonding_curve => "../../idlgen/tests/fixtures/idls/meteora_bonding_curve.json",
    mpl_token_metadata => "../../idlgen/tests/fixtures/idls/mpl_token_metadata.json",
    pumpfun => "../../idlgen/tests/fixtures/idls/pumpfun.json",
    raydium_amm => "../../idlgen/tests/fixtures/idls/raydium_amm.json",
    stake_interface => "../../idlgen/tests/fixtures/idls/stake_interface.json",
}

const SAMPLES: usize = 24;

/// `decode` on `data` is the item `name` and re-encodes to `data`.
fn check(program: &str, kind: &str, name: &str, data: &[u8], decode: impl Fn(&[u8]) -> Decoded) {
    let context = format!("{program} {kind} `{name}`");
    let (decoded_name, encoded) = decode(data)
        .unwrap_or_else(|| panic!("{context}: not dispatched"))
        .unwrap_or_else(|err| panic!("{context}: {err}"));
    assert_eq!(
        decoded_name, name,
        "{context}: dispatched to the wrong item"
    );
    assert_eq!(encoded, data, "{context}: re-encoding differs");
}

#[test]
fn every_item_of_every_fixture_round_trips() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    let mut counts = Vec::new();
    for program in programs() {
        let idl = &program.idl;
        let mut checked = 0;

        for ix in &idl.instructions {
            let accounts = keys(&mut rng, ix.accounts.len());
            for _ in 0..SAMPLES {
                let mut data = ix.discriminator.clone();
                for arg in &ix.args {
                    borsh_value(idl, &arg.ty, &mut rng, 0, &mut data);
                }
                let account_refs: Vec<&[u8]> = accounts.iter().map(|k| k.as_slice()).collect();
                check(program.name, "instruction", &ix.name, &data, |data| {
                    (program.instruction)(&RawInstruction {
                        program_id: program.program_id.as_bytes(),
                        data,
                        accounts: &account_refs,
                    })
                });
            }
            checked += 1;
        }

        for account in idl.accounts.iter().filter(|a| !a.discriminator.is_empty()) {
            for _ in 0..SAMPLES {
                let mut data = account.discriminator.clone();
                match account.layout.serialization {
                    Serialization::Borsh => type_def_value(
                        idl,
                        &account.layout,
                        &Env::default(),
                        &mut rng,
                        0,
                        &mut data,
                    ),
                    _ => zero_copy_value(idl, &account.layout, &mut rng, &mut data),
                }
                check(
                    program.name,
                    "account",
                    &account.name,
                    &data,
                    program.account,
                );
            }
            checked += 1;
        }

        for event in idl.events.iter().filter(|e| !e.discriminator.is_empty()) {
            for _ in 0..SAMPLES {
                let mut data = event.discriminator.clone();
                type_def_value(idl, &event.layout, &Env::default(), &mut rng, 0, &mut data);
                check(program.name, "event", &event.name, &data, program.event);

                let mut cpi = EVENT_IX_TAG_LE.to_vec();
                cpi.extend(&data);
                let decoded = (program.event_cpi)(&RawInstruction {
                    program_id: program.program_id.as_bytes(),
                    data: &cpi,
                    accounts: &[],
                });
                let (name, encoded) = decoded.unwrap().unwrap();
                assert_eq!(name, event.name, "{} self-CPI event", program.name);
                assert_eq!(encoded, data, "{} self-CPI event `{name}`", program.name);
            }
            checked += 1;
        }

        counts.push((program.name, checked));
    }
    // Every fixture contributes items; the edge cases cover each construct.
    for (name, checked) in counts {
        assert!(checked > 0, "{name}: nothing checked");
    }
}
