#![forbid(unsafe_code)]

//! Shared test utility scaffold for Axis Core.
//!
//! This crate provides deterministic LiteSVM setup and inspection helpers for
//! protocol tests. It intentionally does not implement protocol business logic.

pub mod accounts;
pub mod fixture;
pub mod litesvm;
pub mod tokens;

use std::path::PathBuf;

use solana_address::Address;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AxisTestError>;

#[derive(Debug, Error)]
pub enum AxisTestError {
    #[error("LiteSVM account {address:?} was not found")]
    AccountNotFound { address: Address },

    #[error("{diagnostic}")]
    MissingProgramArtifact { path: PathBuf, diagnostic: String },

    #[error("{diagnostic}")]
    InvalidProgramArtifact { path: PathBuf, diagnostic: String },

    #[error("failed to fund deterministic signer {address:?}: {details}")]
    FundingFailed { address: Address, details: String },

    #[error("invalid SPL Token / Token-2022 account data for {address:?}: {reason}")]
    InvalidTokenAccountData { address: Address, reason: String },

    #[error("invalid SPL Token / Token-2022 mint data for {address:?}: {reason}")]
    InvalidTokenMintData { address: Address, reason: String },

    #[error("failed to build SPL Token / Token-2022 instruction: {details}")]
    TokenInstructionBuildFailed { details: String },

    #[error("SPL Token / Token-2022 transaction failed: {details}")]
    TokenTransactionFailed { details: String },

    #[error(
        "SPL Token / Token-2022 amount mismatch for {label}: expected {expected}, got {actual}"
    )]
    TokenAmountMismatch {
        label: String,
        expected: u64,
        actual: u64,
    },
}

pub use accounts::{account_data, account_lamports};
pub use litesvm::{
    axis_core_program_artifact_path, axis_core_program_id, deterministic_keypair, fresh_vm,
    load_axis_core_program, load_axis_core_program_from_path, provision_deterministic_payer,
    provision_funded_signer, workspace_root, FundedTestSigner, ProgramArtifactLoad,
    AXIS_CORE_PROGRAM_ARTIFACT_ENV, DEFAULT_AXIS_CORE_PROGRAM_ARTIFACT, DEFAULT_PAYER_LAMPORTS,
};
pub use tokens::{
    assert_token_2022_amounts, burn_candidate_dtf_tokens, burn_candidate_dtf_tokens_with_program,
    create_candidate_token_2022_dtf_fixture, legacy_token_program_id, mint_candidate_dtf_tokens,
    mint_candidate_dtf_tokens_with_program, token_2022_program_id, token_account_balance,
    token_account_balance_from_data, token_mint_supply, token_mint_supply_from_data,
    CandidateToken2022DtfFixture, CANDIDATE_DTF_DECIMALS,
};
