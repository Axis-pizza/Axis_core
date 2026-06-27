#![no_std]

pub mod constants;
pub mod error;

#[cfg(feature = "bpf-entrypoint")]
pub mod entrypoint;

pub mod processor;
pub mod state;

pub use constants::{BPS_DENOMINATOR, MAX_MARKET_ASSETS};
pub use error::AxisCoreError;
