use super::*;
use crate::block_view::lazy::LazyTransaction;
use crate::pb::sf::solana::r#type::v1 as pb;
use crate::pb::sf::solana::r#type::v1::__buffa::lazy_view::ConfirmedTransactionLazyView;
use buffa::view::LazyMessageView;
use buffa::Message;
use pretty_assertions::assert_eq;

// A small decoder written by hand against the runtime, the way generated code would be.

const PROGRAM: Pubkey = Pubkey([7; 32]);
const OTHER_PROGRAM: Pubkey = Pubkey([8; 32]);

#[derive(Debug, Clone, PartialEq)]
enum Side {
    Buy,
    Sell { limit: Option<u64> },
    Cancel(u32, bool),
}

impl BorshDecode for Side {
    fn decode(r: &mut BorshReader<'_>) -> Result<Self, DecodeError> {
        Ok(match r.read_variant()? {
            0 => Side::Buy,
            1 => Side::Sell { limit: r.read()? },
            2 => Side::Cancel(r.read()?, r.read()?),
            v => return Err(DecodeError::invalid_variant(r, "Side", v)),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
struct SwapArgs {
    amount: u64,
    side: Side,
    memo: String,
}

impl BorshDecode for SwapArgs {
    fn decode(r: &mut BorshReader<'_>) -> Result<Self, DecodeError> {
        Ok(SwapArgs {
            amount: r.read()?,
            side: r.read()?,
            memo: r.read()?,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Swap {
    args: SwapArgs,
    user: Pubkey,
    pool: Pubkey,
    referrer: Option<Pubkey>,
}

impl IdlInstruction for Swap {
    const PROGRAM_ID: Pubkey = PROGRAM;
    const DISCRIMINATOR: &'static [u8] = &[0xf8, 0xc6, 0x9e, 0x91, 0xe1, 0x75, 0x87, 0xc8];

    fn decode_instruction<I: InstructionInput + ?Sized>(
        args: &mut BorshReader<'_>,
        ix: &I,
    ) -> Result<Self, DecodeError> {
        let args = args.read()?;
        let named = NamedAccounts::resolve(ix, &[false, false, true])?;
        Ok(Swap {
            args,
            user: named.required(0)?,
            pool: named.required(1)?,
            referrer: named.optional(2)?,
        })
    }
}

/// An instruction with a custom one-byte discriminator.
#[derive(Debug, PartialEq)]
struct Ping;

impl IdlInstruction for Ping {
    const PROGRAM_ID: Pubkey = PROGRAM;
    const DISCRIMINATOR: &'static [u8] = &[42];

    fn decode_instruction<I: InstructionInput + ?Sized>(
        _: &mut BorshReader<'_>,
        _: &I,
    ) -> Result<Self, DecodeError> {
        Ok(Ping)
    }
}

#[derive(Debug, PartialEq)]
struct PoolState {
    authority: Pubkey,
    reserves: [u64; 2],
}

impl BorshDecode for PoolState {
    fn decode(r: &mut BorshReader<'_>) -> Result<Self, DecodeError> {
        Ok(PoolState {
            authority: r.read()?,
            reserves: r.read()?,
        })
    }
}

impl IdlAccount for PoolState {
    const DISCRIMINATOR: &'static [u8] = &[1, 1, 1, 1, 2, 2, 2, 2];
}

#[derive(Debug, Clone, PartialEq)]
struct Swapped {
    user: Pubkey,
    amount: u64,
}

impl BorshDecode for Swapped {
    fn decode(r: &mut BorshReader<'_>) -> Result<Self, DecodeError> {
        Ok(Swapped {
            user: r.read()?,
            amount: r.read()?,
        })
    }
}

impl IdlEvent for Swapped {
    const PROGRAM_ID: Pubkey = PROGRAM;
    const DISCRIMINATOR: &'static [u8] = &[9, 9, 9, 9, 9, 9, 9, 9];
}

fn prefixed(prefixes: &[&[u8]], value: &[u8]) -> Vec<u8> {
    let mut out = prefixes.concat();
    out.extend_from_slice(value);
    out
}

/// The Borsh bytes of [swap_args].
fn swap_args_bytes() -> Vec<u8> {
    let mut out = 1_000u64.to_le_bytes().to_vec();
    // `Side::Sell { limit: Some(5) }`
    out.extend_from_slice(&[1, 1]);
    out.extend_from_slice(&5u64.to_le_bytes());
    // `memo: "hi"`
    out.extend_from_slice(&[2, 0, 0, 0, b'h', b'i']);
    out
}

/// The Borsh bytes of [event].
fn event_bytes(amount: u64) -> Vec<u8> {
    let mut out = vec![1; 32];
    out.extend_from_slice(&amount.to_le_bytes());
    out
}

fn swap_args() -> SwapArgs {
    SwapArgs {
        amount: 1_000,
        side: Side::Sell { limit: Some(5) },
        memo: "hi".into(),
    }
}

fn event(amount: u64) -> Swapped {
    Swapped {
        user: Pubkey([1; 32]),
        amount,
    }
}

// Account table: 0 user, 1 program, 2 pool, 3 other program; loaded: 4 referrer.
fn transaction() -> pb::ConfirmedTransaction {
    let swap = prefixed(&[Swap::DISCRIMINATOR], &swap_args_bytes());
    let cpi_event = prefixed(&[&EVENT_IX_TAG_LE, Swapped::DISCRIMINATOR], &event_bytes(7));

    pb::ConfirmedTransaction {
        transaction: pb::Transaction {
            signatures: vec![vec![0; 64]],
            message: pb::Message {
                account_keys: vec![
                    vec![1; 32],
                    PROGRAM.0.to_vec(),
                    vec![2; 32],
                    OTHER_PROGRAM.0.to_vec(),
                ],
                instructions: vec![
                    // Swap with a referrer.
                    pb::CompiledInstruction {
                        program_id_index: 1,
                        accounts: vec![0, 2, 4],
                        data: swap.clone(),
                    },
                    // Swap with the optional referrer set to the program id.
                    pb::CompiledInstruction {
                        program_id_index: 1,
                        accounts: vec![0, 2, 1],
                        data: swap.clone(),
                    },
                    // Same data, wrong program.
                    pb::CompiledInstruction {
                        program_id_index: 3,
                        accounts: vec![0, 2],
                        data: swap.clone(),
                    },
                    // Missing the pool account.
                    pb::CompiledInstruction {
                        program_id_index: 1,
                        accounts: vec![0],
                        data: swap,
                    },
                ],
                ..Default::default()
            }
            .into(),
        }
        .into(),
        meta: pb::TransactionStatusMeta {
            loaded_writable_addresses: vec![vec![4; 32]],
            inner_instructions: vec![pb::InnerInstructions {
                index: 0,
                instructions: vec![
                    pb::InnerInstruction {
                        program_id_index: 1,
                        accounts: vec![9],
                        data: cpi_event,
                        stack_height: Some(2),
                    },
                    pb::InnerInstruction {
                        program_id_index: 1,
                        accounts: vec![],
                        data: vec![42],
                        stack_height: Some(2),
                    },
                ],
            }],
            ..Default::default()
        }
        .into(),
    }
}

/// Decodes every instruction of `ixs` the way a Substreams module would.
fn decode_all<I: InstructionInput>(ixs: impl Iterator<Item = I>) -> Vec<String> {
    ixs.map(|ix| {
        if let Some(swap) = Swap::match_and_decode(&ix) {
            format!("swap {swap:?}")
        } else if let Some(ev) = Swapped::match_and_decode_cpi(&ix) {
            format!("event {ev:?}")
        } else if let Some(ping) = Ping::match_and_decode(&ix) {
            format!("ping {ping:?}")
        } else {
            format!(
                "unmatched accounts={} data={}",
                ix.accounts_len(),
                ix.data().len()
            )
        }
    })
    .collect()
}

#[test]
fn owned_and_lazy_views_decode_identically() {
    let owned = transaction();
    let expected = vec![
        format!(
            "swap {:?}",
            Ok::<_, DecodeError>(Swap {
                args: swap_args(),
                user: Pubkey([1; 32]),
                pool: Pubkey([2; 32]),
                referrer: Some(Pubkey([4; 32])),
            })
        ),
        format!("event {:?}", Ok::<_, DecodeError>(event(7))),
        format!("ping {:?}", Ok::<_, DecodeError>(Ping)),
        format!(
            "swap {:?}",
            Ok::<_, DecodeError>(Swap {
                args: swap_args(),
                user: Pubkey([1; 32]),
                pool: Pubkey([2; 32]),
                referrer: None,
            })
        ),
        "unmatched accounts=2 data=32".to_string(),
        format!(
            "swap {:?}",
            Err::<Swap, _>(DecodeError::MissingAccount { index: 1, len: 1 })
        ),
    ];

    assert_eq!(decode_all(owned.walk_instructions()), expected);

    let bytes = owned.encode_to_vec();
    let view = ConfirmedTransactionLazyView::decode_lazy(&bytes).unwrap();
    let lazy = LazyTransaction::new(&view).unwrap();
    assert_eq!(decode_all(lazy.walk_instructions().unwrap()), expected);
}

#[test]
fn views_resolve_accounts_and_stop_at_the_list_end() {
    let owned = transaction();
    let first = owned.walk_instructions().next().unwrap();
    assert_eq!(first.account(2), Some(&[4u8; 32][..]));
    // Past the instruction's own account list.
    let len = first.accounts_len();
    assert_eq!(first.account(len), None);
    assert_eq!(
        first.account_key(len),
        Err(DecodeError::MissingAccount { index: len, len })
    );
}

#[test]
#[should_panic(expected = "out of bounds")]
fn owned_view_panics_outside_the_account_table() {
    let owned = transaction();
    // The second instruction names account index 9, past the account table.
    let _ = owned.walk_instructions().nth(1).unwrap().account(0);
}

#[test]
#[should_panic]
fn lazy_view_panics_outside_the_account_table() {
    let bytes = transaction().encode_to_vec();
    let view = ConfirmedTransactionLazyView::decode_lazy(&bytes).unwrap();
    let lazy = LazyTransaction::new(&view).unwrap();
    let _ = lazy.walk_instructions().unwrap().nth(1).unwrap().account(0);
}

#[test]
#[should_panic(expected = "out of bounds")]
fn unresolvable_program_panics() {
    let mut trx = transaction();
    trx.transaction
        .get_or_insert_default()
        .message
        .get_or_insert_default()
        .instructions[0]
        .program_id_index = 200;
    let ix = trx.walk_instructions().next().unwrap();
    let _ = InstructionInput::program_id(&ix);
}

#[test]
fn instruction_decode_errors() {
    let accounts: &[&[u8]] = &[&[1; 32], &[2; 31]];
    let raw = |data: &'static [u8]| RawInstruction {
        program_id: &PROGRAM.0,
        data,
        accounts,
    };

    assert_eq!(
        Swap::decode(&raw(&[0xf8, 0xc6])),
        Err(DecodeError::UnknownDiscriminator {
            expected: Swap::DISCRIMINATOR,
            found: vec![0xf8, 0xc6]
        })
    );
    assert!(Swap::match_and_decode(&raw(&[0xf8, 0xc6])).is_none());

    let data = prefixed(&[Swap::DISCRIMINATOR], &swap_args_bytes());
    let data: &'static [u8] = Box::leak(data.into_boxed_slice());
    assert_eq!(
        Swap::decode(&raw(data)),
        Err(DecodeError::InvalidAccount { index: 1, len: 31 })
    );

    // Unknown enum variant in the arguments.
    let mut bad = prefixed(&[Swap::DISCRIMINATOR], &1u64.to_le_bytes());
    bad.push(7);
    let bad: &'static [u8] = Box::leak(bad.into_boxed_slice());
    let err = Swap::decode(&raw(bad)).unwrap_err();
    assert_eq!(
        err,
        DecodeError::InvalidEnumVariant {
            offset: 16,
            type_name: "Side",
            variant: 7
        }
    );
    assert_eq!(
        err.to_string(),
        "invalid variant 7 for enum Side at offset 16"
    );
}

#[test]
fn instruction_garbage_never_panics() {
    let accounts: &[&[u8]] = &[&[1; 32], &[2; 32]];
    let full = prefixed(&[Swap::DISCRIMINATOR], &swap_args_bytes());
    for cut in 0..full.len() {
        let ix = RawInstruction {
            program_id: &PROGRAM.0,
            data: &full[..cut],
            accounts,
        };
        assert!(Swap::decode(&ix).is_err(), "cut at {cut}");
    }
    // Trailing bytes are ignored, like Anchor.
    let mut long = full.clone();
    long.extend_from_slice(&[0xde, 0xad]);
    let ix = RawInstruction {
        program_id: &PROGRAM.0,
        data: &long,
        accounts,
    };
    assert!(Swap::decode(&ix).is_ok());
}

#[test]
fn account_decode() {
    let mut data = PoolState::DISCRIMINATOR.to_vec();
    data.extend_from_slice(&[3; 32]);
    data.extend_from_slice(&10u64.to_le_bytes());
    data.extend_from_slice(&20u64.to_le_bytes());
    data.extend_from_slice(&[0; 100]); // Over-allocated account space.

    assert_eq!(
        PoolState::match_and_decode(&data),
        Some(Ok(PoolState {
            authority: Pubkey([3; 32]),
            reserves: [10, 20]
        }))
    );
    assert!(PoolState::match_and_decode(&data[1..]).is_none());
    assert!(PoolState::decode_account(&data[..40]).is_err());
}

#[test]
fn event_from_log_data_and_cpi() {
    let payload = prefixed(&[Swapped::DISCRIMINATOR], &event_bytes(3));
    assert_eq!(
        Swapped::match_and_decode_log_data(&payload),
        Some(Ok(event(3)))
    );
    assert!(Swapped::match_and_decode_log_data(&payload[1..]).is_none());

    let cpi = prefixed(&[&EVENT_IX_TAG_LE, Swapped::DISCRIMINATOR], &event_bytes(4));
    let ix = RawInstruction {
        program_id: &PROGRAM.0,
        data: &cpi,
        accounts: &[],
    };
    assert_eq!(Swapped::match_and_decode_cpi(&ix), Some(Ok(event(4))));

    // Without the event tag it is not a self-CPI.
    let ix = RawInstruction {
        program_id: &PROGRAM.0,
        data: &payload,
        accounts: &[],
    };
    assert!(Swapped::match_and_decode_cpi(&ix).is_none());
    assert!(Swapped::decode_cpi(&ix).is_err());

    // Emitted by another program.
    let ix = RawInstruction {
        program_id: &OTHER_PROGRAM.0,
        data: &cpi,
        accounts: &[],
    };
    assert!(Swapped::match_and_decode_cpi(&ix).is_none());
}

#[test]
fn event_ix_tag_matches_anchor() {
    // anchor_lang::event::EVENT_IX_TAG
    assert_eq!(EVENT_IX_TAG_LE, 0x1d9acb512ea545e4u64.to_le_bytes());
}

fn b64(data: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let mut b = [0u8; 3];
        b[..chunk.len()].copy_from_slice(chunk);
        let n = u32::from_be_bytes([0, b[0], b[1], b[2]]);
        for i in 0..4 {
            if i <= chunk.len() {
                out.push(ALPHABET[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                out.push('=');
            }
        }
    }
    out
}

#[test]
fn base64() {
    assert_eq!(base64_decode_fields(""), Some(vec![]));
    assert_eq!(base64_decode_fields("Zg=="), Some(b"f".to_vec()));
    assert_eq!(base64_decode_fields("Zm8="), Some(b"fo".to_vec()));
    assert_eq!(base64_decode_fields("Zm9v"), Some(b"foo".to_vec()));
    assert_eq!(base64_decode_fields("Zm9vYg"), Some(b"foob".to_vec()));
    assert_eq!(base64_decode_fields("Zm9vYmE"), Some(b"fooba".to_vec()));
    assert_eq!(base64_decode_fields("+/+/"), Some(vec![0xfb, 0xff, 0xbf]));
    assert_eq!(base64_decode_fields("Z"), None);
    assert_eq!(base64_decode_fields("Zm9v!"), None);
    assert_eq!(base64_decode_fields("Zg==Zg=="), None);
    assert_eq!(base64_decode_fields("Zg="), None);
    for len in 0..40u8 {
        let data: Vec<u8> = (0..len).map(|i| i.wrapping_mul(37)).collect();
        assert_eq!(base64_decode_fields(&b64(&data)), Some(data));
    }
}

fn data_line(data: &[u8]) -> String {
    format!("Program data: {}", b64(data))
}

#[test]
fn program_data_follows_nested_cpis() {
    let x = PROGRAM.to_string();
    let y = OTHER_PROGRAM.to_string();
    let z = Pubkey([5; 32]).to_string();
    let logs = vec![
        format!("Program {x} invoke [1]"),
        "Program log: Instruction: Swap".to_string(),
        data_line(b"x1"),
        format!("Program {y} invoke [2]"),
        data_line(b"y1"),
        format!("Program {x} invoke [3]"),
        data_line(b"x2"),
        format!("Program {x} consumed 100 of 200 compute units"),
        format!("Program {x} success"),
        data_line(b"y2"),
        "Program log: success".to_string(),
        format!("Program return: {y} AQ=="),
        format!("Program {y} success"),
        data_line(b"x3"),
        format!("Program {x} success"),
        format!("Program {z} invoke [1]"),
        data_line(b"z1"),
        format!("Program {z} failed: custom program error: 0x1"),
        data_line(b"none"),
    ];

    let collect = |id: &Pubkey| -> Vec<Vec<u8>> { program_data(&logs, id.as_ref()).collect() };
    assert_eq!(
        collect(&PROGRAM),
        vec![b"x1".to_vec(), b"x2".to_vec(), b"x3".to_vec()]
    );
    assert_eq!(
        collect(&OTHER_PROGRAM),
        vec![b"y1".to_vec(), b"y2".to_vec()]
    );
    assert_eq!(collect(&Pubkey([5; 32])), vec![b"z1".to_vec()]);
}

#[test]
fn program_data_resyncs_and_stops_on_truncation() {
    let x = PROGRAM.to_string();
    let y = OTHER_PROGRAM.to_string();
    let logs = [
        format!("Program {y} invoke [1]"),
        format!("Program {y} invoke [2]"),
        // Both frames above are left open; depth 1 resets the stack.
        format!("Program {x} invoke [1]"),
        data_line(b"x1"),
        "Program data: not*base64".to_string(),
        format!("Program data: {} {}", b64(b"a"), b64(b"bc")),
        "Log truncated".to_string(),
        data_line(b"x2"),
    ];
    let got: Vec<Vec<u8>> = program_data(logs.iter(), PROGRAM.as_ref()).collect();
    assert_eq!(got, vec![b"x1".to_vec(), b"abc".to_vec()]);
}

#[test]
fn event_decode_logs() {
    let x = PROGRAM.to_string();
    let y = OTHER_PROGRAM.to_string();
    let logs = vec![
        format!("Program {x} invoke [1]"),
        data_line(&prefixed(&[Swapped::DISCRIMINATOR], &event_bytes(1))),
        data_line(b"some other event"),
        format!("Program {y} invoke [2]"),
        // Same discriminator from another program is ignored.
        data_line(&prefixed(&[Swapped::DISCRIMINATOR], &event_bytes(2))),
        format!("Program {y} success"),
        // Truncated event.
        data_line(Swapped::DISCRIMINATOR),
        format!("Program {x} success"),
    ];

    let events: Vec<_> = Swapped::decode_logs(&logs).collect();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0], Ok(event(1)));
    assert!(events[1].is_err());
}

#[test]
fn log_parsing_garbage_never_panics() {
    let x = PROGRAM.to_string();
    let lines = [
        "Program ".to_string(),
        "Program".to_string(),
        format!("Program {x} invoke ["),
        format!("Program {x} invoke [99999999999999999999999]"),
        format!("Program {x} invoke [0]"),
        format!("Program {x} invoke [1]"),
        format!("Program {x} success"),
        format!("Program {x} success"),
        "Program data: ".to_string(),
        "Program data: ====".to_string(),
        "Program data: A".to_string(),
    ];
    for start in 0..lines.len() {
        let rotated: Vec<&String> = lines[start..].iter().chain(&lines[..start]).collect();
        let _ = program_data(&rotated, PROGRAM.as_ref()).count();
    }
}

// Optional accounts: program id placeholders and omissions.

fn resolve(len: u8, optional: &[bool]) -> Result<Vec<Option<u8>>, DecodeError> {
    let keys: Vec<[u8; 32]> = (1..=len).map(|i| [i; 32]).collect();
    let accounts: Vec<&[u8]> = keys.iter().map(|k| k.as_slice()).collect();
    let ix = RawInstruction {
        program_id: PROGRAM.as_bytes(),
        data: &[],
        accounts: &accounts,
    };
    let named = NamedAccounts::resolve(&ix, optional)?;
    (0..optional.len())
        .map(|slot| match optional[slot] {
            true => named.optional(slot),
            false => named.required(slot).map(Some),
        })
        .map(|key| key.map(|key| key.map(|key| key.0[0])))
        .collect()
}

#[test]
fn omitted_optional_shifts_later_accounts() {
    let layout = [false, false, true, false];
    assert_eq!(
        resolve(4, &layout),
        Ok(vec![Some(1), Some(2), Some(3), Some(4)])
    );
    // One short with one optional slot: it is the one left out.
    assert_eq!(
        resolve(3, &layout),
        Ok(vec![Some(1), Some(2), None, Some(3)])
    );
    assert_eq!(
        resolve(2, &layout),
        Err(DecodeError::MissingAccount { index: 2, len: 2 })
    );
    // A trailing optional account left off.
    assert_eq!(resolve(1, &[false, true]), Ok(vec![Some(1), None]));
}

#[test]
fn partial_omission_is_ambiguous() {
    let layout = [false, true, true, false];
    assert_eq!(
        resolve(3, &layout),
        Err(DecodeError::AmbiguousAccounts {
            len: 3,
            listed: 4,
            optional: 2
        })
    );
    assert_eq!(resolve(2, &layout), Ok(vec![Some(1), None, None, Some(2)]));
}

fn positional(len: u8, optional: &[bool]) -> Result<Vec<Option<u8>>, DecodeError> {
    let keys: Vec<[u8; 32]> = (1..=len).map(|i| [i; 32]).collect();
    let accounts: Vec<&[u8]> = keys.iter().map(|k| k.as_slice()).collect();
    let ix = RawInstruction {
        program_id: PROGRAM.as_bytes(),
        data: &[],
        accounts: &accounts,
    };
    let named = NamedAccounts::positional(&ix, optional)?;
    (0..optional.len())
        .map(|slot| match optional[slot] {
            true => named.optional(slot),
            false => named.required(slot).map(Some),
        })
        .map(|key| key.map(|key| key.map(|key| key.0[0])))
        .collect()
}

#[test]
fn anchor_reads_accounts_by_position() {
    // Trailing optional accounts left off are absent, as Anchor's `Option<T>` reads them.
    let layout = [false, true, true];
    assert_eq!(positional(3, &layout), Ok(vec![Some(1), Some(2), Some(3)]));
    assert_eq!(positional(2, &layout), Ok(vec![Some(1), Some(2), None]));
    assert_eq!(positional(1, &layout), Ok(vec![Some(1), None, None]));
    assert_eq!(
        positional(0, &layout),
        Err(DecodeError::MissingAccount { index: 0, len: 0 })
    );
    // A middle optional account never shifts later ones: a required slot past the end is missing.
    let layout = [false, true, false];
    assert_eq!(
        positional(2, &layout),
        Err(DecodeError::MissingAccount { index: 2, len: 2 })
    );
}

#[test]
fn program_id_placeholder_is_an_omitted_optional_account() {
    let first = [1u8; 32];
    let accounts: [&[u8]; 3] = [&first, PROGRAM.as_bytes(), &first];
    let ix = RawInstruction {
        program_id: PROGRAM.as_bytes(),
        data: &[],
        accounts: &accounts,
    };
    let named = NamedAccounts::resolve(&ix, &[false, true, false]).unwrap();
    assert_eq!(named.optional(1), Ok(None));
    assert_eq!(named.required(2), Ok(Pubkey(first)));
    assert_eq!(ix.account_keys().unwrap().len(), 3);
}
