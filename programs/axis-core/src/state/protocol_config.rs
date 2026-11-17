use crate::error::AxisCoreError;
use crate::state::read_address;
use pinocchio::Address;

/// Singleton protocol configuration. PDA seeds: `["protocol_config"]`, at the
/// canonical bump, so exactly one address can ever hold it.
///
/// Layout (137 bytes):
/// [0..8]     discriminator b"axiscfg3"
/// [8..40]    protocol_authority   governance (Squads): creates markets,
///                                 resumes, changes both roles
/// [40..72]   usdc_mint
/// [72..104]  protocol_treasury    owner of the accounts that receive the fee
/// [104..136] emergency_operator   may pause, and nothing else
/// [136]      bump
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolConfig {
    pub protocol_authority: Address,
    pub usdc_mint: Address,
    pub protocol_treasury: Address,
    pub emergency_operator: Address,
    pub bump: u8,
}

impl ProtocolConfig {
    pub const DISCRIMINATOR: [u8; 8] = *b"axiscfg3";
    pub const LEN: usize = 137;

    pub fn pack(&self, dst: &mut [u8]) -> Result<(), AxisCoreError> {
        if dst.len() < Self::LEN {
            return Err(AxisCoreError::InvalidAccountData);
        }
        dst[0..8].copy_from_slice(&Self::DISCRIMINATOR);
        dst[8..40].copy_from_slice(self.protocol_authority.as_ref());
        dst[40..72].copy_from_slice(self.usdc_mint.as_ref());
        dst[72..104].copy_from_slice(self.protocol_treasury.as_ref());
        dst[104..136].copy_from_slice(self.emergency_operator.as_ref());
        dst[136] = self.bump;
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
            emergency_operator: read_address(&src[104..136]),
            bump: src[136],
        })
    }
}
