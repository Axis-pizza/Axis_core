use pinocchio::{AccountView, Address};

use crate::constants::PROTOCOL_CONFIG_SEED;
use crate::error::AxisCoreError;
use crate::instructions::common::{
    expect_signer, expect_uninitialized, read_address_arg, read_u16_arg, verify_pda,
};
use crate::state::ProtocolConfig;

/// Accounts: [protocol_authority (signer), protocol_config (writable, uninitialized PDA)]
/// Data: usdc_mint(32) | protocol_treasury(32) | mint_fee_bps(2) |
///       creator_share_bps(2) | max_mint_fee_bps(2) | bump(1)
///
/// First-come initialization of the singleton at `["protocol_config"]`. The
/// address is fixed, so at most one can ever exist.
pub fn process_initialize_protocol_config(
    program_id: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
) -> Result<(), AxisCoreError> {
    let [authority, config, ..] = accounts else {
        return Err(AxisCoreError::MissingAccount);
    };
    expect_signer(authority)?;

    let bump = *data.get(70).ok_or(AxisCoreError::InvalidInstruction)?;
    let state = ProtocolConfig {
        protocol_authority: authority.address().clone(),
        usdc_mint: read_address_arg(data, 0)?,
        protocol_treasury: read_address_arg(data, 32)?,
        mint_fee_bps: read_u16_arg(data, 64)?,
        creator_share_bps: read_u16_arg(data, 66)?,
        max_mint_fee_bps: read_u16_arg(data, 68)?,
        bump,
    };
    state.validate()?;

    expect_uninitialized(config, program_id, ProtocolConfig::LEN)?;
    verify_pda(
        config.address(),
        &[PROTOCOL_CONFIG_SEED, &[bump]],
        program_id,
    )?;

    let mut buf = config
        .try_borrow_mut()
        .map_err(|_| AxisCoreError::InvalidAccountData)?;
    state.pack(&mut buf)
}
