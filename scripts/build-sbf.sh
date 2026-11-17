#!/usr/bin/env bash
# Build the Axis Core SBF artifact into target/deploy/axis_core.so.
#
# Two problems with the stock `cargo build-sbf` on this workspace, both worked
# around here rather than in anyone's shell profile.
#
# 1. Toolchain name parsing. cargo-build-sbf 3.0.15 reads `rustup toolchain
#    list -v`, whose lines are "<name>\t<path>", and passes the whole line to
#    `rustup toolchain uninstall` when the Solana toolchain is already linked.
#    Every build links it, so every build after the first one fails with
#    "invalid toolchain name". Unlinking first avoids that branch, and
#    --no-rustup-override keeps it from mattering at all.
#
# 2. Compiler version. The platform-tools that ship with solana-cli 3.0.15 are
#    v1.51 with rustc 1.84.1, and pinocchio 0.11 needs 1.89. platform-tools
#    v1.57 carries rustc 1.95, so this pins the SDK to that instead. The
#    solana-cli itself is left alone.
set -euo pipefail

TOOLS_VERSION="${AXIS_PLATFORM_TOOLS:-v1.57}"
TOOLS_DIR="${HOME}/.cache/solana/${TOOLS_VERSION}/platform-tools"
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ ! -x "${TOOLS_DIR}/rust/bin/rustc" ]]; then
  echo "platform-tools ${TOOLS_VERSION} not found, installing..." >&2
  cargo build-sbf --install-only --force-tools-install --tools-version "${TOOLS_VERSION}"
fi

echo "using $("${TOOLS_DIR}/rust/bin/rustc" --version)" >&2

# Drop any stale link so the next run does not trip over problem 1.
unlink_sbpf_toolchains() {
  # grep exits 1 when there is nothing linked, which is the normal case and
  # not a failure. Without the guard, pipefail plus set -e ends the script.
  local linked
  linked="$(rustup toolchain list -v 2>/dev/null | grep -i sbpf | cut -f1 || true)"
  [[ -z "${linked}" ]] && return 0
  while read -r tc; do
    [[ -n "${tc}" ]] && rustup toolchain uninstall "${tc}" >/dev/null 2>&1 || true
  done <<< "${linked}"
  return 0
}

unlink_sbpf_toolchains

PATH="${TOOLS_DIR}/rust/bin:${TOOLS_DIR}/llvm/bin:${PATH}" \
RUSTC="${TOOLS_DIR}/rust/bin/rustc" \
  cargo build-sbf \
    --manifest-path "${REPO_ROOT}/programs/axis-core/Cargo.toml" \
    --no-rustup-override \
    --skip-tools-install \
    --features bpf-entrypoint \
    "$@"

# cargo-build-sbf re-links the toolchain on the way out, which is what breaks
# the following run. Remove it again so the next build starts clean.
unlink_sbpf_toolchains

ls -la "${REPO_ROOT}/target/deploy/axis_core.so"
