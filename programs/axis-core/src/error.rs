use pinocchio::error::ProgramError;

#[repr(u32)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AxisCoreError {
    ScaffoldOnly = 1,
    InvalidFeeRate = 2,
    InvalidFeeShare = 3,
    InvalidMarketAssetCount = 4,
    InvalidMarketWeight = 5,
    InvalidMarketWeightTotal = 6,
    DuplicateMarketAsset = 7,
    InvalidAssetExecutionPolicy = 8,
    FeeVaultMatchesReserveVault = 9,
}

impl From<AxisCoreError> for ProgramError {
    fn from(error: AxisCoreError) -> Self {
        ProgramError::Custom(error as u32)
    }
}
