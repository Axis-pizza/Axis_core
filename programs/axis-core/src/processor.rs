use pinocchio::{AccountView, Address, ProgramResult};

use crate::error::AxisCoreError;
use crate::instructions::{
    process_create_market, process_initialize_protocol_config, AxisInstruction,
};

#[inline(never)]
pub fn process_instruction(
    program_id: &Address,
    accounts: &mut [AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    let (tag, payload) = instruction_data
        .split_first()
        .ok_or(AxisCoreError::InvalidInstruction)?;

    match AxisInstruction::try_from(*tag)? {
        AxisInstruction::InitializeProtocolConfig => {
            process_initialize_protocol_config(program_id, accounts, payload)?
        }
        AxisInstruction::CreateMarket => process_create_market(program_id, accounts, payload)?,
    }
    Ok(())
}
