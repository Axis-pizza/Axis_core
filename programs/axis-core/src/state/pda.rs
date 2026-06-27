use pinocchio::Address;

use crate::constants::{
    ASSET_CONFIG_SEED_PREFIX, DTF_MINT_SEED_PREFIX, FEE_VAULT_SEED_PREFIX, MARKET_SEED_PREFIX,
    PRICING_SOURCE_SEED_PREFIX, PROTOCOL_CONFIG_SEED_PREFIX, RESERVE_VAULT_SEED_PREFIX,
};

pub fn protocol_config_seed_components() -> [&'static [u8]; 1] {
    [PROTOCOL_CONFIG_SEED_PREFIX]
}

// TODO(account-model): Finalize market_nonce seed encoding.
pub fn market_seed_components<'a>(creator: &'a Address, market_nonce: &'a [u8]) -> [&'a [u8]; 3] {
    [MARKET_SEED_PREFIX, creator.as_ref(), market_nonce]
}

pub fn dtf_mint_seed_components(market: &Address) -> [&[u8]; 2] {
    [DTF_MINT_SEED_PREFIX, market.as_ref()]
}

pub fn reserve_vault_seed_components<'a>(
    market: &'a Address,
    asset_mint: &'a Address,
) -> [&'a [u8]; 3] {
    [
        RESERVE_VAULT_SEED_PREFIX,
        market.as_ref(),
        asset_mint.as_ref(),
    ]
}

pub fn fee_vault_seed_components(market: &Address) -> [&[u8]; 2] {
    [FEE_VAULT_SEED_PREFIX, market.as_ref()]
}

pub fn asset_config_seed_components(asset_mint: &Address) -> [&[u8]; 2] {
    [ASSET_CONFIG_SEED_PREFIX, asset_mint.as_ref()]
}

pub fn pricing_source_seed_components(asset_mint: &Address) -> [&[u8]; 2] {
    [PRICING_SOURCE_SEED_PREFIX, asset_mint.as_ref()]
}

pub fn validate_distinct_custody_accounts(
    reserve_vault: &Address,
    fee_vault: &Address,
) -> Result<(), crate::error::AxisCoreError> {
    if reserve_vault == fee_vault {
        return Err(crate::error::AxisCoreError::FeeVaultMatchesReserveVault);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custody_seed_namespaces_are_distinct() {
        let market = Address::new_from_array([1; 32]);
        let asset_mint = Address::new_from_array([2; 32]);

        assert_eq!(
            reserve_vault_seed_components(&market, &asset_mint)[0],
            RESERVE_VAULT_SEED_PREFIX
        );
        assert_eq!(fee_vault_seed_components(&market)[0], FEE_VAULT_SEED_PREFIX);
        assert_ne!(RESERVE_VAULT_SEED_PREFIX, FEE_VAULT_SEED_PREFIX);
    }
}
