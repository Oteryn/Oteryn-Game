#!/usr/bin/env bash
set -euo pipefail

# Cached Codex Cloud continuations should be cheap. Do not perform dependency
# discovery, workspace-wide fetches, builds, or tests here. The material worker
# selects focused validation for the exact cell it is changing.

readonly RUST_TOOLCHAIN="${OTERYN_CODEX_RUST_TOOLCHAIN:-1.94.0}"

rustc "+${RUST_TOOLCHAIN}" --version
cargo "+${RUST_TOOLCHAIN}" --version
git status --short --branch
