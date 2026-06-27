pub const MAX_MARKET_ASSETS: usize = 5;
pub const BPS_DENOMINATOR: u16 = 10_000;
pub const MIN_MARKET_ASSETS: usize = 2;
pub const MIN_ASSET_WEIGHT_BPS: u16 = 100;

pub const PROTOCOL_CONFIG_SEED_PREFIX: &[u8] = b"protocol_config";
pub const MARKET_SEED_PREFIX: &[u8] = b"market";
pub const DTF_MINT_SEED_PREFIX: &[u8] = b"dtf_mint";
pub const RESERVE_VAULT_SEED_PREFIX: &[u8] = b"reserve";
pub const FEE_VAULT_SEED_PREFIX: &[u8] = b"fee_vault";
pub const ASSET_CONFIG_SEED_PREFIX: &[u8] = b"asset";
pub const PRICING_SOURCE_SEED_PREFIX: &[u8] = b"pricing";

// TODO(account-model): ApprovedRoute PDA granularity is unresolved.
pub const APPROVED_ROUTE_SEED_PREFIX: &[u8] = b"route";
