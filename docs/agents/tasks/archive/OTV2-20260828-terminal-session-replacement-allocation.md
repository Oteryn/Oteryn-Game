# OTV2-20260828-terminal-session-replacement-allocation

```yaml
task_id: OTV2-20260828-terminal-session-replacement-allocation
title: Allocate canonical terminal GameSession replacement repair
mode: COORDINATE
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: coord/terminal-session-replacement-allocation-250
issue: 250
parent_coordinator_issue: 162
architecture_issue: 248
architecture_pr: 249
architecture_merge_sha: a47e15fdc41373e32935b6fea19f51850f655cfc
affected_issue: 167
historical_pr: 243
historical_head: eb28c42125c346e7f6f1c72e69d51af35af8fc1f
allocation_pr: 251
admission_main_sha: a47e15fdc41373e32935b6fea19f51850f655cfc
control_plane_profile: OTV2_WORK_DELIVERY_COORDINATOR
owner: ChatGPT Work Delivery Coordinator
worker_alias: Oteryn: sol durability lead
worker_task_id: OTV2-20260828-terminal-session-replacement-repair
worker_branch: impl/game-terminal-session-replacement-250
worker_pr: null
created_at: 2026-08-28T20:08:00+02:00
updated_at: 2026-08-28T20:31:00+02:00
execution_budget_minutes: 120
large_budget_reason: serialized cross-lane Foundation/Durability repair with real PostgreSQL race/restart proof
owned_paths:
  - docs/agents/tasks/active/OTV2-20260828-terminal-session-replacement-allocation.md
  - docs/agents/tasks/active/OTV2-20260828-terminal-session-replacement-repair.md
  - docs/superpowers/plans/2026-08-28-game-terminal-session-replacement-repair.md
runtime_write_authority: none
production_authority: none
external_repositories: []
```

## Outcome

Create one merged, exact-base, docs-only control-plane allocation that releases a fresh serialized implementation lane for canonical `DUR-TERMINAL-SESSION-REPLACEMENT-V1`. The allocation itself changes no runtime behavior.

## Architecture and source of truth

- `PROVEN`: protected `main` at allocation admission is `a47e15fdc41373e32935b6fea19f51850f655cfc`.
- `PROVEN`: PR #249 is squash-merged at that SHA and makes `docs/architecture/reviews/OTERYN_GAME_TERMINAL_SESSION_REPLACEMENT_COLLISION_RECONCILIATION_DECISION_2026-08-28.md` canonical.
- `PROVEN`: Issue #248 is closed/completed after protected-main readback.
- `PROVEN`: PR #243 remains Draft at `eb28c42125c346e7f6f1c72e69d51af35af8fc1f`; it is historical/paused source evidence and receives no new write or merge authority.
- `PROVEN`: open Server Seam #247 mentions `apps/game-server/src/lib.rs` only as a future conditional lease and explicitly has no current write authority.
- `PROVEN`: no open Issue other than the coordinator references active write authority over `apps/game-server/src/foundation/admission_recovery_inner.rs`.
- `DERIVED`: one serialized repair lane can safely own the canonical eleven implementation/composition paths after this allocation merges because no conflicting current writer is proven.

Governing decision: `DUR-TERMINAL-SESSION-REPLACEMENT-V1` on protected main. Existing `DUR-RECONNECT-AUTHORITY-V1` and `DUR-RECONNECT-TRANSPORT-REF-UNIQUENESS-V1` remain binding.

## Allocation decision

After this allocation PR merges and its merge SHA is read back from protected `main`, `Oteryn: sol durability lead` becomes the exclusive worker for task `OTV2-20260828-terminal-session-replacement-repair` on a new branch created from that merge SHA:

`impl/game-terminal-session-replacement-250`

The worker must not continue, merge-up, rebase, reset, cherry-pick or force-update PR #243. PR #243 is read-only evidence. Its exact head may supply file contents only for the frozen nine-file source manifest below.

### Frozen read-only source manifest from PR #243

```yaml
source_pr: 243
source_head: eb28c42125c346e7f6f1c72e69d51af35af8fc1f
mode: COPY_FILE_CONTENTS_ONLY_NO_COMMIT_ANCESTRY_NO_QUALIFICATION_INHERITANCE
files:
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

Any different PR #243 head/blob requires a fresh control-plane source-admission decision before use.

## Exact worker implementation authority after merge

Semantic implementation allowlist, exactly as canonical #249 requires:

- `apps/game-server/src/foundation/admission_recovery_inner.rs`
- `apps/game-server/build.rs`
- `apps/game-server/migrations/0001_admission_reconnect_journal.sql`
- `apps/game-server/src/bin/oteryn-game-migrate.rs`
- `apps/game-server/src/durability/admission_journal.rs`
- `apps/game-server/src/durability/db.rs`
- `apps/game-server/src/durability/mod.rs`
- `apps/game-server/src/durability/schema.rs`
- `apps/game-server/tests/durability_postgres.rs`
- `apps/game-server/tests/support/postgres.rs`
- `apps/game-server/src/lib.rs`

Worker-governance metadata additionally owned after allocation merge:

- `docs/agents/tasks/active/OTV2-20260828-terminal-session-replacement-repair.md`

`apps/game-server/src/lib.rs` is an exclusive serialized composition lease for this task only. The Foundation file is an explicit cross-lane implementation path whose semantics are already frozen by the canonical ADR; the Durability lead may implement those accepted semantics but may not redesign them.

## Mandatory execution order

1. Allocation PR qualifies and squash-merges with expected-head protection.
2. Work reads the merge SHA from protected `main` and creates the worker branch from exactly that SHA.
3. Worker reconstructs the nine frozen PR #243 source files by file contents only. This is baseline reconstruction, not repair qualification.
4. Worker adds fresh repair tests only and proves RED for every new canonical Section 9 terminal-replacement/V2 obligation before adding repair implementation, including every Foundation authorization binding, direct typed V2 replay, exact receipt replay/conflict, predecessor PREPARED fencing, mid-transaction rollback, typed reconciliation and contention.
5. Worker implements the accepted Foundation V2/terminal-replacement contract, PostgreSQL replacement transaction/schema, exact receipt idempotency/conflict handling, predecessor PREPARED fencing, rollback atomicity, typed direct replay/reconciliation and `src/lib.rs` composition.
6. Worker proves GREEN with Foundation contract tests plus real isolated PostgreSQL tests, including restart/replay/contention/late-COMMIT/mid-transaction rollback cases, then whole package/workspace validation.
7. Worker freezes an exact final head, performs whole-diff self-review, requests its own standing-authorized `CODEX_REQUIRED` exact-head review, repairs findings only inside the allocation, and returns `READY_FOR_INTEGRATION` with zero unresolved required threads.
8. Work integrates only after independent verification; Server Seam remains blocked until the repaired Durability delivery is merged and read back from protected main.

## Acceptance criteria

- [ ] Allocation diff is docs-only and exactly the allocation task, worker task and implementation plan.
- [ ] Exact admission base is `a47e15fdc41373e32935b6fea19f51850f655cfc` with `behind_by=0` at merge preflight.
- [ ] Worker source manifest is bound to exact PR #243 head/blob evidence and grants no ancestry/review inheritance.
- [ ] Worker implementation authority is exactly the canonical 11-path set plus its task metadata.
- [ ] `apps/game-server/src/lib.rs` is explicitly serialized and does not overlap a current writer.
- [ ] No Cargo/lockfile/workflow/registry/Server Seam/production/external write authority is introduced.
- [ ] Fresh Foundation RED independently rejects predecessor session/connection/lease and candidate session/account/character/world mismatches before V2 request construction.
- [ ] Fresh Foundation RED separately proves direct typed V2 `ExistingTerminal` collision handling and collision-only bounded fresh-attempt eligibility, not only reconciliation.
- [ ] Fresh PostgreSQL RED proves exact replacement-receipt replay/conflict, predecessor PREPARED late-COMMIT fencing and rollback after replacement mutation has begun but before candidate PREPARED authority.
- [ ] Fresh PostgreSQL/Foundation RED proves collision/concurrent/stale typed outcomes remain distinct and only collision unlocks the existing bounded replacement-attempt path.
- [ ] Allocation receives required exact-head governance/architecture/merge-authority/merge-gate checks, whole-diff self-review, and independent review under current policy before merge.
- [ ] Protected-main readback occurs before any worker write.

## Excluded scope

No runtime code, migration, schema, Cargo/lockfile, workflow, registry/stable-ID, production, secret, live-data, Server Seam/Client/Movement/Combat/gameplay, Platform/Atlas/META or external-repository mutation is permitted in this allocation PR. It creates authority only.

## Validation

### Focused

- command/run: `python tools/agents/validate_governance.py`
- result: pending fresh exact-head workflow evidence after final review repair

### Component/integration

- command/run: `python tools/repository/validate_repository_policy.py`
- result: pending fresh exact-head merge-gate evidence after final review repair

### E2E

- scenario: `NOT_APPLICABLE` — allocation is documentation/authority only
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: resolved from live PR #251 after final tracked review-repair generation
- trigger source: pull_request
- workflow/run/job: pending fresh generation
- runner assignment: GitHub-hosted where applicable
- classification: governance/allocation docs
- result: pending

## Self-review

- exact head: resolved from live PR #251 after final tracked review-repair generation
- method/reviewer: Work coordinator whole-diff review
- material findings: pending fresh review
- verdict: pending

## Independent review

- required: `YES` — allocation changes cross-lane runtime write ownership and serialized composition authority
- exact head: resolved from live PR #251 after final tracked review-repair generation
- method/auditor: fresh strict read-only Codex exact-head review under protected-main policy
- material findings: prior P1 findings `3882998652`, `3883051604`, `3883051614`, `3883051624` repaired; fresh review pending
- verdict: pending

## Context checkpoint

```yaml
last_progress: all four Codex P1 allocation findings are incorporated into complete canonical RED obligations before worker implementation authority can release
status: validating
branch: coord/terminal-session-replacement-allocation-250
head_sha: LIVE_PR_251_HEAD
pr: 251
final_head_sha: LIVE_PR_251_HEAD
final_head_frozen_at: after this final tracked review-repair generation
ci_trigger_source: pull_request
ci_check_generation: pending_new_head
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 2
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: fresh_exact_head_ci_and_independent_review
next_action: run whole-diff Work self-review and fresh exact-head repository CI plus independent Codex review on the unchanged current PR #251 head; if all are clean, expected-head squash-merge the allocation
```
