# axis-core

Reserve-backed DTF program for Axis v1.

## What v1 is

One atomic transaction per mint and per USDC redeem when the Jupiter routes
fit, at most 3 assets per market, with `RedeemInKind` as the guaranteed exit
underneath. Three assets is the v1 product ceiling, not a guarantee that every
3-asset composition or trade size fits. Mint and USDC Redeem availability is checked separately for
each quote; Mint is refused when the reverse USDC Redeem check fails. Preliminary
account-budget evidence and scripts live in Axis_docs
`docs/spikes/2026-09-10-rebalance-and-mint-locks/`.

Target weights are fixed when the market is created. No instruction changes
constituents or weights; a Strategy Update is post-MVP, and Drift Rebalance
only restores the existing targets.

No keeper, no escrow, no off-chain executor. The accounting model is the
2026-09-18 product decision on Axis_core PR #61; background in Axis_docs
`requirements/01-definitions-and-decision-log.md` §30.

## The two rules everything else follows from

**Pro rata against live balances, in both directions.**

```txt
mint    required_i = ceil(reserve_i * dtf_out / total_supply)
redeem  release_i  = floor(reserve_i * dtf_in  / total_supply)
```

Rounding always favours the holders who did not move, and every remainder
stays in the reserve. `round_trip_never_profits` in `tests/math_tests.rs`
sweeps 175 combinations to pin the consequence: minting and immediately
redeeming the same amount never returns more than it delivered.

**No instruction reads a price.** There is no `PricingSource`, no NAV, no
oracle anywhere on the mint or redeem path. Prices belong to the App and the
indexer. This is what frees the per-asset accounts the lock budget cannot
afford, and it removes oracle staleness, depeg exposure, and the
mispriced-asset over-mint that a value-weighted derivation admits.

## Account budget

The provisional interface target is 11 fixed accounts plus one reserve vault
per asset, so 14 at 3 assets, before variable route accounts and cross-set
deduplication. Mint and Redeem target the same base shape. The backend refuses
a mint whenever the reverse USDC redeem check is failing, so a heavier redeem
list would close mint too.

That budget is why the asset table is inline in `DTFMarket` rather than one
account per asset, and why the proposed Mint path does not read
`ProtocolConfig`. The Treasury fee destination takes the slot the prototype's
fee vault held, so the target is unchanged by the fee decision. The production
handlers must reconcile their actual account lists against this target before
it becomes a certified ABI or execution result.

## What is implemented here

```txt
constants.rs   caps, MINIMUM_LIQUIDITY, FEE_BPS, seeds, program ids
math.rs        delivery, release, the 30 bps fee, redeemable supply
state/         ProtocolConfig (105 bytes), DTFMarket (308 bytes, inline asset table)
instructions/  InitializeProtocolConfig, CreateMarket, PDA creation
```

The fee is 30 bps on Mint and on Redeem, paid to the market's treasury in the
same instruction. There is no fee vault, no claim instruction, and no creator
share, so no fee state is stored. `DTFMarket` snapshots the treasury from
`ProtocolConfig` at creation, so Mint and Redeem never load `ProtocolConfig`.

What the 30 bps is measured on is still open (Axis_docs CANDIDATE-10: a USDC
fee measured on the user's named account can be avoided by funding the route
from another account). The answer can still move the layout. Charged in USDC,
Mint and Redeem also need the USDC mint's identity to check the Treasury
account, so `DTFMarket` would snapshot `usdc_mint` too (340 bytes) or Mint
would load `ProtocolConfig`. Charged in DTF, neither is needed.

Both instructions create their own PDA through the System Program at the
canonical bump, and still succeed if someone pre-funded the address.
`InitializeProtocolConfig` requires the program's upgrade authority, so it must
run before the program is finalized.

20 host tests cover the arithmetic invariants and the account layout; 12
LiteSVM tests run both instructions on-chain without pre-injecting any
Axis-owned account.

## Building

```bash
./scripts/build-sbf.sh
```

Produces `target/deploy/axis_core.so`, which the LiteSVM integration tests
load and execute. The script exists because stock `cargo build-sbf` fails
twice on this workspace: it mis-parses an already-linked Solana toolchain, and
the platform-tools shipped with solana-cli 3.0.15 carry rustc 1.84 while
pinocchio needs 1.89. The script unlinks before and after, and pins
platform-tools v1.57 with rustc 1.95 without touching the solana-cli install.

## What is not implemented yet

No instruction here moves a token. The only CPIs are System Program account
creation.

```txt
SeedMarket                zero-supply bootstrap, Created -> Active    PROPOSED
Mint                      USDC -> Jupiter legs -> reserve vaults      not implemented
Redeem                    reserve vaults -> Jupiter legs -> USDC      not implemented
RedeemInKind              burn DTF, release pro-rata reserves         not implemented
Pause/Unpause/Deprecate   lifecycle                                   not implemented
Drift Rebalance           restore target weights                      not implemented
```

`CreateMarket` records the DTF mint and the reserve vault addresses without
validating them (Axis_docs CANDIDATE-09). That has to be fixed before any of
the instructions above: `RedeemInKind` is only a guaranteed exit if the market
PDA really controls every vault.

### Zero-supply bootstrap (proposed, not approved)

`Mint` is undefined at zero supply, since there is nothing to be proportional
to (`required_delivery` returns `MarketNotSeeded`), so the first issuance
cannot go through it. The proposal is `SeedMarket`, the only `Created ->
Active` transition:

```txt
creator deposits an explicit in-kind basket, amount_i >= SEED_MIN_AMOUNT_PER_ASSET
creator names initial_supply >= SEED_MIN_INITIAL_SUPPLY
Core mints initial_supply:
    MINIMUM_LIQUIDITY  to a market-owned account no instruction can spend
    the rest           to the creator
```

`total_supply >= MINIMUM_LIQUIDITY` then holds for the life of the market, so
the pro-rata ratio is always defined and the first-depositor rounding attack
is gone. `RedeemInKind` treats `total_supply <= MINIMUM_LIQUIDITY` as fully
wound down, and that is the only point where it refuses a live market.

Open: the two `SEED_MIN_*` values, and whether anything should check the
seed basket against the target weights. Core has no price to check it with,
and every later mint copies the seed basket's quantity ratios.

### Swap authority and destination validation (proposed)

The aggregator is Jupiter, by CPI. `ApprovedRoute` and direct venue CPI are
not carried into v2.

- **Mint.** Core never signs a swap leg; the user is the authority on the
  route. After the CPI, Core checks on measured balance deltas that each
  reserve vault gained at least `required_i` and that no reserve vault lost
  anything (`ReserveDebitedOnMint`). A route that pays anywhere else fails the
  delivery check, so no destination list is needed.
- **Redeem.** Core has to authorize the reserve assets leaving, and signing
  the route with the market PDA exposes custody even when every balance looks
  right: a route can approve a delegate, change a vault owner, or move the
  locked minimum liquidity (Axis_docs CANDIDATE-07, reproduced by execution).
  Proposed guard: before the CPI, reject any route account that is a
  market-owned token account other than this market's reserve vaults, plus
  the DTF mint and Axis Core; after it, every reserve vault keeps its owner,
  length and Initialized state, with no delegate and no close authority. Per-leg
  `min_out` and aggregate `min_usdc_out` are enforced on measured deltas.
- **Alternative for Redeem, not decided.** Core releases `release_i` to
  accounts the user controls and the user signs the swaps, so Core's signature
  never enters a route and CANDIDATE-07 disappears. The cost is that the
  aggregate `min_usdc_out` and a USDC-denominated fee move out of Core.

### Evidence

Nothing here is production evidence yet. The measured Mint, Redeem and
RedeemInKind figures (locks, bytes, CU) come from a prototype with a stand-in
venue, and the Jupiter account counts come from quotes (Axis_docs PR #10).
Account locks, transaction bytes, compute units, CPI authorization and rollback
still have to be shown on real transactions with production Jupiter routes
before three constituents can be called supported.
