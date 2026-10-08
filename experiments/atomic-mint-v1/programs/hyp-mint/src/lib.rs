#![cfg_attr(target_os = "solana", no_std)]

#[cfg(feature = "bpf-entrypoint")]
mod entrypoint;

use pinocchio::cpi::invoke;
use pinocchio::error::ProgramError;
use pinocchio::instruction::{InstructionAccount, InstructionView};
use pinocchio::{AccountView, Address, ProgramResult};

const INVALID_INPUT: u32 = 1;
const INSUFFICIENT_DELIVERY: u32 = 2;
const MAX_INPUT_EXCEEDED: u32 = 3;
const ZERO_SUPPLY: u32 = 4;

pub fn process_instruction(
    program_id: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
) -> ProgramResult {
    let [user, user_usdc, market, reserve, user_dtf, route_program, ..] = accounts else {
        return Err(ProgramError::Custom(INVALID_INPUT));
    };

    if !user.is_signer()
        || !user_usdc.is_writable()
        || !market.is_writable()
        || !reserve.is_writable()
        || !user_dtf.is_writable()
        || !market.owned_by(program_id)
        || !user_dtf.owned_by(program_id)
        || !user_usdc.owned_by(route_program.address())
        || !reserve.owned_by(route_program.address())
        || market.data_len() < 8
        || user_usdc.data_len() < 8
        || reserve.data_len() < 8
        || user_dtf.data_len() < 8
    {
        return Err(ProgramError::Custom(INVALID_INPUT));
    }

    let dtf_out = read_u64(data, 0)?;
    let max_usdc_in = read_u64(data, 8)?;
    let route_data = data
        .get(16..32)
        .ok_or(ProgramError::Custom(INVALID_INPUT))?;
    let supply_pre = read_balance(market)?;
    if supply_pre == 0 {
        return Err(ProgramError::Custom(ZERO_SUPPLY));
    }
    let usdc_pre = read_balance(user_usdc)?;
    let reserve_pre = read_balance(reserve)?;
    let required = div_ceil(
        (reserve_pre as u128)
            .checked_mul(dtf_out as u128)
            .ok_or(ProgramError::ArithmeticOverflow)?,
        supply_pre as u128,
    )?;

    let route_accounts = [
        InstructionAccount::writable(user_usdc.address()),
        InstructionAccount::writable(reserve.address()),
    ];
    let route_instruction = InstructionView {
        program_id: route_program.address(),
        accounts: &route_accounts,
        data: route_data,
    };
    invoke(&route_instruction, &[&*user_usdc, &*reserve])?;

    let usdc_post = read_balance(user_usdc)?;
    let reserve_post = read_balance(reserve)?;
    let spent = usdc_pre
        .checked_sub(usdc_post)
        .ok_or(ProgramError::Custom(MAX_INPUT_EXCEEDED))?;
    if spent > max_usdc_in {
        return Err(ProgramError::Custom(MAX_INPUT_EXCEEDED));
    }
    let delivered = reserve_post
        .checked_sub(reserve_pre)
        .ok_or(ProgramError::Custom(INSUFFICIENT_DELIVERY))?;
    if (delivered as u128) < required {
        return Err(ProgramError::Custom(INSUFFICIENT_DELIVERY));
    }

    let user_dtf_pre = read_balance(user_dtf)?;
    write_balance(
        market,
        supply_pre
            .checked_add(dtf_out)
            .ok_or(ProgramError::ArithmeticOverflow)?,
    )?;
    write_balance(
        user_dtf,
        user_dtf_pre
            .checked_add(dtf_out)
            .ok_or(ProgramError::ArithmeticOverflow)?,
    )
}

fn div_ceil(value: u128, divisor: u128) -> Result<u128, ProgramError> {
    value
        .checked_add(divisor - 1)
        .and_then(|adjusted| adjusted.checked_div(divisor))
        .ok_or(ProgramError::ArithmeticOverflow)
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
