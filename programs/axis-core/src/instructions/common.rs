use pinocchio::{AccountView, Address};

use crate::error::AxisCoreError;

pub fn expect_signer(account: &AccountView) -> Result<(), AxisCoreError> {
    if !account.is_signer() {
        return Err(AxisCoreError::AccountNotSigner);
    }
    Ok(())
}

pub fn expect_writable(account: &AccountView) -> Result<(), AxisCoreError> {
    if !account.is_writable() {
        return Err(AxisCoreError::AccountNotWritable);
    }
    Ok(())
}

/// Every state account Axis Core reads or writes must be owned by Axis Core.
/// Without this a caller can supply a look-alike account with a matching
/// discriminator that they control, and the program will trust its contents.
pub fn expect_owned_by_program(
    account: &AccountView,
    program_id: &Address,
) -> Result<(), AxisCoreError> {
    if !account.owned_by(program_id) {
        return Err(AxisCoreError::InvalidAccountOwner);
    }
    Ok(())
}

/// A fresh state account: owned by this program, writable, large enough, and
/// with an all-zero discriminator so an initialized account is never reused.
pub fn expect_uninitialized(
    account: &AccountView,
    program_id: &Address,
    min_len: usize,
) -> Result<(), AxisCoreError> {
    expect_owned_by_program(account, program_id)?;
    expect_writable(account)?;
    if account.data_len() < min_len {
        return Err(AxisCoreError::InvalidAccountData);
    }
    let data = account
        .try_borrow()
        .map_err(|_| AxisCoreError::InvalidAccountData)?;
    if data[0..8].iter().any(|b| *b != 0) {
        return Err(AxisCoreError::AccountAlreadyInitialized);
    }
    Ok(())
}

/// Re-derives the PDA from `seeds`, bump included as the last seed, and checks
/// it matches. Used when an account is created; afterwards the owner check
/// plus the discriminator identify it, because only this program can write a
/// discriminator into an account it owns, and only at a verified address.
pub fn verify_pda(
    expected: &Address,
    seeds: &[&[u8]],
    program_id: &Address,
) -> Result<(), AxisCoreError> {
    let derived = Address::create_program_address(seeds, program_id)
        .map_err(|_| AxisCoreError::InvalidPda)?;
    if &derived != expected {
        return Err(AxisCoreError::InvalidPda);
    }
    Ok(())
}

pub fn read_u16_arg(data: &[u8], offset: usize) -> Result<u16, AxisCoreError> {
    let bytes: [u8; 2] = data
        .get(offset..offset + 2)
        .ok_or(AxisCoreError::InvalidInstruction)?
        .try_into()
        .map_err(|_| AxisCoreError::InvalidInstruction)?;
    Ok(u16::from_le_bytes(bytes))
}

pub fn read_address_arg(data: &[u8], offset: usize) -> Result<Address, AxisCoreError> {
    let bytes: [u8; 32] = data
        .get(offset..offset + 32)
        .ok_or(AxisCoreError::InvalidInstruction)?
        .try_into()
        .map_err(|_| AxisCoreError::InvalidInstruction)?;
    Ok(Address::from(bytes))
}
