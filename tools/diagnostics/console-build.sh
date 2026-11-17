#!/usr/bin/env bash
# tokio-console diagnostics build of oteryn-game-server (local debugging only).
#
#   tools/diagnostics/console-build.sh [extra cargo build args, e.g. --release]
#   TOKIO_CONSOLE_BIND=127.0.0.1:6669 target/diagnostics/debug/oteryn-game-server serve --config <path>
#   tokio-console http://127.0.0.1:6669
#
# The `diagnostics` feature needs `--cfg tokio_unstable`. Changing RUSTFLAGS rebuilds every crate,
# so the build goes into its own target directory and the normal target stays untouched.
set -euo pipefail
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
export RUSTFLAGS="${RUSTFLAGS:+$RUSTFLAGS }--cfg tokio_unstable"
exec cargo build --locked \
  --manifest-path "$root/Cargo.toml" \
  --target-dir "${OTERYN_DIAGNOSTICS_TARGET_DIR:-$root/target/diagnostics}" \
  -p oteryn-game-server --bin oteryn-game-server --features diagnostics "$@"
