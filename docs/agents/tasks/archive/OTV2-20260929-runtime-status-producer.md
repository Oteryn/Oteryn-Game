# OTV2-20260929-runtime-status-producer

```yaml
task_id: OTV2-20260929-runtime-status-producer
title: Q16b runtime-status producer - the game node reports its committed readiness to Platform
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/runtime-status-producer
pr: null
base_sha: c0f7e238
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "runtime-status producer writer (Claude Code)"
created_at: 2026-09-29T00:00:00Z
updated_at: 2026-09-29T00:00:00Z
execution_policy: continuous_progress
coordination: "#162 owner-accepted package Q21 (5900403086)"
contract: docs/contracts/OTERYN_GAME_NATIVE_RUNTIME_STATUS_PRODUCER_V1.md
platform_contract: "Oteryn/Oteryn-Platform PR #1420 @ 892d640d, OTERYN_V2_NATIVE_GATEWAY_LOGIN_CONTRACT.md section 7"
owned_paths:
  - apps/game-server/src/native_admission_source/**
  - apps/game-server/src/node/** (reporter wiring only)
  - apps/game-server/tests/*runtime_status*
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json (NRS-* rows)
  - docs/agents/tasks/archive/OTV2-20260929-runtime-status-producer.md
```

## Outcome

- `native_admission_source/runtime_status.rs`: exact `ReportRuntimeStatusV1` wire (22 members, no endpoint), grammar checks, exact response decoder, purpose-bound `RuntimeStatusDescriptor` (refuses the evidence certificate or key), bounded delivery over the existing `http1_mtls` exchange on its own capacity (never an admission slot), and the report loop: send each committed publication once, heartbeat H = 5 s while `ready` and the gate holds, latest wins, ends after the last unseen publication. `route_revision(version, descriptor)` recomputes the Registry value for comparison only (JCS, SHA-256, 32 hex).
- `http1_mtls::exchange_at`: compiled path with its own body cap (2048 for the report); `exchange` keeps 1024 for the evidence operations.
- `node`: optional `[platform.runtime_status]` (`client_certificate_file`, `client_key_file`, `assignment_epoch`); absent means reporting off (contract section 15 rollback). The reporter starts after the `ready = true` commit, stops heartbeats at shutdown, reports the `ready = false` withdrawal and ends within the shutdown budget. Heartbeat gate: serving, durability root ready, assignment still `Assigned` to this incarnation at the published generation. `Readiness::publish` now returns the committed publication.
- Registry: `NRS-REPORT-BYTES`, `NRS-RESPONSE-BYTES`, `NRS-INFLIGHT`, `NRS-HEARTBEAT`.

## Assumptions

- `assignment_epoch` is a declared configuration value until its Game storage exists (contract U-RS5); a wrong value is refused by Platform (`409`) and never affects the node.
- The node does not compute its `route_revision` at boot: the contract defines no configuration source for the Registry route descriptor, so the helper exists for comparison and fixture use only.

## High-risk authority/recovery qualification

- AuthorityInvariant: the report is evidence only; it never mutates Game state. Identity/binding: the report is a projection of the committed publication (field-by-field unit test); purpose identity is separate (local refusal plus stub `401`). Liveness: heartbeats require serving, root readiness and the unchanged assignment. Temporal: `observed_at >= published_at`, never future-dated (clock read at send).
- ConsumerBoundary: Platform ingestion (stub in tests); no Game mutation boundary consumes the report.
- MutationOperators: missing facts (zero epoch, generation, revision refused); stale generation (gate compares the current assignment); mismatched identity (evidence certificate/key refused); expired/future time (`observed_at < published_at` refused); provenance substitution (only `Applied`/`Existing` commits are reported); replay/concurrency (one in flight, latest wins). Fenced durable writes, restart and PostgreSQL reload: `NOT_APPLICABLE` (no write).

## Next slice (#162)

- `oteryn-game-ops` `ReportScopeAssignmentV1` re-report subcommand with the ownership-authority certificate and a durable assignment outbox (needs a migration: shared lease, next free migration after 0018).
- `assignment_epoch` storage and operator procedure (U-RS5); joint E2E through the real Platform ingestion (#1419 item 5).

## Closeout

- Validation: see the PR body (fmt, clippy, `cargo test -p oteryn-game-server`, governance and repository-policy validators, `git diff --check`).
- Review: independent review required; run by the control plane after freeze.
- Merge commit/result: squash merge of the PR named in the PR field once merged.
