//! Route legs: the swaps a purchase or a USDC redemption runs between Core's
//! balance snapshots.
//!
//! Core never signs a leg. Each leg is invoked with exactly the signer and
//! writable flags the transaction gave its accounts, so the only authority a
//! route can use is the user's own signature. The market PDA, which controls
//! the reserve vaults and the DTF mint, is never a signer here, and the
//! Solana runtime refuses re-entry into Axis Core from another program.
//! Custody therefore does not depend on what the swap program does; every
//! result is judged afterwards on measured balances.
//!
//! Encoding, after the instruction's own arguments:
//!
//! ```txt
//! leg_count(1) | leg_count × { account_count(1) | data_len(2, LE) | data }
//! ```
//!
//! Legs consume the instruction's trailing accounts in order, with no
//! account left over.

use pinocchio::cpi::invoke_with_bounds;
use pinocchio::instruction::{InstructionAccount, InstructionView};
use pinocchio::{AccountView, Address, ProgramResult};

use crate::constants::{MAX_ROUTE_LEGS, MAX_ROUTE_LEG_ACCOUNTS, SYSTEM_PROGRAM_ID};
use crate::error::AxisCoreError;
use crate::state::DTFMarket;

/// Checks that `swap_program` is the one governance approved for the market.
pub fn expect_swap_program(
    swap_program: &AccountView,
    market: &DTFMarket,
) -> Result<(), AxisCoreError> {
    if market.swap_program == SYSTEM_PROGRAM_ID {
        return Err(AxisCoreError::SwapProgramNotConfigured);
    }
    if swap_program.address() != &market.swap_program {
        return Err(AxisCoreError::InvalidProgramAccount);
    }
    Ok(())
}

#[inline(never)]
pub fn invoke_route_legs(
    swap_program: &Address,
    route_accounts: &[AccountView],
    route_data: &[u8],
) -> ProgramResult {
    let (leg_count, mut rest) = route_data
        .split_first()
        .ok_or(AxisCoreError::InvalidRouteData)?;
    if *leg_count as usize > MAX_ROUTE_LEGS {
        return Err(AxisCoreError::TooManyRouteLegs.into());
    }

    let mut cursor = 0usize;
    for _ in 0..*leg_count {
        if rest.len() < 3 {
            return Err(AxisCoreError::InvalidRouteData.into());
        }
        let account_count = rest[0] as usize;
        let data_len = u16::from_le_bytes([rest[1], rest[2]]) as usize;
        let data = rest
            .get(3..3 + data_len)
            .ok_or(AxisCoreError::InvalidRouteData)?;
        rest = &rest[3 + data_len..];

        let leg_accounts = route_accounts
            .get(cursor..cursor + account_count)
            .ok_or(AxisCoreError::MissingAccount)?;
        cursor += account_count;

        invoke_leg(swap_program, leg_accounts, data)?;
    }

    if !rest.is_empty() || cursor != route_accounts.len() {
        return Err(AxisCoreError::InvalidRouteData.into());
    }
    Ok(())
}

#[inline(never)]
fn invoke_leg(swap_program: &Address, accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    if accounts.len() > MAX_ROUTE_LEG_ACCOUNTS {
        return Err(AxisCoreError::TooManyRouteAccounts.into());
    }
    let mut metas: [InstructionAccount; MAX_ROUTE_LEG_ACCOUNTS] =
        [const { InstructionAccount::readonly(&SYSTEM_PROGRAM_ID) }; MAX_ROUTE_LEG_ACCOUNTS];
    for (meta, account) in metas.iter_mut().zip(accounts) {
        *meta = InstructionAccount::new(
            account.address(),
            account.is_writable(),
            account.is_signer(),
        );
    }
    invoke_bounded(
        &InstructionView {
            program_id: swap_program,
            accounts: &metas[..accounts.len()],
            data,
        },
        accounts,
    )
}

/// Kept out of line so the CPI account array gets a stack frame of its own.
#[inline(never)]
fn invoke_bounded(instruction: &InstructionView, accounts: &[AccountView]) -> ProgramResult {
    invoke_with_bounds::<MAX_ROUTE_LEG_ACCOUNTS, AccountView>(instruction, accounts)
}
