//! The usage snippet of the repository README, kept compiling.

use substreams_solana::pb::sf::solana::r#type::v1::Block;
use substreams_solana::IdlInstruction;
use substreams_solana_idlgen_tests::generated::pumpfun;

#[allow(dead_code)]
fn decode(block: &Block) {
    for trx in block.transactions() {
        for ix in trx.walk_instructions() {
            // One instruction type, or any of the program's instructions.
            if let Some(Ok(buy)) = pumpfun::instructions::Buy::match_and_decode(&ix) {
                println!("buy {} for at most {}", buy.amount, buy.max_sol_cost);
            }
            if let Some(Ok(ix)) = pumpfun::Instruction::match_and_decode(&ix) {
                println!("{} with {} accounts", ix.name(), ix.account_keys().len());
            }
            // Events emitted through a self-CPI (`emit_cpi!`).
            if let Some(Ok(event)) = pumpfun::Event::match_and_decode_cpi(&ix) {
                println!("{}", event.name());
            }
        }

        // Events logged as `Program data:` (`emit!`).
        for event in pumpfun::Event::decode_logs(&trx.meta.log_messages) {
            if let Ok(pumpfun::Event::TradeEvent(trade)) = event {
                println!("trade of {} lamports", trade.sol_amount);
            }
        }
    }
}

// Account data, from an account block or an RPC response.
#[allow(dead_code)]
fn decode_account(data: &[u8]) {
    if let Some(Ok(pumpfun::Account::BondingCurve(curve))) = pumpfun::Account::decode(data) {
        println!("complete: {}", curve.complete);
    }
}

#[test]
fn readme_snippet_compiles() {
    decode(&Block::default());
    decode_account(&[]);
}
