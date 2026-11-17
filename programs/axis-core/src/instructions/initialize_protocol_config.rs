use pinocchio::cpi::Seed;
use pinocchio::{AccountView, Address, ProgramResult};

use crate::constants::{PROTOCOL_CONFIG_SEED, SYSTEM_PROGRAM_ID};
use crate::error::AxisCoreError;
use crate::instructions::common::{
    create_pda_account, expect_canonical_pda, expect_signer, expect_upgrade_authority,
    read_address_arg,
};
use crate::state::ProtocolConfig;

/// Accounts: [upgrade_authority (signer, writable, pays rent),
///            protocol_config (writable, not yet created),
///            system_program,
///            program_data (Axis Core's ProgramData account)]
/// Data: protocol_authority(32) | usdc_mint(32) | protocol_treasury(32) |
///       emergency_operator(32)
///
/// Creates the singleton at `["protocol_config"]` with the canonical bump, so
/// at most one can ever exist. Only the program's upgrade authority may call
/// it; otherwise whoever lands the first transaction after deployment would
/// own the protocol. The governance authority is named in the data, so the
/// deployer can hand governance to the Squads vault in the same step.
pub fn process_initialize_protocol_config(
    program_id: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
) -> ProgramResult {
    let [authority, config, system_program, program_data, ..] = accounts else {
        return Err(AxisCoreError::MissingAccount.into());
    };
    expect_signer(authority)?;
    expect_upgrade_authority(program_data, authority.address(), program_id)?;
    let bump = expect_canonical_pda(config, &[PROTOCOL_CONFIG_SEED], program_id)?;

    let state = ProtocolConfig {
        protocol_authority: read_address_arg(data, 0)?,
        usdc_mint: read_address_arg(data, 32)?,
        protocol_treasury: read_address_arg(data, 64)?,
        emergency_operator: read_address_arg(data, 96)?,
        bump,
    };
    // Nobody can sign for the zero address, so governance would be lost.
    if state.protocol_authority == SYSTEM_PROGRAM_ID {
        return Err(AxisCoreError::InvalidAuthority.into());
    }

    let bump_seed = [bump];
    create_pda_account(
        authority,
        config,
        system_program,
        program_id,
        ProtocolConfig::LEN,
        &[Seed::from(PROTOCOL_CONFIG_SEED), Seed::from(&bump_seed)],
    )?;

    let mut buf = config
        .try_borrow_mut()
        .map_err(|_| AxisCoreError::InvalidAccountData)?;
    state.pack(&mut buf)?;
    Ok(())
}
