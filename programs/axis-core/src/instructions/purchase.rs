use pinocchio::cpi::{Seed, Signer};
use pinocchio::{AccountView, Address, ProgramResult};

use crate::constants::{MARKET_SEED, MAX_ASSETS};
use crate::error::AxisCoreError;
use crate::instructions::common::{expect_signer, load_market, read_u64_arg};
use crate::instructions::route::{expect_swap_program, invoke_route_legs};
use crate::instructions::token::{
    expect_token_program, mint_supply, mint_to, read_token_account, token_balance,
    token_program_kind_of,
};
use crate::math;
use crate::state::DTFMarket;

/// The buyer's USDC account: the one whose balance decrease is held to the
/// signed `max_usdc_in`, or whose increase is held to `min_net_usdc_out`.
pub fn expect_user_usdc(
    user_usdc: &AccountView,
    user: &AccountView,
    market: &DTFMarket,
) -> Result<(), AxisCoreError> {
    let kind = token_program_kind_of(user_usdc)?;
    let state = read_token_account(user_usdc, kind, false)?;
    if state.mint != market.usdc_mint {
        return Err(AxisCoreError::TokenAccountMintMismatch);
    }
    if &state.owner != user.address() {
        return Err(AxisCoreError::TokenAccountOwnerMismatch);
    }
    Ok(())
}

/// The fee destination: a DTF account owned by the market's treasury.
pub fn expect_treasury_dtf(
    treasury_dtf: &AccountView,
    market: &DTFMarket,
) -> Result<(), AxisCoreError> {
    let state = read_token_account(treasury_dtf, market.dtf_token_program, false)?;
    if state.mint != market.dtf_mint {
        return Err(AxisCoreError::TokenAccountMintMismatch);
    }
    if state.owner != market.treasury {
        return Err(AxisCoreError::TokenAccountOwnerMismatch);
    }
    Ok(())
}

/// Checks that `vaults` are the market's reserve vaults, in table order.
pub fn expect_reserve_vaults(
    vaults: &[AccountView],
    market: &DTFMarket,
) -> Result<(), AxisCoreError> {
    for (i, vault) in vaults.iter().enumerate() {
        if vault.address() != &market.asset(i)?.reserve_vault {
            return Err(AxisCoreError::AccountMismatch);
        }
    }
    Ok(())
}

const FIXED_ACCOUNTS: usize = 8;

/// `Purchase` accounts:
///   [user (signer), market, dtf_mint (writable), user_usdc (writable),
///    user_dtf (writable), treasury_dtf (writable), dtf_token_program,
///    swap_program,
///    reserve_vault × asset_count (writable),
///    route accounts...]
/// `InitialPurchase` adds `liquidity_lock (writable)` right after
/// `swap_program`.
/// Data: max_usdc_in(8) | min_net_dtf_out(8) | route legs (see `route`)
///
/// One atomic purchase:
///
/// 1. Snapshot DTF supply, every reserve balance and the buyer's USDC.
/// 2. Run the route legs under the buyer's signature only.
/// 3. Measure. No reserve may have lost balance, and the buyer may not have
///    spent more USDC than `max_usdc_in`. USDC the route did not spend never
///    left the buyer's account.
/// 4. Gross issuance is `min_i floor(delta_i * supply_before / reserve_i_before)`
///    over the actual reserve increases, so the constituent with the least
///    delivery sets it. Whatever the other legs delivered beyond that stays
///    in the reserves, backing every holder.
/// 5. 30 bps of the gross is minted to the treasury, the rest to the buyer,
///    and the buyer's part must reach `min_net_dtf_out`.
///
/// Any failure reverts the whole transaction, swaps included.
///
/// At zero supply there is no ratio to follow, so the first purchase is
/// measured against the governance-approved reference basket instead:
/// `reference_amount_i` stands in for the reserve and `reference_supply` for
/// the supply. The first purchaser picks the size, not the allocation or the
/// issuance rate. `min_liquidity` of the first issuance is minted to the
/// market's lock account and can never move, so supply never returns to zero
/// and the ratio stays defined for the life of the market.
///
/// `InitialPurchase` on a market that is already funded runs as an ordinary
/// purchase: the loser of a race for the first purchase is still held to its
/// own signed bounds and needs no rebuilt transaction.
#[inline(never)]
pub fn process_purchase(
    program_id: &Address,
    accounts: &mut [AccountView],
    data: &[u8],
    initial: bool,
) -> ProgramResult {
    let accounts: &[AccountView] = accounts;
    let fixed = FIXED_ACCOUNTS + initial as usize;
    if accounts.len() < fixed {
        return Err(AxisCoreError::MissingAccount.into());
    }
    let user = &accounts[0];
    let market_account = &accounts[1];
    let dtf_mint = &accounts[2];
    let user_usdc = &accounts[3];
    let user_dtf = &accounts[4];
    let treasury_dtf = &accounts[5];
    let dtf_token_program = &accounts[6];
    let swap_program = &accounts[7];

    expect_signer(user)?;
    let max_usdc_in = read_u64_arg(data, 0)?;
    let min_net_dtf_out = read_u64_arg(data, 8)?;
    let route_data = &data[16..];

    let market = load_market(market_account, program_id)?;
    if market.purchase_paused {
        return Err(AxisCoreError::PurchasePaused.into());
    }
    if dtf_mint.address() != &market.dtf_mint {
        return Err(AxisCoreError::AccountMismatch.into());
    }
    expect_token_program(dtf_token_program, market.dtf_token_program)?;
    expect_swap_program(swap_program, &market)?;

    let count = market.asset_count as usize;
    let vaults = accounts
        .get(fixed..fixed + count)
        .ok_or(AxisCoreError::MissingAccount)?;
    let route_accounts = &accounts[fixed + count..];
    expect_reserve_vaults(vaults, &market)?;
    expect_user_usdc(user_usdc, user, &market)?;
    expect_treasury_dtf(treasury_dtf, &market)?;

    let supply_before = mint_supply(dtf_mint)?;
    let seeding = supply_before == 0;
    let liquidity_lock = if seeding {
        if !initial {
            return Err(AxisCoreError::MarketNotSeeded.into());
        }
        let lock = &accounts[FIXED_ACCOUNTS];
        if lock.address() != &market.liquidity_lock {
            return Err(AxisCoreError::AccountMismatch.into());
        }
        Some(lock)
    } else {
        None
    };

    let usdc_before = token_balance(user_usdc)?;
    let mut reserves_before = [0u64; MAX_ASSETS];
    for (before, vault) in reserves_before.iter_mut().zip(vaults) {
        *before = token_balance(vault)?;
    }

    invoke_route_legs(swap_program.address(), route_accounts, route_data)?;

    // Only the market PDA can mint, and it signs nothing during the route.
    if mint_supply(dtf_mint)? != supply_before {
        return Err(AxisCoreError::SupplyChangedDuringRoute.into());
    }
    let spent = usdc_before.saturating_sub(token_balance(user_usdc)?);
    if spent > max_usdc_in {
        return Err(AxisCoreError::MaxInputExceeded.into());
    }
    let mut deltas = [0u64; MAX_ASSETS];
    for i in 0..count {
        deltas[i] = token_balance(&vaults[i])?
            .checked_sub(reserves_before[i])
            .ok_or(AxisCoreError::ReserveDebitedOnMint)?;
    }

    let gross = if seeding {
        let mut reference = [0u64; MAX_ASSETS];
        for (i, amount) in reference.iter_mut().enumerate().take(count) {
            *amount = market.asset(i)?.reference_amount;
        }
        math::gross_issuance(
            &deltas[..count],
            &reference[..count],
            market.reference_supply,
        )?
    } else {
        math::gross_issuance(&deltas[..count], &reserves_before[..count], supply_before)?
    };
    if gross == 0 {
        return Err(AxisCoreError::ZeroAmount.into());
    }

    let fee = math::fee(gross);
    let locked = if seeding { market.min_liquidity } else { 0 };
    let net = (gross - fee)
        .checked_sub(locked)
        .ok_or(AxisCoreError::SupplyBelowMinimumLiquidity)?;
    if net == 0 {
        return Err(if seeding {
            AxisCoreError::SupplyBelowMinimumLiquidity.into()
        } else {
            AxisCoreError::ZeroAmount.into()
        });
    }
    if net < min_net_dtf_out {
        return Err(AxisCoreError::SlippageExceeded.into());
    }

    let bump = [market.bump];
    let seeds = [
        Seed::from(MARKET_SEED),
        Seed::from(market.dtf_mint.as_ref()),
        Seed::from(&bump),
    ];
    let signer = [Signer::from(&seeds)];

    if let Some(lock) = liquidity_lock {
        mint_to(
            dtf_token_program,
            dtf_mint,
            lock,
            market_account,
            locked,
            &signer,
        )?;
    }
    if fee > 0 {
        mint_to(
            dtf_token_program,
            dtf_mint,
            treasury_dtf,
            market_account,
            fee,
            &signer,
        )?;
    }
    mint_to(
        dtf_token_program,
        dtf_mint,
        user_dtf,
        market_account,
        net,
        &signer,
    )
}
