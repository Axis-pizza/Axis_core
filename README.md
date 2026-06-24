# Axis Core

Axis Core is the new on-chain program repository for Axis v1.

This repository will contain the contract implementation for the reserve-backed DTF protocol.

## Scope

Axis Core is responsible for:

- protocol configuration
- DTF market creation
- asset registry / asset enablement
- approved route validation
- reserve vault custody
- USDC mint entry
- USDC redeem exit
- DTF mint / burn
- fee accounting
- mint / redeem accounting tests

## Repository Structure

```txt
programs/
  axis-core/

crates/
  axis-core-client/
  axis-core-test-utils/

tests/

sdk/

docs/
  specs/
  requirements/
  decisions/
  test-plans/

scripts/
```

## Current Status

This repository is currently in the initial scaffold phase.

The first milestone is to set up the Rust workspace, program scaffold, CI, and test harness before implementing production protocol logic.

## Notes

Existing Axis contract repositories are treated as reference only.

Axis Core should be implemented as a clean new program for the USDC-in / USDC-out reserve-backed DTF model.
