use crate::constants::{MAX_ASSETS, MIN_ASSETS};
use crate::error::AxisCoreError;
use crate::state::{read_address, read_u64};
use pinocchio::Address;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenProgramKind {
    LegacySplToken = 0,
    Token2022 = 1,
}

impl TryFrom<u8> for TokenProgramKind {
    type Error = AxisCoreError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::LegacySplToken),
            1 => Ok(Self::Token2022),
            _ => Err(AxisCoreError::InvalidAccountData),
        }
    }
}

/// One market constituent. Stored inline in `DTFMarket`, never as its own
/// account: the 64 account-lock budget for an atomic purchase cannot afford
/// one account per asset.
///
/// 73 bytes: asset_mint | reserve_vault | reference_amount | token_program
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketAsset {
    pub asset_mint: Address,
    pub reserve_vault: Address,
    /// Quantity of this asset that backs `DTFMarket::reference_supply` in the
    /// approved initial allocation. Only the first purchase reads it.
    pub reference_amount: u64,
    pub token_program: TokenProgramKind,
}

impl MarketAsset {
    pub const LEN: usize = 73;
}

/// DTFMarket. PDA seeds: `["market", dtf_mint]` at the canonical bump, so a
/// DTF mint has at most one market. The PDA is the reserve authority and the
/// DTF mint authority. Only governance can create one, and creating it is the
/// approval for public availability.
///
/// Layout (408 bytes):
/// [0..8]     discriminator b"dtfmkt05"
/// [8..40]    dtf_mint
/// [40..72]   treasury          snapshot of ProtocolConfig::protocol_treasury, immutable
/// [72..104]  usdc_mint         snapshot of ProtocolConfig::usdc_mint, immutable
/// [104..136] swap_program      the only program a route leg may invoke; zero = none
/// [136..168] liquidity_lock    market-owned DTF account holding the locked minimum
/// [168..176] reference_supply  DTF the reference basket backs (first purchase only)
/// [176..184] min_liquidity     DTF locked for good at the first purchase
/// [184]      asset_count
/// [185]      bump
/// [186]      dtf_token_program
/// [187]      purchase_paused
/// [188]      usdc_redemption_paused
/// [189..408] assets, MAX_ASSETS * MarketAsset::LEN
///
/// Supply and reserve balances are not stored: every instruction reads them
/// live from the DTF mint and the reserve vaults. Purchase and redemption
/// therefore never write this account, and it is only locked read-only.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DTFMarket {
    pub dtf_mint: Address,
    pub treasury: Address,
    pub usdc_mint: Address,
    pub swap_program: Address,
    pub liquidity_lock: Address,
    pub reference_supply: u64,
    pub min_liquidity: u64,
    pub asset_count: u8,
    pub bump: u8,
    pub dtf_token_program: TokenProgramKind,
    pub purchase_paused: bool,
    pub usdc_redemption_paused: bool,
    pub assets: [Option<MarketAsset>; MAX_ASSETS],
}

const ASSETS_OFFSET: usize = 189;

fn read_flag(byte: u8) -> Result<bool, AxisCoreError> {
    match byte {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(AxisCoreError::InvalidAccountData),
    }
}

impl DTFMarket {
    pub const DISCRIMINATOR: [u8; 8] = *b"dtfmkt05";
    pub const LEN: usize = ASSETS_OFFSET + MAX_ASSETS * MarketAsset::LEN;

    /// Live constituents, in index order.
    pub fn assets(&self) -> impl Iterator<Item = &MarketAsset> {
        self.assets[..self.asset_count as usize]
            .iter()
            .filter_map(|a| a.as_ref())
    }

    pub fn asset(&self, index: usize) -> Result<&MarketAsset, AxisCoreError> {
        if index >= self.asset_count as usize {
            return Err(AxisCoreError::UnknownAsset);
        }
        self.assets[index]
            .as_ref()
            .ok_or(AxisCoreError::UnknownAsset)
    }

    /// Composition rules that must hold at creation and can never be repaired
    /// afterwards, since the table is immutable.
    pub fn validate_composition(&self) -> Result<(), AxisCoreError> {
        let count = self.asset_count as usize;
        if count < MIN_ASSETS {
            return Err(AxisCoreError::TooFewAssets);
        }
        if count > MAX_ASSETS {
            return Err(AxisCoreError::TooManyAssets);
        }
        // A zero reference would divide by zero or issue nothing; a zero
        // locked minimum would bring back the first-depositor rounding attack.
        if self.reference_supply == 0 || self.min_liquidity == 0 {
            return Err(AxisCoreError::InvalidReferenceBasket);
        }

        for i in 0..count {
            let a = self.assets[i].as_ref().ok_or(AxisCoreError::UnknownAsset)?;
            if a.reference_amount == 0 {
                return Err(AxisCoreError::InvalidReferenceBasket);
            }
            if a.asset_mint == self.dtf_mint || a.asset_mint == self.usdc_mint {
                return Err(AxisCoreError::ForbiddenAsset);
            }
            // A duplicate mint would let one vault satisfy two delivery
            // requirements, so the same tokens would back two claims.
            for j in 0..i {
                let b = self.assets[j].as_ref().ok_or(AxisCoreError::UnknownAsset)?;
                if a.asset_mint == b.asset_mint || a.reserve_vault == b.reserve_vault {
                    return Err(AxisCoreError::DuplicateAsset);
                }
            }
        }
        Ok(())
    }

    pub fn pack(&self, dst: &mut [u8]) -> Result<(), AxisCoreError> {
        if dst.len() < Self::LEN {
            return Err(AxisCoreError::InvalidAccountData);
        }
        dst[0..8].copy_from_slice(&Self::DISCRIMINATOR);
        dst[8..40].copy_from_slice(self.dtf_mint.as_ref());
        dst[40..72].copy_from_slice(self.treasury.as_ref());
        dst[72..104].copy_from_slice(self.usdc_mint.as_ref());
        dst[104..136].copy_from_slice(self.swap_program.as_ref());
        dst[136..168].copy_from_slice(self.liquidity_lock.as_ref());
        dst[168..176].copy_from_slice(&self.reference_supply.to_le_bytes());
        dst[176..184].copy_from_slice(&self.min_liquidity.to_le_bytes());
        dst[184] = self.asset_count;
        dst[185] = self.bump;
        dst[186] = self.dtf_token_program as u8;
        dst[187] = self.purchase_paused as u8;
        dst[188] = self.usdc_redemption_paused as u8;

        for (i, slot) in self.assets.iter().enumerate() {
            let base = ASSETS_OFFSET + i * MarketAsset::LEN;
            let entry = &mut dst[base..base + MarketAsset::LEN];
            match slot {
                Some(a) => {
                    entry[0..32].copy_from_slice(a.asset_mint.as_ref());
                    entry[32..64].copy_from_slice(a.reserve_vault.as_ref());
                    entry[64..72].copy_from_slice(&a.reference_amount.to_le_bytes());
                    entry[72] = a.token_program as u8;
                }
                None => entry.fill(0),
            }
        }
        Ok(())
    }

    pub fn unpack(src: &[u8]) -> Result<Self, AxisCoreError> {
        if src.len() < Self::LEN {
            return Err(AxisCoreError::InvalidAccountData);
        }
        if src[0..8] != Self::DISCRIMINATOR {
            return Err(AxisCoreError::InvalidDiscriminator);
        }
        let asset_count = src[184];
        if asset_count as usize > MAX_ASSETS {
            return Err(AxisCoreError::TooManyAssets);
        }

        let mut assets: [Option<MarketAsset>; MAX_ASSETS] = [const { None }; MAX_ASSETS];
        for (i, slot) in assets.iter_mut().enumerate() {
            if i >= asset_count as usize {
                break;
            }
            let base = ASSETS_OFFSET + i * MarketAsset::LEN;
            let entry = &src[base..base + MarketAsset::LEN];
            *slot = Some(MarketAsset {
                asset_mint: read_address(&entry[0..32]),
                reserve_vault: read_address(&entry[32..64]),
                reference_amount: read_u64(&entry[64..72])?,
                token_program: TokenProgramKind::try_from(entry[72])?,
            });
        }

        Ok(Self {
            dtf_mint: read_address(&src[8..40]),
            treasury: read_address(&src[40..72]),
            usdc_mint: read_address(&src[72..104]),
            swap_program: read_address(&src[104..136]),
            liquidity_lock: read_address(&src[136..168]),
            reference_supply: read_u64(&src[168..176])?,
            min_liquidity: read_u64(&src[176..184])?,
            asset_count,
            bump: src[185],
            dtf_token_program: TokenProgramKind::try_from(src[186])?,
            purchase_paused: read_flag(src[187])?,
            usdc_redemption_paused: read_flag(src[188])?,
            assets,
        })
    }
}
