//! SPL Token and Token-2022 account reads and CPIs.
//!
//! Admission is deliberately narrow until the asset policy in requirements §8
//! is decided: a mint must be the bare 82-byte layout, so no Token-2022 mint
//! extension (transfer fee, transfer hook, permanent delegate, ...) is
//! accepted on the DTF mint or on a constituent. That is also what makes the
//! plain `Transfer` instruction safe to use for both programs.

use pinocchio::cpi::{invoke_signed, Signer};
use pinocchio::instruction::{InstructionAccount, InstructionView};
use pinocchio::{AccountView, Address, ProgramResult};

use crate::constants::{SPL_TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID};
use crate::error::AxisCoreError;
use crate::state::TokenProgramKind;

pub const MINT_LEN: usize = 82;
pub const TOKEN_ACCOUNT_LEN: usize = 165;

const IX_TRANSFER: u8 = 3;
const IX_MINT_TO: u8 = 7;
const IX_BURN: u8 = 8;

/// Token-2022 `AccountType::Account`, stored right after the base layout.
const ACCOUNT_TYPE_ACCOUNT: u8 = 2;
/// A Token-2022 account that carries exactly the `ImmutableOwner` extension
/// (type 7, length 0), which is what the associated token program creates.
const IMMUTABLE_OWNER_ACCOUNT_LEN: usize = 170;
const IMMUTABLE_OWNER_TLV: [u8; 4] = [7, 0, 0, 0];

pub fn token_program_id(kind: TokenProgramKind) -> &'static Address {
    match kind {
        TokenProgramKind::LegacySplToken => &SPL_TOKEN_PROGRAM_ID,
        TokenProgramKind::Token2022 => &TOKEN_2022_PROGRAM_ID,
    }
}

/// Which token program owns `account`, if either does.
pub fn token_program_kind_of(account: &AccountView) -> Result<TokenProgramKind, AxisCoreError> {
    if account.owned_by(&SPL_TOKEN_PROGRAM_ID) {
        Ok(TokenProgramKind::LegacySplToken)
    } else if account.owned_by(&TOKEN_2022_PROGRAM_ID) {
        Ok(TokenProgramKind::Token2022)
    } else {
        Err(AxisCoreError::InvalidTokenProgram)
    }
}

/// Checks that `program` is the token program account for `kind`.
pub fn expect_token_program(
    program: &AccountView,
    kind: TokenProgramKind,
) -> Result<(), AxisCoreError> {
    if program.address() != token_program_id(kind) {
        return Err(AxisCoreError::InvalidTokenProgram);
    }
    Ok(())
}

fn coption_address(src: &[u8]) -> Result<Option<Address>, AxisCoreError> {
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&src[4..36]);
    match src[0..4] {
        [0, 0, 0, 0] => Ok(None),
        [1, 0, 0, 0] => Ok(Some(Address::from(bytes))),
        _ => Err(AxisCoreError::InvalidAccountData),
    }
}

fn le_u64(src: &[u8]) -> u64 {
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&src[0..8]);
    u64::from_le_bytes(bytes)
}

pub struct MintState {
    pub kind: TokenProgramKind,
    pub mint_authority: Option<Address>,
    pub supply: u64,
    pub freeze_authority: Option<Address>,
}

/// Reads an initialized mint with no extensions, owned by either token
/// program.
pub fn read_mint(mint: &AccountView) -> Result<MintState, AxisCoreError> {
    let kind = token_program_kind_of(mint)?;
    let data = mint.try_borrow().map_err(|_| AxisCoreError::InvalidMint)?;
    if data.len() < MINT_LEN {
        return Err(AxisCoreError::InvalidMint);
    }
    if data.len() != MINT_LEN {
        return Err(AxisCoreError::UnsupportedTokenExtension);
    }
    if data[45] != 1 {
        return Err(AxisCoreError::InvalidMint);
    }
    Ok(MintState {
        kind,
        mint_authority: coption_address(&data[0..36]).map_err(|_| AxisCoreError::InvalidMint)?,
        supply: le_u64(&data[36..44]),
        freeze_authority: coption_address(&data[46..82]).map_err(|_| AxisCoreError::InvalidMint)?,
    })
}

/// Live supply of a mint whose identity and layout were checked at
/// `CreateMarket`.
pub fn mint_supply(mint: &AccountView) -> Result<u64, AxisCoreError> {
    let data = mint.try_borrow().map_err(|_| AxisCoreError::InvalidMint)?;
    if data.len() < MINT_LEN {
        return Err(AxisCoreError::InvalidMint);
    }
    Ok(le_u64(&data[36..44]))
}

pub struct TokenAccountState {
    pub mint: Address,
    pub owner: Address,
    pub amount: u64,
    pub has_delegate: bool,
    pub has_close_authority: bool,
}

fn expect_token_account_layout(
    data: &[u8],
    kind: TokenProgramKind,
    strict_extensions: bool,
) -> Result<(), AxisCoreError> {
    let ok = match (kind, data.len()) {
        (_, TOKEN_ACCOUNT_LEN) => true,
        (TokenProgramKind::LegacySplToken, _) => false,
        (TokenProgramKind::Token2022, len) if len > TOKEN_ACCOUNT_LEN => {
            if data[TOKEN_ACCOUNT_LEN] != ACCOUNT_TYPE_ACCOUNT {
                false
            } else if strict_extensions {
                if len != IMMUTABLE_OWNER_ACCOUNT_LEN || data[166..170] != IMMUTABLE_OWNER_TLV {
                    return Err(AxisCoreError::UnsupportedTokenExtension);
                }
                true
            } else {
                true
            }
        }
        _ => false,
    };
    if !ok {
        return Err(AxisCoreError::InvalidTokenAccount);
    }
    // 1 = Initialized. Uninitialized and Frozen accounts are both refused.
    if data[108] != 1 {
        return Err(AxisCoreError::InvalidTokenAccount);
    }
    Ok(())
}

/// Reads an initialized, unfrozen token account owned by the token program
/// for `kind`. With `strict_extensions`, a Token-2022 account may carry
/// nothing beyond `ImmutableOwner`; custody accounts are read this way.
pub fn read_token_account(
    account: &AccountView,
    kind: TokenProgramKind,
    strict_extensions: bool,
) -> Result<TokenAccountState, AxisCoreError> {
    if !account.owned_by(token_program_id(kind)) {
        return Err(AxisCoreError::InvalidTokenAccount);
    }
    let data = account
        .try_borrow()
        .map_err(|_| AxisCoreError::InvalidTokenAccount)?;
    if data.len() < TOKEN_ACCOUNT_LEN {
        return Err(AxisCoreError::InvalidTokenAccount);
    }
    expect_token_account_layout(&data, kind, strict_extensions)?;
    let mut mint = [0u8; 32];
    mint.copy_from_slice(&data[0..32]);
    let mut owner = [0u8; 32];
    owner.copy_from_slice(&data[32..64]);
    Ok(TokenAccountState {
        mint: Address::from(mint),
        owner: Address::from(owner),
        amount: le_u64(&data[64..72]),
        has_delegate: data[72..76] != [0, 0, 0, 0],
        has_close_authority: data[129..133] != [0, 0, 0, 0],
    })
}

/// A custody account: the right mint, controlled by `authority` alone. No
/// delegate could move its balance and no close authority could remove it.
pub fn expect_custody_account(
    account: &AccountView,
    kind: TokenProgramKind,
    mint: &Address,
    authority: &Address,
) -> Result<(), AxisCoreError> {
    let state = read_token_account(account, kind, true)?;
    if &state.mint != mint {
        return Err(AxisCoreError::TokenAccountMintMismatch);
    }
    if &state.owner != authority {
        return Err(AxisCoreError::TokenAccountOwnerMismatch);
    }
    if state.has_delegate {
        return Err(AxisCoreError::TokenAccountHasDelegate);
    }
    if state.has_close_authority {
        return Err(AxisCoreError::TokenAccountHasCloseAuthority);
    }
    Ok(())
}

/// Live balance of a token account whose identity is established elsewhere:
/// a reserve vault matched by address, or an account already read in full.
pub fn token_balance(account: &AccountView) -> Result<u64, AxisCoreError> {
    let data = account
        .try_borrow()
        .map_err(|_| AxisCoreError::InvalidTokenAccount)?;
    if data.len() < TOKEN_ACCOUNT_LEN {
        return Err(AxisCoreError::InvalidTokenAccount);
    }
    Ok(le_u64(&data[64..72]))
}

fn amount_instruction(tag: u8, amount: u64) -> [u8; 9] {
    let mut data = [0u8; 9];
    data[0] = tag;
    data[1..9].copy_from_slice(&amount.to_le_bytes());
    data
}

/// `Transfer`. Pass `signers` when `authority` is the market PDA; a user
/// authority signs the transaction and needs none.
pub fn transfer(
    token_program: &AccountView,
    from: &AccountView,
    to: &AccountView,
    authority: &AccountView,
    amount: u64,
    signers: &[Signer],
) -> ProgramResult {
    let data = amount_instruction(IX_TRANSFER, amount);
    invoke_signed(
        &InstructionView {
            program_id: token_program.address(),
            accounts: &[
                InstructionAccount::writable(from.address()),
                InstructionAccount::writable(to.address()),
                InstructionAccount::readonly_signer(authority.address()),
            ],
            data: &data,
        },
        &[from, to, authority],
        signers,
    )
}

pub fn mint_to(
    token_program: &AccountView,
    mint: &AccountView,
    to: &AccountView,
    authority: &AccountView,
    amount: u64,
    signers: &[Signer],
) -> ProgramResult {
    let data = amount_instruction(IX_MINT_TO, amount);
    invoke_signed(
        &InstructionView {
            program_id: token_program.address(),
            accounts: &[
                InstructionAccount::writable(mint.address()),
                InstructionAccount::writable(to.address()),
                InstructionAccount::readonly_signer(authority.address()),
            ],
            data: &data,
        },
        &[mint, to, authority],
        signers,
    )
}

/// `Burn` from `from`, authorized by its owner, who signs the transaction.
pub fn burn(
    token_program: &AccountView,
    from: &AccountView,
    mint: &AccountView,
    authority: &AccountView,
    amount: u64,
) -> ProgramResult {
    let data = amount_instruction(IX_BURN, amount);
    invoke_signed(
        &InstructionView {
            program_id: token_program.address(),
            accounts: &[
                InstructionAccount::writable(from.address()),
                InstructionAccount::writable(mint.address()),
                InstructionAccount::readonly_signer(authority.address()),
            ],
            data: &data,
        },
        &[from, mint, authority],
        &[],
    )
}
