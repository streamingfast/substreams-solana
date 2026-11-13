# Fixture IDLs

Real program IDLs, read by `tests/fixtures.rs` and turned into bindings by
`idlgen-tests`, which adds the synthetic `idlgen-tests/idls/edge_cases.json`.

| Fixture | Format | Covers |
|---|---|---|
| `pumpfun.json` | Anchor legacy | Discriminators computed from names, legacy events |
| `jupiter_v4_swap.json` | Anchor legacy | Composite account groups, data-carrying enums, an event and an enum sharing a name |
| `meteora_bonding_curve.json` | Anchor 0.30 | Zero-copy `repr(C)` accounts, PDAs, fixed addresses |
| `raydium_amm.json` | Shank | One-byte discriminants, no account or event discriminators, omitted optional accounts |
| `mpl_token_metadata.json` | Shank | `hashMap`, `isOptionalSigner`, `legacyOptionalAccountsStrategy`, program-id placeholders |
| `stake_interface.json` | Shank | Undefined clock aliases (`UnixTimestamp`, `Epoch`) and `{"defined": "f64"}` |

## Patches

`raydium_amm.json` marks `swapBaseOut` account 4 (`ammTargetOrders`) `"isOptional": true`, as `swapBaseIn` does. The program
(`process_swap_base_out`) accepts 17 or 18 accounts and leaves it out of the 17-account form, which 29% of mainnet swaps use
(slots 300,000,000 to 300,004,999).

`mpl_token_metadata.json` keeps 5 of the 58 instructions of metaplex-foundation/mpl-token-metadata's `idls/token_metadata.json`,
2 of its accounts, the types they use and the first 5 errors. `stake_interface.json` is the solana-stake-interface IDL. The
fixtures are otherwise kept as published.
