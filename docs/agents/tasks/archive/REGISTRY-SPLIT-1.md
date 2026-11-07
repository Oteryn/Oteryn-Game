# REGISTRY-SPLIT-1

```yaml
task_id: REGISTRY-SPLIT-1
title: "REGISTRY-SPLIT-1: the protocol and resource-limit registries are one file per entry"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: claude/happy-davinci-ijz3wy-4
branch: regsplit/1
pr: null
base_sha: 6ec8f7bdc
created_at: 2026-10-23
updated_at: 2026-10-23
execution_policy: continuous_progress
depends_on: []
owned_paths:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json (deleted)
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json (deleted)
  - docs/contracts/protocol-oteryn/v1/registry/**
  - docs/contracts/resource-limits/**
  - crates/protocol-oteryn/{build.rs,Cargo.toml,src/contract_registries.rs}
  # include_str! of the old files replaced by the crate const, nothing else
  - crates/protocol-oteryn/src/*.rs
  - crates/gameplay/{dispatch,durability,combat-rules,social-rules}/src/**/*.rs
  - apps/game-server/examples/dur03_{native_one_item_audit,reference_one_item_resource_prototype}.rs
  - crates/gameplay/{combat-rules,social-rules}/Cargo.toml
  - workspace-boundaries.toml
  - Cargo.lock
  - content/world/pins/oteryn.json (inputs_digest only)
  - tools/contracts/**
  - tools/agents/validate_governance.py
  - tools/agents/tests/test_governance_lifecycle_first_production_content_registry.py
  - tools/errors/{registry.py,explain.py,tests/test_registry.py}
  - docs/agents/GOVERNANCE_CONTRACT.json
  - docs/architecture/README.md
  - docs/agents/tasks/archive/REGISTRY-SPLIT-1.md
public_contracts: []
registry: "layout only: no entry, id or value changes"
```

## Outcome

- `PROTOCOL_OTERYN_V1_REGISTRY.json` is `docs/contracts/protocol-oteryn/v1/registry/registry.json`
  plus one file per capability (25), command type (19) and state domain (21), named
  `<id>-<name>.json`. `RESOURCE_LIMITS_REGISTRY.json` is `docs/contracts/resource-limits/registry.json`
  plus one file per entry (591), named `<ordinal>-<id>.json`; the ordinal keeps today's order.
  Each directory has a README for authors: a new mechanic adds files, it does not edit a list.
- `crates/protocol-oteryn/build.rs` scans the fragment directories (no list file), emits
  `rerun-if-changed` for each directory and file, checks names, ids (unique, case-insensitive;
  the protocol prefix is the `id`), required fields and the placeholders, and writes the
  aggregates to `OUT_DIR`. Rust reads `oteryn_protocol_oteryn::{PROTOCOL_OTERYN_V1_REGISTRY_JSON,
  RESOURCE_LIMITS_REGISTRY_JSON}`; 61 `include_str!` sites in 43 files moved to them.
  combat-rules and social-rules gain a dev-dependency on protocol-oteryn (`[dev_edges]`).
- Python reads the registries through `tools/contracts/registry_fragments.py` (validate_governance,
  tools/errors, the first-production-content test). `tools/errors` reads a base ref from before
  the split from the old file.
- Both aggregates are byte-identical to the files at `6ec8f7bdc` (sha256
  `de98d1db5ab9f1ff52204e5059658507ca12acac71d90146dac886dc6eafb562` and
  `f2d8bb0bb307abba57adfe6431a339f78050429deac38131582da8e86949c08f`), from both build.rs and
  the Python helper. Permanent tests: `tools/contracts/tests` (split and assemble round-trip to the
  committed tree, validation) and `contract_registries` in protocol-oteryn (aggregate = base +
  every fragment, ids unique and sorted).
- No file pinned either registry's hash. Cargo.lock gains the two dev-dependency lines, so the
  World pin `inputs_digest` is recomputed; the bundle digest is unchanged.
- `CROSS_REPOSITORY_CONTRACT_LOCK.json` still names the old paths: they are the LOCKED record of
  its `canonical_commit`. `error_codes` and `allocation_ranges` stay in the protocol base file.

## Validation

Toolchain 1.99.1. Without `OTERYN_TEST_POSTGRES_ADMIN_URL` (no PostgreSQL route was used).

- aggregates (build.rs `OUT_DIR` and `registry_fragments.load_text`) = `git show 6ec8f7bdc:<old file>`: byte-identical
- git diff --check: pass
- cargo +1.99.1 fmt --all --check: pass
- cargo +1.99.1 clippy --locked --workspace --all-targets -- -D warnings: pass
- cargo +1.99.1 run --locked -p oteryn-architecture-check --quiet -- workspace .: pass
- cargo +1.99.1 run --locked --release -p oteryn-world-bundle-compiler -- pin-check . <out>: pass (recomputed `inputs_digest`, bundle digest unchanged)
- python3 tools/agents/validate_governance.py: pass
- python3 -m unittest discover -s tools/agents/tests: 124 OK
- python3 tools/repository/validate_repository_policy.py: pass
- python3 -m unittest discover -s tools/errors/tests: 15 OK; tools/contracts/tests: 5 OK
- cargo +1.99.1 test --locked: protocol-oteryn 292, combat-rules 46, social-rules 47,
  durability 389, dispatch 925 (14 ignored): pass
- cargo +1.99.1 test --locked -p oteryn-game-server --no-fail-fast: 3678 passed, 5 failed; the 5
  are `character_authority_postgres` tutorial cases that need routed PostgreSQL
