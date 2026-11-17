use pinocchio::cpi::{Seed, Signer};
use pinocchio::error::ProgramError;
use pinocchio::{AccountView, Address, ProgramResult};

use crate::constants::{MARKET_SEED, MAX_ASSETS};
use crate::error::AxisCoreError;
use crate::instructions::common::{expect_signer, load_market, read_u64_arg};
use crate::instructions::purchase::{expect_reserve_vaults, expect_treasury_dtf, expect_user_usdc};
use crate::instructions::route::{expect_swap_program, invoke_route_legs};
use crate::instructions::token::{
    burn, expect_token_program, mint_supply, token_balance, transfer,
};
use crate::math;
use crate::state::{DTFMarket, TokenProgramKind};

struct Exit<'a> {
    user: &'a AccountView,
    market_account: &'a AccountView,
    dtf_mint: &'a AccountView,
    user_dtf: &'a AccountView,
    treasury_dtf: &'a AccountView,
    token_program: &'a AccountView,
    token_2022_program: &'a AccountView,
    vaults: &'a [AccountView],
    destinations: &'a [AccountView],
}

struct Released {
    supply_after: u64,
    reserves_after: [u64; MAX_ASSETS],
}

impl<'a> Exit<'a> {
    /// The token program account for `kind`. Each slot is only checked when
    /// an instruction needs it, so a market that uses one token program can
    /// repeat that program in the other slot instead of locking a second one.
    fn program_for(&self, kind: TokenProgramKind) -> Result<&'a AccountView, AxisCoreError> {
        let program = match kind {
            TokenProgramKind::LegacySplToken => self.token_program,
            TokenProgramKind::Token2022 => self.token_2022_program,
        };
        expect_token_program(program, kind)?;
        Ok(program)
    }

    /// The exit both redemption paths share: 30 bps of `dtf_in` goes to the
    /// treasury, the rest is burned, and the holder's exact pro-rata share of
    /// every reserve, rounded down, is transferred to `destinations`.
    ///
    /// The market PDA signs these plain token transfers and nothing else, in
    /// any instruction.
    fn release(&self, market: &DTFMarket, dtf_in: u64) -> Result<Released, ProgramError> {
        if dtf_in == 0 {
            return Err(AxisCoreError::ZeroAmount.into());
        }
        if self.dtf_mint.address() != &market.dtf_mint {
            return Err(AxisCoreError::AccountMismatch.into());
        }
        let count = market.asset_count as usize;
        if self.vaults.len() != count || self.destinations.len() != count {
            return Err(AxisCoreError::MissingAccount.into());
        }
        expect_reserve_vaults(self.vaults, market)?;
        expect_treasury_dtf(self.treasury_dtf, market)?;
        // Releasing a reserve into a reserve would burn the holder's DTF for
        // nothing.
        for destination in self.destinations {
            if market
                .assets()
                .any(|a| &a.reserve_vault == destination.address())
            {
                return Err(AxisCoreError::AccountMismatch.into());
            }
        }

        let fee = math::fee(dtf_in);
        let burned = dtf_in - fee;
        let supply_before = mint_supply(self.dtf_mint)?;

        let mut releases = [0u64; MAX_ASSETS];
        let mut reserves_after = [0u64; MAX_ASSETS];
        let mut any = false;
        for i in 0..count {
            let reserve = token_balance(&self.vaults[i])?;
            releases[i] = math::pro_rata_release(reserve, burned, supply_before)?;
            reserves_after[i] = reserve
                .checked_sub(releases[i])
                .ok_or(AxisCoreError::MathOverflow)?;
            any |= releases[i] > 0;
        }
        // A redemption too small to release a single base unit of anything
        // would only destroy the holder's DTF.
        if !any {
            return Err(AxisCoreError::ZeroAmount.into());
        }

        let dtf_program = self.program_for(market.dtf_token_program)?;
        if fee > 0 {
            transfer(
                dtf_program,
                self.user_dtf,
                self.treasury_dtf,
                self.user,
                fee,
                &[],
            )?;
        }
        burn(dtf_program, self.user_dtf, self.dtf_mint, self.user, burned)?;

        let bump = [market.bump];
        let seeds = [
            Seed::from(MARKET_SEED),
            Seed::from(market.dtf_mint.as_ref()),
            Seed::from(&bump),
        ];
        let signer = [Signer::from(&seeds)];
        for i in 0..count {
            if releases[i] == 0 {
                continue;
            }
            transfer(
                self.program_for(market.asset(i)?.token_program)?,
                &self.vaults[i],
                &self.destinations[i],
                self.market_account,
                releases[i],
                &signer,
            )?;
        }

        Ok(Released {
            supply_after: supply_before
                .checked_sub(burned)
                .ok_or(AxisCoreError::MathOverflow)?,
            reserves_after,
        })
    }
}

const IN_KIND_FIXED: usize = 7;

/// Accounts: [user (signer), market, dtf_mint (writable), user_dtf (writable),
///            treasury_dtf (writable), token_program, token_2022_program,
///            reserve_vault × asset_count (writable),
///            destination × asset_count (writable)]
/// Data: dtf_in(8)
///
/// The exit that needs no swap and no route: 30 bps of `dtf_in` goes to the
/// treasury, the rest is burned, and the holder receives the proportional
/// quantity of every constituent directly.
///
/// This handler reads neither pause flag and no swap program, so an ordinary
/// pause cannot close it. It still fails if a constituent itself cannot be
/// transferred, for example a frozen destination account.
#[inline(never)]
pub fn process_redeem_in_kind(
    program_id: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
) -> ProgramResult {
    let accounts: &[AccountView] = accounts;
    if accounts.len() < IN_KIND_FIXED {
        return Err(AxisCoreError::MissingAccount.into());
    }
    let user = &accounts[0];
    expect_signer(user)?;
    let dtf_in = read_u64_arg(data, 0)?;
    if data.len() != 8 {
        return Err(AxisCoreError::InvalidInstruction.into());
    }

    let market = load_market(&accounts[1], program_id)?;
    let count = market.asset_count as usize;
    if accounts.len() != IN_KIND_FIXED + 2 * count {
        return Err(AxisCoreError::MissingAccount.into());
    }
    Exit {
        user,
        market_account: &accounts[1],
        dtf_mint: &accounts[2],
        user_dtf: &accounts[3],
        treasury_dtf: &accounts[4],
        token_program: &accounts[5],
        token_2022_program: &accounts[6],
        vaults: &accounts[IN_KIND_FIXED..IN_KIND_FIXED + count],
        destinations: &accounts[IN_KIND_FIXED + count..],
    }
    .release(&market, dtf_in)?;
    Ok(())
}

const USDC_FIXED: usize = 9;

/// Accounts: [user (signer), market, dtf_mint (writable), user_dtf (writable),
///            treasury_dtf (writable), user_usdc (writable), token_program,
///            token_2022_program, swap_program,
///            reserve_vault × asset_count (writable),
///            user_asset × asset_count (writable),
///            route accounts...]
/// Data: dtf_in(8) | min_net_usdc_out(8) | route legs (see `route`)
///
/// A USDC exit is the in-kind exit followed, in the same instruction, by
/// swaps the holder authorizes:
///
/// 1. 30 bps of `dtf_in` goes to the treasury and the rest is burned.
/// 2. The holder's pro-rata share of every reserve is released to the
///    holder's own token accounts.
/// 3. The route legs swap those accounts into USDC under the holder's
///    signature. The market PDA never signs a route, so a route cannot reach
///    the reserves that stay behind.
/// 4. The holder's USDC account must have gained at least
///    `min_net_usdc_out`, which is after the fee because the fee was taken
///    in DTF before anything was released. Every reserve must hold exactly
///    what the release left, or more.
///
/// A failed swap or a missed minimum reverts the burn, the fee and every
/// release.
#[inline(never)]
pub fn process_redeem_usdc(
    program_id: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
) -> ProgramResult {
    let accounts: &[AccountView] = accounts;
    if accounts.len() < USDC_FIXED {
        return Err(AxisCoreError::MissingAccount.into());
    }
    let user = &accounts[0];
    let dtf_mint = &accounts[2];
    let user_usdc = &accounts[5];
    let swap_program = &accounts[8];

    expect_signer(user)?;
    let dtf_in = read_u64_arg(data, 0)?;
    let min_net_usdc_out = read_u64_arg(data, 8)?;
    let route_data = &data[16..];

    let market = load_market(&accounts[1], program_id)?;
    if market.usdc_redemption_paused {
        return Err(AxisCoreError::UsdcRedemptionPaused.into());
    }
    expect_swap_program(swap_program, &market)?;
    expect_user_usdc(user_usdc, user, &market)?;

    let count = market.asset_count as usize;
    let vaults = accounts
        .get(USDC_FIXED..USDC_FIXED + count)
        .ok_or(AxisCoreError::MissingAccount)?;
    let destinations = accounts
        .get(USDC_FIXED + count..USDC_FIXED + 2 * count)
        .ok_or(AxisCoreError::MissingAccount)?;
    let route_accounts = &accounts[USDC_FIXED + 2 * count..];

    let usdc_before = token_balance(user_usdc)?;
    let released = Exit {
        user,
        market_account: &accounts[1],
        dtf_mint,
        user_dtf: &accounts[3],
        treasury_dtf: &accounts[4],
        token_program: &accounts[6],
        token_2022_program: &accounts[7],
        vaults,
        destinations,
    }
    .release(&market, dtf_in)?;

    invoke_route_legs(swap_program.address(), route_accounts, route_data)?;

    let received = token_balance(user_usdc)?.saturating_sub(usdc_before);
    if received < min_net_usdc_out {
        return Err(AxisCoreError::SlippageExceeded.into());
    }
    if mint_supply(dtf_mint)? != released.supply_after {
        return Err(AxisCoreError::SupplyChangedDuringRoute.into());
    }
    for (vault, after) in vaults.iter().zip(released.reserves_after) {
        if token_balance(vault)? < after {
            return Err(AxisCoreError::ReserveOverdrawn.into());
        }
    }
    Ok(())
}
