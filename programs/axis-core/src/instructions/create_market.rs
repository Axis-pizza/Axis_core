use pinocchio::{AccountView, Address};

use crate::constants::{MARKET_SEED, MAX_ASSETS};
use crate::error::AxisCoreError;
use crate::instructions::common::{
    expect_owned_by_program, expect_signer, expect_uninitialized, read_address_arg, read_u16_arg,
    verify_pda,
};
use crate::state::{DTFMarket, MarketAsset, MarketStatus, ProtocolConfig, TokenProgramKind};

const ENTRY_LEN: usize = 67;

/// Accounts: [creator (signer), protocol_config (readonly), market (writable,
///            uninitialized PDA), dtf_mint (readonly)]
/// Data: creator_fee_destination(32) | asset_count(1) | bump(1) |
///       asset_count × { asset_mint(32) | reserve_vault(32) | weight_bps(2) |
///                       token_program(1) }
///
/// The market is created in `Created` and holds its asset table inline. Fee
/// terms are snapshotted from `ProtocolConfig` and are immutable afterwards,
/// so a later protocol fee change cannot reprice an existing market.
pub fn process_create_market(
    program_id: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
) -> Result<(), AxisCoreError> {
    let [creator, config, market, dtf_mint, ..] = accounts else {
        return Err(AxisCoreError::MissingAccount);
    };
    expect_signer(creator)?;

    let creator_fee_destination = read_address_arg(data, 0)?;
    let asset_count = *data.get(32).ok_or(AxisCoreError::InvalidInstruction)?;
    let bump = *data.get(33).ok_or(AxisCoreError::InvalidInstruction)?;
    if asset_count as usize > MAX_ASSETS {
        return Err(AxisCoreError::TooManyAssets);
    }

    let config_state = {
        expect_owned_by_program(config, program_id)?;
        let buf = config
            .try_borrow()
            .map_err(|_| AxisCoreError::InvalidAccountData)?;
        ProtocolConfig::unpack(&buf)?
    };
    config_state.validate()?;

    let mut assets: [Option<MarketAsset>; MAX_ASSETS] = [const { None }; MAX_ASSETS];
    for (i, slot) in assets.iter_mut().enumerate() {
        if i >= asset_count as usize {
            break;
        }
        let base = 34 + i * ENTRY_LEN;
        *slot = Some(MarketAsset {
            asset_mint: read_address_arg(data, base)?,
            reserve_vault: read_address_arg(data, base + 32)?,
            weight_bps: read_u16_arg(data, base + 64)?,
            token_program: TokenProgramKind::try_from(
                *data
                    .get(base + 66)
                    .ok_or(AxisCoreError::InvalidInstruction)?,
            )?,
        });
    }

    let state = DTFMarket {
        creator: creator.address().clone(),
        dtf_mint: dtf_mint.address().clone(),
        creator_fee_destination,
        accrued_creator_fee_usdc: 0,
        accrued_protocol_fee_usdc: 0,
        mint_fee_bps: config_state.mint_fee_bps,
        creator_share_bps: config_state.creator_share_bps,
        asset_count,
        status: MarketStatus::Created,
        bump,
        assets,
    };
    state.validate_composition()?;

    expect_uninitialized(market, program_id, DTFMarket::LEN)?;
    verify_pda(
        market.address(),
        &[MARKET_SEED, dtf_mint.address().as_ref(), &[bump]],
        program_id,
    )?;

    let mut buf = market
        .try_borrow_mut()
        .map_err(|_| AxisCoreError::InvalidAccountData)?;
    state.pack(&mut buf)
}
