//! Protocol constants. See Axis_docs `specs/constants.md` and `01` §30.

use pinocchio::Address;

/// Markets hold 2 to 3 assets. Three is the v1 product ceiling; it does not
/// guarantee that every 3-asset composition or trade fits the account-lock
/// cap. Supported quotes are checked separately in both directions.
/// Preliminary evidence: Axis_docs
/// `docs/spikes/2026-09-10-rebalance-and-mint-locks/`.
pub const MIN_ASSETS: usize = 2;
pub const MAX_ASSETS: usize = 3;

/// DTF minted to a market-owned account at SeedMarket that no instruction can
/// burn or transfer. Keeps `total_supply > 0` for the life of the market, so
/// the pro-rata ratio every mint and redeem depends on is always defined, and
/// removes the first-depositor rounding attack.
pub const MINIMUM_LIQUIDITY: u64 = 1_000;

/// Basis-point denominator for every fee and weight calculation.
pub const BPS_DENOMINATOR: u64 = 10_000;

/// Fee on Mint and on Redeem, sent to the market's treasury in the same
/// instruction (2026-09-18 decision). There is no fee vault, no claim and no
/// creator share, so there is nothing to configure or accrue.
pub const FEE_BPS: u64 = 30;

/// Market weights must sum to exactly this, and no single weight may be below
/// MIN_WEIGHT_BPS.
pub const TOTAL_WEIGHT_BPS: u16 = 10_000;
pub const MIN_WEIGHT_BPS: u16 = 100;

pub const PROTOCOL_CONFIG_SEED: &[u8] = b"protocol_config";
pub const MARKET_SEED: &[u8] = b"market";

/// `11111111111111111111111111111111`
pub const SYSTEM_PROGRAM_ID: Address = Address::new_from_array([0; 32]);

/// `BPFLoaderUpgradeab1e11111111111111111111111`, owner of the ProgramData
/// account that records Axis Core's upgrade authority.
pub const BPF_LOADER_UPGRADEABLE_ID: Address = Address::new_from_array([
    2, 168, 246, 145, 78, 136, 161, 176, 226, 16, 21, 62, 247, 99, 174, 43, 0, 194, 185, 61, 22,
    193, 36, 210, 192, 83, 122, 16, 4, 128, 0, 0,
]);
