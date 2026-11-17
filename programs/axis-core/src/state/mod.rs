pub mod market;
pub mod protocol_config;

pub use market::{DTFMarket, MarketAsset, TokenProgramKind};
pub use protocol_config::ProtocolConfig;

use crate::error::AxisCoreError;
use pinocchio::Address;

pub(crate) fn read_address(src: &[u8]) -> Address {
    let mut bytes = [0u8; 32];
    bytes.copy_from_slice(&src[0..32]);
    Address::from(bytes)
}

pub(crate) fn read_u64(src: &[u8]) -> Result<u64, AxisCoreError> {
    let bytes: [u8; 8] = src
        .get(0..8)
        .ok_or(AxisCoreError::InvalidAccountData)?
        .try_into()
        .map_err(|_| AxisCoreError::InvalidAccountData)?;
    Ok(u64::from_le_bytes(bytes))
}
