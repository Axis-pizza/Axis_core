//! Account layout and composition rules. The asset table is inline, so a
//! layout bug here silently mis-reads a reserve vault address rather than
//! failing loudly, which is why the round trip is tested field by field.

use axis_core::constants::{MAX_ASSETS, MIN_WEIGHT_BPS};
use axis_core::error::AxisCoreError;
use axis_core::state::{DTFMarket, MarketAsset, MarketStatus, ProtocolConfig, TokenProgramKind};
use pinocchio::Address;

fn addr(b: u8) -> Address {
    Address::new_from_array([b; 32])
}

fn asset(mint: u8, vault: u8, weight: u16, tp: TokenProgramKind) -> MarketAsset {
    MarketAsset {
        asset_mint: addr(mint),
        reserve_vault: addr(vault),
        weight_bps: weight,
        token_program: tp,
    }
}

fn market(assets: Vec<MarketAsset>) -> DTFMarket {
    let count = assets.len() as u8;
    let mut slots: [Option<MarketAsset>; MAX_ASSETS] = [const { None }; MAX_ASSETS];
    for (slot, a) in slots.iter_mut().zip(assets) {
        *slot = Some(a);
    }
    DTFMarket {
        creator: addr(1),
        dtf_mint: addr(2),
        treasury: addr(3),
        asset_count: count,
        status: MarketStatus::Active,
        bump: 254,
        assets: slots,
    }
}

/// The byte counts in the layout comments are what clients size accounts and
/// offsets from, so they are pinned here rather than trusted.
#[test]
fn account_sizes_match_the_documented_layouts() {
    assert_eq!(MarketAsset::LEN, 67);
    assert_eq!(DTFMarket::LEN, 308);
    assert_eq!(DTFMarket::LEN, 107 + MAX_ASSETS * MarketAsset::LEN);
    assert_eq!(ProtocolConfig::LEN, 105);
}

#[test]
fn market_survives_a_pack_unpack_round_trip_field_by_field() {
    let original = market(vec![
        asset(10, 20, 5_000, TokenProgramKind::LegacySplToken),
        asset(11, 21, 3_000, TokenProgramKind::Token2022),
        asset(12, 22, 2_000, TokenProgramKind::LegacySplToken),
    ]);

    let mut buf = vec![0u8; DTFMarket::LEN];
    original.pack(&mut buf).unwrap();
    let decoded = DTFMarket::unpack(&buf).unwrap();

    assert_eq!(decoded, original);
    for (got, want) in decoded.assets().zip(original.assets()) {
        assert_eq!(got.asset_mint, want.asset_mint);
        assert_eq!(got.reserve_vault, want.reserve_vault);
        assert_eq!(got.weight_bps, want.weight_bps);
        assert_eq!(got.token_program, want.token_program);
    }
}

#[test]
fn a_short_buffer_is_rejected_rather_than_read_past_the_end() {
    let m = market(vec![
        asset(10, 20, 5_000, TokenProgramKind::LegacySplToken),
        asset(11, 21, 5_000, TokenProgramKind::LegacySplToken),
    ]);
    let mut small = vec![0u8; DTFMarket::LEN - 1];
    assert_eq!(m.pack(&mut small), Err(AxisCoreError::InvalidAccountData));
    assert_eq!(
        DTFMarket::unpack(&small),
        Err(AxisCoreError::InvalidAccountData)
    );
}

#[test]
fn a_foreign_discriminator_is_rejected() {
    let mut buf = vec![0u8; DTFMarket::LEN];
    buf[0..8].copy_from_slice(b"notaxis1");
    assert_eq!(
        DTFMarket::unpack(&buf),
        Err(AxisCoreError::InvalidDiscriminator)
    );
}

#[test]
fn composition_accepts_two_and_three_assets_and_nothing_else() {
    assert!(market(vec![
        asset(10, 20, 5_000, TokenProgramKind::LegacySplToken),
        asset(11, 21, 5_000, TokenProgramKind::LegacySplToken),
    ])
    .validate_composition()
    .is_ok());

    let mut one = market(vec![asset(
        10,
        20,
        10_000,
        TokenProgramKind::LegacySplToken,
    )]);
    one.asset_count = 1;
    assert_eq!(one.validate_composition(), Err(AxisCoreError::TooFewAssets));

    // The cap is measured: a 4-leg atomic mint does not fit 64 account locks.
    let mut four = market(vec![
        asset(10, 20, 2_500, TokenProgramKind::LegacySplToken),
        asset(11, 21, 2_500, TokenProgramKind::LegacySplToken),
        asset(12, 22, 2_500, TokenProgramKind::LegacySplToken),
    ]);
    four.asset_count = 4;
    assert_eq!(
        four.validate_composition(),
        Err(AxisCoreError::TooManyAssets)
    );
}

/// A repeated mint would let one vault satisfy two delivery requirements, so
/// the same tokens would back two separate claims.
#[test]
fn a_repeated_mint_or_vault_is_rejected() {
    let same_mint = market(vec![
        asset(10, 20, 5_000, TokenProgramKind::LegacySplToken),
        asset(10, 21, 5_000, TokenProgramKind::LegacySplToken),
    ]);
    assert_eq!(
        same_mint.validate_composition(),
        Err(AxisCoreError::DuplicateAsset)
    );

    let same_vault = market(vec![
        asset(10, 20, 5_000, TokenProgramKind::LegacySplToken),
        asset(11, 20, 5_000, TokenProgramKind::LegacySplToken),
    ]);
    assert_eq!(
        same_vault.validate_composition(),
        Err(AxisCoreError::DuplicateAsset)
    );
}

#[test]
fn weights_must_sum_to_exactly_ten_thousand_and_clear_the_floor() {
    let under = market(vec![
        asset(10, 20, 4_999, TokenProgramKind::LegacySplToken),
        asset(11, 21, 5_000, TokenProgramKind::LegacySplToken),
    ]);
    assert_eq!(
        under.validate_composition(),
        Err(AxisCoreError::InvalidWeightSum)
    );

    let dust = market(vec![
        asset(10, 20, MIN_WEIGHT_BPS - 1, TokenProgramKind::LegacySplToken),
        asset(
            11,
            21,
            10_000 - MIN_WEIGHT_BPS + 1,
            TokenProgramKind::LegacySplToken,
        ),
    ]);
    assert_eq!(
        dust.validate_composition(),
        Err(AxisCoreError::WeightBelowMinimum)
    );
}

#[test]
fn an_asset_count_beyond_the_cap_is_refused_at_decode_time() {
    let m = market(vec![
        asset(10, 20, 5_000, TokenProgramKind::LegacySplToken),
        asset(11, 21, 5_000, TokenProgramKind::LegacySplToken),
    ]);
    let mut buf = vec![0u8; DTFMarket::LEN];
    m.pack(&mut buf).unwrap();
    buf[104] = (MAX_ASSETS + 1) as u8;
    assert_eq!(DTFMarket::unpack(&buf), Err(AxisCoreError::TooManyAssets));
}

#[test]
fn protocol_config_round_trips() {
    let config = ProtocolConfig {
        protocol_authority: addr(1),
        usdc_mint: addr(2),
        protocol_treasury: addr(3),
        bump: 255,
    };
    let mut buf = vec![0u8; ProtocolConfig::LEN];
    config.pack(&mut buf).unwrap();
    assert_eq!(ProtocolConfig::unpack(&buf).unwrap(), config);

    let mut small = vec![0u8; ProtocolConfig::LEN - 1];
    assert_eq!(
        config.pack(&mut small),
        Err(AxisCoreError::InvalidAccountData)
    );
}

#[test]
fn every_status_byte_decodes_and_an_unknown_one_does_not() {
    for (byte, want) in [
        (0u8, MarketStatus::Created),
        (1, MarketStatus::Active),
        (2, MarketStatus::Paused),
        (3, MarketStatus::Deprecated),
    ] {
        assert_eq!(MarketStatus::try_from(byte).unwrap(), want);
    }
    assert_eq!(
        MarketStatus::try_from(4),
        Err(AxisCoreError::InvalidAccountData)
    );
    assert_eq!(
        TokenProgramKind::try_from(2),
        Err(AxisCoreError::InvalidAccountData)
    );
}
