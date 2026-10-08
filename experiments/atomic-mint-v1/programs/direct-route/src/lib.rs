#![cfg_attr(target_os = "solana", no_std)]

#[cfg(feature = "bpf-entrypoint")]
mod entrypoint;

use pinocchio::error::ProgramError;
use pinocchio::{AccountView, Address, ProgramResult};

const INVALID_INPUT: u32 = 1;
const INSUFFICIENT_FUNDS: u32 = 2;

pub fn process_instruction(
    program_id: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
) -> ProgramResult {
    let [user_usdc, reserve, ..] = accounts else {
        return Err(ProgramError::Custom(INVALID_INPUT));
    };
    if !user_usdc.is_writable()
        || !reserve.is_writable()
        || !user_usdc.owned_by(program_id)
        || !reserve.owned_by(program_id)
        || user_usdc.data_len() < 8
        || reserve.data_len() < 8
    {
        return Err(ProgramError::Custom(INVALID_INPUT));
    }

    let spend = read_u64(data, 0)?;
    let delivery = read_u64(data, 8)?;
    let user_pre = read_balance(user_usdc)?;
    let reserve_pre = read_balance(reserve)?;
    if user_pre < spend {
        return Err(ProgramError::Custom(INSUFFICIENT_FUNDS));
    }

    write_balance(user_usdc, user_pre - spend)?;
    write_balance(
        reserve,
        reserve_pre
            .checked_add(delivery)
            .ok_or(ProgramError::ArithmeticOverflow)?,
    )
}

fn read_u64(data: &[u8], offset: usize) -> Result<u64, ProgramError> {
    let bytes: [u8; 8] = data
        .get(offset..offset + 8)
        .ok_or(ProgramError::Custom(INVALID_INPUT))?
        .try_into()
        .map_err(|_| ProgramError::Custom(INVALID_INPUT))?;
    Ok(u64::from_le_bytes(bytes))
}

fn read_balance(account: &AccountView) -> Result<u64, ProgramError> {
    let data = account.try_borrow()?;
    let bytes: [u8; 8] = data[..8]
        .try_into()
        .map_err(|_| ProgramError::InvalidAccountData)?;
    Ok(u64::from_le_bytes(bytes))
}

fn write_balance(account: &mut AccountView, value: u64) -> ProgramResult {
    let mut data = account.try_borrow_mut()?;
    data[..8].copy_from_slice(&value.to_le_bytes());
    Ok(())
}
