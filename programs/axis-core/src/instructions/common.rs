use pinocchio::cpi::{invoke, invoke_signed, Seed, Signer};
use pinocchio::instruction::{InstructionAccount, InstructionView};
use pinocchio::sysvars::{rent::Rent, Sysvar};
use pinocchio::{AccountView, Address, ProgramResult};

use crate::constants::{BPF_LOADER_UPGRADEABLE_ID, SYSTEM_PROGRAM_ID};
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

/// Allocates a state account at the program-derived address `target`: owned
/// by this program, `space` bytes, zeroed, rent-exempt, paid for by `payer`.
/// `seeds` derive `target`, bump included, and only this program can sign
/// with them, so nothing else can put an account in this state.
///
/// An account this program already owns is refused, so a state account is
/// created once and never re-initialized.
///
/// Anyone can send lamports to a PDA before it exists, and System
/// `CreateAccount` refuses an address that already holds any. For the
/// `protocol_config` singleton that would block initialization permanently,
/// so a pre-funded address is topped up, allocated and assigned instead.
pub fn create_pda_account(
    payer: &AccountView,
    target: &AccountView,
    system_program: &AccountView,
    program_id: &Address,
    space: usize,
    seeds: &[Seed],
) -> ProgramResult {
    if target.owned_by(program_id) {
        return Err(AxisCoreError::AccountAlreadyInitialized.into());
    }
    if !target.owned_by(&SYSTEM_PROGRAM_ID) {
        return Err(AxisCoreError::InvalidAccountOwner.into());
    }
    if system_program.address() != &SYSTEM_PROGRAM_ID {
        return Err(AxisCoreError::InvalidProgramAccount.into());
    }
    expect_writable(payer)?;
    expect_writable(target)?;

    let rent = Rent::get()?.try_minimum_balance(space)?;
    let signer = [Signer::from(seeds)];
    let held = target.lamports();

    if held == 0 {
        let mut data = [0u8; 52];
        data[0..4].copy_from_slice(&SYSTEM_CREATE_ACCOUNT.to_le_bytes());
        data[4..12].copy_from_slice(&rent.to_le_bytes());
        data[12..20].copy_from_slice(&(space as u64).to_le_bytes());
        data[20..52].copy_from_slice(program_id.as_ref());
        return invoke_signed(
            &InstructionView {
                program_id: &SYSTEM_PROGRAM_ID,
                accounts: &[
                    InstructionAccount::writable_signer(payer.address()),
                    InstructionAccount::writable_signer(target.address()),
                ],
                data: &data,
            },
            &[payer, target],
            &signer,
        );
    }

    if held < rent {
        let mut data = [0u8; 12];
        data[0..4].copy_from_slice(&SYSTEM_TRANSFER.to_le_bytes());
        data[4..12].copy_from_slice(&(rent - held).to_le_bytes());
        invoke(
            &InstructionView {
                program_id: &SYSTEM_PROGRAM_ID,
                accounts: &[
                    InstructionAccount::writable_signer(payer.address()),
                    InstructionAccount::writable(target.address()),
                ],
                data: &data,
            },
            &[payer, target],
        )?;
    }

    let mut data = [0u8; 12];
    data[0..4].copy_from_slice(&SYSTEM_ALLOCATE.to_le_bytes());
    data[4..12].copy_from_slice(&(space as u64).to_le_bytes());
    invoke_signed(
        &InstructionView {
            program_id: &SYSTEM_PROGRAM_ID,
            accounts: &[InstructionAccount::writable_signer(target.address())],
            data: &data,
        },
        &[target],
        &signer,
    )?;

    let mut data = [0u8; 36];
    data[0..4].copy_from_slice(&SYSTEM_ASSIGN.to_le_bytes());
    data[4..36].copy_from_slice(program_id.as_ref());
    invoke_signed(
        &InstructionView {
            program_id: &SYSTEM_PROGRAM_ID,
            accounts: &[InstructionAccount::writable_signer(target.address())],
            data: &data,
        },
        &[target],
        &signer,
    )
}

const SYSTEM_CREATE_ACCOUNT: u32 = 0;
const SYSTEM_ASSIGN: u32 = 1;
const SYSTEM_TRANSFER: u32 = 2;
const SYSTEM_ALLOCATE: u32 = 8;

/// Checks that `account` is the PDA of `seeds` at the canonical (highest
/// valid) bump, and returns that bump.
///
/// Every bump that lands off the curve derives a valid but different address.
/// A caller-chosen bump would therefore let anyone create a second
/// `protocol_config`, or a second market for the same DTF mint. Deriving the
/// bump here leaves exactly one address per seed set.
pub fn expect_canonical_pda(
    account: &AccountView,
    seeds: &[&[u8]],
    program_id: &Address,
) -> Result<u8, AxisCoreError> {
    let (derived, bump) =
        Address::try_find_program_address(seeds, program_id).ok_or(AxisCoreError::InvalidPda)?;
    if account.address() != &derived {
        return Err(AxisCoreError::InvalidPda);
    }
    Ok(bump)
}

/// Checks that `authority` is the upgrade authority recorded in Axis Core's
/// ProgramData account under the upgradeable BPF loader.
///
/// An immutable program has no upgrade authority, so this always fails once
/// the program is finalized: initialize before finalizing.
pub fn expect_upgrade_authority(
    program_data: &AccountView,
    authority: &Address,
    program_id: &Address,
) -> Result<(), AxisCoreError> {
    let (derived, _) =
        Address::try_find_program_address(&[program_id.as_ref()], &BPF_LOADER_UPGRADEABLE_ID)
            .ok_or(AxisCoreError::InvalidPda)?;
    if program_data.address() != &derived || !program_data.owned_by(&BPF_LOADER_UPGRADEABLE_ID) {
        return Err(AxisCoreError::InvalidProgramAccount);
    }
    let data = program_data
        .try_borrow()
        .map_err(|_| AxisCoreError::InvalidAccountData)?;
    // UpgradeableLoaderState::ProgramData:
    // tag u32 = 3 | slot u64 | Option<Address> upgrade_authority
    if data.len() < 45 || data[0..4] != 3u32.to_le_bytes() {
        return Err(AxisCoreError::InvalidProgramAccount);
    }
    if data[12] != 1 || &data[13..45] != authority.as_ref() {
        return Err(AxisCoreError::UnauthorizedProtocolAuthority);
    }
    Ok(())
}

/// Re-derives the PDA from `seeds`, bump included as the last seed, and checks
/// it matches. For accounts that already exist: the bump is the canonical one
/// stored at creation, which is cheaper than deriving it again.
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
