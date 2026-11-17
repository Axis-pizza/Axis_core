//! Value arithmetic for purchase and redemption.
//!
//! Both directions are pro-rata against live reserve balances and the live DTF
//! supply. No price, NAV, or oracle enters any function here.
//!
//! ```txt
//! purchase  gross   = min_i floor(delta_i * supply / reserve_i)   rounds against the buyer
//! redeem    release = floor(reserve_i * dtf_burned / supply)      rounds against the redeemer
//! ```
//!
//! Every remainder in either direction stays in the reserve and accrues to
//! all holders. `round_trip_never_profits` in the tests pins the consequence.

use crate::error::AxisCoreError;

type Result<T> = core::result::Result<T, AxisCoreError>;

/// DTF that one constituent's actual reserve increase `delta` supports, given
/// the reserve and supply from before the purchase. Rounds down, so a buyer
/// never acquires a claim larger than the reserves they funded.
pub fn issuance_for_delivery(delta: u64, reserve_before: u64, supply_before: u64) -> Result<u64> {
    if supply_before == 0 {
        return Err(AxisCoreError::MarketNotSeeded);
    }
    if reserve_before == 0 {
        return Err(AxisCoreError::EmptyReserve);
    }
    let issued = (delta as u128)
        .checked_mul(supply_before as u128)
        .ok_or(AxisCoreError::MathOverflow)?
        / (reserve_before as u128);
    u64::try_from(issued).map_err(|_| AxisCoreError::MathOverflow)
}

/// Gross DTF a purchase may issue: the smallest per-constituent result, so
/// the constituent with the least delivery sets it and DTF is never issued
/// beyond its backing. The surplus on every other leg stays in the reserves.
///
/// For the first purchase, `reserves_before` and `supply_before` are the
/// governance-approved reference basket instead of live balances.
pub fn gross_issuance(deltas: &[u64], reserves_before: &[u64], supply_before: u64) -> Result<u64> {
    if deltas.is_empty() || deltas.len() != reserves_before.len() {
        return Err(AxisCoreError::UnknownAsset);
    }
    let mut gross = u64::MAX;
    for (delta, reserve) in deltas.iter().zip(reserves_before) {
        let issued = issuance_for_delivery(*delta, *reserve, supply_before)?;
        if issued < gross {
            gross = issued;
        }
    }
    Ok(gross)
}

/// Reserve delivery that issuing `dtf_out` gross requires for asset `i`.
/// Rounds up. Not used on-chain: clients size route legs with it.
pub fn required_delivery(reserve_balance: u64, dtf_out: u64, total_supply: u64) -> Result<u64> {
    if total_supply == 0 {
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

/// Reserve amount a redemption that burns `dtf_in` releases for asset `i`.
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

/// The fee on a DTF `amount`, at the fixed `FEE_BPS`, rounded down: taken
/// from the gross DTF a purchase issues, or from the DTF a redemption hands
/// in.
///
/// `FEE_BPS` is below the denominator, so the fee never exceeds `amount` and
/// the result always fits.
pub fn fee(amount: u64) -> u64 {
    ((amount as u128 * FEE_BPS as u128) / BPS as u128) as u64
}

const BPS: u64 = crate::constants::BPS_DENOMINATOR;
const FEE_BPS: u64 = crate::constants::FEE_BPS;
const _: () = assert!(FEE_BPS < BPS);

/// Supply that holders can ever redeem: everything except the minimum
/// liquidity locked at the first purchase.
pub fn redeemable_supply(total_supply: u64, min_liquidity: u64) -> Result<u64> {
    total_supply
        .checked_sub(min_liquidity)
        .ok_or(AxisCoreError::SupplyBelowMinimumLiquidity)
}
