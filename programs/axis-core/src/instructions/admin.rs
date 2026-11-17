//! Pause, resume and role changes.
//!
//! | Action                                    | Who                               |
//! | ----------------------------------------- | --------------------------------- |
//! | Pause purchases or USDC redemption        | emergency operator, or governance |
//! | Resume purchases or USDC redemption       | governance only                   |
//! | Change governance or the emergency operator | governance only                 |
//! | Change a market's swap program            | governance only                   |
//!
//! No flag here reaches `RedeemInKind`, and no role can move a reserve.

use pinocchio::{AccountView, Address, ProgramResult};

use crate::constants::SYSTEM_PROGRAM_ID;
use crate::error::AxisCoreError;
use crate::instructions::common::{
    expect_signer, expect_writable, load_config, load_market, read_address_arg,
};
use crate::state::DTFMarket;

pub const PAUSE_PURCHASE: u8 = 1 << 0;
pub const PAUSE_USDC_REDEMPTION: u8 = 1 << 1;

fn read_pause_flags(data: &[u8]) -> Result<u8, AxisCoreError> {
    let [flags] = data else {
        return Err(AxisCoreError::InvalidInstruction);
    };
    if *flags == 0 || *flags & !(PAUSE_PURCHASE | PAUSE_USDC_REDEMPTION) != 0 {
        return Err(AxisCoreError::InvalidPauseFlags);
    }
    Ok(*flags)
}

fn store_market(market_account: &mut AccountView, market: &DTFMarket) -> ProgramResult {
    expect_writable(market_account)?;
    let mut buf = market_account
        .try_borrow_mut()
        .map_err(|_| AxisCoreError::InvalidAccountData)?;
    market.pack(&mut buf)?;
    Ok(())
}

/// Accounts: [emergency_operator or protocol_authority (signer),
///            protocol_config, market (writable)]
/// Data: flags(1), bit 0 = purchases, bit 1 = USDC redemption
///
/// Pauses the named operations on one market. Pausing an already paused
/// operation succeeds, so an operator never has to read state first.
pub fn process_pause(
    program_id: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
) -> ProgramResult {
    let [signer, config, market_account, ..] = accounts else {
        return Err(AxisCoreError::MissingAccount.into());
    };
    expect_signer(signer)?;
    let config_state = load_config(config, program_id)?;
    if signer.address() != &config_state.emergency_operator
        && signer.address() != &config_state.protocol_authority
    {
        return Err(AxisCoreError::UnauthorizedPauseAuthority.into());
    }
    let flags = read_pause_flags(data)?;

    let mut market = load_market(market_account, program_id)?;
    market.purchase_paused |= flags & PAUSE_PURCHASE != 0;
    market.usdc_redemption_paused |= flags & PAUSE_USDC_REDEMPTION != 0;
    store_market(market_account, &market)
}

/// Accounts: [protocol_authority (signer), protocol_config, market (writable)]
/// Data: flags(1), bit 0 = purchases, bit 1 = USDC redemption
///
/// Resumes the named operations. The emergency operator cannot call this.
pub fn process_resume(
    program_id: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
) -> ProgramResult {
    let [authority, config, market_account, ..] = accounts else {
        return Err(AxisCoreError::MissingAccount.into());
    };
    expect_signer(authority)?;
    let config_state = load_config(config, program_id)?;
    if authority.address() != &config_state.protocol_authority {
        return Err(AxisCoreError::UnauthorizedProtocolAuthority.into());
    }
    let flags = read_pause_flags(data)?;

    let mut market = load_market(market_account, program_id)?;
    if flags & PAUSE_PURCHASE != 0 {
        market.purchase_paused = false;
    }
    if flags & PAUSE_USDC_REDEMPTION != 0 {
        market.usdc_redemption_paused = false;
    }
    store_market(market_account, &market)
}

/// Accounts: [protocol_authority (signer), protocol_config (writable)]
/// Data: new_protocol_authority(32) | new_emergency_operator(32)
///
/// Replaces both roles; pass the current value to keep one. The treasury and
/// the USDC mint cannot be changed.
pub fn process_update_authorities(
    program_id: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
) -> ProgramResult {
    let [authority, config, ..] = accounts else {
        return Err(AxisCoreError::MissingAccount.into());
    };
    expect_signer(authority)?;
    let mut state = load_config(config, program_id)?;
    if authority.address() != &state.protocol_authority {
        return Err(AxisCoreError::UnauthorizedProtocolAuthority.into());
    }
    if data.len() != 64 {
        return Err(AxisCoreError::InvalidInstruction.into());
    }
    let new_authority = read_address_arg(data, 0)?;
    // Nobody can sign for the zero address, so governance would be lost.
    if new_authority == SYSTEM_PROGRAM_ID {
        return Err(AxisCoreError::InvalidAuthority.into());
    }
    state.protocol_authority = new_authority;
    state.emergency_operator = read_address_arg(data, 32)?;

    expect_writable(config)?;
    let mut buf = config
        .try_borrow_mut()
        .map_err(|_| AxisCoreError::InvalidAccountData)?;
    state.pack(&mut buf)?;
    Ok(())
}

/// Accounts: [protocol_authority (signer), protocol_config, market (writable)]
/// Data: swap_program(32)
///
/// Sets the only program this market's route legs may invoke. The zero
/// address leaves purchases and USDC redemption unavailable; in-kind
/// redemption does not use a swap program.
pub fn process_set_swap_program(
    program_id: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
) -> ProgramResult {
    let [authority, config, market_account, ..] = accounts else {
        return Err(AxisCoreError::MissingAccount.into());
    };
    expect_signer(authority)?;
    let config_state = load_config(config, program_id)?;
    if authority.address() != &config_state.protocol_authority {
        return Err(AxisCoreError::UnauthorizedProtocolAuthority.into());
    }
    if data.len() != 32 {
        return Err(AxisCoreError::InvalidInstruction.into());
    }
    let swap_program = read_address_arg(data, 0)?;
    // Axis Core as its own route would let a leg re-enter Purchase.
    if &swap_program == program_id {
        return Err(AxisCoreError::InvalidProgramAccount.into());
    }

    let mut market = load_market(market_account, program_id)?;
    market.swap_program = swap_program;
    store_market(market_account, &market)
}
