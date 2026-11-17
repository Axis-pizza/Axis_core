//! Value arithmetic. These are the invariants that decide whether a holder's
//! backing can be diluted, so they are tested by sweep rather than by example.

use axis_core::error::AxisCoreError;
use axis_core::math::{
    fee, gross_issuance, issuance_for_delivery, pro_rata_release, redeemable_supply,
    required_delivery,
};

/// Fixture value only. The locked minimum is approved per market.
const MINIMUM_LIQUIDITY: u64 = 1_000;

#[test]
fn required_delivery_rounds_up_and_release_rounds_down() {
    // 1000 * 3 / 7 = 428.57...
    assert_eq!(required_delivery(1_000, 3, 7).unwrap(), 429);
    assert_eq!(pro_rata_release(1_000, 3, 7).unwrap(), 428);

    // Exact division rounds nowhere.
    assert_eq!(required_delivery(1_000, 2, 4).unwrap(), 500);
    assert_eq!(pro_rata_release(1_000, 2, 4).unwrap(), 500);
}

#[test]
fn a_dust_share_still_costs_a_whole_base_unit_to_mint() {
    // Rounding up is what stops a minter acquiring a claim they did not fund,
    // even when their proportional share is a fraction of one base unit.
    assert_eq!(required_delivery(1, 1, 1_000_000).unwrap(), 1);
    assert_eq!(pro_rata_release(1, 1, 1_000_000).unwrap(), 0);
}

/// The property the whole design rests on: minting `x` and immediately
/// redeeming the same `x` in kind must never return more reserve than the mint
/// delivered. If it could, the round trip would be a pump on the other
/// holders' backing, repeatable for free.
#[test]
fn round_trip_never_profits() {
    let reserves = [1u64, 7, 999, 1_000, 123_456, 10u64.pow(9), 10u64.pow(12)];
    let supplies = [MINIMUM_LIQUIDITY, 1_001, 7_919, 10u64.pow(6), 10u64.pow(9)];
    let mints = [1u64, 3, 1_000, 999_983, 10u64.pow(6)];

    for &reserve in &reserves {
        for &supply in &supplies {
            for &x in &mints {
                let delivered = required_delivery(reserve, x, supply).unwrap();
                let reserve_after = reserve.checked_add(delivered).unwrap();
                let supply_after = supply.checked_add(x).unwrap();
                let returned = pro_rata_release(reserve_after, x, supply_after).unwrap();

                assert!(
                    returned <= delivered,
                    "round trip profited: reserve={reserve} supply={supply} x={x} \
                     delivered={delivered} returned={returned}"
                );
            }
        }
    }
}

/// Holders who do not move must never be worse off after someone else's round
/// trip: reserve per unit of supply may only rise.
#[test]
fn round_trip_never_dilutes_the_holders_who_stayed() {
    let cases = [
        (1_000u64, 10_000u64, 1u64),
        (999, 7_919, 3),
        (123_456, 1_000_000, 999_983),
        (10u64.pow(9), 10u64.pow(6), 1_000),
    ];

    for (reserve, supply, x) in cases {
        let delivered = required_delivery(reserve, x, supply).unwrap();
        let returned = pro_rata_release(reserve + delivered, x, supply + x).unwrap();

        // Supply returns to its original value, so comparing reserve alone is
        // enough to compare backing per unit.
        assert!(
            reserve + delivered - returned >= reserve,
            "backing per unit fell: reserve={reserve} supply={supply} x={x}"
        );
    }
}

#[test]
fn zero_supply_is_rejected_rather_than_dividing_by_zero() {
    assert_eq!(
        required_delivery(1_000, 1, 0),
        Err(AxisCoreError::MarketNotSeeded)
    );
    assert_eq!(
        pro_rata_release(1_000, 1, 0),
        Err(AxisCoreError::MarketNotSeeded)
    );
}

#[test]
fn extreme_values_overflow_rather_than_wrap() {
    // The intermediate product exceeds u64 but stays inside u128, so this is
    // a real result and not an overflow.
    assert_eq!(required_delivery(u64::MAX, 1, u64::MAX).unwrap(), 1);
    assert_eq!(
        pro_rata_release(u64::MAX, u64::MAX, u64::MAX).unwrap(),
        u64::MAX
    );

    // A result that genuinely cannot fit must be reported, never truncated.
    assert_eq!(
        pro_rata_release(u64::MAX, u64::MAX, 1),
        Err(AxisCoreError::MathOverflow)
    );
}

#[test]
fn fee_is_thirty_bps_rounded_down() {
    // 100 USDC at 30 bps.
    assert_eq!(fee(100_000_000), 300_000);
    // 334 * 30 / 10_000 = 1.002, and 333 rounds to nothing.
    assert_eq!(fee(334), 1);
    assert_eq!(fee(333), 0);
    assert_eq!(fee(0), 0);
}

#[test]
fn fee_never_exceeds_its_base() {
    for amount in [1u64, 999, 1_000_000, u64::MAX / 2, u64::MAX] {
        assert!(fee(amount) <= amount, "fee exceeded base at {amount}");
    }
    assert_eq!(fee(u64::MAX), ((u64::MAX as u128 * 30) / 10_000) as u64);
}

#[test]
fn minimum_liquidity_is_never_redeemable() {
    assert_eq!(
        redeemable_supply(MINIMUM_LIQUIDITY, MINIMUM_LIQUIDITY).unwrap(),
        0
    );
    assert_eq!(
        redeemable_supply(MINIMUM_LIQUIDITY + 5, MINIMUM_LIQUIDITY).unwrap(),
        5
    );
    assert_eq!(
        redeemable_supply(MINIMUM_LIQUIDITY - 1, MINIMUM_LIQUIDITY),
        Err(AxisCoreError::SupplyBelowMinimumLiquidity)
    );
}

/// With the locked minimum in place the ratio is always defined, which is the
/// whole reason it exists.
#[test]
fn the_locked_minimum_keeps_the_ratio_defined_after_every_holder_leaves() {
    let supply = MINIMUM_LIQUIDITY + 500_000;
    let reserve = 1_000_000u64;
    let everyone = redeemable_supply(supply, MINIMUM_LIQUIDITY).unwrap();

    let released = pro_rata_release(reserve, everyone, supply).unwrap();
    let reserve_left = reserve - released;
    let supply_left = supply - everyone;

    assert_eq!(supply_left, MINIMUM_LIQUIDITY);
    assert!(
        reserve_left > 0,
        "the locked supply must keep backing behind it"
    );
    assert!(required_delivery(reserve_left, 1, supply_left).is_ok());
}

/// The acceptance fixture for an additional purchase, in integer base units.
#[test]
fn the_limiting_constituent_sets_gross_issuance() {
    let supply = 100_000;
    let reserves = [200_000, 500_000];
    let deltas = [2_100, 5_000];

    assert_eq!(
        issuance_for_delivery(2_100, 200_000, supply).unwrap(),
        1_050
    );
    assert_eq!(
        issuance_for_delivery(5_000, 500_000, supply).unwrap(),
        1_000
    );

    let gross = gross_issuance(&deltas, &reserves, supply).unwrap();
    assert_eq!(gross, 1_000, "limited by B");
    assert_eq!(fee(gross), 3);
    assert_eq!(gross - fee(gross), 997);
}

#[test]
fn issuance_rounds_down_and_never_exceeds_any_legs_backing() {
    // 7 * 1000 / 3000 = 2.33...
    assert_eq!(issuance_for_delivery(7, 3_000, 1_000).unwrap(), 2);
    for supply in [1u64, 999, 1_000_000, 123_456_789] {
        for reserves in [[1u64, 1], [3, 1_000_003], [5_000_000, 77]] {
            for deltas in [[0u64, 9], [1, 1], [12_345, 6], [1_000_000, 1_000_000]] {
                let gross = gross_issuance(&deltas, &reserves, supply).unwrap();
                for i in 0..2 {
                    // What `gross` DTF claims of leg i afterwards never
                    // exceeds what the purchase delivered to it.
                    let claim =
                        pro_rata_release(reserves[i] + deltas[i], gross, supply + gross).unwrap();
                    assert!(claim <= deltas[i], "over-issued at {supply} {reserves:?}");
                }
            }
        }
    }
}

#[test]
fn issuance_needs_a_supply_a_reserve_and_matching_legs() {
    assert_eq!(
        issuance_for_delivery(1, 1, 0),
        Err(AxisCoreError::MarketNotSeeded)
    );
    assert_eq!(
        issuance_for_delivery(1, 0, 1),
        Err(AxisCoreError::EmptyReserve)
    );
    assert_eq!(
        gross_issuance(&[1, 1], &[1], 1),
        Err(AxisCoreError::UnknownAsset)
    );
    assert_eq!(
        gross_issuance(&[], &[], 1),
        Err(AxisCoreError::UnknownAsset)
    );
    assert_eq!(
        issuance_for_delivery(u64::MAX, 1, u64::MAX),
        Err(AxisCoreError::MathOverflow)
    );
}
