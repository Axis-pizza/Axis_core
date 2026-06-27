use pinocchio::Address;

use crate::{constants::BPS_DENOMINATOR, error::AxisCoreError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProtocolFeeConfig {
    pub mint_fee_bps: u16,
    pub redeem_fee_bps: u16,
    pub max_mint_fee_bps: u16,
    pub max_redeem_fee_bps: u16,
    pub creator_share_bps: u16,
    pub protocol_share_bps: u16,
}

impl ProtocolFeeConfig {
    pub fn validate(&self) -> Result<(), AxisCoreError> {
        if self.mint_fee_bps > self.max_mint_fee_bps
            || self.redeem_fee_bps > self.max_redeem_fee_bps
            || self.max_mint_fee_bps > BPS_DENOMINATOR
            || self.max_redeem_fee_bps > BPS_DENOMINATOR
        {
            return Err(AxisCoreError::InvalidFeeRate);
        }

        if self.creator_share_bps.checked_add(self.protocol_share_bps) != Some(BPS_DENOMINATOR) {
            return Err(AxisCoreError::InvalidFeeShare);
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolConfig {
    pub authority: Address,
    pub pause_authority: Address,
    pub asset_registry_authority: Address,
    pub route_registry_authority: Address,
    pub pricing_registry_authority: Address,
    pub protocol_treasury: Address,
    pub usdc_mint: Address,
    pub fee_config: ProtocolFeeConfig,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fee_config_rejects_invalid_caps_and_shares() {
        let mut config = ProtocolFeeConfig {
            mint_fee_bps: 300,
            redeem_fee_bps: 0,
            max_mint_fee_bps: 300,
            max_redeem_fee_bps: 0,
            creator_share_bps: 8_000,
            protocol_share_bps: 2_000,
        };
        assert_eq!(config.validate(), Ok(()));

        config.mint_fee_bps = 301;
        assert_eq!(config.validate(), Err(AxisCoreError::InvalidFeeRate));

        config.mint_fee_bps = 300;
        config.protocol_share_bps = 1_999;
        assert_eq!(config.validate(), Err(AxisCoreError::InvalidFeeShare));
    }
}
