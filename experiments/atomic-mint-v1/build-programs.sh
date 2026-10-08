#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
TOOLS_VERSION="${AXIS_PLATFORM_TOOLS:-v1.57}"
TOOLS_DIR="${AXIS_PLATFORM_TOOLS_DIR:-${HOME}/.cache/solana/${TOOLS_VERSION}/platform-tools}"

if [[ ! -x "${TOOLS_DIR}/rust/bin/rustc" ]]; then
  cargo build-sbf --install-only --force-tools-install --tools-version "${TOOLS_VERSION}"
fi

unlink_sbpf_toolchains() {
  local linked
  linked="$(rustup toolchain list -v 2>/dev/null | grep -i sbpf | cut -f1 || true)"
  [[ -z "${linked}" ]] && return 0
  while read -r toolchain; do
    [[ -n "${toolchain}" ]] && rustup toolchain uninstall "${toolchain}" >/dev/null 2>&1 || true
  done <<< "${linked}"
}

build_program() {
  local manifest="$1"
  unlink_sbpf_toolchains
  PATH="${TOOLS_DIR}/rust/bin:${TOOLS_DIR}/llvm/bin:${PATH}" \
  RUSTC="${TOOLS_DIR}/rust/bin/rustc" \
    cargo build-sbf \
      --manifest-path "${manifest}" \
      --no-rustup-override \
      --skip-tools-install \
      --features bpf-entrypoint
}

trap unlink_sbpf_toolchains EXIT
build_program "${SCRIPT_DIR}/programs/direct-route/Cargo.toml"
build_program "${SCRIPT_DIR}/programs/hyp-mint/Cargo.toml"
