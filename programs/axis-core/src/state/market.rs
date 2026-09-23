use crate::constants::{MAX_ASSETS, MIN_ASSETS, MIN_WEIGHT_BPS, TOTAL_WEIGHT_BPS};
use crate::error::AxisCoreError;
use crate::state::{read_address, read_u16};
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

/// Created --SeedMarket--> Active <--Pause/Unpause--> Paused
/// Active | Paused --Deprecate--> Deprecated (permanent)
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MarketStatus {
    Created = 0,
    Active = 1,
    Paused = 2,
    Deprecated = 3,
}

impl TryFrom<u8> for MarketStatus {
    type Error = AxisCoreError;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            0 => Ok(Self::Created),
            1 => Ok(Self::Active),
            2 => Ok(Self::Paused),
            3 => Ok(Self::Deprecated),
            _ => Err(AxisCoreError::InvalidAccountData),
        }
    }
}

/// One market constituent. Stored inline in `DTFMarket`, never as its own
/// account: the 64 account-lock budget for an atomic mint cannot afford one
/// account per asset. See `01` §30 V11.
///
/// 67 bytes: asset_mint | reserve_vault | weight_bps | token_program
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MarketAsset {
    pub asset_mint: Address,
    pub reserve_vault: Address,
    pub weight_bps: u16,
    pub token_program: TokenProgramKind,
}

impl MarketAsset {
    pub const LEN: usize = 67;
}

/// DTFMarket. PDA seeds: `["market", dtf_mint]` at the canonical bump, so a
/// DTF mint has at most one market. The PDA is also the reserve authority and
/// the DTF mint authority.
///
/// Layout (308 bytes):
/// [0..8]     discriminator b"dtfmkt04"
/// [8..40]    creator
/// [40..72]   dtf_mint
/// [72..104]  treasury     (snapshot of ProtocolConfig::protocol_treasury, immutable)
/// [104]      asset_count
/// [105]      status
/// [106]      bump
/// [107..308] assets, MAX_ASSETS * MarketAsset::LEN
///
/// The fee rate is the constant `FEE_BPS` and is paid out in the same
/// instruction, so no fee terms or balances are stored. The treasury is
/// snapshotted so Mint and Redeem can pay the fee without loading
/// `ProtocolConfig`, and so a later protocol change cannot redirect the fees of
/// an existing market.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DTFMarket {
    pub creator: Address,
    pub dtf_mint: Address,
    pub treasury: Address,
    pub asset_count: u8,
    pub status: MarketStatus,
    pub bump: u8,
    pub assets: [Option<MarketAsset>; MAX_ASSETS],
}

const ASSETS_OFFSET: usize = 107;

impl DTFMarket {
    pub const DISCRIMINATOR: [u8; 8] = *b"dtfmkt03";
    pub const LEN: usize = ASSETS_OFFSET + MAX_ASSETS * MarketAsset::LEN;

    /// Live constituents, in index order.
    pub fn assets(&self) -> impl Iterator<Item = &MarketAsset> {
        self.assets[..self.asset_count as usize]
            .iter()
            .filter_map(|a| a.as_ref())
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

        let mut total: u32 = 0;
        for i in 0..count {
            let a = self.assets[i].as_ref().ok_or(AxisCoreError::UnknownAsset)?;
            if a.weight_bps < MIN_WEIGHT_BPS {
                return Err(AxisCoreError::WeightBelowMinimum);
            }
            total += a.weight_bps as u32;
            // A duplicate mint would let one vault satisfy two delivery
            // requirements, so the same tokens would back two claims.
            for j in 0..i {
                let b = self.assets[j].as_ref().ok_or(AxisCoreError::UnknownAsset)?;
                if a.asset_mint == b.asset_mint || a.reserve_vault == b.reserve_vault {
                    return Err(AxisCoreError::DuplicateAsset);
                }
            }
        }
        if total != TOTAL_WEIGHT_BPS as u32 {
            return Err(AxisCoreError::InvalidWeightSum);
        }
        Ok(())
    }

    pub fn pack(&self, dst: &mut [u8]) -> Result<(), AxisCoreError> {
        if dst.len() < Self::LEN {
            return Err(AxisCoreError::InvalidAccountData);
        }
        dst[0..8].copy_from_slice(&Self::DISCRIMINATOR);
        dst[8..40].copy_from_slice(self.creator.as_ref());
        dst[40..72].copy_from_slice(self.dtf_mint.as_ref());
        dst[72..104].copy_from_slice(self.treasury.as_ref());
        dst[104] = self.asset_count;
        dst[105] = self.status as u8;
        dst[106] = self.bump;

        for (i, slot) in self.assets.iter().enumerate() {
            let base = ASSETS_OFFSET + i * MarketAsset::LEN;
            let entry = &mut dst[base..base + MarketAsset::LEN];
            match slot {
                Some(a) => {
                    entry[0..32].copy_from_slice(a.asset_mint.as_ref());
                    entry[32..64].copy_from_slice(a.reserve_vault.as_ref());
                    entry[64..66].copy_from_slice(&a.weight_bps.to_le_bytes());
                    entry[66] = a.token_program as u8;
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
        let asset_count = src[104];
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
                weight_bps: read_u16(&entry[64..66])?,
                token_program: TokenProgramKind::try_from(entry[66])?,
            });
        }

        Ok(Self {
            creator: read_address(&src[8..40]),
            dtf_mint: read_address(&src[40..72]),
            treasury: read_address(&src[72..104]),
            asset_count,
            status: MarketStatus::try_from(src[105])?,
            bump: src[106],
            assets,
        })
    }
}
