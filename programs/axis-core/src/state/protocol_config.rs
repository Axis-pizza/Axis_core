use crate::error::AxisCoreError;
use crate::state::{read_address, read_u16};
use pinocchio::Address;

/// Singleton protocol configuration. PDA seeds: `["protocol_config"]`.
///
/// Layout (111 bytes):
/// [0..8]    discriminator b"axiscfg1"
/// [8..40]   protocol_authority
/// [40..72]  usdc_mint
/// [72..104] protocol_treasury
/// [104..106] mint_fee_bps
/// [106..108] creator_share_bps
/// [108..110] max_mint_fee_bps
/// [110]     bump
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolConfig {
    pub protocol_authority: Address,
    pub usdc_mint: Address,
    pub protocol_treasury: Address,
    pub mint_fee_bps: u16,
    pub creator_share_bps: u16,
    pub max_mint_fee_bps: u16,
    pub bump: u8,
}

impl ProtocolConfig {
    pub const DISCRIMINATOR: [u8; 8] = *b"axiscfg1";
    pub const LEN: usize = 111;

    pub fn validate(&self) -> Result<(), AxisCoreError> {
        if self.mint_fee_bps > self.max_mint_fee_bps {
            return Err(AxisCoreError::InvalidFeeConfig);
        }
        if self.creator_share_bps as u64 > crate::constants::BPS_DENOMINATOR {
            return Err(AxisCoreError::InvalidFeeConfig);
        }
        Ok(())
    }

    pub fn pack(&self, dst: &mut [u8]) -> Result<(), AxisCoreError> {
        if dst.len() < Self::LEN {
            return Err(AxisCoreError::InvalidAccountData);
        }
        dst[0..8].copy_from_slice(&Self::DISCRIMINATOR);
        dst[8..40].copy_from_slice(self.protocol_authority.as_ref());
        dst[40..72].copy_from_slice(self.usdc_mint.as_ref());
        dst[72..104].copy_from_slice(self.protocol_treasury.as_ref());
        dst[104..106].copy_from_slice(&self.mint_fee_bps.to_le_bytes());
        dst[106..108].copy_from_slice(&self.creator_share_bps.to_le_bytes());
        dst[108..110].copy_from_slice(&self.max_mint_fee_bps.to_le_bytes());
        dst[110] = self.bump;
        Ok(())
    }

    pub fn unpack(src: &[u8]) -> Result<Self, AxisCoreError> {
        if src.len() < Self::LEN {
            return Err(AxisCoreError::InvalidAccountData);
        }
        if src[0..8] != Self::DISCRIMINATOR {
            return Err(AxisCoreError::InvalidDiscriminator);
        }
        Ok(Self {
            protocol_authority: read_address(&src[8..40]),
            usdc_mint: read_address(&src[40..72]),
            protocol_treasury: read_address(&src[72..104]),
            mint_fee_bps: read_u16(&src[104..106])?,
            creator_share_bps: read_u16(&src[106..108])?,
            max_mint_fee_bps: read_u16(&src[108..110])?,
            bump: src[110],
        })
    }
}
