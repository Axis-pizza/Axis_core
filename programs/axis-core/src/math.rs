//! Value arithmetic for mint and redeem.
//!
//! Both directions are pro-rata against live reserve balances and the live DTF
//! supply. No price, NAV, or oracle enters any function here (`01` §30 V13).
//!
//! The two directions round opposite ways and that asymmetry is the safety
//! property, not an accident:
//!
//! ```txt
//! mint    required_i = ceil(reserve_i * dtf_out / supply)     rounds against the minter
//! redeem  release_i  = floor(reserve_i * dtf_in / supply)     rounds against the redeemer
//! ```
//!
//! Every remainder in either direction stays in the reserve and accrues to the
//! holders who did not move. `round_trip_never_profits` in the tests pins the
//! consequence: minting X and immediately redeeming X returns strictly less
//! than was delivered whenever any rounding occurred at all.

use crate::error::AxisCoreError;

type Result<T> = core::result::Result<T, AxisCoreError>;

/// Reserve delivery a mint of `dtf_out` must produce for asset `i`.
///
/// Rounds up, so a minter can never acquire a claim larger than the reserves
/// they actually funded.
pub fn required_delivery(reserve_balance: u64, dtf_out: u64, total_supply: u64) -> Result<u64> {
    if total_supply == 0 {
        // Proportional delivery is undefined with nothing to be proportional
        // to. SeedMarket carries the first issuance instead.
        return Err(AxisCoreError::MarketNotSeeded);
    }
    let numerator = (reserve_balance as u128)
        .checked_mul(dtf_out as u128)
        .ok_or(AxisCoreError::MathOverflow)?;
    let denominator = total_supply as u128;
    let quotient = numerator / denominator;
    let rounded = if numerator % denominator == 0 {
        quotient
    } else {
        quotient.checked_add(1).ok_or(AxisCoreError::MathOverflow)?
    };
    u64::try_from(rounded).map_err(|_| AxisCoreError::MathOverflow)
}

/// Reserve amount a redemption of `dtf_in` releases for asset `i`.
///
/// Rounds down, so a redeemer can never withdraw more than their exact share.
pub fn pro_rata_release(reserve_balance: u64, dtf_in: u64, total_supply: u64) -> Result<u64> {
    if total_supply == 0 {
        return Err(AxisCoreError::MarketNotSeeded);
    }
    let numerator = (reserve_balance as u128)
        .checked_mul(dtf_in as u128)
        .ok_or(AxisCoreError::MathOverflow)?;
    let released = numerator / (total_supply as u128);
    u64::try_from(released).map_err(|_| AxisCoreError::MathOverflow)
}

/// The Mint and Redeem fee on `amount`, at the fixed `FEE_BPS`, rounded down.
///
/// The caller chooses the base. `FEE_BPS` is below the denominator, so the fee
/// never exceeds `amount` and the result always fits.
pub fn fee(amount: u64) -> u64 {
    ((amount as u128 * FEE_BPS as u128) / BPS as u128) as u64
}

const BPS: u64 = crate::constants::BPS_DENOMINATOR;
const FEE_BPS: u64 = crate::constants::FEE_BPS;
const _: () = assert!(FEE_BPS < BPS);

/// Supply that may still be redeemed, excluding the permanently locked
/// MINIMUM_LIQUIDITY.
pub fn redeemable_supply(total_supply: u64) -> Result<u64> {
    total_supply
        .checked_sub(crate::constants::MINIMUM_LIQUIDITY)
        .ok_or(AxisCoreError::SupplyBelowMinimumLiquidity)
}
