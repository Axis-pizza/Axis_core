# crates/axis-core-test-utils

Shared test utilities for Axis Core.

Current scaffold responsibilities:

- LiteSVM helpers
- deterministic payer and signer setup
- Axis Core program artifact path resolution
- explicit LiteSVM program loading diagnostics
- raw account data and lamport reads
- SPL Token / Token-2022 base account balance reads

`src/fixture.rs` builds whole markets for the invocation tests: real SPL Token
and Token-2022 mints and accounts, instruction builders for every Axis Core
instruction, and balance snapshots for asserting that a failed instruction
committed nothing. Its amounts are fixtures, not production parameters.

Purchases and USDC redemptions in those tests swap through a route stand-in:
the legacy SPL Token program approved as the market's swap program, and a
maker who co-signs plain token transfers. It exercises Core's real CPI path.
It is not Jupiter, and says nothing about a production route's account locks,
transaction size or compute use.

Program loading expects a LiteSVM-loadable Solana SBF shared object. Build it
with `scripts/build-sbf.sh`, then run the opt-in `program_invocation` target
with the `sbf-integration` feature. A missing `target/deploy/axis_core.so` is
reported as a blocker instead of being treated as a successful load.
