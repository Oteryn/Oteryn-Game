# Build and test matrix

Status: bootstrap; replace assumptions with discovered workspace commands as code is introduced.

## Selection principles

- Validate proportionally to changed paths and risk.
- Cheap focused checks run during implementation; heavy checks run at coherent package/final head.
- Exact-head required checks cannot be replaced by historical or parent results.
- Do not claim commands exist until their manifests/workflows are present.

## Current repository checks

| Change | Focused validation | Exact-head validation |
|---|---|---|
| Agent governance/prompt/task docs | `python tools/agents/validate_governance.py` | `agent-governance` workflow |
| Architecture/contracts only | governance validator plus link/JSON checks | documentation/governance workflow |
| GitHub workflow | syntax/review of permissions/triggers | workflow run on exact head |

## Planned Rust workspace checks

Once a root Cargo workspace exists, update this matrix with exact commands. Expected baseline, only after discovery:

- `cargo fmt --all -- --check`;
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` or a documented feature-safe equivalent;
- `cargo test --workspace` with focused package tests during development;
- dependency/advisory/license checks selected by accepted tooling;
- parser property/fuzz tests for untrusted protocol/content inputs;
- deterministic simulation/replay tests;
- platform-specific client build/runtime tests;
- multichannel integration, crash-recovery and soak scenarios.

## High-risk acceptance

| Area | Minimum additional evidence |
|---|---|
| Protocol/framing | limits, negative cases, sequencing, replay/downgrade, golden fixtures, client/server E2E |
| Character lease/relog | double-login, stale writer, crash/recovery, cross-channel misuse |
| Inventory/loot/market | idempotency, concurrency, rollback, no-duplication failure paths |
| Multichannel runtime | two-channel isolation, shared-world services, channel failure, revision compatibility |
| Persistence/migrations | isolated migration tests, rollback/compatibility plan, concurrent mutation tests |
| Client renderer/UI | named platform/hardware/scene, interaction and device-loss/recovery where relevant |
| Assets/updater | provenance, signatures/hashes, traversal/decompression limits, rollback |

## Documentation-only rule

A documentation-only final commit does not require a nonexistent Rust build. It does require the governance/document validation selected by current workflows and an accurate `NOT_APPLICABLE` reason for runtime E2E.
