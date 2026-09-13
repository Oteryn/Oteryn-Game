#!/usr/bin/env bash
set -euo pipefail

# Focused Codex Cloud setup for the long-running WP3 SQLx/rustls/Tokio lane.
# This script intentionally does NOT run a workspace-wide `cargo fetch` or scan
# unrelated applications/experiments. Validation still belongs to the worker
# and repository CI; this phase only prepares the dependency cache needed by
# the active WP3 crates.

readonly RUST_TOOLCHAIN="${OTERYN_CODEX_RUST_TOOLCHAIN:-1.94.0}"
readonly TARGET="${OTERYN_CODEX_TARGET:-x86_64-unknown-linux-gnu}"

manifests=(
  "vendor/tokio-1.53.1/Cargo.toml"
  "vendor/rustls-0.23.43/Cargo.toml"
  "vendor/sqlx-core-0.9.0/Cargo.toml"
  "vendor/sqlx-postgres-0.9.0/Cargo.toml"
)

missing_manifests=()
for manifest in "${manifests[@]}"; do
  if [[ ! -f "${manifest}" ]]; then
    missing_manifests+=("${manifest}")
  fi
done

if (( ${#missing_manifests[@]} > 0 )); then
  echo "Oteryn WP3 Codex Cloud warm-up deferred: this checkout does not contain all WP3 vendor manifests." >&2
  printf 'missing: %s\n' "${missing_manifests[@]}" >&2
  echo "This is expected on bootstrap refs that do not carry the WP3 vendor tree; continue setup successfully and warm dependencies after switching to the WP3 ref." >&2
  exit 0
fi

if ! command -v rustup >/dev/null 2>&1; then
  echo "rustup is required by the Oteryn WP3 Codex Cloud environment" >&2
  exit 1
fi

rustup toolchain install "${RUST_TOOLCHAIN}" --profile minimal --no-self-update

for manifest in "${manifests[@]}"; do
  cargo "+${RUST_TOOLCHAIN}" fetch \
    --manifest-path "${manifest}" \
    --target "${TARGET}"
done

printf 'Oteryn WP3 Codex Cloud cache prepared for %s with Rust %s\n' \
  "${TARGET}" "${RUST_TOOLCHAIN}"
