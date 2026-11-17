# Local Commands

This workspace pins Rust `1.93.1` in `rust-toolchain.toml` and requires the
`rustfmt` component.

## Format

```bash
cargo fmt --check
```

## Build

```bash
cargo build
```

## Test

```bash
cargo test
```

Run only the LiteSVM scaffold smoke tests:

```bash
cargo test -p axis-core-test-utils --test litesvm_smoke
```

## LiteSVM Program Artifact

Build the SBF artifact before running the integration tests:

```bash
./scripts/build-sbf.sh
```

It writes `target/deploy/axis_core.so`, which
`crates/axis-core-test-utils/tests/program_invocation.rs` loads and executes.

Run the opt-in SBF integration target after building the artifact:

```bash
cargo test -p axis-core-test-utils \
  --features sbf-integration \
  --test program_invocation
```

```txt
AXIS_CORE_PROGRAM_ARTIFACT     optional path override
AXIS_PLATFORM_TOOLS            optional platform-tools version, default v1.57
target/deploy/axis_core.so     default workspace-relative path
```

If the artifact is missing, the smoke test reports the blocker and does not
pretend the program was loaded. The opt-in invocation target fails outright
rather than skipping, because a silently skipped integration test is how a
broken toolchain stays invisible. Normal `cargo test` does not select that
target, so a clean host-only CI checkout does not require Solana SBF tooling.

## Why the SBF build needs a script

Stock `cargo build-sbf` fails twice on this workspace.

**Toolchain name parsing.** cargo-build-sbf 3.0.15 reads `rustup toolchain
list -v`, whose lines are `<name>\t<path>`, and passes the whole line to
`rustup toolchain uninstall` when the Solana toolchain is already linked.
Every build links it, so every build after the first fails with
`invalid toolchain name`. The script unlinks before and after, and passes
`--no-rustup-override`.

**Compiler version.** The platform-tools that ship with solana-cli 3.0.15 are
v1.51 with rustc 1.84.1, and pinocchio 0.11 requires 1.89. platform-tools
v1.57 carries rustc 1.95, so the script pins the SDK to that. The solana-cli
itself is left untouched, so nothing else on the machine changes.

## Solana / Pinocchio Build

The Pinocchio entrypoint feature can also be checked with the host toolchain,
which compiles but does not produce a loadable artifact:

```bash
cargo build -p axis-core --features bpf-entrypoint
```
