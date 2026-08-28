# OTV2-20260828-terminal-session-replacement-repair

```yaml
task_id: OTV2-20260828-terminal-session-replacement-repair
title: Implement terminal GameSession replacement and typed reconciliation
mode: REPAIR
status: waiting
integration_state: WAITING_FRESH_ALLOCATION_MERGE
repository: Oteryn/Oteryn-Game
base_branch: main
branch: impl/game-terminal-session-replacement-250
issue: 250
affected_issue: 167
parent_coordinator_issue: 162
architecture_issue: 248
architecture_pr: 249
architecture_merge_sha: a47e15fdc41373e32935b6fea19f51850f655cfc
historical_source_pr: 243
historical_source_head: eb28c42125c346e7f6f1c72e69d51af35af8fc1f
allocation_branch: coord/terminal-session-replacement-allocation-250
allocation_pr: null
allocation_merge_sha: null
admission_main_sha: a47e15fdc41373e32935b6fea19f51850f655cfc
pr: null
owner: Oteryn: sol durability lead
created_at: 2026-08-28T20:08:00+02:00
updated_at: 2026-08-28T20:08:00+02:00
execution_budget_minutes: 180
large_budget_reason: cross-lane Foundation/Durability repair with real PostgreSQL contention, replay and restart proof
write_authority: none_until_allocation_merge
source_snapshot_mode: COPY_FILE_CONTENTS_ONLY_NO_COMMIT_ANCESTRY_NO_REVIEW_OR_CI_INHERITANCE
serialized_composition_lease: apps/game-server/src/lib.rs
external_repositories: []
owned_paths:
  - apps/game-server/src/foundation/admission_recovery_inner.rs
  - apps/game-server/build.rs
  - apps/game-server/migrations/0001_admission_reconnect_journal.sql
  - apps/game-server/src/bin/oteryn-game-migrate.rs
  - apps/game-server/src/durability/admission_journal.rs
  - apps/game-server/src/durability/db.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/durability/schema.rs
  - apps/game-server/tests/durability_postgres.rs
  - apps/game-server/tests/support/postgres.rs
  - apps/game-server/src/lib.rs
  - docs/agents/tasks/active/OTV2-20260828-terminal-session-replacement-repair.md
public_contracts:
  - DUR-TERMINAL-SESSION-REPLACEMENT-V1
  - DUR-RECONNECT-AUTHORITY-V1
  - DUR-RECONNECT-TRANSPORT-REF-UNIQUENESS-V1
depends_on:
  - Oteryn/Oteryn-Game#249 merged as a47e15fdc41373e32935b6fea19f51850f655cfc
blocks:
  - Oteryn/Oteryn-Game#247
cross_repository_coordination_id: null
```

## Outcome

Deliver a clean-history, independently qualified Foundation + PostgreSQL implementation of canonical terminal predecessor replacement and typed durable terminal replay/reconciliation while preserving one authoritative non-terminal `GameSession` per `CharacterId`.

## Start gate

This task is **read-only** until the docs-only allocation on `coord/terminal-session-replacement-allocation-250` merges and Work reads its merge SHA from protected `main`.

After that readback, Work creates `impl/game-terminal-session-replacement-250` from exactly the allocation merge SHA. The worker must not reuse PR #243 branch ancestry or mutate/merge-up PR #243.

## Canonical architecture

Read before any mutation:

- `docs/architecture/reviews/OTERYN_GAME_TERMINAL_SESSION_REPLACEMENT_COLLISION_RECONCILIATION_DECISION_2026-08-28.md`
- accepted `DUR-RECONNECT-AUTHORITY-V1`
- accepted `DUR-RECONNECT-TRANSPORT-REF-UNIQUENESS-V1`
- current `docs/agents/CODEX_REVIEW_POLICY.json`
- `docs/agents/prompts/OTV2_SOL_DURABILITY_LEAD.md`
- `docs/superpowers/plans/2026-08-28-game-terminal-session-replacement-repair.md`

The worker implements the accepted architecture; it has no authority to redefine Foundation lifecycle semantics, transport-ref attempt rules, resource maxima or production behavior.

## Frozen baseline reconstruction source

Only the following exact blobs from PR #243 head `eb28c42125c346e7f6f1c72e69d51af35af8fc1f` are admitted as read-only baseline bytes:

```yaml
apps/game-server/build.rs: 3a8149ef075f6896a7435c716cb8a4de5d94606b
apps/game-server/migrations/0001_admission_reconnect_journal.sql: 1281fae90744a1b906148a48453e7c09142300c5
apps/game-server/src/bin/oteryn-game-migrate.rs: 80e72fcdeeb70359986a5f93fe287362c0d205a1
apps/game-server/src/durability/admission_journal.rs: c4b289c16d12b41798268325a202c20e798d9971
apps/game-server/src/durability/db.rs: 48746007625646dee9d8a44972005cacb2a97c73
apps/game-server/src/durability/mod.rs: f37fd5e1d8ae50e8b71391a85da73369ac25fcb5
apps/game-server/src/durability/schema.rs: 8c92e301bd420a386f8684025ba429903b1b6e91
apps/game-server/tests/durability_postgres.rs: 2a1b99c670efc13e9464537129adeaa59b3c54c0
apps/game-server/tests/support/postgres.rs: bcb243f6c4823a14ec8116b72439c2c79c115d94
```

Copy file contents only. Do not cherry-pick, merge, rebase, reset to, or otherwise inherit PR #243 commits. Historical test/review/CI evidence does not qualify this task.

## Mandatory TDD lifecycle

### Baseline reconstruction

Recreate the nine frozen files on the new allocation-based branch and commit them as provenance-preserved baseline. Do not add terminal-replacement repair semantics in this step.

### RED

Before repair implementation, add focused failing tests that prove at least:

1. Foundation cannot issue terminal replacement authorization for Active/Reconnectable predecessor or a predecessor with a current transport.
2. Foundation authorization carries the exact current terminal scope generation even when it is greater than the persisted predecessor fence.
3. legacy generic V1 `ExistingTerminal` cannot generic-terminal-complete the migrated flow and instead requires typed same-attempt reconciliation.
4. PostgreSQL exact terminal predecessor replacement accepts `stored_scope < authorized_current_scope` only by exact forward synchronization inside the same transaction.
5. `stored_scope > authorized_current_scope`, predecessor/candidate mismatch and live predecessor all fail closed with no candidate PREPARED authority.
6. direct same-PREPARE replay after lost collision response returns the original typed collision disposition.
7. concurrent replacement attempts for one `CharacterId` have at most one winner.

Publish a Draft PR and preserve the exact RED head/run evidence. Skipped/not-run is not RED.

### GREEN

Implement only the canonical repairs needed to satisfy the RED suite:

- Foundation terminal replacement authorization and typed V2 replay/reconciliation types in `admission_recovery_inner.rs`;
- PostgreSQL terminal session state / replacement receipt or equivalent canonical-safe representation;
- locked predecessor->candidate replacement CAS with monotonic exact-forward scope synchronization;
- terminalization/fencing of predecessor PREPARED attempts in the same transaction;
- typed durable terminal reason mapping for direct `ExistingTerminal` replay and reconciliation;
- `pub mod durability;` composition in `apps/game-server/src/lib.rs` without changing gameplay availability semantics;
- schema/migration contract tests and real PostgreSQL race/restart/idempotency tests.

## Required invariants

- one `CharacterId` has at most one authoritative non-terminal `GameSession` at every commit boundary;
- only Foundation proves terminality and the exact current terminal scope generation;
- Durability never infers terminality from deadline/row age;
- persisted scope can move only monotonically forward to the exact Foundation-authorized value inside terminal replacement; no backwards/local invention;
- connection and CharacterLease fences remain exact;
- predecessor/candidate/account/character/world mismatch fails closed;
- lost replacement response is idempotent only for the exact replacement receipt/binding;
- collision stays terminal for that attempt; same-attempt remint remains forbidden; the existing eight-attempt loss-epoch budget is unchanged;
- generic V1 terminality is never reinterpreted as collision proof;
- no SQLx/network wait is introduced into Foundation's logical writer.

## Excluded scope

No Cargo/lockfile/workflow/resource-registry/Server Seam/Client/Movement/Combat/gameplay/production/secret/live-data/Platform/Atlas/META/external-repository mutation. No new resource maximum. Any need for another implementation path is `SHARED_LEASE_REQUIRED` and the worker must stop for explicit control-plane expansion.

## Required validation

### Focused RED -> GREEN

- Foundation exact tests in `admission_recovery_inner.rs` for terminal authorization, scope drift and typed replay/reconciliation.
- `cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres` against isolated PostgreSQL 17 for replacement, race, restart, replay and existing Durability regressions.

### Component/integration

- `cargo +1.94.0 fmt --all -- --check`
- strict Clippy required by current repository workflow for affected game-server/workspace targets
- game-server/package/workspace tests required by current `BUILD_TEST_MATRIX.md`
- migration binary build against the embedded ledger
- `apps/game-server/src/lib.rs` composition proves Durability is compiled without opening gameplay availability

### Exact-head qualification

1. Finish implementation and task metadata before freezing the final head.
2. Perform mandatory whole-diff self-review on the exact final head.
3. Resolve current risk routing; SESSION/RECONNECT/FENCING/DURABLE_SCHEMA remains `CODEX_REQUIRED` unless canonical policy changes.
4. As the allocated lane lead, request a fresh strict read-only `@codex review` on the exact final head.
5. Repair any findings only inside allocation, freeze a new head, repeat applicable CI/self-review/review.
6. Require zero unresolved P0/P1/P2 findings and required review threads, and fresh exact-head repository CI.
7. Return `READY_FOR_INTEGRATION` to Work; do not self-merge.

## PR and closeout

PR #243 remains open Draft historical evidence until Work decides its terminal disposition after the new repair candidate exists. This worker never mutates #243.

## Context checkpoint

```yaml
last_progress: canonical architecture merged; waiting for fresh cross-lane allocation
status: waiting
branch: impl/game-terminal-session-replacement-250
head_sha: null
pr: null
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: null
ci_check_generation: null
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: fresh_allocation_not_yet_merged
next_action: after allocation merge and protected-main readback, create the worker branch from that exact merge SHA and reconstruct only the frozen nine-file baseline by file contents
```
