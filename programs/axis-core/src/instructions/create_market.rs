use pinocchio::cpi::Seed;
use pinocchio::{AccountView, Address, ProgramResult};

use crate::constants::{MARKET_SEED, MAX_ASSETS};
use crate::error::AxisCoreError;
use crate::instructions::common::{
    create_pda_account, expect_canonical_pda, expect_signer, load_config, read_address_arg,
    read_u64_arg,
};
use crate::instructions::token::{expect_custody_account, read_mint};
use crate::state::{DTFMarket, MarketAsset};

const HEADER_LEN: usize = 49;

/// Accounts: [protocol_authority (signer, writable, pays rent),
///            protocol_config,
///            market (writable, not yet created),
///            dtf_mint,
///            liquidity_lock (DTF token account owned by the market PDA),
///            system_program,
///            reserve_vault × asset_count,
///            asset_mint × asset_count]
/// Data: swap_program(32) | reference_supply(8) | min_liquidity(8) |
///       asset_count(1) | asset_count × reference_amount(8)
///
/// Only the governance authority in `ProtocolConfig` may create a market, and
/// creating it is the approval for public availability: the first public
/// purchase can fund it from then on. Governance approves the constituents
/// and the initial allocation here, as the reference basket
/// `reference_amount_i` per `reference_supply` DTF. The first purchaser can
/// change neither.
///
/// The market is created at `["market", dtf_mint]` with the canonical bump.
/// The treasury and the USDC mint are snapshotted from `ProtocolConfig` and
/// are immutable afterwards.
///
/// Custody is checked before anything is recorded:
///
/// - the DTF mint has no extensions, zero supply, no freeze authority, and
///   the market PDA as its only mint authority, so DTF exists only through
///   Core's own issuance;
/// - every constituent mint is an initialized mint with no extensions;
/// - every reserve vault holds its constituent's mint under its mint's token
///   program, is owned by the market PDA, and has no delegate and no close
///   authority, so only a Core-signed transfer can move a reserve;
/// - the liquidity lock is a DTF account under the same rules, so the locked
///   minimum can never move: no instruction signs a DTF transfer or burn for
///   the market PDA.
pub fn process_create_market(
    program_id: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
) -> ProgramResult {
    let [authority, config, market, dtf_mint, liquidity_lock, system_program, rest @ ..] = accounts
    else {
        return Err(AxisCoreError::MissingAccount.into());
    };
    expect_signer(authority)?;
    let config_state = load_config(config, program_id)?;
    if authority.address() != &config_state.protocol_authority {
        return Err(AxisCoreError::UnauthorizedProtocolAuthority.into());
    }

    let swap_program = read_address_arg(data, 0)?;
    // Axis Core as its own route would let a leg re-enter Purchase.
    if &swap_program == program_id {
        return Err(AxisCoreError::InvalidProgramAccount.into());
    }
    let reference_supply = read_u64_arg(data, 32)?;
    let min_liquidity = read_u64_arg(data, 40)?;
    let asset_count = *data.get(48).ok_or(AxisCoreError::InvalidInstruction)?;
    let count = asset_count as usize;
    if count > MAX_ASSETS {
        return Err(AxisCoreError::TooManyAssets.into());
    }
    if data.len() != HEADER_LEN + count * 8 {
        return Err(AxisCoreError::InvalidInstruction.into());
    }
    if rest.len() < count * 2 {
        return Err(AxisCoreError::MissingAccount.into());
    }
    let (vaults, mints) = rest.split_at(count);

    let bump = expect_canonical_pda(
        market,
        &[MARKET_SEED, dtf_mint.address().as_ref()],
        program_id,
    )?;

    let dtf = read_mint(dtf_mint)?;
    if dtf.mint_authority.as_ref() != Some(market.address()) {
        return Err(AxisCoreError::InvalidMintAuthority.into());
    }
    if dtf.freeze_authority.is_some() {
        return Err(AxisCoreError::MintHasFreezeAuthority.into());
    }
    if dtf.supply != 0 {
        return Err(AxisCoreError::MintSupplyNotZero.into());
    }
    expect_custody_account(
        liquidity_lock,
        dtf.kind,
        dtf_mint.address(),
        market.address(),
    )?;

    let mut assets: [Option<MarketAsset>; MAX_ASSETS] = [const { None }; MAX_ASSETS];
    for (i, slot) in assets.iter_mut().enumerate().take(count) {
        let asset_mint = &mints[i];
        let vault = &vaults[i];
        let mint_state = read_mint(asset_mint)?;
        expect_custody_account(
            vault,
            mint_state.kind,
            asset_mint.address(),
            market.address(),
        )?;
        *slot = Some(MarketAsset {
            asset_mint: asset_mint.address().clone(),
            reserve_vault: vault.address().clone(),
            reference_amount: read_u64_arg(data, HEADER_LEN + i * 8)?,
            token_program: mint_state.kind,
        });
    }

    let state = DTFMarket {
        dtf_mint: dtf_mint.address().clone(),
        treasury: config_state.protocol_treasury,
        usdc_mint: config_state.usdc_mint,
        swap_program,
        liquidity_lock: liquidity_lock.address().clone(),
        reference_supply,
        min_liquidity,
        asset_count,
        bump,
        dtf_token_program: dtf.kind,
        purchase_paused: false,
        usdc_redemption_paused: false,
        assets,
    };
    state.validate_composition()?;

    let bump_seed = [bump];
    create_pda_account(
        authority,
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
