# OTV2-20260929-runtime-status-producer

```yaml
task_id: OTV2-20260929-runtime-status-producer
title: Q16b runtime-status producer - the game node reports its committed readiness to Platform
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/runtime-status-producer
pr: 1302
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

- `native_admission_source/runtime_status.rs`:
  - the exact `ReportRuntimeStatusV1` wire (22 members, no endpoint), grammar checks and an exact response decoder;
  - a purpose-bound `RuntimeStatusDescriptor`, which refuses a certificate or key whose SPKI another purpose uses and a key that does not match its certificate;
  - `deliver` on its own capacity (never an admission slot), with a classified `NotDelivered` result (`400`/`401`/`409`/`429`/unavailable);
  - the report loop, driven by an injectable clock, gate and sink: each committed publication is sent once; the heartbeat H = 5 s runs at a fixed rate while `ready` and the gate holds; a busy check is retried with bounded backoff (100 ms doubling to 1 s) inside H; the latest publication wins;
  - `route_revision(version, descriptor)`, which recomputes the Registry value for comparison only.
- `Operation::ReportRuntimeStatusV1` with a per-operation body cap (2048; evidence operations keep 1024). `http1_mtls::exchange_with_status` returns final non-200 statuses for this operation only.
- `node`:
  - optional `[platform.runtime_status]` (`client_certificate_file`, `client_key_file`, `assignment_epoch`); when it is absent, reporting is off (section 15 rollback);
  - `Reporter` starts only from the committed `ready = true` publication, stops heartbeats at shutdown, reports only a committed `ready = false` withdrawal and ends within the remaining shutdown budget;
  - the heartbeat gate takes no durability transaction per heartbeat: the snapshot verified by the commit stands for `RECHECK` = 10 s. After that, one non-blocking read (`try_issue_semantic_pass`) re-verifies it. A busy connection keeps the snapshot for `BUSY_GRACE` = 5 s (RECHECK + BUSY_GRACE = F), then reports `Busy`. `Busy`, `NotReady` and `Lost` are distinct; `Lost` is final.
- Registry: `NRS-REPORT-BYTES`, `NRS-RESPONSE-BYTES`, `NRS-INFLIGHT`, `NRS-HEARTBEAT`.

## Assumptions

- `assignment_epoch` is a declared configuration value until its Game storage exists (contract U-RS5); a wrong value is refused by Platform (`409`) and never affects the node.
- The node does not compute its `route_revision` at boot: the contract defines no configuration source for the Registry route descriptor, so the helper exists for comparison and fixture use only.

## High-risk authority/recovery qualification

- AuthorityInvariant: the report is evidence only; it never mutates Game state. Identity/binding: the report is a projection of the committed publication (field-by-field unit test); purpose identity is separate (local refusal plus stub `401`). Liveness: heartbeats require serving and an assignment verified within RECHECK (+ BUSY_GRACE while the connection is busy). Temporal: `observed_at >= published_at`, never future-dated (clock read at send).
- ConsumerBoundary: Platform ingestion (stub in tests); no Game mutation boundary consumes the report.
- MutationOperators: missing facts (zero epoch, generation, revision refused); stale generation (gate compares the current assignment); mismatched identity (evidence certificate/key refused); expired/future time (`observed_at < published_at` refused); provenance substitution (only `Applied`/`Existing` commits are reported); replay/concurrency (one in flight, latest wins). Fenced durable writes, restart and PostgreSQL reload: `NOT_APPLICABLE` (no write).

## Next slice (#162)

- `oteryn-game-ops` `ReportScopeAssignmentV1` re-report subcommand with the ownership-authority certificate and a durable assignment outbox (needs a migration: shared lease, next free migration after 0018).
- `assignment_epoch` storage and operator procedure (U-RS5); joint E2E through the real Platform ingestion (#1419 item 5).

## Closeout

- Validation: fmt PASS; clippy -D warnings PASS; `cargo test -p oteryn-game-server` 9436 passed, 0 failed (focused rerun after final tests PASS); governance and repository-policy validators PASS; `git diff --check` clean.
- Review: round 1 on `1ba81be7` returned FIX. Fixed in one push: (1) busy durability connection, material: a cached assignment snapshot, non-blocking re-check with grace and a bounded busy retry, and a test showing no gap above F plus a bounded try count; (2) wiring tests for the start, the committed and failed withdrawal and the budget, plus the real assignment classification; (3) a real other-purpose certificate refused with `401`; (4) SPKI comparison; (5) a `409` conflict class; (6) the `Operation` variant; (8) virtual-time heartbeat tests. Re-review is returned to the control plane.
- Merge commit/result: squash merge of #1302.
