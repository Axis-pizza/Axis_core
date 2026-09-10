# axis-core

Reserve-backed DTF program for Axis v1.

## What v1 is

One atomic transaction per mint and per USDC redeem, at most 3 assets per
market, with `RedeemInKind` available unconditionally underneath. The 3-asset
cap is measured, not chosen: an atomic 3-leg mint fits the 64 account-lock
limit and a 4-leg mint does not. Evidence and scripts live in Axis_docs
`docs/spikes/2026-09-10-rebalance-and-mint-locks/`.

No keeper, no escrow, no off-chain executor. See Axis_docs
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

11 fixed accounts plus one reserve vault per asset, so 14 at 3 assets, for
mint and for redeem alike. The symmetry is required: the backend refuses a
mint whenever the USDC redeem check is failing, so a heavier redeem list
would close mint too.

That budget is why the asset table is inline in `DTFMarket` rather than one
account per asset, and why `ProtocolConfig` is not read during a mint.

## What is implemented here

```txt
constants.rs   caps, MINIMUM_LIQUIDITY, seeds
math.rs        delivery, release, fee base and split, redeemable supply
state/         ProtocolConfig, DTFMarket with the inline asset table
```

19 host tests cover the arithmetic invariants and the account layout.

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

Token movement and the swap CPI, so `Mint`, `Redeem` and `RedeemInKind` are
not here. `InitializeProtocolConfig` and `CreateMarket` are, and they run
on-chain under LiteSVM.

Account creation is not done through a System Program CPI yet either: both
handlers expect an allocated, program-owned, zeroed account, which is the
state a `create_account` CPI leaves behind.

Two interface questions must be settled before the handlers are written:

- **Who signs the swap CPI on mint.** If the program signs with the reserve
  authority, a route can name a reserve vault as its source and drain it. The
  proposal is that the program never signs on mint: the user is the swap
  authority, the program verifies each leg's destination is the matching
  reserve vault, and reserves can then only ever be credited on that path.
- **Aggregator or direct venue CPI.** Both lock measurements used Jupiter's
  aggregator, while `requirements/05` EXEC-001 to 005 describe validating an
  `ApprovedRoute` and calling the venue directly. Only one of those fits the
  measured budget.
