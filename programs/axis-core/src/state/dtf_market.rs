use pinocchio::Address;

use crate::{
    constants::{BPS_DENOMINATOR, MAX_MARKET_ASSETS, MIN_ASSET_WEIGHT_BPS, MIN_MARKET_ASSETS},
    error::AxisCoreError,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarketStatus {
    Created,
    Active,
    Paused,
    Deprecated,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketAsset {
    pub asset_mint: Address,
    /// Advisory only; reserve balances remain the accounting truth.
    pub target_weight_bps: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MarketFeeConfig {
    pub mint_fee_bps: u16,
    pub redeem_fee_bps: u16,
    pub creator_share_bps: u16,
    pub protocol_share_bps: u16,
}

// TODO(account-model): Serialization and exact layout remain unfrozen.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DTFMarket {
    pub creator: Address,
    /// Immutable in v1.
    pub creator_fee_destination: Address,
    pub dtf_mint: Address,
    pub market_nonce: u64,
    pub status: MarketStatus,
    pub asset_count: u8,
    pub assets: [MarketAsset; MAX_MARKET_ASSETS],
    pub fee_config: MarketFeeConfig,
    pub accrued_creator_fee_usdc: u64,
    pub accrued_protocol_fee_usdc: u64,
}

impl DTFMarket {
    pub fn validate_asset_shape(&self) -> Result<(), AxisCoreError> {
        let asset_count = usize::from(self.asset_count);
        if !(MIN_MARKET_ASSETS..=MAX_MARKET_ASSETS).contains(&asset_count) {
            return Err(AxisCoreError::InvalidMarketAssetCount);
        }

        let mut weight_total = 0u16;
        for (index, asset) in self.assets[..asset_count].iter().enumerate() {
            if asset.target_weight_bps < MIN_ASSET_WEIGHT_BPS {
                return Err(AxisCoreError::InvalidMarketWeight);
            }

            weight_total = weight_total
                .checked_add(asset.target_weight_bps)
                .ok_or(AxisCoreError::InvalidMarketWeightTotal)?;

            if self.assets[..index]
                .iter()
                .any(|existing| existing.asset_mint == asset.asset_mint)
            {
                return Err(AxisCoreError::DuplicateMarketAsset);
            }
        }

        if weight_total != BPS_DENOMINATOR {
            return Err(AxisCoreError::InvalidMarketWeightTotal);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn market_with_weights(weights: [u16; MAX_MARKET_ASSETS], asset_count: u8) -> DTFMarket {
        DTFMarket {
            creator: Address::new_from_array([10; 32]),
            creator_fee_destination: Address::new_from_array([11; 32]),
            dtf_mint: Address::new_from_array([12; 32]),
            market_nonce: 1,
            status: MarketStatus::Created,
            asset_count,
            assets: core::array::from_fn(|index| MarketAsset {
                asset_mint: Address::new_from_array([index as u8 + 1; 32]),
                target_weight_bps: weights[index],
            }),
            fee_config: MarketFeeConfig {
                mint_fee_bps: 100,
                redeem_fee_bps: 0,
                creator_share_bps: 8_000,
                protocol_share_bps: 2_000,
            },
            accrued_creator_fee_usdc: 0,
            accrued_protocol_fee_usdc: 0,
        }
    }

    #[test]
    fn inline_asset_shape_enforces_count_and_weight_total() {
        let market = market_with_weights([5_000, 5_000, 0, 0, 0], 2);
        assert_eq!(market.validate_asset_shape(), Ok(()));

        let invalid_count = market_with_weights([10_000, 0, 0, 0, 0], 1);
        assert_eq!(
            invalid_count.validate_asset_shape(),
            Err(AxisCoreError::InvalidMarketAssetCount)
        );

        let invalid_total = market_with_weights([4_000, 5_000, 0, 0, 0], 2);
        assert_eq!(
            invalid_total.validate_asset_shape(),
            Err(AxisCoreError::InvalidMarketWeightTotal)
        );
    }
}
