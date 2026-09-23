use pinocchio::cpi::Seed;
use pinocchio::{AccountView, Address, ProgramResult};

use crate::constants::{MARKET_SEED, MAX_ASSETS, PROTOCOL_CONFIG_SEED};
use crate::error::AxisCoreError;
use crate::instructions::common::{
    create_pda_account, expect_canonical_pda, expect_owned_by_program, expect_signer,
    read_address_arg, read_u16_arg, verify_pda,
};
use crate::state::{DTFMarket, MarketAsset, MarketStatus, ProtocolConfig, TokenProgramKind};

const ENTRY_LEN: usize = 67;

/// Accounts: [creator (signer, writable, pays rent), protocol_config,
///            market (writable, not yet created), dtf_mint, system_program]
/// Data: asset_count(1) |
///       asset_count × { asset_mint(32) | reserve_vault(32) | weight_bps(2) |
///                       token_program(1) }
///
/// The market is created in `Created` at `["market", dtf_mint]` with the
/// canonical bump and holds its asset table inline. The treasury is
/// snapshotted from `ProtocolConfig` and is immutable afterwards.
///
/// NOT YET VALIDATED: the DTF mint and the reserve vault addresses are
/// recorded as given. Nothing that moves tokens may ship until CreateMarket
/// checks them (Axis_docs CANDIDATE-09).
pub fn process_create_market(
    program_id: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
) -> ProgramResult {
    let [creator, config, market, dtf_mint, system_program, ..] = accounts else {
        return Err(AxisCoreError::MissingAccount.into());
    };
    expect_signer(creator)?;

    let asset_count = *data.first().ok_or(AxisCoreError::InvalidInstruction)?;
    if asset_count as usize > MAX_ASSETS {
        return Err(AxisCoreError::TooManyAssets.into());
    }

    // Only the canonical singleton carries protocol terms. Its stored bump is
    // the canonical one, since InitializeProtocolConfig derives it.
    let config_state = {
        expect_owned_by_program(config, program_id)?;
        let buf = config
            .try_borrow()
            .map_err(|_| AxisCoreError::InvalidAccountData)?;
        ProtocolConfig::unpack(&buf)?
    };
    verify_pda(
        config.address(),
        &[PROTOCOL_CONFIG_SEED, &[config_state.bump]],
        program_id,
    )?;

    let mut assets: [Option<MarketAsset>; MAX_ASSETS] = [const { None }; MAX_ASSETS];
    for (i, slot) in assets.iter_mut().enumerate() {
        if i >= asset_count as usize {
            break;
        }
        let base = 1 + i * ENTRY_LEN;
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

    let bump = expect_canonical_pda(
        market,
        &[MARKET_SEED, dtf_mint.address().as_ref()],
        program_id,
    )?;
    let state = DTFMarket {
        creator: creator.address().clone(),
        dtf_mint: dtf_mint.address().clone(),
        treasury: config_state.protocol_treasury,
        asset_count,
        status: MarketStatus::Created,
        bump,
        assets,
    };
    state.validate_composition()?;

    let bump_seed = [bump];
    create_pda_account(
        creator,
        market,
        system_program,
        program_id,
        DTFMarket::LEN,
        &[
            Seed::from(MARKET_SEED),
            Seed::from(dtf_mint.address().as_ref()),
            Seed::from(&bump_seed),
        ],
    )?;

    let mut buf = market
        .try_borrow_mut()
        .map_err(|_| AxisCoreError::InvalidAccountData)?;
    state.pack(&mut buf)?;
    Ok(())
}
