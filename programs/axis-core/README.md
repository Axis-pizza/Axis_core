# axis-core

Reserve-backed DTF program for the Axis Mainnet initial release (V1).

Status: implemented for review on `feat/mainnet-v1-contract`. Nothing here is
approved for Mainnet. The open decisions and the missing production evidence
are listed in
[`docs/reviews/axis-core-v1-review-summary.md`](../../docs/reviews/axis-core-v1-review-summary.md).

## What V1 is

A market is one DTF: a mint, and one reserve vault per constituent, for 2 to
3 constituents. Governance creates it. The first public purchaser funds it.
After that anyone can buy more with USDC, exit to USDC, or exit in kind.

- Core holds the spot assets and never rebalances. Weights drift with price.
- **No instruction reads a price.** There is no oracle, NAV or pricing
  account. Issuance and redemption are quantity arithmetic on live balances.
- One purchase or one USDC redemption is one instruction, so its swaps,
  reserve movements, fee and issuance or burn succeed or revert together.
- The fee is 30 bps, taken in DTF and paid to the treasury in the same
  instruction. Nothing accrues and nothing is claimed.

Quote admission, failure acknowledgments, holdings display and PnL belong to
the App and API. Core enforces the signed bounds, custody and accounting at
execution.

## Accounting

```txt
purchase   gross   = min_i floor(delta_i * supply_before / reserve_i_before)
           fee     = floor(gross * 30 / 10_000)       minted to the treasury
           net     = gross - fee                      minted to the buyer

redeem     fee     = floor(dtf_in * 30 / 10_000)      transferred to the treasury
           burned  = dtf_in - fee
           out_i   = floor(reserve_i * burned / supply_before)
```

`delta_i` is the measured increase of reserve vault `i` across the route, not
a quoted amount. The constituent that supports the least issuance sets it, so
DTF is never issued beyond its backing. Whatever the other legs delivered
beyond that stays in the reserves and backs every holder. USDC the route did
not spend never leaves the buyer's account.

Both directions round against the party who moves, and every remainder stays
in the reserve. `round_trip_never_profits` and
`issuance_rounds_down_and_never_exceeds_any_legs_backing` in
`tests/math_tests.rs` pin the consequence.

### The first purchase

At zero supply there is no ratio to follow. Governance approves the initial
allocation in `CreateMarket` as a **reference basket**: `reference_amount_i`
of each constituent per `reference_supply` DTF. The first purchase uses the
same formula with the reference basket in place of the live reserves and
supply. The first purchaser picks the size, not the allocation or the issuance
rate.

`min_liquidity` DTF of that first issuance is minted to the market's lock
account, a DTF account owned by the market PDA. No instruction signs a DTF
transfer or burn for the PDA, so it never moves. Supply therefore never
returns to zero, the ratio stays defined after every holder leaves, and
inflating the value of one DTF unit by donating to the reserves costs the
donor the locked share of the donation.

`reference_supply`, `reference_amount_i` and `min_liquidity` are per-market
inputs approved by governance. The program hard-codes none of them.

## Instructions

Tag is the first byte of the instruction data. Integers are little-endian.

| Tag | Instruction | Signer |
| --- | --- | --- |
| 0 | `InitializeProtocolConfig` | program upgrade authority |
| 1 | `CreateMarket` | governance |
| 2 | `InitialPurchase` | buyer |
| 3 | `Purchase` | buyer |
| 4 | `RedeemUsdc` | holder |
| 5 | `RedeemInKind` | holder |
| 6 | `Pause` | emergency operator or governance |
| 7 | `Resume` | governance |
| 8 | `UpdateAuthorities` | governance |
| 9 | `SetSwapProgram` | governance |

Account lists and data layouts are documented on each handler in
`src/instructions/`. The value paths:

```txt
Purchase         [buyer (s), market, dtf_mint (w), buyer_usdc (w), buyer_dtf (w),
                  treasury_dtf (w), dtf_token_program, swap_program,
                  reserve_vault × N (w), route accounts...]
                 max_usdc_in(8) | min_net_dtf_out(8) | route legs

InitialPurchase  as Purchase, with liquidity_lock (w) after swap_program

RedeemUsdc       [holder (s), market, dtf_mint (w), holder_dtf (w), treasury_dtf (w),
                  holder_usdc (w), token_program, token_2022_program, swap_program,
                  reserve_vault × N (w), holder_asset × N (w), route accounts...]
                 dtf_in(8) | min_net_usdc_out(8) | route legs

RedeemInKind     [holder (s), market, dtf_mint (w), holder_dtf (w), treasury_dtf (w),
                  token_program, token_2022_program,
                  reserve_vault × N (w), destination × N (w)]
                 dtf_in(8)

route legs       leg_count(1) | leg_count × { account_count(1) | data_len(2) | data }
```

`Purchase` and `RedeemUsdc` lock 8 + N and 9 + 2N accounts before route
accounts and deduplication. `RedeemInKind` locks 7 + 2N and needs no route.
The market account is read-only on every value path, since supply and reserve
balances are read live from the mint and the vaults.

### Signed bounds

- `max_usdc_in`: the buyer's USDC account, which must be theirs and hold the
  market's USDC mint, may not lose more than this across the route.
- `min_net_dtf_out`: the DTF minted to the buyer, after the fee, must reach
  this.
- `min_net_usdc_out`: the holder's USDC account must gain at least this. It is
  net of the Axis fee because the fee is taken in DTF before anything is
  released.

A missed bound or a failed leg reverts the whole transaction. Solana network
fees are still charged.

### Routes and custody

A route leg is an instruction for the market's approved swap program. Core
invokes each leg with exactly the signer and writable flags the transaction
gave its accounts. **Core never signs a leg.** The market PDA signs only two
things anywhere in the program: minting DTF in a purchase, and the plain token
transfers that release a redemption.

- On a purchase, the buyer is the authority on the swaps and the reserve
  vaults are the destinations. Core then requires that no vault lost balance
  and that supply did not move.
- A USDC redemption is the in-kind exit followed by swaps in the same
  instruction. The holder's share is released to the holder's own token
  accounts, and the legs sell from there under the holder's signature. A
  route cannot reach the reserves that stay behind, because nothing with
  authority over them is a signer.

So custody does not depend on what the swap program does. The swap program is
still restricted to one governance-approved program per market, it can never
be Axis Core itself, and the zero address means none is approved: purchases
and USDC redemption are then unavailable, and in-kind redemption is not.

### `CreateMarket` validation

Only the governance authority in `ProtocolConfig` can create a market, and
creating it is the approval for public availability. Before anything is
recorded:

- the DTF mint has no extensions, zero supply, no freeze authority, and the
  market PDA as its only mint authority;
- every constituent mint is an initialized mint with no extensions, and is
  neither the DTF mint nor the USDC mint;
- every reserve vault holds its constituent's mint under that mint's token
  program, is owned by the market PDA, is not frozen, and has no delegate and
  no close authority;
- the liquidity lock is a DTF account under the same rules;
- the reference basket and `min_liquidity` are all positive.

A Token-2022 custody account may carry `ImmutableOwner` and nothing else. No
Token-2022 mint extension is admitted until the asset policy is decided.

### Roles

| Action | Who |
| --- | --- |
| Create a DTF and approve it for public availability | governance |
| Pause purchases or USDC redemption, per DTF | emergency operator, or governance |
| Resume purchases or USDC redemption | governance only |
| Change governance or the emergency operator | governance only |
| Change a market's swap program | governance only |

The two pause flags are independent and per market. `RedeemInKind` reads
neither. No role can move a reserve, mint DTF, or change a market's treasury,
constituents or reference basket.

## Layout

```txt
constants.rs     caps, FEE_BPS, seeds, program ids
math.rs          issuance, release, the 30 bps fee
state/           ProtocolConfig (137 bytes), DTFMarket (408 bytes, inline asset table)
instructions/
  initialize_protocol_config.rs, create_market.rs
  purchase.rs    InitialPurchase and Purchase
  redeem.rs      RedeemUsdc and RedeemInKind
  admin.rs       Pause, Resume, UpdateAuthorities, SetSwapProgram
  route.rs       route leg decoding and CPI
  token.rs       SPL Token / Token-2022 reads and CPIs
  common.rs      PDA creation and account checks
```

## Building and testing

```bash
cargo fmt --all -- --check
cargo check --workspace --all-targets
cargo test --workspace
./scripts/build-sbf.sh
cargo test -p axis-core-test-utils --features sbf-integration --test program_invocation
```

`scripts/build-sbf.sh` produces `target/deploy/axis_core.so`. It pins
platform-tools v1.57 because pinocchio needs rustc 1.89, and works around a
toolchain-link bug in `cargo build-sbf`. Its install step expects solana-cli
3.x; with an older CLI, install the tools once with
`cargo build-sbf --force-tools-install --tools-version v1.57`.

25 host tests cover the arithmetic and the account layouts. 45 LiteSVM tests
run every instruction on the SBF artifact through its real entry point,
without injecting any Axis-owned account.

## Evidence boundary

Every purchase and USDC redemption test swaps through a **route stand-in**:
the legacy SPL Token program approved as the market's swap program, and a
maker who co-signs plain token transfers
(`crates/axis-core-test-utils/src/fixture.rs`). That proves Core's
measurement, bounds, fee, issuance, release and rollback on a real CPI path.

It is not evidence about Jupiter. Account locks, transaction size, compute
units, CPI depth and rollback on a production route depend on the launch
basket, the trade size and the final transaction, and have to be shown on
those. Three constituents is a product ceiling, not a guarantee that a given
basket or size fits.

## Not in V1

Rebalancing, constituent replacement, fee claims, oracle or NAV reads, and
recurring purchases. Core emits no events; consumers read token balance
changes from the transaction.
