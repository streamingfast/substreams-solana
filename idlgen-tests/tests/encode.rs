//! Generated encoders write the bytes the decoders read: instruction data, account keys, event
//! payloads and zero-copy layouts.

mod support;

use substreams_solana::borsh::{self, BorshDecode, BorshEncode, Pubkey};
use substreams_solana::idl::{IdlAccount, IdlInstruction, RawInstruction};
use substreams_solana_idlgen_tests::generated::{
    edge_cases, jupiter_v4_swap, meteora_bonding_curve, mpl_token_metadata, pumpfun, raydium_amm,
};
use support::bytes::Bytes;

fn key(n: u8) -> [u8; 32] {
    [n; 32]
}

fn decode<T: IdlInstruction>(program_id: &[u8], data: &[u8], keys: &[[u8; 32]]) -> T {
    let accounts: Vec<&[u8]> = keys.iter().map(|k| k.as_slice()).collect();
    T::decode(&RawInstruction {
        program_id,
        data,
        accounts: &accounts,
    })
    .unwrap()
}

#[test]
fn instruction_data_and_account_keys() {
    let data = Bytes::with(&[102, 6, 61, 18, 1, 218, 235, 234])
        .u64(1_000_000)
        .u64(2_500_000_000)
        .build();
    let keys: Vec<[u8; 32]> = (0..12).map(key).collect();
    let buy: pumpfun::instructions::Buy = decode(pumpfun::PROGRAM_ID.as_bytes(), &data, &keys);
    assert_eq!(buy.encode_data(), data);
    assert_eq!(
        buy.accounts.as_ref().unwrap().to_account_keys(),
        buy.account_keys
    );
    let rebuilt =
        pumpfun::instructions::Buy::new(1_000_000, 2_500_000_000, buy.accounts.clone().unwrap());
    assert_eq!(rebuilt, buy);
}

#[test]
fn absent_optional_accounts_are_encoded_as_the_idl_says() {
    // Raydium has no `legacyOptionalAccountsStrategy`: an absent account is written as the
    // program id, so a 17-account swap re-encodes to 18 accounts.
    let data = Bytes::with(&[9]).u64(5).u64(4).build();
    let keys: Vec<[u8; 32]> = (0..17).map(key).collect();
    let swap: raydium_amm::instructions::SwapBaseIn =
        decode(raydium_amm::PROGRAM_ID.as_bytes(), &data, &keys);
    let encoded = swap.accounts.unwrap().to_account_keys();
    assert_eq!(encoded.len(), 18);
    assert_eq!(encoded[4], raydium_amm::PROGRAM_ID);
    assert_eq!(encoded[5], Pubkey::new(key(4)));

    // Metaplex `Burn` writes its program-id placeholders back.
    let keys: Vec<[u8; 32]> = (0..14)
        .map(
            |i| match mpl_token_metadata::instructions::BurnAccounts::OPTIONAL[i as usize] {
                true => mpl_token_metadata::PROGRAM_ID.to_bytes(),
                false => key(i),
            },
        )
        .collect();
    let burn: mpl_token_metadata::instructions::Burn = decode(
        mpl_token_metadata::PROGRAM_ID.as_bytes(),
        &Bytes::with(&[41, 0]).u64(7).build(),
        &keys,
    );
    assert_eq!(burn.account_keys, burn.accounts.unwrap().to_account_keys());

    // `CreateMetadataAccountV3` uses `legacyOptionalAccountsStrategy`: the absent `rent` account
    // is left out.
    let accounts = mpl_token_metadata::instructions::CreateMetadataAccountV3Accounts {
        rent: None,
        ..Default::default()
    };
    assert_eq!(accounts.to_account_keys().len(), 6);
}

#[test]
fn edge_case_arguments() {
    use edge_cases::{instructions, types};

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
    assert_eq!(borsh::encode(&option), [1, 1, 2, 1, 3, 0, 4, 0]);

    let swap = instructions::Swap::new(
        7,
        types::Shape::Rect(2, 3),
        9,
        instructions::SwapAccounts2 {
            user: Pubkey::new(key(1)),
            pool_state: Pubkey::new(key(2)),
            pool_vault: Pubkey::new(key(3)),
            referrer: None,
            token_program: Pubkey::new(key(4)),
        },
    );
    assert_eq!(
        &swap.encode_data()[8..],
        &[7, 0, 0, 0, 0, 0, 0, 0, 1, 2, 0, 3, 0, 9]
    );
    // An Anchor-style placeholder for the absent `referrer`.
    assert_eq!(swap.account_keys[3], edge_cases::PROGRAM_ID);

    fn round_trip<T: BorshDecode + BorshEncode + PartialEq + std::fmt::Debug>(value: T) {
        assert_eq!(borsh::decode::<T>(&borsh::encode(&value)), Ok(value));
    }
    round_trip(types::Shape::Circle { radius: 1.5 });
    round_trip(types::Pair::<u8, 3> {
        first: 1,
        items: [1, 2, 3],
    });
}

#[test]
fn events() {
    let event = jupiter_v4_swap::events::Swap {
        amm: Pubkey::new(key(1)),
        input_mint: Pubkey::new(key(2)),
        input_amount: 3,
        output_mint: Pubkey::new(key(4)),
        output_amount: 5,
    };
    let data = event.encode_log_data();
    assert_eq!(
        jupiter_v4_swap::Event::decode_log_data(&data),
        Some(Ok(jupiter_v4_swap::Event::Swap(event.clone())))
    );
    assert_eq!(
        jupiter_v4_swap::Event::decode_cpi_data(&event.encode_cpi_data()),
        Some(Ok(jupiter_v4_swap::Event::Swap(event)))
    );
    assert_eq!(borsh::encode(&jupiter_v4_swap::types::Swap::Saber), [0]);
}

#[test]
fn zero_copy_layouts() {
    use edge_cases::{accounts, types};

    // Padding bytes are 0xee on the way in; re-encoding zeroes them, nested padding included.
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
    let mut expected = data.clone();
    for (start, end) in [(1, 8), (9, 16), (25, 32), (41, 48), (108, 112)] {
        expected[8 + start..8 + end].fill(0);
    }
    assert_eq!(state.encode_account(), expected);

    let packed = types::ZcPacked {
        a: 1,
        b: 0x0302_0100,
        c: [4, 5, 6],
    };
    assert_eq!(packed.encode_zero_copy(), [1, 0, 1, 2, 3, 4, 0, 5, 0, 6, 0]);

    let mut data =
        <meteora_bonding_curve::accounts::VirtualPool as IdlAccount>::DISCRIMINATOR.to_vec();
    data.resize(8 + 416, 0);
    let pool = meteora_bonding_curve::accounts::VirtualPool::decode_account(&data).unwrap();
    assert_eq!(pool.encode_account(), data);
}
