//! Value arithmetic. These are the invariants that decide whether a holder's
//! backing can be diluted, so they are tested by sweep rather than by example.

use axis_core::constants::{BPS_DENOMINATOR, MINIMUM_LIQUIDITY};
use axis_core::error::AxisCoreError;
use axis_core::math::{
    mint_fee, pro_rata_release, redeemable_supply, required_delivery, split_fee,
};

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
fn mint_fee_is_charged_on_spent_usdc_not_on_the_gross_amount() {
    // 100 USDC spent at 100 bps.
    assert_eq!(mint_fee(100_000_000, 100).unwrap(), 1_000_000);
    // Nothing spent, nothing charged: a delivery-driven mint refunds whatever
    // the legs did not consume, and refunded money must not be taxed.
    assert_eq!(mint_fee(0, 100).unwrap(), 0);
    assert_eq!(mint_fee(100_000_000, 0).unwrap(), 0);
}

#[test]
fn fee_split_is_exact_and_leaves_no_dust() {
    for fee in [0u64, 1, 2, 3, 7, 999, 1_000_000, u64::MAX / 10_000] {
        for share in [0u16, 1, 4_000, 6_000, 9_999, 10_000] {
            let (creator, protocol) = split_fee(fee, share).unwrap();
            assert_eq!(
                creator.checked_add(protocol).unwrap(),
                fee,
                "split lost value: fee={fee} share={share}"
            );
        }
    }
    assert_eq!(
        split_fee(1_000, (BPS_DENOMINATOR + 1) as u16),
        Err(AxisCoreError::InvalidFeeConfig)
    );
}

#[test]
fn minimum_liquidity_is_never_redeemable() {
    assert_eq!(redeemable_supply(MINIMUM_LIQUIDITY).unwrap(), 0);
    assert_eq!(redeemable_supply(MINIMUM_LIQUIDITY + 5).unwrap(), 5);
    assert_eq!(
        redeemable_supply(MINIMUM_LIQUIDITY - 1),
        Err(AxisCoreError::SupplyBelowMinimumLiquidity)
    );
}

/// With the locked minimum in place the ratio is always defined, which is the
/// whole reason it exists.
#[test]
fn the_locked_minimum_keeps_the_ratio_defined_after_every_holder_leaves() {
    let supply = MINIMUM_LIQUIDITY + 500_000;
    let reserve = 1_000_000u64;
    let everyone = redeemable_supply(supply).unwrap();

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
