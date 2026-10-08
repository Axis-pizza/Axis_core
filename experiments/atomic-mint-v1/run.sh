#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ "${AXIS_SKIP_SBF_BUILD:-0}" != "1" ]]; then
  "${SCRIPT_DIR}/build-programs.sh"
fi
cargo run --quiet --manifest-path "${SCRIPT_DIR}/Cargo.toml" -- "$@"
