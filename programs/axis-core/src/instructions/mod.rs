pub mod common;
pub mod create_market;
pub mod initialize_protocol_config;

pub use common::*;
pub use create_market::process_create_market;
pub use initialize_protocol_config::process_initialize_protocol_config;

use crate::error::AxisCoreError;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AxisInstruction {
    InitializeProtocolConfig = 0,
    CreateMarket = 1,
}

impl TryFrom<u8> for AxisInstruction {
    type Error = AxisCoreError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::InitializeProtocolConfig),
            1 => Ok(Self::CreateMarket),
            _ => Err(AxisCoreError::InvalidInstruction),
        }
    }
}
