pub mod admin;
pub mod common;
pub mod create_market;
pub mod initialize_protocol_config;
pub mod purchase;
pub mod redeem;
pub mod route;
pub mod token;

pub use admin::{
    process_pause, process_resume, process_set_swap_program, process_update_authorities,
};
pub use common::*;
pub use create_market::process_create_market;
pub use initialize_protocol_config::process_initialize_protocol_config;
pub use purchase::process_purchase;
pub use redeem::{process_redeem_in_kind, process_redeem_usdc};

use crate::error::AxisCoreError;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AxisInstruction {
    InitializeProtocolConfig = 0,
    CreateMarket = 1,
    InitialPurchase = 2,
    Purchase = 3,
    RedeemUsdc = 4,
    RedeemInKind = 5,
    Pause = 6,
    Resume = 7,
    UpdateAuthorities = 8,
    SetSwapProgram = 9,
}

impl TryFrom<u8> for AxisInstruction {
    type Error = AxisCoreError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::InitializeProtocolConfig),
            1 => Ok(Self::CreateMarket),
            2 => Ok(Self::InitialPurchase),
            3 => Ok(Self::Purchase),
            4 => Ok(Self::RedeemUsdc),
            5 => Ok(Self::RedeemInKind),
            6 => Ok(Self::Pause),
            7 => Ok(Self::Resume),
            8 => Ok(Self::UpdateAuthorities),
            9 => Ok(Self::SetSwapProgram),
            _ => Err(AxisCoreError::InvalidInstruction),
        }
    }
}
