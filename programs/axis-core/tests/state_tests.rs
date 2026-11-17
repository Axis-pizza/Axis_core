//! Account layout and composition rules. The asset table is inline, so a
//! layout bug here silently mis-reads a reserve vault address rather than
//! failing loudly, which is why the round trip is tested field by field.

use axis_core::constants::MAX_ASSETS;
use axis_core::error::AxisCoreError;
use axis_core::state::{DTFMarket, MarketAsset, ProtocolConfig, TokenProgramKind};
use pinocchio::Address;

fn addr(b: u8) -> Address {
    Address::new_from_array([b; 32])
}

fn asset(mint: u8, vault: u8, reference_amount: u64, tp: TokenProgramKind) -> MarketAsset {
    MarketAsset {
        asset_mint: addr(mint),
        reserve_vault: addr(vault),
        reference_amount,
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
        dtf_mint: addr(2),
        treasury: addr(3),
        usdc_mint: addr(4),
        swap_program: addr(5),
        liquidity_lock: addr(6),
        reference_supply: 1_000_000,
        min_liquidity: 1_000,
        asset_count: count,
        bump: 254,
        dtf_token_program: TokenProgramKind::Token2022,
        purchase_paused: false,
        usdc_redemption_paused: true,
        assets: slots,
    }
}

/// The byte counts in the layout comments are what clients size accounts and
/// offsets from, so they are pinned here rather than trusted.
#[test]
fn account_sizes_match_the_documented_layouts() {
    assert_eq!(MarketAsset::LEN, 73);
    assert_eq!(DTFMarket::LEN, 408);
    assert_eq!(DTFMarket::LEN, 189 + MAX_ASSETS * MarketAsset::LEN);
    assert_eq!(ProtocolConfig::LEN, 137);
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
        assert_eq!(got.reference_amount, want.reference_amount);
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

/// `unpack` only checks for a minimum length, so every layout change takes a
/// new discriminator. An account in an earlier layout must not decode with
/// its fields shifted.
#[test]
fn earlier_layouts_are_rejected_by_their_discriminator() {
    let m = market(vec![
        asset(10, 20, 5_000, TokenProgramKind::LegacySplToken),
        asset(11, 21, 5_000, TokenProgramKind::LegacySplToken),
    ]);
    let mut current = vec![0u8; DTFMarket::LEN];
    m.pack(&mut current).unwrap();
    assert_eq!(&current[0..8], b"dtfmkt05");

    for tag in [b"dtfmkt03", b"dtfmkt04"] {
        let mut previous = current.clone();
        previous[0..8].copy_from_slice(tag);
        assert_eq!(
            DTFMarket::unpack(&previous),
            Err(AxisCoreError::InvalidDiscriminator)
        );
    }

    let config = ProtocolConfig {
        protocol_authority: addr(1),
        usdc_mint: addr(2),
        protocol_treasury: addr(3),
        emergency_operator: addr(4),
        bump: 255,
    };
    let mut buf = vec![0u8; ProtocolConfig::LEN];
    config.pack(&mut buf).unwrap();
    buf[0..8].copy_from_slice(b"axiscfg2");
    assert_eq!(
        ProtocolConfig::unpack(&buf),
        Err(AxisCoreError::InvalidDiscriminator)
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

/// The reference basket is the approved initial allocation. A zero anywhere
/// in it would divide by zero or issue nothing at the first purchase, and a
/// zero locked minimum would bring back the first-depositor rounding attack.
#[test]
fn the_reference_basket_and_the_locked_minimum_must_be_positive() {
    let two = || {
        market(vec![
            asset(10, 20, 2_000, TokenProgramKind::LegacySplToken),
            asset(11, 21, 5_000, TokenProgramKind::LegacySplToken),
        ])
    };
    assert!(two().validate_composition().is_ok());

    let mut no_supply = two();
    no_supply.reference_supply = 0;
    let mut no_lock = two();
    no_lock.min_liquidity = 0;
    let no_amount = market(vec![
        asset(10, 20, 2_000, TokenProgramKind::LegacySplToken),
        asset(11, 21, 0, TokenProgramKind::LegacySplToken),
    ]);
    for bad in [no_supply, no_lock, no_amount] {
        assert_eq!(
            bad.validate_composition(),
            Err(AxisCoreError::InvalidReferenceBasket)
        );
    }
}

/// The DTF cannot back itself, and USDC is the purchase currency whose
/// balance change the signed bounds are measured on.
#[test]
fn the_dtf_mint_and_the_usdc_mint_cannot_be_constituents() {
    for forbidden in [2, 4] {
        let m = market(vec![
            asset(10, 20, 2_000, TokenProgramKind::LegacySplToken),
            asset(forbidden, 21, 5_000, TokenProgramKind::LegacySplToken),
        ]);
        assert_eq!(m.validate_composition(), Err(AxisCoreError::ForbiddenAsset));
    }
}

#[test]
fn an_asset_count_beyond_the_cap_is_refused_at_decode_time() {
    let m = market(vec![
        asset(10, 20, 5_000, TokenProgramKind::LegacySplToken),
        asset(11, 21, 5_000, TokenProgramKind::LegacySplToken),
    ]);
    let mut buf = vec![0u8; DTFMarket::LEN];
    m.pack(&mut buf).unwrap();
    buf[184] = (MAX_ASSETS + 1) as u8;
    assert_eq!(DTFMarket::unpack(&buf), Err(AxisCoreError::TooManyAssets));
}

#[test]
fn protocol_config_round_trips() {
    let config = ProtocolConfig {
        protocol_authority: addr(1),
        usdc_mint: addr(2),
        protocol_treasury: addr(3),
        emergency_operator: addr(4),
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
fn unknown_flag_and_token_program_bytes_do_not_decode() {
    let m = market(vec![
        asset(10, 20, 5_000, TokenProgramKind::LegacySplToken),
        asset(11, 21, 5_000, TokenProgramKind::LegacySplToken),
    ]);
    let mut buf = vec![0u8; DTFMarket::LEN];
    m.pack(&mut buf).unwrap();
    for offset in [186, 187, 188] {
        let mut bad = buf.clone();
        bad[offset] = 2;
        assert_eq!(
            DTFMarket::unpack(&bad),
            Err(AxisCoreError::InvalidAccountData)
        );
    }
    assert_eq!(
        TokenProgramKind::try_from(2),
        Err(AxisCoreError::InvalidAccountData)
    );
}
