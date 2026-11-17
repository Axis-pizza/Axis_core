#![cfg_attr(target_os = "solana", no_std)]

pub mod constants;
pub mod error;
pub mod instructions;
pub mod math;
pub mod processor;
pub mod state;

#[cfg(feature = "bpf-entrypoint")]
pub mod entrypoint;

pub use error::AxisCoreError;
