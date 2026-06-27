use pinocchio::Address;

use crate::{constants::BPS_DENOMINATOR, error::AxisCoreError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AssetExecutionFlags {
    pub creation_enabled: bool,
    pub mint_enabled: bool,
    pub redeem_enabled: bool,
    pub rebalance_enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AssetExecutionPolicy {
    pub flags: AssetExecutionFlags,
    pub hard_min_allocation_usdc: u64,
    pub max_trade_usdc: u64,
    pub max_weight_bps: u16,
    pub max_price_impact_bps: u16,
    pub max_pricing_deviation_bps: u16,
    pub pricing_source_required: bool,
    pub approved_route_required: bool,
}

impl AssetExecutionPolicy {
    pub fn validate(&self) -> Result<(), AxisCoreError> {
        if self.max_weight_bps > BPS_DENOMINATOR
            || self.max_price_impact_bps > BPS_DENOMINATOR
            || self.max_pricing_deviation_bps > BPS_DENOMINATOR
        {
            return Err(AxisCoreError::InvalidAssetExecutionPolicy);
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AssetConfig {
    pub asset_mint: Address,
    pub token_program: Address,
    pub decimals: u8,
    pub policy: AssetExecutionPolicy,
}
