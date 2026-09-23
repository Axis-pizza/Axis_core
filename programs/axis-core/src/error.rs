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
    UnauthorizedProtocolAuthority = 10,
    UnauthorizedUser = 11,
    MathOverflow = 12,
    /// A program or loader account is not the one the instruction requires.
    InvalidProgramAccount = 13,

    // Market composition
    TooFewAssets = 20,
    TooManyAssets = 21,
    DuplicateAsset = 22,
    InvalidWeightSum = 23,
    WeightBelowMinimum = 24,
    UnknownAsset = 25,

    // Lifecycle
    MarketNotCreated = 30,
    MarketNotActive = 31,
    /// Total supply is zero, so no pro-rata ratio exists yet.
    MarketNotSeeded = 32,
    SupplyBelowMinimumLiquidity = 33,

    // Value
    ZeroAmount = 40,
    /// A swap leg delivered less than the mint required for that asset.
    InsufficientDelivery = 42,
    SlippageExceeded = 43,
    /// A reserve vault lost balance on a path that may only credit it.
    ReserveDebitedOnMint = 44,
}

impl From<AxisCoreError> for ProgramError {
    fn from(error: AxisCoreError) -> Self {
        ProgramError::Custom(error as u32)
    }
}
