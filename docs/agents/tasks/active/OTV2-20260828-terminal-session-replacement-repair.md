# OTV2-20260828-terminal-session-replacement-repair

```yaml
task_id: OTV2-20260828-terminal-session-replacement-repair
title: Implement terminal GameSession replacement and typed reconciliation
mode: REPAIR
status: qualifying
integration_state: FINAL_QUALIFICATION_PENDING
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
allocation_pr: 251
allocation_merge_sha: 12d4ca5326d62a7a2c46d80cd5e167e99f109d1d
worker_base_sha: 12d4ca5326d62a7a2c46d80cd5e167e99f109d1d
admission_main_sha: a47e15fdc41373e32935b6fea19f51850f655cfc
pr: 252
owner: Oteryn: sol durability lead
created_at: 2026-08-28T20:08:00+02:00
updated_at: 2026-08-28T22:47:24+02:00
execution_budget_minutes: 180
large_budget_reason: cross-lane Foundation/Durability repair with real PostgreSQL contention, replay and restart proof
write_authority: exact_allocated_worker_scope_after_12d4ca5326d62a7a2c46d80cd5e167e99f109d1d
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

This task is **read-only** until docs-only allocation PR #251 merges and Work reads its merge SHA from protected `main`.

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

Before repair implementation, add fresh focused failing tests that prove **every new canonical Section 9 obligation**, including at least:

1. Foundation rejects Active/Reconnectable predecessor and Terminal predecessor with a current transport.
2. Foundation authorization carries the exact current terminal scope generation even when greater than the persisted predecessor fence.
3. Foundation constructor rejects each predecessor binding mismatch independently: predecessor GameSessionId, connection generation and CharacterLease generation.
4. Foundation constructor rejects each candidate binding mismatch independently: candidate GameSessionId, account, CharacterId and WorldId.
5. legacy generic V1 `ExistingTerminal` cannot generic-terminal-complete and instead requires typed same-attempt reconciliation.
6. direct V2 same-PREPARE `ExistingTerminal { TransportRefCollision }` marks the exact attempt collision-terminal and unlocks a fresh attempt only when the unchanged attempt budget has capacity; direct ConcurrentPrepared/StaleAuthority never unlock a fresh attempt.
7. V2 reconciliation keeps collision, concurrent-prepared and stale-authority terminal dispositions distinct; only collision may unlock a fresh attempt under the unchanged remaining-capacity rule.
8. PostgreSQL exact terminal predecessor replacement accepts `stored_scope < authorized_current_scope` only by exact forward synchronization inside the same transaction.
9. `stored_scope > authorized_current_scope`, predecessor/candidate mismatch and live predecessor all fail closed with no candidate PREPARED authority.
10. direct same-PREPARE replay after lost collision response returns the original typed collision disposition.
11. a lost predecessor->candidate replacement response replays idempotently only for the exact persisted replacement receipt binding.
12. a conflicting predecessor or candidate against an existing replacement receipt fails closed without actor-anchor mutation.
13. an outstanding predecessor PREPARED attempt is terminalized/fenced by replacement and a later predecessor COMMIT cannot restore authority.
14. a forced database failure after replacement mutation has begun but before candidate PREPARED authority proves full transaction rollback: predecessor scope/session/attempt state unchanged, no receipt committed, candidate absent/unprepared.
15. PostgreSQL V2 reconciliation round-trips collision, concurrent-prepared and stale-authority terminal reasons distinctly after process restart/reload.
16. concurrent replacement attempts for one `CharacterId` have at most one winner.

The detailed plan freezes exact test names and run commands for all cases. Publish a Draft PR and preserve exact RED head/run evidence before adding repair implementation. Skipped/not-run is not RED.

### GREEN

Implement only the canonical repairs needed to satisfy the complete RED suite:

- Foundation terminal replacement authorization with independent validation of every predecessor/candidate identity/fence binding;
- typed V2 direct replay/reconciliation types and state-machine handling in `admission_recovery_inner.rs`;
- PostgreSQL terminal session state / replacement receipt or equivalent canonical-safe representation;
- locked predecessor->candidate replacement CAS with monotonic exact-forward scope synchronization;
- terminalization/fencing of predecessor PREPARED attempts in the same transaction;
- exact replacement-receipt idempotency with conflicting predecessor/candidate rejection;
- one-transaction rollback safety after mutations begin;
- typed durable terminal reason mapping for direct `ExistingTerminal` replay and reconciliation, with collision-only fresh-attempt eligibility;
- `pub mod durability;` composition in `apps/game-server/src/lib.rs` without changing gameplay availability semantics;
- schema/migration contract tests and real PostgreSQL race/restart/idempotency tests.

## Required invariants

- one `CharacterId` has at most one authoritative non-terminal `GameSession` at every commit boundary;
- only Foundation proves terminality and the exact current terminal scope generation;
- every predecessor/candidate identity and fence required by canonical authorization is independently validated before a V2 request exists;
- Durability never infers terminality from deadline/row age;
- persisted scope can move only monotonically forward to the exact Foundation-authorized value inside terminal replacement; no backwards/local invention;
- connection and CharacterLease fences remain exact;
- predecessor/candidate/account/character/world mismatch fails closed;
- lost replacement response is idempotent only for the exact replacement receipt/binding; a conflicting receipt binding is never replay-equivalent;
- replacement atomically fences predecessor PREPARED attempts so a late predecessor COMMIT cannot reactivate authority;
- any failure after replacement mutations begin rolls back predecessor/receipt/candidate changes as one SQL transaction;
- collision stays terminal for that attempt; same-attempt remint remains forbidden; the existing eight-attempt loss-epoch budget is unchanged;
- collision/concurrent/stale durable terminal outcomes remain typed and distinct after direct replay and restart/reconciliation; only collision can unlock the existing bounded replacement-attempt path;
- generic V1 terminality is never reinterpreted as collision proof;
- no SQLx/network wait is introduced into Foundation's logical writer.

## Excluded scope

No Cargo/lockfile/workflow/resource-registry/Server Seam/Client/Movement/Combat/gameplay/production/secret/live-data/Platform/Atlas/META/external-repository mutation. No new resource maximum. Any need for another implementation path is `SHARED_LEASE_REQUIRED` and the worker must stop for explicit control-plane expansion.

## Proven TDD checkpoint

- Schema/receipt/live-anchor RED: `b9f1cda4d3693158eb2224208b51696e1bdb2766`, workflow run `33201919633`, PostgreSQL job `98953210787`: 49 PASS / 8 expected FAIL.
- Foundation V2/terminal-authorization RED: `bff218dcc35a19b10c0a3dc1dbbc78e2cb41b306`, workflow run `33202516767`, job `98955532165`: expected missing canonical Foundation contract.
- Complete runtime PostgreSQL RED: `0fc9de255394ba4ce1b919ad71ea47eeb3247e05`, workflow run `33204365030`, PostgreSQL job `98961526112`: 72 PASS / 6 expected FAIL on PostgreSQL 17.6.
- Runtime GREEN before final composition: `aea85c41268f62486a45e37ed6142cd684ad89df`, workflow run `33207651906`, PostgreSQL job `98972703970`: 78 PASS / 0 FAIL on PostgreSQL 17.6.
- Final composition head: `560eb1d30ad94986b9af3375735c3380b76d7070`; PR #252 was Draft, open and mergeable with exactly the 12 allocated paths at that generation.
- On `560eb1d...`, Rust workspace run `33208062237`, Architecture semantic audit `33208062163`, Merge authority audit `33208062198`, Agent governance `33208062254`, and Merge gate `33208062302` completed successfully.
- A later Agent governance generation `33208318821` failed before checkout because PR metadata lacked the exact required `## Scope` and `## Validation` headings; corresponding Merge gate `33208318794` was cancelled.
- After `560eb1d...` and before the first checkpoint commit, concurrent exact commit `1be6f9c6fb1c2ee78a076a5e86f00771a8d9e7b4` added the fresh test-only RED `runtime_reconciliation_requires_exact_replacement_receipt_binding` in `apps/game-server/src/durability/schema.rs`. It is in the current branch ancestry. Its hosted RED and subsequent GREEN have **not** been established by this checkpoint.
- First durable checkpoint commit: `5382963922bd54a07ba5f301efd26e310500955f`, parent `1be6f9c6fb1c2ee78a076a5e86f00771a8d9e7b4`. This proved the save did not overwrite the concurrent RED commit.

Any tracked checkpoint commit moves the PR head and invalidates prior exact-head qualification as final integration proof. On resume, live GitHub is authoritative; resolve the current branch head, PR body, CI and review state before further mutation.

## Required validation

### Focused RED -> GREEN

- Foundation exact tests in `admission_recovery_inner.rs` for terminal lifecycle, current scope, every predecessor/candidate constructor binding, generic V1 fallback, direct typed V2 replay, reconciliation and collision-only fresh-attempt eligibility.
- `cargo +1.94.0 test --locked -p oteryn-game-server --test durability_postgres` against isolated PostgreSQL 17 for replacement, scope drift, exact-receipt replay/conflict, predecessor late-COMMIT fencing, forced mid-transaction rollback, contention, restart, typed reconciliation and existing Durability regressions.

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
last_progress: runtime GREEN is proven at aea85c41268f62486a45e37ed6142cd684ad89df with 78/78 PostgreSQL tests; final composition reached 560eb1d30ad94986b9af3375735c3380b76d7070; concurrent commit 1be6f9c6fb1c2ee78a076a5e86f00771a8d9e7b4 then added receipt-bound reconciliation RED; durable checkpoint 5382963922bd54a07ba5f301efd26e310500955f preserved that ancestry; current tracked correction records the new RED explicitly
status: qualifying
branch: impl/game-terminal-session-replacement-250
checkpoint_source_head: 1be6f9c6fb1c2ee78a076a5e86f00771a8d9e7b4
previous_checkpoint_commit: 5382963922bd54a07ba5f301efd26e310500955f
head_sha: resolve_live_branch_on_resume
pr: 252
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: checkpoint_commit_requires_fresh_exact_head_generation
ci_checks_for_current_head: 0
ci_run_ids:
  - 33207651906
  - 33208062237
  - 33208062163
  - 33208062198
  - 33208062254
  - 33208062302
  - 33208318821
  - 33208318794
ci_job_ids:
  runtime_green_postgres: 98972703970
  latest_governance_metadata_failure: 98974989692
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: receipt-bound reconciliation RED from 1be6f9c6fb1c2ee78a076a5e86f00771a8d9e7b4 still needs hosted RED classification and GREEN repair; PR #252 metadata also lacks exact `## Scope` and `## Validation` headings; all final exact-head CI/review must be regenerated after checkpoint commits
next_action: run the Durability PostgreSQL harness on the live checkpoint head to classify `runtime_reconciliation_requires_exact_replacement_receipt_binding` before any production repair
```
