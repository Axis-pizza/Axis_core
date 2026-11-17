# Axis Core V1: review summary

For Muse and Kidney. Prepared 2026-11-17.

- **Base:** `821a1da4de63dcc771d54bf1a3f5c8215d5336c2`, the head of Core PR #61,
  unchanged since the handoff snapshot.
- **Branch:** `feat/mainnet-v1-contract`, local only. Nothing is pushed.
- **Scope:** the on-chain program, its tests and its READMEs. No deployment.

Everything below is implemented and passes locally. None of it is approved for
Mainnet: five decisions are open, production Jupiter evidence does not exist
yet, and the security reviews in requirements §7 have not started.

## Flows

Paths are under `programs/axis-core/src/instructions/`. Tests are in
`crates/axis-core-test-utils/tests/program_invocation.rs` and run the SBF
artifact through its real entry point.

| Flow | Status | Implementation | Test | Remaining dependency |
| --- | --- | --- | --- | --- |
| Governance-bound creation and approval | Done | `create_market.rs` | `governance_creates_a_market_with_validated_custody`, `only_governance_can_create_a_market` | Squads vault address as governance (D5) |
| DTF mint and reserve vault validation | Done | `create_market.rs`, `token.rs` | `create_market_validates_the_dtf_mint`, `..._every_reserve_vault`, `..._the_composition` | Asset admission policy (D3) |
| First public purchase | Done, mechanism needs sign-off | `purchase.rs` (seeding branch) | `the_first_public_purchase_funds_the_approved_basket`, `the_first_purchaser_cannot_change_the_allocation_or_the_issuance`, `a_first_purchase_must_issue_more_than_the_locked_minimum` | D1, and per-DTF launch values |
| Competing first purchases | Done | `purchase.rs` | `the_loser_of_a_first_purchase_race_buys_as_an_ordinary_purchaser` | D1 |
| Additional purchase | Done | `purchase.rs`, `math.rs` | `an_additional_purchase_issues_against_actual_reserve_increases` | Jupiter evidence (D2) |
| Signed max USDC and min net DTF | Done | `purchase.rs` | `a_purchase_below_the_signed_minimum_net_output_fails_atomically`, `a_purchase_above_the_signed_maximum_usdc_fails_atomically` | None in Core |
| Surplus stays in reserves, unspent USDC stays with the buyer | Done | `purchase.rs` | `an_additional_purchase_issues_against_actual_reserve_increases`, `the_first_purchaser_cannot_change_the_allocation_or_the_issuance` | Surplus limit, if wanted (defaults table) |
| USDC redemption, partial and full | Done | `redeem.rs` | `a_partial_usdc_redemption_pays_the_fee_in_dtf_and_delivers_net_usdc`, `a_full_usdc_redemption_exits_the_whole_position` | Jupiter evidence (D2) |
| Signed min net USDC | Done | `redeem.rs` | `a_usdc_redemption_below_the_signed_minimum_net_usdc_fails_atomically` | None in Core |
| In-kind redemption, partial and full | Done | `redeem.rs` | `a_partial_in_kind_redemption_transfers_the_proportional_quantities`, `full_in_kind_exits_leave_the_locked_minimum_backed` | D3 (a frozen constituent blocks it) |
| 30 bps fee in DTF to the treasury, all three paths | Done | `purchase.rs`, `redeem.rs`, `math.rs` | Treasury balance asserted in each purchase and redemption test above | Fee rounding (D4) |
| Atomic rollback on a failed bound or leg | Done | whole instruction | The `..._fails_atomically` tests, `a_failed_route_leg_reverts_the_legs_before_it`, `a_failed_redemption_swap_reverts_the_burn_the_fee_and_the_releases` | Rollback on a real route (D2) |
| Per-DTF pause of purchases and of USDC redemption | Done | `admin.rs` | `purchases_and_usdc_redemption_pause_independently_and_in_kind_stays_open`, `a_pause_applies_to_one_dtf_only` | None in Core |
| In-kind exit during pauses | Done | `redeem.rs` reads no flag | same test | None in Core |
| Operator pauses, governance resumes, governance changes roles | Done | `admin.rs` | `only_the_operator_or_governance_pauses_and_only_governance_resumes`, `only_governance_changes_governance_and_the_emergency_operator` | D5 |
| Production Jupiter route | **Not available** | `route.rs` is generic; no market has Jupiter approved | None | D2 |

For every rejected bound and every intermediate failure, the test compares a
full balance snapshot before and after: DTF supply, every reserve, the lock,
the treasury, and the buyer's, holder's and maker's USDC, DTF and constituent
accounts. They are equal.

### The additional-purchase fixture

`an_additional_purchase_issues_against_actual_reserve_increases` reaches the
starting state through real instructions and reproduces the handoff table
exactly:

| Item | Expected | Observed |
| --- | --- | --- |
| Supply before | 100,000 | 100,000 |
| Reserves before | A = 200,000, B = 500,000 | same |
| Reserve increases | A = 2,100, B = 5,000 | same |
| Gross DTF | 1,000, limited by B | 1,000 |
| Treasury and buyer | 3 and 997 | 3 and 997 |
| Reserves after | A = 202,100, B = 505,000 | same |
| Signed minimum net of 998 | fails atomically | `SlippageExceeded`, snapshot unchanged |

## Commands and results

Run on the final tree, macOS, rustc 1.93.1 host, platform-tools v1.57.

| Command | Result |
| --- | --- |
| `cargo fmt --all -- --check` | Pass |
| `cargo check --workspace --all-targets` | Pass, no warnings |
| `cargo test --workspace` | Pass: 13 math, 12 state, 4 LiteSVM smoke, 8 Token-2022 spike |
| `./scripts/build-sbf.sh` | Pass: `target/deploy/axis_core.so`, 62,888 bytes |
| `cargo test -p axis-core-test-utils --features sbf-integration --test program_invocation` | Pass: 45 of 45 |
| `cargo build -p axis-core --features bpf-entrypoint` (CI job) | Pass |

Not run: `cargo clippy`, which is not installed for the pinned toolchain and
is not a CI job.

One environment note. `scripts/build-sbf.sh` installs platform-tools with
`cargo build-sbf --install-only`, which the solana-cli 2.2.21 on this machine
does not have. The tools were installed once with
`cargo build-sbf --force-tools-install --tools-version v1.57`; the script then
ran unmodified. The script itself was not changed.

## Local evidence and production evidence

**Local, on a test adapter.** Every purchase and USDC redemption test swaps
through a route stand-in: the legacy SPL Token program approved as the
market's swap program, with a maker who co-signs plain token transfers
(`crates/axis-core-test-utils/src/fixture.rs`). This proves Core's balance
measurement, bounds, fee, issuance, release and rollback over a real CPI. The
tests that do not depend on it are `InitializeProtocolConfig`, `CreateMarket`,
`RedeemInKind`, pause, resume and role changes.

Compute use with the stand-in, for orientation only: additional purchase
10,639 CU (2 assets) and 12,355 CU (3 assets); USDC redemption 13,230 CU and
16,164 CU; in-kind redemption 8,754 CU (2 assets). These measure Core plus
trivial token transfers. They are not a Jupiter estimate.

**Production, missing.** Nothing here shows a Jupiter route. Still to
establish, for the exact launch baskets and trade sizes: the resolved account
list against the 64-lock cap, serialized size against 1,232 bytes, compute
units, CPI depth with Core as the caller, and rollback on a real route. Core's
own account cost is 8 + N for a purchase and 9 + 2N for a USDC redemption
before route accounts.

## Decisions needed

### D1. How the first purchase is priced and what it locks

**Question.** Do you approve (a) governance fixing the initial allocation as a
reference basket, `reference_amount_i` per `reference_supply` DTF, and (b)
locking `min_liquidity` DTF of the first issuance for good? If so, what are
the values for each launch DTF?

**Recommendation.** Yes to both. Core has no price, so quantities per DTF are
the only form of "initial allocation" it can enforce. The lock is what keeps
supply above zero and makes the donation-and-rounding attack unprofitable.

**For a holder.** The first purchaser receives `gross - fee - min_liquidity`.
With a lock of 1,000 base units of a 6-decimal DTF priced near 1 USDC, that is
about 0.001 USDC, once per DTF. The fee is charged on the full gross,
including the locked part. Supply never returns to zero, so "zero supply after
a full exit" does not arise; the locked share of the reserves stays behind.
The loser of a race for the first purchase is not rejected: their
`InitialPurchase` runs as an ordinary purchase under their own signed bounds.

**Blocks.** The `CreateMarket` data for any real market. The seeding branch of
`purchase.rs` and the first-purchase tests encode this mechanism. Fixture
values (2 A and 5 B per DTF, lock 1,000) exist only in tests.

### D2. The Jupiter integration and the shape of a USDC exit

**Question.** Do you approve (a) one governance-approved swap program per
market, invoked by CPI with the user's signature only, and (b) a USDC exit
that first releases the holder's share to the holder's own token accounts and
then swaps from there?

**Recommendation.** Yes. Under (b) the market PDA never signs a route, so no
route can reach the remaining reserves and no route-account guard is needed.
The alternative in the PR #61 README, signing the route with the market PDA,
exposes custody to whatever the route does with that signature.

**For a holder.** A USDC exit needs a token account for each constituent
(rent is recoverable by closing them), and costs N more account locks than
the PDA-signed design. If the route sells less than was released, the
remainder stays in the holder's account. On a purchase the route delivers
straight into the reserve vaults.

**Blocks.** Setting any market's swap program to Jupiter. That should wait for
a fork replay or equivalent with real routes for the launch baskets and
sizes. `MAX_ROUTE_LEG_ACCOUNTS` is 48 per leg, set by the CPI stack frame;
raise it only with evidence that a leg needs more.

### D3. Asset admission: extensions and freeze authority

**Question.** Which Token-2022 extensions, if any, are admitted on the DTF
mint and on constituents? Should Core refuse a constituent whose mint has a
freeze authority?

**Recommendation.** Keep the current rule until a specific asset needs more:
no mint extensions at all, and no freeze authority on the DTF mint. Do not
refuse freeze authority on constituents in Core; make it an explicit item in
the governance approval of each basket.

**For a holder.** A constituent with a freeze authority can have its reserve
vault, or the holder's own account, frozen by that issuer. `RedeemInKind`
transfers every constituent or none, so one frozen constituent blocks the
in-kind exit for that DTF. This is the limitation requirements §4 says must be
explained. If you want a holder to be able to leave a frozen constituent
behind, that is a new instruction and a product decision.

**Blocks.** Approving a basket with a Token-2022 constituent that has any
extension, or a DTF mint with on-mint metadata. Both are rejected by
`read_mint` today.

### D4. Fee rounding on dust

**Question.** Should the fee round down, so that an amount under 334 base
units pays no fee, or round up?

**Recommendation.** Keep rounding down, as in PR #61. Avoiding the fee means
moving under 334 base units per transaction, which costs more in network fees
than it saves.

**For a holder.** Rounding down never charges more than 0.30%. Rounding up
would charge up to one extra base unit per operation.

**Blocks.** `math::fee` and the exact treasury amounts asserted in tests.

### D5. Governance, operator and treasury setup

**Question.** Which addresses are governance (the Squads vault), the emergency
operator and the treasury owner? Who holds the program upgrade authority? Is
a one-step governance handover acceptable, and must the treasury be
changeable?

**Recommendation.** Name the Squads vault as governance at initialization.
Keep the one-step `UpdateAuthorities` for V1, since Squads' own review is the
check on the new address. Leave the treasury fixed.

**For a holder.** No role can move a reserve, mint DTF or change a market's
constituents. Governance can pause and resume purchases and USDC redemption
and can change the swap program. The upgrade authority can replace the
program, which is outside what Core can constrain.

**Blocks.** `InitializeProtocolConfig` on any real deployment. A wrong
governance address in `UpdateAuthorities` is not recoverable, and neither the
protocol treasury nor a market's snapshot of it can be changed.

## Defaults chosen, for your veto

These follow from the requirements or are small enough that I picked one.
Each is a few lines to change.

| Area | What the code does | Alternative |
| --- | --- | --- |
| Creation and approval | One governance instruction does both. | Create paused, approve with `Resume`. |
| Minimum purchase | None in Core beyond issuing at least 1 base unit to the buyer. | A per-market minimum. The App can enforce one today. |
| Surplus limit | None. The buyer's `min_net_dtf_out` is the protection. | A per-market cap on surplus per leg. |
| Direct transfers | Tokens sent to a vault are a gift to all holders, before or after launch. They never count as a buyer's delivery. | None needed. |
| USDC as a constituent | Refused, with the DTF mint. | Allow it; it needs no swap leg. |
| Constituent count | 2 or 3, as in PR #61. | Allow 1. |
| Who can pause | The operator, and governance too. | Operator only. |
| Dust redemption | Refused if it would release nothing of any constituent. | Allow it. |
| `max_usdc_in` | Measured on the one USDC account named in the instruction. | None; the App names the account the route spends from. |
| Events | None. Consumers read token balance changes. | Emit logs or return data once the App says what it needs. |

## Changes to the PR #61 foundation

- `ProtocolConfig` gained `emergency_operator` (137 bytes, `axiscfg3`), and
  `InitializeProtocolConfig` now names governance in its data instead of
  taking the upgrade authority as governance.
- `DTFMarket` is 408 bytes under `dtfmkt05`. Target weights, `creator` and
  `status` are gone. It now stores the USDC mint, swap program, liquidity lock,
  reference basket, DTF token program and the two pause flags.
- `MINIMUM_LIQUIDITY`, the weight constants and `SeedMarket` are removed. The
  locked minimum is per market and comes from the first public purchase.
- `CreateMarket` no longer takes mint and vault addresses in its data. It
  reads them from the accounts it validates.
