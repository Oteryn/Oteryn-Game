# OTV2-20260806-rust-client-atomic-cutover

```yaml
task_id: OTV2-20260806-rust-client-atomic-cutover
title: Perform atomic Rust client cutover
mode: MIGRATION
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: migrate/rust-client-cutover-c923ad8
pr: 50
lifecycle_pr: 53
final_head_sha: 5092f868a42d545f47a98c0b9723210570cd9d45
merge_commit_sha: 78988f72a80cc904aa9176ae850c50d4efa0b0f0
source_repository: blakinio/otclient
source_commit: c923ad8a1dff17b4933a6110931b0823cec2c590
source_subtree: oteryn-client/
source_subtree_tree: c0928dafca6df19ff11d7901e503ed85a5199439
coordination_id: OTV2-RUST-CLIENT-CUTOVER-20260806
owner: released
created_at: 2026-08-06T10:32:00+02:00
completed_at: 2026-08-06T13:14:22+02:00
archived_at: 2026-08-06T13:16:00+02:00
next_ordered_action: create the source-only historical marker in blakinio/otclient, then begin FND-ID-01
```

## Result

The complete accepted 19-member Rust workspace was migrated into `blakinio/Oteryn-v2` in one atomic destination pull request. The merged workspace uses `MPL-2.0`, preserves truthful machine-readable provenance, separates production and synthetic dependency closures, and exposes a launchable pre-native client that fails closed before gameplay credentials, routing or transport.

## Acceptance criteria

- [x] Exactly 19 workspace members exist and match FND-01.
- [x] No Canary, gameplay protocol, transport, Game Session or speculative native crate was introduced.
- [x] Production app and synthetic harness closures are separated as required.
- [x] Toolchain, lockfile, formatting, cargo-deny and machine boundary policies are present.
- [x] Platform and Identity use bounded cancellable async I/O on the application-owned Tokio runtime.
- [x] Windows pre-native client build, strict Clippy, visible-shell smoke and synthetic harness pass.
- [x] Linux workspace build, strict Clippy, tests and synthetic harness pass.
- [x] Provenance, dependency delta and transformation evidence are complete.
- [x] Exact-head governance, Dependency Review, CodeQL and Rust workspace checks pass.
- [x] Adversarial migration audit passes with zero material findings.
- [x] PR #50 was squash-merged and destination `main` was verified.
- [x] Task ownership is released through lifecycle PR #53.

## Validation and merge evidence

Validated implementation head: `8177ce3a7a8aed57e644ad8a15fac1920433a68a`.

- Terminal cutover audit run `31095053578`: PASS across Linux, Windows, supply-chain and adversarial migration audit.

Exact final head: `5092f868a42d545f47a98c0b9723210570cd9d45`.

- Agent governance run `31095853261`: PASS.
- Dependency Review run `31095853437`: PASS.
- CodeQL run `31095853606`: PASS.
- Rust workspace run `31095853343`: PASS across Linux, Windows, policy/metadata and supply-chain jobs.
- Review threads: zero.
- Final mergeability: mergeable.
- Squash merge commit: `78988f72a80cc904aa9176ae850c50d4efa0b0f0`.
- Post-merge `main` verification: `78988f72a80cc904aa9176ae850c50d4efa0b0f0` is the verified implementation main head.
- Lifecycle archive PR: #53.

## Excluded scope preserved

- No write was made to `blakinio/otclient` by this task.
- No `protocol-oteryn`, `protocol-canary`, gameplay transport or Game Session implementation was introduced.
- No production deployment, live Identity/Gateway call or proprietary asset import occurred.

## Handover

The destination cutover is complete. The source repository must now receive an explicit source-only historical marker for `oteryn-client/`. After that marker is merged, the next foundational architecture decision is `FND-ID-01`.
