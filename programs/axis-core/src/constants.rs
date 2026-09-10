//! Protocol constants. See Axis_docs `specs/constants.md` and `01` §30.

/// Markets hold 2 to 3 assets. The upper bound is measured, not chosen: an
/// atomic 3-leg mint fits the 64 account-lock cap and a 4-leg mint does not.
/// Evidence: Axis_docs `docs/spikes/2026-09-10-rebalance-and-mint-locks/`.
pub const MIN_ASSETS: usize = 2;
pub const MAX_ASSETS: usize = 3;

/// DTF minted to a market-owned account at SeedMarket that no instruction can
/// burn or transfer. Keeps `total_supply > 0` for the life of the market, so
/// the pro-rata ratio every mint and redeem depends on is always defined, and
/// removes the first-depositor rounding attack.
pub const MINIMUM_LIQUIDITY: u64 = 1_000;

/// Basis-point denominator for every fee and weight calculation.
pub const BPS_DENOMINATOR: u64 = 10_000;

/// Market weights must sum to exactly this, and no single weight may be below
/// MIN_WEIGHT_BPS.
pub const TOTAL_WEIGHT_BPS: u16 = 10_000;
pub const MIN_WEIGHT_BPS: u16 = 100;

pub const PROTOCOL_CONFIG_SEED: &[u8] = b"protocol_config";
pub const MARKET_SEED: &[u8] = b"market";
