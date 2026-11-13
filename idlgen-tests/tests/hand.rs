//! Hand-built and real bytes checked against expected field values, the generated
//! names of tricky IDL constructs, and zero-copy layouts.

mod support;

use substreams_solana::borsh::{self, BorshDecode, COption, DecodeError, Pubkey, I256, U256};
use substreams_solana::idl::{IdlAccount, IdlEvent, IdlInstruction, RawInstruction};
use substreams_solana_idlgen_tests::generated::{
    edge_cases, jupiter_v4_swap, meteora_bonding_curve, mpl_token_metadata, pumpfun, raydium_amm,
};
use support::bytes::Bytes;

fn key(n: u8) -> [u8; 32] {
    [n; 32]
}

fn base64(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | (*b as u32) << (16 - 8 * i));
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
fn pumpfun_buy_from_hand_built_bytes() {
    let mut data = vec![102, 6, 61, 18, 1, 218, 235, 234];
    data.extend(1_000_000u64.to_le_bytes());
    data.extend(2_500_000_000u64.to_le_bytes());
    let keys: Vec<[u8; 32]> = (0..12).map(key).collect();
    let accounts: Vec<&[u8]> = keys.iter().map(|k| k.as_slice()).collect();
    let ix = RawInstruction {
        program_id: pumpfun::PROGRAM_ID.as_bytes(),
        data: &data,
        accounts: &accounts,
    };

    let buy = pumpfun::instructions::Buy::match_and_decode(&ix)
        .unwrap()
        .unwrap();
    assert_eq!(buy.amount, 1_000_000);
    assert_eq!(buy.max_sol_cost, 2_500_000_000);
    let named = buy.accounts.as_ref().unwrap();
    assert_eq!(named.global, Pubkey::new(key(0)));
    assert_eq!(named.fee_recipient, Pubkey::new(key(1)));
    assert_eq!(named.user, Pubkey::new(key(6)));
    assert_eq!(named.program, Pubkey::new(key(11)));
    let expected_keys: Vec<Pubkey> = keys.iter().map(|k| Pubkey::new(*k)).collect();
    assert_eq!(buy.account_keys, expected_keys);

    match pumpfun::Instruction::match_and_decode(&ix) {
        Some(Ok(pumpfun::Instruction::Buy(decoded))) => assert_eq!(decoded, buy),
        other => panic!("unexpected {other:?}"),
    }

    // Too few accounts (Anchor: fixed positions): the arguments and raw keys
    // still decode, the named accounts report the missing one.
    let short = RawInstruction {
        accounts: &accounts[..5],
        ..ix
    };
    let partial = pumpfun::instructions::Buy::decode(&short).unwrap();
    assert_eq!(
        (partial.amount, partial.max_sol_cost),
        (1_000_000, 2_500_000_000)
    );
    assert_eq!(
        partial.accounts,
        Err(DecodeError::MissingAccount { index: 5, len: 5 })
    );
    assert_eq!(partial.account_keys, expected_keys[..5]);
    // Truncated arguments.
    let truncated = RawInstruction {
        data: &data[..20],
        ..ix
    };
    assert!(matches!(
        pumpfun::Instruction::decode(&truncated),
        Some(Err(DecodeError::UnexpectedEof { .. }))
    ));
}

#[test]
fn pumpfun_trade_event_from_logs() {
    let event = pumpfun::events::TradeEvent {
        mint: Pubkey::new(key(1)),
        sol_amount: 10,
        token_amount: 20,
        is_buy: true,
        user: Pubkey::new(key(2)),
        timestamp: -3,
        virtual_sol_reserves: 4,
        virtual_token_reserves: 5,
        real_sol_reserves: 6,
        real_token_reserves: 7,
    };
    let payload = trade_event_payload();

    let pump = pumpfun::PROGRAM_ID.to_base58();
    let other = Pubkey::new(key(9)).to_base58();
    let logs = [
        format!("Program {pump} invoke [1]"),
        "Program log: Instruction: Buy".to_string(),
        format!("Program {other} invoke [2]"),
        // Logged by another program: not attributed to pump.fun.
        format!("Program data: {}", base64(&payload)),
        format!("Program {other} success"),
        format!("Program data: {}", base64(&payload)),
        format!("Program {pump} success"),
    ];
    let events: Vec<_> = pumpfun::Event::decode_logs(&logs).collect();
    assert_eq!(events, vec![Ok(pumpfun::Event::TradeEvent(event.clone()))]);
    let typed: Vec<_> = pumpfun::events::TradeEvent::decode_logs(&logs).collect();
    assert_eq!(typed, vec![Ok(event)]);
}

/// The `TradeEvent` of `pumpfun_trade_event_from_logs`.
fn trade_event_payload() -> Vec<u8> {
    Bytes::with(&[189, 219, 127, 211, 78, 230, 97, 238])
        .key(key(1))
        .u64(10)
        .u64(20)
        .bool(true)
        .key(key(2))
        .i64(-3)
        .u64(4)
        .u64(5)
        .u64(6)
        .u64(7)
        .build()
}

fn pubkeys(keys: &[&str]) -> Vec<[u8; 32]> {
    keys.iter()
        .map(|k| k.parse::<Pubkey>().unwrap().to_bytes())
        .collect()
}

fn hex(data: &str) -> Vec<u8> {
    (0..data.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&data[i..i + 2], 16).unwrap())
        .collect()
}

/// Slot 300000000, transaction `3D9DnsXd…KFQ6`, instruction walk index 5:
/// a SwapBaseIn with 17 accounts, `amm_target_orders` left out.
const SWAP_17: &[&str] = &[
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
    "9DvGvRHfW2wKRitehomqHGVYoHtj2AfSV3fCzxFNbhkc",
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1",
    "9DvGvRHfW2wKRitehomqHGVYoHtj2AfSV3fCzxFNbhkc",
    "CptpNyHo8A943LnPbnz1TvWZe2mnPhDLL2snpcFh5bBw",
    "ARMz5KZuHx2QKktUtAm4npMxj18bcLp3wYYFLHtQMWPg",
    "9DvGvRHfW2wKRitehomqHGVYoHtj2AfSV3fCzxFNbhkc",
    "9DvGvRHfW2wKRitehomqHGVYoHtj2AfSV3fCzxFNbhkc",
    "9DvGvRHfW2wKRitehomqHGVYoHtj2AfSV3fCzxFNbhkc",
    "9DvGvRHfW2wKRitehomqHGVYoHtj2AfSV3fCzxFNbhkc",
    "9DvGvRHfW2wKRitehomqHGVYoHtj2AfSV3fCzxFNbhkc",
    "9DvGvRHfW2wKRitehomqHGVYoHtj2AfSV3fCzxFNbhkc",
    "9DvGvRHfW2wKRitehomqHGVYoHtj2AfSV3fCzxFNbhkc",
    "9DvGvRHfW2wKRitehomqHGVYoHtj2AfSV3fCzxFNbhkc",
    "6L6bAvRj69JP1Q4x3DkCi9pmFLEfBBWnJKwEvVAjDdMr",
    "7v3DTKvYy9vRgPtoUhJL7HQ2GS1m5ERE4F6ERY51SYSP",
    "BDFMkurHjWM8HCibLRncxaSF8eQ3oJJztPSr4jUEo3nZ",
];

/// Slot 300000000, transaction `2Kd6MXyM…EBy8`, instruction walk index 5:
/// a SwapBaseIn on the same pool with all 18 accounts.
const SWAP_18: &[&str] = &[
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",
    "9DvGvRHfW2wKRitehomqHGVYoHtj2AfSV3fCzxFNbhkc",
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1",
    "3tkRHzycw7nCLsFoidi9PrxHTXUYwW42sXn2QS3G6a83",
    "3XjgQtZP9nK2pFqCQ1X9BdXMiyRMSr9ACfMBXfdPDwzs",
    "CptpNyHo8A943LnPbnz1TvWZe2mnPhDLL2snpcFh5bBw",
    "ARMz5KZuHx2QKktUtAm4npMxj18bcLp3wYYFLHtQMWPg",
    "srmqPvymJeFKQ4zGQed1GFppgkRHL9kaELCbyksJtPX",
    "8NbPNKpRL6EPWwNYZmjeDgcz1wtvYZLDBiJ1oPccef7Z",
    "ES9x3STo8mexrwWoSyPkfPAkETgkXnqiHrzsxcgZx6qU",
    "CBFuJamE8BMyuKxJAmaw6sHNqWsQaXpSWD6WUZTsmQ2",
    "TnNiNavmoQMeaWtzYx4FCjFnBz7RaULB1pFjDuH8zNv",
    "5SwJHL5USxzy5tPVDfZbm1XGfHuYH8aLuqzv4Nu7jVLF",
    "7ikpE6HMFvzbfSTuomAtP6PpKV4rxHERN23eUnmwuLzZ",
    "46mCxRUNTdiVazWiSBtdg8JRsdhebashuxFHVUCr1Tm7",
    "3T91ZpjXGLrAKhbFJaopA6mbgHF8hr5P9frFD1kgcd5s",
    "BhDw9iyqemDkb1wpxDg6Z15kcLXpuJd3XDA28Dg9bY2M",
    "BNMJMUAdx2tnFdfqkq1mHvjs1ndNLc5uCHaTaELxsgKq",
];

fn raydium_swap(data: &[u8], keys: &[[u8; 32]]) -> raydium_amm::Instruction {
    let accounts: Vec<&[u8]> = keys.iter().map(|k| k.as_slice()).collect();
    raydium_amm::Instruction::match_and_decode(&RawInstruction {
        program_id: raydium_amm::PROGRAM_ID.as_bytes(),
        data,
        accounts: &accounts,
    })
    .unwrap()
    .unwrap()
}

fn pk(key: &str) -> Pubkey {
    key.parse().unwrap()
}

#[test]
fn raydium_swap_with_17_accounts_shifts_past_the_omitted_one() {
    let keys = pubkeys(SWAP_17);
    let raydium_amm::Instruction::SwapBaseIn(swap) =
        raydium_swap(&hex("096814ef218c070000008c0c0800000000"), &keys)
    else {
        panic!("not a SwapBaseIn");
    };
    assert_eq!(swap.amount_in, 8_298_446_132_328);
    assert_eq!(swap.minimum_amount_out, 135_040_000);
    assert_eq!(swap.account_keys.len(), 17);

    let named = swap.accounts.as_ref().unwrap();
    assert_eq!(
        named.amm,
        pk("9DvGvRHfW2wKRitehomqHGVYoHtj2AfSV3fCzxFNbhkc")
    );
    assert_eq!(named.amm_target_orders, None);
    // Index 4 is the pool's coin vault, not the target orders.
    assert_eq!(
        named.pool_coin_token_account,
        pk("CptpNyHo8A943LnPbnz1TvWZe2mnPhDLL2snpcFh5bBw")
    );
    assert_eq!(
        named.pool_pc_token_account,
        pk("ARMz5KZuHx2QKktUtAm4npMxj18bcLp3wYYFLHtQMWPg")
    );
    assert_eq!(
        named.user_source_owner,
        pk("BDFMkurHjWM8HCibLRncxaSF8eQ3oJJztPSr4jUEo3nZ")
    );

    // SwapBaseOut has the same layout (see the fixtures README).
    let mut data = vec![11];
    data.extend(5u64.to_le_bytes());
    data.extend(4u64.to_le_bytes());
    let raydium_amm::Instruction::SwapBaseOut(swap) = raydium_swap(&data, &keys) else {
        panic!("not a SwapBaseOut");
    };
    let named = swap.accounts.unwrap();
    assert_eq!(named.amm_target_orders, None);
    assert_eq!(
        named.pool_coin_token_account,
        pk("CptpNyHo8A943LnPbnz1TvWZe2mnPhDLL2snpcFh5bBw")
    );
}

#[test]
fn raydium_swap_with_18_accounts_is_positional() {
    let keys = pubkeys(SWAP_18);
    let raydium_amm::Instruction::SwapBaseIn(swap) =
        raydium_swap(&hex("09007b4f2bc8000000cb85b40600000000"), &keys)
    else {
        panic!("not a SwapBaseIn");
    };
    assert_eq!(swap.amount_in, 859_720_088_320);
    assert_eq!(swap.minimum_amount_out, 112_494_027);
    let named = swap.accounts.as_ref().unwrap();
    assert_eq!(
        named.amm_target_orders,
        Some(pk("3XjgQtZP9nK2pFqCQ1X9BdXMiyRMSr9ACfMBXfdPDwzs"))
    );
    assert_eq!(
        named.pool_coin_token_account,
        pk("CptpNyHo8A943LnPbnz1TvWZe2mnPhDLL2snpcFh5bBw")
    );
    assert_eq!(
        named.user_source_owner,
        pk("BNMJMUAdx2tnFdfqkq1mHvjs1ndNLc5uCHaTaELxsgKq")
    );

    // Shank accounts and events have no discriminator: they are decoded
    // through their own type, not dispatched.
    assert!(<raydium_amm::events::SwapBaseIn as IdlEvent>::DISCRIMINATOR.is_empty());
    assert!(<raydium_amm::accounts::Fees as IdlAccount>::DISCRIMINATOR.is_empty());
}

#[test]
fn raydium_ambiguous_or_missing_accounts_keep_arguments() {
    // MonitorStep lists 21 accounts, 19 and 20 optional: with 20, either
    // could be the omitted one.
    let mut data = vec![2];
    for limit in [1u16, 2, 3] {
        data.extend(limit.to_le_bytes());
    }
    let keys: Vec<[u8; 32]> = (0..20).map(key).collect();
    let raydium_amm::Instruction::MonitorStep(step) = raydium_swap(&data, &keys) else {
        panic!("not a MonitorStep");
    };
    assert_eq!(
        (
            step.plan_order_limit,
            step.place_order_limit,
            step.cancel_order_limit
        ),
        (1, 2, 3)
    );
    assert_eq!(
        step.accounts,
        Err(DecodeError::AmbiguousAccounts {
            len: 20,
            listed: 21,
            optional: 2
        })
    );
    assert_eq!(step.account_keys.len(), 20);
    // With both omitted, there is a single reading.
    let keys: Vec<[u8; 32]> = (0..19).map(key).collect();
    let raydium_amm::Instruction::MonitorStep(step) = raydium_swap(&data, &keys) else {
        panic!("not a MonitorStep");
    };
    assert!(step.accounts.is_ok());

    // A swap with 16 accounts cannot be explained by optional accounts.
    let keys = pubkeys(&SWAP_17[..16]);
    let swap = raydium_swap(&hex("096814ef218c070000008c0c0800000000"), &keys);
    assert_eq!(
        swap.accounts_error(),
        Some(&DecodeError::MissingAccount { index: 16, len: 16 })
    );
    assert_eq!(swap.account_keys().len(), 16);
    let raydium_amm::Instruction::SwapBaseIn(swap) = swap else {
        panic!("not a SwapBaseIn");
    };
    assert_eq!(swap.amount_in, 8_298_446_132_328);
}

#[test]
fn edge_case_names() {
    use edge_cases::{instructions, types};

    // `swap_accounts` keeps its name; the accounts struct of `swap` yields.
    let _: instructions::SwapAccounts = instructions::SwapAccounts {
        accounts: Ok(instructions::SwapAccountsAccounts {}),
        account_keys: Vec::new(),
    };
    // Keywords, a type named `Option`, and the `accounts` argument renamed.
    let option = types::Option {
        r#type: 1,
        self_: true,
        r: 2,
        w: Some(Box::new(types::Option {
            r#type: 3,
            self_: false,
            r: 4,
            w: None,
        })),
    };
    assert_eq!(
        borsh::decode::<types::Option>(&[1, 1, 2, 1, 3, 0, 4, 0]),
        Ok(option)
    );

    // `foo_bar` and `FooBar` both become `FooBar`: IDL order decides.
    let _ = (types::FooBar { a: 1 }, types::FooBar2 { b: 2 });
    let _ = types::Shape::Self_;
}

#[test]
fn edge_case_swap_accounts_and_args() {
    use edge_cases::{instructions, types};

    // amount 7, Shape::Rect(2, 3), the `accounts` argument (renamed `accounts2`) 9.
    let data = Bytes::with(&[248, 198, 158, 145, 225, 117, 135, 200])
        .u64(7)
        .u8(1)
        .u16(2)
        .u16(3)
        .u8(9)
        .build();
    // The optional `referrer` is passed as the program id.
    let keys = [
        key(1),
        key(2),
        key(3),
        edge_cases::PROGRAM_ID.to_bytes(),
        key(4),
    ];
    let accounts: Vec<&[u8]> = keys.iter().map(|k| k.as_slice()).collect();
    let ix = RawInstruction {
        program_id: edge_cases::PROGRAM_ID.as_bytes(),
        data: &data,
        accounts: &accounts,
    };
    let swap = instructions::Swap {
        amount: 7,
        shape: types::Shape::Rect(2, 3),
        accounts2: 9,
        accounts: Ok(instructions::SwapAccounts2 {
            user: Pubkey::new(key(1)),
            pool_state: Pubkey::new(key(2)),
            pool_vault: Pubkey::new(key(3)),
            referrer: None,
            token_program: Pubkey::new(key(4)),
        }),
        account_keys: keys.iter().map(|k| Pubkey::new(*k)).collect(),
    };
    assert_eq!(instructions::Swap::decode(&ix), Ok(swap));

    // An optional account given as a real address.
    let referrer = key(8);
    let mut with_referrer = accounts.clone();
    with_referrer[3] = &referrer;
    let decoded = instructions::Swap::decode(&RawInstruction {
        accounts: &with_referrer,
        ..ix
    })
    .unwrap();
    assert_eq!(
        decoded.accounts.unwrap().referrer,
        Some(Pubkey::new(referrer))
    );

    // One account short with one optional slot: the optional account is the one left out.
    let short = instructions::Swap::decode(&RawInstruction {
        accounts: &[accounts[0], accounts[1], accounts[2], accounts[4]],
        ..ix
    })
    .unwrap();
    let named = short.accounts.unwrap();
    assert_eq!(named.referrer, None);
    assert_eq!(named.token_program, Pubkey::new(key(4)));
    assert_eq!((short.amount, short.accounts2), (7, 9));

    // An unknown enum variant is an error naming the type.
    let mut bad = data.clone();
    bad[16] = 9;
    assert_eq!(
        instructions::Swap::decode(&RawInstruction { data: &bad, ..ix }),
        Err(DecodeError::InvalidEnumVariant {
            offset: 16,
            type_name: "Shape",
            variant: 9
        })
    );
}

#[test]
fn edge_case_generics_aliases_and_wide_integers() {
    use edge_cases::{instructions, types};

    let wide = types::Wide {
        big: U256::from_u128(u128::MAX),
        signed_big: I256::from_i128(-2),
        c_amount: COption::Some(5),
        owner: None,
        labels: vec!["a".to_string(), "bc".to_string()],
        blob: vec![1, 2],
        signed: i128::MIN + 1,
        unsigned: u128::MAX - 1,
        ratio: 0.5,
        point: types::Point(-1, 1),
        marker: types::Marker,
    };
    let ix = instructions::Type {
        r#match: types::Pair {
            first: 1,
            items: [2, 3],
        },
        wide,
        accounts: Ok(instructions::TypeAccounts {
            self_: Pubkey::new(key(1)),
        }),
        account_keys: vec![Pubkey::new(key(1))],
    };
    // `Wrapper` is `Pair<u64, 2>`: three u64. Then `Wide`, field by field.
    let data = Bytes::with(&[7, 1])
        .u64(1)
        .u64(2)
        .u64(3)
        .u128(u128::MAX)
        .raw(&[0; 16])
        .i128(-2)
        .raw(&[0xff; 16])
        .u32(1)
        .u64(5)
        .u8(0)
        .len(2)
        .string("a")
        .string("bc")
        .len(2)
        .raw(&[1, 2])
        .i128(i128::MIN + 1)
        .u128(u128::MAX - 1)
        .f64(0.5)
        .i64(-1)
        .i64(1)
        .build();
    let mut r = borsh::BorshReader::new(&data[26..]);
    let wide = types::Wide::decode(&mut r).unwrap();
    assert_eq!(wide.unsigned, u128::MAX - 1);
    assert_eq!(wide.signed, i128::MIN + 1);
    assert_eq!(wide.big.to_u128(), Some(u128::MAX));
    assert_eq!(wide.signed_big.to_i128(), Some(-2));
    assert!(r.is_empty());

    let key1 = key(1);
    let accounts: [&[u8]; 1] = [&key1];
    let decoded = edge_cases::Instruction::match_and_decode(&RawInstruction {
        program_id: edge_cases::PROGRAM_ID.as_bytes(),
        data: &data,
        accounts: &accounts,
    });
    assert_eq!(decoded, Some(Ok(edge_cases::Instruction::Type(ix))));

    // NaN is rejected by Borsh.
    let mut nan = vec![0u8];
    nan.extend(f32::NAN.to_le_bytes());
    assert!(matches!(
        borsh::decode::<types::Shape>(&nan),
        Err(DecodeError::InvalidFloat { .. })
    ));
}

#[test]
fn edge_case_zero_copy_layout() {
    use edge_cases::{accounts, types};

    assert_eq!((types::ZcInner::SIZE, types::ZcInner::ALIGN), (16, 8));
    assert_eq!((types::ZcState::SIZE, types::ZcState::ALIGN), (112, 8));
    assert_eq!((types::ZcPacked::SIZE, types::ZcPacked::ALIGN), (11, 1));

    // flag@0, inner@8 (a@8, b@16), list@24 and @40, value@56, ratio@72, key@76, tail to 112.
    let mut raw = vec![0xeeu8; 112];
    raw[0] = 1;
    raw[8] = 2;
    raw[16..24].copy_from_slice(&3u64.to_le_bytes());
    raw[24] = 4;
    raw[32..40].copy_from_slice(&5u64.to_le_bytes());
    raw[40] = 6;
    raw[48..56].copy_from_slice(&7u64.to_le_bytes());
    raw[56..72].copy_from_slice(&(u128::MAX - 8).to_le_bytes());
    raw[72..76].copy_from_slice(&f32::NAN.to_le_bytes());
    raw[76..108].copy_from_slice(&key(9));

    let mut data = vec![1, 2, 3, 4, 5, 6, 7, 8];
    data.extend(&raw);
    let state = accounts::ZcState::decode_account(&data).unwrap();
    assert_eq!(state.flag, 1);
    assert_eq!(state.inner, types::ZcInner { a: 2, b: 3 });
    assert_eq!(
        state.list,
        [types::ZcInner { a: 4, b: 5 }, types::ZcInner { a: 6, b: 7 }]
    );
    assert_eq!(state.value, u128::MAX - 8);
    // Raw memory may hold NaN.
    assert!(state.ratio.is_nan());
    assert_eq!(state.key, Pubkey::new(key(9)));

    // Too short for the layout.
    assert_eq!(
        accounts::ZcState::decode_account(&data[..8 + 111]),
        Err(DecodeError::UnexpectedEof {
            offset: 0,
            needed: 112,
            remaining: 111
        })
    );
    match edge_cases::Account::decode(&data) {
        Some(Ok(edge_cases::Account::ZcState(_))) => {}
        other => panic!("unexpected {other:?}"),
    }

    // Packed: no padding at all.
    assert_eq!(
        types::ZcPacked::decode_zero_copy(&[1, 0, 1, 2, 3, 4, 0, 5, 0, 6, 0]),
        Ok(types::ZcPacked {
            a: 1,
            b: 0x0302_0100,
            c: [4, 5, 6],
        })
    );
}

#[test]
fn fixture_zero_copy_sizes() {
    // Hand-summed field sizes: these Pod / packed types have no padding.
    assert_eq!(meteora_bonding_curve::types::PoolConfig::SIZE, 1040);
    assert_eq!(meteora_bonding_curve::types::VirtualPool::SIZE, 416);
    assert_eq!(meteora_bonding_curve::types::VirtualPool::ALIGN, 8);
    assert_eq!(
        meteora_bonding_curve::types::MeteoraDammV2Metadata::SIZE,
        222
    );

    // Account data shorter than discriminator + SIZE fails cleanly.
    let mut data =
        <meteora_bonding_curve::accounts::VirtualPool as IdlAccount>::DISCRIMINATOR.to_vec();
    data.resize(8 + 415, 0);
    assert!(meteora_bonding_curve::accounts::VirtualPool::decode_account(&data).is_err());
    data.push(0);
    assert!(meteora_bonding_curve::accounts::VirtualPool::decode_account(&data).is_ok());
}

#[test]
fn constants_are_typed() {
    use edge_cases::constants;

    let precision: u64 = constants::PRECISION;
    assert_eq!(precision, 1_000_000_000);
    let seed: &[u8] = constants::SEED;
    assert_eq!(seed, b"seed");
    let label: &str = constants::LABEL;
    assert_eq!(label, "edge");
    assert_eq!(constants::OFFSET, -42i32);
    assert_eq!(constants::RATIO, 0.25f64);
    const { assert!(constants::ENABLED) };
    assert_eq!(constants::ADMIN, edge_cases::PROGRAM_ID);
    assert_eq!(constants::TABLE, [[1u16, 2], [3, 255]]);
    assert_eq!(constants::r#type, 7u8);
}

#[test]
fn program_errors() {
    use edge_cases::ProgramError;

    let error = ProgramError::from_code(6000).unwrap();
    assert_eq!(error, ProgramError::TooSmall);
    assert_eq!(
        (error.code(), error.name(), error.msg()),
        (6000, "TooSmall", Some("Amount is too small"))
    );
    assert_eq!(error.to_string(), "TooSmall (6000): Amount is too small");

    // A code listed twice maps back to its first error.
    assert_eq!(ProgramError::from_code(6001), Some(ProgramError::NoMessage));
    assert_eq!(ProgramError::Duplicate.code(), 6001);
    assert_eq!(ProgramError::NoMessage.msg(), None);
    assert_eq!(ProgramError::NoMessage.to_string(), "no_message (6001)");
    assert_eq!(ProgramError::from_code(1), None);

    assert_eq!(
        pumpfun::ProgramError::from_code(6000).map(|e| e.name()),
        Some("NotAuthorized")
    );
    // Shank programs number their errors from 0.
    assert_eq!(
        raydium_amm::ProgramError::from_code(0).map(|e| e.name()),
        Some("AlreadyInUse")
    );
}

#[test]
fn jupiter_legacy_event_and_enum_share_a_name() {
    // A legacy IDL: the event `Swap` and the unrelated enum `Swap` live in
    // different modules.
    let event = jupiter_v4_swap::events::Swap {
        amm: Pubkey::new(key(1)),
        input_mint: Pubkey::new(key(2)),
        input_amount: 3,
        output_mint: Pubkey::new(key(4)),
        output_amount: 5,
    };
    let data = Bytes::with(<jupiter_v4_swap::events::Swap as IdlEvent>::DISCRIMINATOR)
        .key(key(1))
        .key(key(2))
        .u64(3)
        .key(key(4))
        .u64(5)
        .build();
    assert_eq!(
        jupiter_v4_swap::Event::decode_log_data(&data),
        Some(Ok(jupiter_v4_swap::Event::Swap(event)))
    );
    assert_eq!(
        borsh::decode::<jupiter_v4_swap::types::Swap>(&[0]),
        Ok(jupiter_v4_swap::types::Swap::Saber)
    );
}

#[test]
fn metaplex_program_id_placeholder() {
    use mpl_token_metadata::{instructions, types};

    // `Burn` passes the program id for its absent optional accounts.
    let keys: Vec<[u8; 32]> = (0..14)
        .map(|i| {
            let optional = instructions::BurnAccounts::OPTIONAL[i as usize];
            if optional {
                mpl_token_metadata::PROGRAM_ID.to_bytes()
            } else {
                key(i)
            }
        })
        .collect();
    let accounts: Vec<&[u8]> = keys.iter().map(|k| k.as_slice()).collect();
    let mut data = vec![41, 0];
    data.extend(7u64.to_le_bytes());
    let ix = RawInstruction {
        program_id: mpl_token_metadata::PROGRAM_ID.as_bytes(),
        data: &data,
        accounts: &accounts,
    };
    let burn = instructions::Burn::decode(&ix).unwrap();
    assert_eq!(burn.burn_args, types::BurnArgs::V1 { amount: 7 });
    let named = burn.accounts.unwrap();
    assert_eq!(named.authority, Pubkey::new(key(0)));
    assert_eq!(named.collection_metadata, None);
    assert_eq!(named.metadata, Pubkey::new(key(2)));
    assert_eq!(named.edition, None);
}
