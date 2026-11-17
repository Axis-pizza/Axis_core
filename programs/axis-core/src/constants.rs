//! Protocol constants.
//!
//! Only structural limits and the agreed 30 bps fee live here. Every economic
//! launch parameter that the November 8 requirements leave open (reference
//! basket, locked minimum liquidity, swap program) is approved per market by
//! governance in `CreateMarket` and stored in `DTFMarket`.

use pinocchio::Address;

/// Markets hold 2 to 3 assets. Three is the v1 product ceiling; it does not
/// guarantee that every 3-asset composition or trade fits the account-lock
/// cap.
pub const MIN_ASSETS: usize = 2;
pub const MAX_ASSETS: usize = 3;

/// Basis-point denominator for the fee calculation.
pub const BPS_DENOMINATOR: u64 = 10_000;

/// Fee on Purchase, RedeemUsdc and RedeemInKind, taken in DTF and sent to the
/// treasury's DTF account in the same instruction. There is no fee vault, no
/// claim and no accrual.
pub const FEE_BPS: u64 = 30;

/// Accounts one route leg may pass to the swap program. Bounded by the stack
/// frame that holds the CPI account array, not by a product decision.
pub const MAX_ROUTE_LEG_ACCOUNTS: usize = 48;

/// Route legs per instruction. A purchase needs one per constituent; the
/// headroom is for a route that splits a leg.
pub const MAX_ROUTE_LEGS: usize = 6;

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

/// `TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA`
pub const SPL_TOKEN_PROGRAM_ID: Address = Address::new_from_array([
    6, 221, 246, 225, 215, 101, 161, 147, 217, 203, 225, 70, 206, 235, 121, 172, 28, 180, 133, 237,
    95, 91, 55, 145, 58, 140, 245, 133, 126, 255, 0, 169,
]);

/// `TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb`
pub const TOKEN_2022_PROGRAM_ID: Address = Address::new_from_array([
    6, 221, 246, 225, 238, 117, 143, 222, 24, 66, 93, 188, 228, 108, 205, 218, 182, 26, 252, 77,
    131, 185, 13, 39, 254, 189, 249, 40, 216, 161, 139, 252,
]);
