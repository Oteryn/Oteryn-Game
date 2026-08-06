# OTV2-20260806-rust-client-atomic-cutover

```yaml
task_id: OTV2-20260806-rust-client-atomic-cutover
title: Perform atomic Rust client cutover
mode: MIGRATION
status: ready_for_merge
repository: blakinio/Oteryn-v2
base_branch: main
branch: migrate/rust-client-cutover-c923ad8
pr: 50
base_sha: ea5363795dc7a6c66b2956c6eb0ce201246cf8c5
source_repository: blakinio/otclient
source_commit: c923ad8a1dff17b4933a6110931b0823cec2c590
source_subtree: oteryn-client/
source_subtree_tree: c0928dafca6df19ff11d7901e503ed85a5199439
coordination_id: OTV2-RUST-CLIENT-CUTOVER-20260806
owner: GPT-5.6-Thinking-migration-coordinator
created_at: 2026-08-06T10:32:00+02:00
updated_at: 2026-08-06T13:05:00+02:00
execution_budget_minutes: 120
owned_paths:
  - Cargo.toml
  - Cargo.lock
  - rust-toolchain.toml
  - rustfmt.toml
  - deny.toml
  - workspace-boundaries.toml
  - apps/client/**
  - crates/foundation/**
  - crates/diagnostics/**
  - crates/client-runtime/**
  - crates/platform-contracts/**
  - crates/platform-client/**
  - crates/identity/**
  - crates/client-domain/**
  - crates/client-simulation/**
  - crates/input-actions/**
  - crates/input-platform/**
  - crates/renderer/**
  - crates/synthetic-assets/**
  - crates/test-support/**
  - tests/security/auth/**
  - tests/pre-native-client/**
  - tools/architecture-check/**
  - tools/synthetic-asset-compiler/**
  - tools/synthetic-client-harness/**
  - tests/fixtures/synthetic-assets/**
  - docs/migration/rust-client-provenance.json
  - docs/migration/rust-dependency-delta.json
  - docs/migration/rust-client-transformation-evidence.md
  - .github/workflows/rust.yml
  - .github/workflows/rust-cutover-terminal-audit.yml
  - docs/agents/tasks/active/OTV2-20260806-rust-client-atomic-cutover.md
public_contracts:
  - workspace-boundaries.toml
  - docs/migration/rust-client-provenance.json
  - docs/migration/rust-dependency-delta.json
depends_on:
  - FND-01 accepted and archived
  - VSL-02 merged as 824599bb2d696e15e22319007b4a919b4438f394
blocks:
  - source-marker task in blakinio/otclient
  - FND-ID-01
```

## Goal

Create and validate the complete 19-member Rust workspace in one destination PR, migrate or rewrite the selected client capabilities, preserve truthful provenance, keep production and synthetic closures separate, and expose a launchable pre-native client that fails closed before gameplay credentials, routing or transport.

## Acceptance criteria

- [x] Exactly 19 workspace members exist and match FND-01.
- [x] No Canary, protocol, gameplay transport, Game Session or speculative native crate is present.
- [x] Production app and synthetic harness dependency closures are disjoint as required.
- [x] Root toolchain, lockfile, formatting, deny and machine boundary policy exist.
- [x] Platform and Identity use bounded cancellable async I/O on the application-owned Tokio runtime.
- [x] Windows client launches a visible pre-native shell and cannot request gameplay entry.
- [x] Linux shared crates compile, lint and test.
- [x] Synthetic harness runs deterministic offline scenarios.
- [x] Provenance, dependency delta and transformation evidence are complete.
- [x] Required implementation-head CI and adversarial audit pass with zero material findings.
- [ ] PR is squash-merged, destination main verified, task archived and ownership released.

## Excluded scope

- No write to blakinio/otclient in this task.
- No protocol-oteryn, protocol-canary, gameplay transport or Game Session implementation.
- No production deployment, live Identity/Gateway call or proprietary asset import.

## Terminal validation evidence

Validated implementation head: `8177ce3a7a8aed57e644ad8a15fac1920433a68a`.

- Agent governance run `31095056168`: PASS.
- Dependency Review run `31095056077`: PASS.
- CodeQL run `31095055948`: PASS.
- Rust workspace run `31095056043`: PASS across Linux, Windows, policy/metadata and supply-chain jobs.
- Terminal cutover audit run `31095053578`: PASS across Linux workspace, Windows pre-native client, supply-chain and adversarial migration audit.
- Review threads: zero.
- PR mergeability: mergeable.

The final head differs from the validated implementation head only by this task checkpoint. Exact-head repository checks must pass again before merge.

## Context checkpoint

```yaml
status: ready_for_merge
validated_implementation_head: 8177ce3a7a8aed57e644ad8a15fac1920433a68a
final_head: pending_exact_head_recheck_after_task_record_commit
pr: 50
last_progress: all implementation and adversarial validation gates passed with zero material findings
blocker: exact-head recheck for the documentation-only terminal checkpoint
next_action: confirm exact-head checks, mark PR ready, squash-merge, verify main, archive task and release ownership
```
