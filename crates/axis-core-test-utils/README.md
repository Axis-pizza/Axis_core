# crates/axis-core-test-utils

Shared test utilities for Axis Core.

Current scaffold responsibilities:

- LiteSVM helpers
- deterministic payer and signer setup
- Axis Core program artifact path resolution
- explicit LiteSVM program loading diagnostics
- raw account data and lamport reads
- SPL Token / Token-2022 base account balance reads

Deferred responsibilities:

- mock USDC mint setup
- mock asset mint setup
- user token account creation helpers
- reserve vault fixtures
- protocol account builders
- mint, redeem, fee, routing, custody, DEX CPI, and Token-2022 business-flow tests

Program loading expects a LiteSVM-loadable Solana SBF shared object. Build it
with `scripts/build-sbf.sh`, then run the opt-in `program_invocation` target
with the `sbf-integration` feature. A missing `target/deploy/axis_core.so` is
reported as a blocker instead of being treated as a successful load.
