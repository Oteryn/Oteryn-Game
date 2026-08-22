# OTV2-20260822-impl-vsl-content

```yaml
task_id: OTV2-20260822-impl-vsl-content
title: Implement minimal native VSL content compiler loader seam
mode: IMPLEMENT
status: implementation_complete_waiting_integration_authority
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-impl-vsl-content-01
issue: 54
pr: null
base_sha: fd39c6aa026e82062a8b29af24811d467c115f19
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
owner: chat-github-20260822-vsl-content
created_at: 2026-08-22T18:11:00+02:00
updated_at: 2026-08-22T21:34:00+02:00
execution_budget_minutes: 60
owned_paths:
  - apps/game-server/src/content/**
  - docs/agents/tasks/active/OTV2-20260822-impl-vsl-content.md
public_contracts:
  - docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md
  - docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md
  - docs/architecture/VSL-CONTENT-01_MINIMAL_NATIVE_CONTENT_SLICE_CONTRACT_CANDIDATE.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
```

## Outcome

Deliver the minimum typed canonical graph plus deterministic compiler/projection/loader evidence seam needed by the first movement/combat slice, without freezing the permanent World Project/Bundle representation.

## Source facts

- `PROVEN`: VSL fixture/evidence physical representation is explicitly non-production and replaceable.
- `PROVEN`: final world/bundle encoding, chunk packing, compression and Studio source representation remain undecided.
- `PROVEN`: client projection is allowlisted and non-authoritative; server-only fields must not leak.
- `PROVEN`: GAME-CHANNEL multiplicity classes used by the fixture are canonical accepted policy vocabulary.
- `PROVEN`: Issue #54 now tracks this substantial task lifecycle.
- `UNKNOWN`: Reference formulas/content values remain test-only fixtures and cannot establish parity.
- `BLOCKER`: accepted DUR-04/VSL loader/compiler hard maxima are absent from `RESOURCE_LIMITS_REGISTRY.json`; implementation therefore accepts only explicitly injected `evidence:*` limit profiles and cannot claim production acceptance.
- `BLOCKER`: shared composition/workspace lease remains with FOUNDATION; CONTENT cannot mutate `apps/game-server/src/lib.rs`, Cargo workspace files or the registry until coordinator allocation changes.

## Acceptance criteria

- [x] TDD-first stable namespaced keys/revisions and canonical typed graph validation.
- [x] Deterministic compilation is independent of source enumeration order.
- [x] Separate server and client-safe projections with negative leakage tests.
- [x] Non-production evidence artifact has explicit profile/version, manifest/revision/provenance identity, bounded sections and SHA-256 integrity checks.
- [x] Corrupt/truncated/oversized/missing-reference/unknown-critical/incompatible artifacts fail before activation.
- [x] Staging is separate from activation and valid activation is all-or-nothing.
- [ ] Production/composed acceptance: blocked pending coordinator shared lease plus accepted DUR-04/VSL registry limits.

## Implementation delivered in primary path

- nominal `PackageKey`, `ContentKey`, `WorldId` and revision identities;
- bounded typed semantic graph for region/area/cells/collision/relocation, creature/spawn, ability/effect, loot/XP, materializable item and synthetic presentation;
- deterministic canonical ordering and reference/key/source-classification validation;
- explicit `ServerAuthoritative` and allowlisted `ClientSafe` projections;
- disposable `VSL_BUNDLE_EVIDENCE_PROFILE/v1/non-production` binary evidence artifact with manifest, section table, per-section and artifact SHA-256 digests;
- checked offset/length arithmetic, bounded record/string/section parsing and unknown-critical rejection;
- exact server/client revision-pair verification plus explicit expected revision/provenance verification;
- isolated staging and all-or-nothing activation preserving the previous active revision on validation failure;
- ordinary-release compile rejection for fixture-only profiles.

## Validation

### TDD focused
- RED: initial standalone Rust 1.94 test compile failed on the intentionally absent content API before implementation.
- final command: `rustup run 1.94.0 rustc --edition 2024 --test apps/game-server/src/content/mod.rs -D warnings -o target/vsl-content-tests.exe && target/vsl-content-tests.exe`
- final result: `PASS`, 14 passed / 0 failed.

### Strict lint / workspace
- standalone content Clippy harness on Rust 1.94.0 with `-D warnings -D clippy::all`: `PASS`.
- `cargo +1.94.0 run --locked -p oteryn-architecture-check -- workspace .`: `PASS`.
- `cargo +1.94.0 test --locked --workspace --all-targets`: `PASS`.
- `cargo +1.94.0 clippy --locked --workspace --all-targets -- -D warnings`: `PASS`.

### Formatting / diff
- content files formatted with Rust 1.94 rustfmt and revalidated after formatting.
- repository-wide Windows checkout `cargo fmt --all -- --check` reports pre-existing newline-style findings in numerous untouched files; this worker does not mutate them outside allocation.
- `git diff --check`: `PASS`.
- `python tools/repository/validate_repository_policy.py`: baseline `FAIL` on pinned MPL-2.0 text; reproduced on clean `main@a2a5da955dd8f580c9e768c8ac6a741db388cb22` with identical LICENSE blob `d0a1fa1482eea82e19510e7920cbe3a03e41f691`; this lane does not own or modify LICENSE/policy.
- changed implementation paths remain only `apps/game-server/src/content/**` plus this owned task record.

### E2E
- Movement/Combat consumption: `NOT_EVALUATED`; dependent runtime composition is outside the current shared lease and later QA/consumer work remains required.

## Excluded scope

No permanent `.omap`/`.owb` contract, compression/chunk/CDN/signing decision, Studio UI, proprietary assets, broad content set, production distribution or Reference-parity claim.

## Context checkpoint

```yaml
last_progress: primary-path implementation and focused/workspace verification are complete; Issue #54 tracks lifecycle; exact worker code is ready for a draft PR while integration authority remains blocked.
status: implementation_complete_waiting_integration_authority
branch: agent/otv2-impl-vsl-content-01
head_sha: pending_commit_and_main_reconcile
pr: null
blocker: FOUNDATION still owns serialized shared composition paths, and accepted DUR-04/VSL resource-limit registry entries are missing.
owner_action_required: null
next_action: commit primary-path implementation and checkpoint, reconcile merged main, push/open draft PR, run exact-head gates, then wait for lawful coordinator lease/registry allocation before composition/terminal merge.
```