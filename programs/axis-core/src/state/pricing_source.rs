use pinocchio::Address;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PricingSourceType {
    ExternalOracle,
    DexTwap,
    DexSpot,
    StablePeg,
    LstExchangeRate,
    StockTokenOracle,
}

// TODO(account-model): Define the P0 subset and source-specific identity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PricingSource {
    pub asset_mint: Address,
    pub source_type: PricingSourceType,
    pub max_staleness_slots: u64,
    pub max_deviation_bps: u16,
    pub enabled: bool,
}
