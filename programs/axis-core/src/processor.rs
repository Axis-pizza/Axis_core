use pinocchio::{AccountView, Address, ProgramResult};

use crate::error::AxisCoreError;
use crate::instructions::{
    process_create_market, process_initialize_protocol_config, process_pause, process_purchase,
    process_redeem_in_kind, process_redeem_usdc, process_resume, process_set_swap_program,
    process_update_authorities, AxisInstruction,
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
            process_initialize_protocol_config(program_id, accounts, payload)
        }
        AxisInstruction::CreateMarket => process_create_market(program_id, accounts, payload),
        AxisInstruction::InitialPurchase => process_purchase(program_id, accounts, payload, true),
        AxisInstruction::Purchase => process_purchase(program_id, accounts, payload, false),
        AxisInstruction::RedeemUsdc => process_redeem_usdc(program_id, accounts, payload),
        AxisInstruction::RedeemInKind => process_redeem_in_kind(program_id, accounts, payload),
        AxisInstruction::Pause => process_pause(program_id, accounts, payload),
        AxisInstruction::Resume => process_resume(program_id, accounts, payload),
        AxisInstruction::UpdateAuthorities => {
            process_update_authorities(program_id, accounts, payload)
        }
        AxisInstruction::SetSwapProgram => process_set_swap_program(program_id, accounts, payload),
    }
}
