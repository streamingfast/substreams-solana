//! Decodes every instruction, account and event of every fixture IDL from bytes built from the
//! IDL model alone, checks dispatch, and feeds truncated and random bytes to every decoder.

mod support;

use substreams_solana::borsh::{DecodeError, Pubkey};
use substreams_solana::idl::{RawInstruction, EVENT_IX_TAG_LE};
use substreams_solana_idlgen::idl::{Idl, Serialization};
use substreams_solana_idlgen_tests::generated::*;
use support::model::{borsh_value, keys, type_def_value, zero_copy_value, Env, Rng};

type Decoded = Option<Result<&'static str, DecodeError>>;

struct Program {
    name: &'static str,
    idl: Idl,
    program_id: Pubkey,
    instruction: fn(&RawInstruction<'_>) -> Decoded,
    instruction_matched: fn(&RawInstruction<'_>) -> bool,
    /// Why the accounts of a decoded instruction could not be named.
    accounts_error: fn(&RawInstruction<'_>) -> Option<DecodeError>,
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
                    $module::Instruction::decode(ix).map(|r| r.map(|v| v.name()))
                },
                instruction_matched: |ix| $module::Instruction::match_and_decode(ix).is_some(),
                accounts_error: |ix| {
                    let decoded = $module::Instruction::decode(ix)
                        .expect("dispatched")
                        .expect("arguments decode");
                    decoded.accounts_error().cloned()
                },
                account: |data| {
                    $module::Account::decode(data).map(|r| r.map(|v| v.name()))
                },
                event: |data| {
                    $module::Event::decode_log_data(data).map(|r| r.map(|v| v.name()))
                },
                event_cpi: |ix| {
                    $module::Event::match_and_decode_cpi(ix).map(|r| r.map(|v| v.name()))
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

/// `decode` on `data` is the item `name`, and fails on every truncation.
fn check(program: &str, kind: &str, name: &str, data: &[u8], decode: impl Fn(&[u8]) -> Decoded) {
    let context = format!("{program} {kind} `{name}`");
    let decoded_name = decode(data)
        .unwrap_or_else(|| panic!("{context}: not dispatched"))
        .unwrap_or_else(|err| panic!("{context}: {err}"));
    assert_eq!(
        decoded_name, name,
        "{context}: dispatched to the wrong item"
    );
    for len in 0..data.len() {
        // A shorter discriminator may legitimately name another item.
        if let Some(Ok(other)) = decode(&data[..len]) {
            assert_ne!(
                other,
                name,
                "{context}: decoded from {len} of {} bytes",
                data.len()
            );
        }
    }
}

#[test]
fn every_item_of_every_fixture_decodes() {
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
                let ix_view = RawInstruction {
                    program_id: program.program_id.as_bytes(),
                    data: &data,
                    accounts: &account_refs,
                };
                assert!((program.instruction_matched)(&ix_view));
                if !ix.accounts.is_empty() {
                    let fewer = RawInstruction {
                        accounts: &account_refs[..ix.accounts.len() - 1],
                        ..ix_view
                    };
                    // The arguments decode whatever the account count; the named
                    // accounts follow the layout rules of the IDL's origin.
                    let optional = ix.accounts.iter().filter(|a| a.optional).count();
                    let named = optional == 1;
                    let error = (program.accounts_error)(&fewer);
                    assert_eq!(
                        error.is_none(),
                        named,
                        "{} `{}` with one account fewer: {error:?}",
                        program.name,
                        ix.name
                    );
                    if optional > 1 {
                        assert!(matches!(error, Some(DecodeError::AmbiguousAccounts { .. })));
                    }
                }
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
                assert_eq!(
                    decoded.unwrap().unwrap(),
                    event.name,
                    "{} self-CPI event",
                    program.name
                );
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

#[test]
fn dispatchers_reject_foreign_programs_and_unknown_discriminators() {
    let mut rng = Rng(42);
    for program in programs() {
        let idl = &program.idl;
        let unknown = [0xffu8; 16];
        let foreign = Pubkey::new([0xab; 32]);
        let accounts = keys(&mut rng, 64);
        let account_refs: Vec<&[u8]> = accounts.iter().map(|k| k.as_slice()).collect();

        for ix in &idl.instructions {
            let mut data = ix.discriminator.clone();
            for arg in &ix.args {
                borsh_value(idl, &arg.ty, &mut rng, 0, &mut data);
            }
            let foreign_ix = RawInstruction {
                program_id: foreign.as_bytes(),
                data: &data,
                accounts: &account_refs,
            };
            assert!(
                !(program.instruction_matched)(&foreign_ix),
                "{}",
                program.name
            );
        }

        let ix = RawInstruction {
            program_id: program.program_id.as_bytes(),
            data: &unknown,
            accounts: &account_refs,
        };
        assert!((program.instruction)(&ix).is_none(), "{}", program.name);
        assert!(!(program.instruction_matched)(&ix), "{}", program.name);
        assert!((program.account)(&unknown).is_none(), "{}", program.name);
        assert!((program.event)(&unknown).is_none(), "{}", program.name);
        assert!((program.instruction)(&RawInstruction { data: &[], ..ix }).is_none());

        for event in idl.events.iter().filter(|e| !e.discriminator.is_empty()) {
            let mut cpi = EVENT_IX_TAG_LE.to_vec();
            cpi.extend(&event.discriminator);
            type_def_value(idl, &event.layout, &Env::default(), &mut rng, 0, &mut cpi);
            let foreign_cpi = RawInstruction {
                program_id: foreign.as_bytes(),
                data: &cpi,
                accounts: &[],
            };
            assert!(
                (program.event_cpi)(&foreign_cpi).is_none(),
                "{}",
                program.name
            );
        }
    }
}

#[test]
fn garbage_never_panics() {
    let mut rng = Rng(7);
    for program in programs() {
        let idl = &program.idl;
        let accounts = keys(&mut rng, 64);
        let account_refs: Vec<&[u8]> = accounts.iter().map(|k| k.as_slice()).collect();
        let prefixes = idl
            .instructions
            .iter()
            .map(|ix| ix.discriminator.clone())
            .chain(idl.accounts.iter().map(|a| a.discriminator.clone()))
            .chain(idl.events.iter().map(|e| e.discriminator.clone()));
        for prefix in prefixes {
            for _ in 0..64 {
                let mut data = prefix.clone();
                let len = rng.below(300) as usize;
                rng.bytes(len, &mut data);
                let ix = RawInstruction {
                    program_id: program.program_id.as_bytes(),
                    data: &data,
                    accounts: &account_refs[..rng.below(64) as usize],
                };
                let _ = (program.instruction)(&ix);
                let _ = (program.account)(&data);
                let _ = (program.event)(&data);
            }
        }
    }
}
