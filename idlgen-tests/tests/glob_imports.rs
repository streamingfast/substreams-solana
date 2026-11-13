//! Generated decode methods stay unambiguous with every runtime trait
//! glob-imported.

#![allow(unused_imports)]

mod support;

use substreams_solana::borsh::*;
use substreams_solana::idl::*;
use substreams_solana_idlgen_tests::generated::pumpfun;
use support::bytes::Bytes;

#[test]
fn methods_resolve_with_all_traits_in_scope() {
    let data = Bytes::with(&[102, 6, 61, 18, 1, 218, 235, 234])
        .u64(1)
        .u64(2)
        .build();
    let keys = [[0u8; 32]; 12];
    let accounts: Vec<&[u8]> = keys.iter().map(|k| k.as_slice()).collect();
    let buy = pumpfun::instructions::Buy {
        amount: 1,
        max_sol_cost: 2,
        accounts: Ok(pumpfun::instructions::BuyAccounts::default()),
        account_keys: vec![Pubkey::default(); 12],
    };
    let ix = RawInstruction {
        program_id: pumpfun::PROGRAM_ID.as_bytes(),
        data: &data,
        accounts: &accounts,
    };
    assert_eq!(pumpfun::instructions::Buy::decode(&ix), Ok(buy.clone()));
    assert_eq!(
        pumpfun::instructions::Buy::match_and_decode(&ix),
        Some(Ok(buy))
    );

    let curve = pumpfun::accounts::BondingCurve {
        virtual_token_reserves: 1,
        virtual_sol_reserves: 2,
        real_token_reserves: 3,
        real_sol_reserves: 4,
        token_total_supply: 5,
        complete: true,
    };
    let account_data = Bytes::with(<pumpfun::accounts::BondingCurve as IdlAccount>::DISCRIMINATOR)
        .u64(1)
        .u64(2)
        .u64(3)
        .u64(4)
        .u64(5)
        .bool(true)
        .build();
    assert_eq!(
        pumpfun::accounts::BondingCurve::decode_account(&account_data),
        Ok(curve.clone())
    );
    assert_eq!(
        pumpfun::accounts::BondingCurve::match_and_decode(&account_data),
        Some(Ok(curve.clone()))
    );
    // `decode` on an account type is `BorshDecode::decode`, fields only.
    let mut r = BorshReader::new(&account_data[8..]);
    assert_eq!(pumpfun::accounts::BondingCurve::decode(&mut r), Ok(curve));

    let event = pumpfun::events::CompleteEvent {
        user: Pubkey::new([1; 32]),
        mint: Pubkey::new([2; 32]),
        bonding_curve: Pubkey::new([3; 32]),
        timestamp: 4,
    };
    let log_data = Bytes::with(<pumpfun::events::CompleteEvent as IdlEvent>::DISCRIMINATOR)
        .key([1; 32])
        .key([2; 32])
        .key([3; 32])
        .i64(4)
        .build();
    assert_eq!(
        pumpfun::events::CompleteEvent::decode_log_data(&log_data),
        Ok(event.clone())
    );
    let cpi_data = Bytes::with(&EVENT_IX_TAG_LE).raw(&log_data).build();
    let cpi = RawInstruction {
        program_id: pumpfun::PROGRAM_ID.as_bytes(),
        data: &cpi_data,
        accounts: &[],
    };
    assert_eq!(
        pumpfun::events::CompleteEvent::match_and_decode_cpi(&cpi),
        Some(Ok(event))
    );
}
