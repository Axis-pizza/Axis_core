use pinocchio::error::ProgramError;

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AxisCoreError {
    InvalidInstruction = 1,
    MissingAccount = 2,
    AccountNotSigner = 3,
    AccountNotWritable = 4,
    /// Account is not owned by Axis Core.
    InvalidAccountOwner = 5,
    InvalidAccountData = 6,
    InvalidDiscriminator = 7,
    AccountAlreadyInitialized = 8,
    /// Address does not match the expected program-derived address.
    InvalidPda = 9,
    /// Signer is not the governance authority recorded in `ProtocolConfig`.
    UnauthorizedProtocolAuthority = 10,
    UnauthorizedUser = 11,
    MathOverflow = 12,
    /// A program or loader account is not the one the instruction requires.
    InvalidProgramAccount = 13,
    /// Signer is neither the emergency operator nor the governance authority.
    UnauthorizedPauseAuthority = 14,
    /// The zero address, or Axis Core itself, cannot hold this role.
    InvalidAuthority = 15,

    // Market composition
    TooFewAssets = 20,
    TooManyAssets = 21,
    DuplicateAsset = 22,
    UnknownAsset = 25,
    /// Reference supply, a reference amount or the locked minimum is zero.
    InvalidReferenceBasket = 26,
    /// The DTF mint or the USDC mint cannot be a constituent.
    ForbiddenAsset = 27,

    // Lifecycle
    /// Total supply is zero, so no pro-rata ratio exists yet. The first
    /// purchase goes through `InitialPurchase`.
    MarketNotSeeded = 32,
    /// The first purchase did not issue more than the locked minimum
    /// liquidity plus the fee.
    SupplyBelowMinimumLiquidity = 33,
    PurchasePaused = 34,
    UsdcRedemptionPaused = 35,
    /// No swap program is approved for this market yet.
    SwapProgramNotConfigured = 36,
    /// Pause and resume flags must name at least one known operation.
    InvalidPauseFlags = 37,

    // Value
    ZeroAmount = 40,
    /// The buyer or redeemer would receive less than the signed minimum.
    SlippageExceeded = 43,
    /// A reserve vault lost balance on a path that may only credit it.
    ReserveDebitedOnMint = 44,
    /// The route spent more USDC than the buyer signed for.
    MaxInputExceeded = 45,
    /// A reserve holds nothing while supply is positive.
    EmptyReserve = 46,
    /// DTF supply moved while the route was running.
    SupplyChangedDuringRoute = 47,
    /// A reserve vault lost more than the redemption released.
    ReserveOverdrawn = 48,

    // Token account and mint admission
    /// Not the SPL Token or Token-2022 program the instruction requires.
    InvalidTokenProgram = 50,
    InvalidMint = 51,
    /// Token-2022 extensions are not admitted until the asset policy is set.
    UnsupportedTokenExtension = 52,
    InvalidMintAuthority = 53,
    MintHasFreezeAuthority = 54,
    MintSupplyNotZero = 55,
    InvalidTokenAccount = 56,
    TokenAccountMintMismatch = 57,
    TokenAccountOwnerMismatch = 58,
    TokenAccountHasDelegate = 59,
    TokenAccountHasCloseAuthority = 60,
    /// Account passed is not the one recorded in the market.
    AccountMismatch = 61,

    // Route
    InvalidRouteData = 70,
    TooManyRouteAccounts = 71,
    TooManyRouteLegs = 72,
}

impl From<AxisCoreError> for ProgramError {
    fn from(error: AxisCoreError) -> Self {
        ProgramError::Custom(error as u32)
    }
}
