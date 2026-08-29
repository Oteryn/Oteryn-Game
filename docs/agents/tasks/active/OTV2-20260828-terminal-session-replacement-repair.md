# OTV2-20260828-terminal-session-replacement-repair

```yaml
task_id: OTV2-20260828-terminal-session-replacement-repair
title: Implement terminal GameSession replacement and typed reconciliation
mode: REPAIR
status: qualifying
integration_state: FINAL_EXACT_HEAD_QUALIFICATION_PENDING
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
integration_main_sha: 138c5add957718bd26149820626e538068a35a58
main_reconciliation_merge_sha: fc27d64f5803c15e70d78e6e7a0f9cd63980f89f
pr: 252
owner: Oteryn: sol durability lead
created_at: 2026-08-28T20:08:00+02:00
updated_at: 2026-08-29T18:25:29+02:00
execution_budget_minutes: 180
large_budget_reason: cross-lane Foundation/Durability repair with real PostgreSQL contention, replay and restart proof
write_authority: exact_allocated_worker_scope_after_12d4ca5326d62a7a2c46d80cd5e167e99f109d1d
source_snapshot_mode: COPY_FILE_CONTENTS_ONLY_NO_COMMIT_ANCESTRY_NO_REVIEW_OR_CI_INHERITANCE
serialized_composition_lease: apps/game-server/src/lib.rs
remote_desktop_used: exception_isolated_terminal_edit_and_local_tests_only
remote_desktop_production_or_live_access: false
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

## Proven TDD and review-repair checkpoints

- Schema/receipt/live-anchor RED: `b9f1cda4d3693158eb2224208b51696e1bdb2766`, workflow run `33201919633`, PostgreSQL job `98953210787`: 49 PASS / 8 expected FAIL.
- Foundation V2/terminal-authorization RED: `bff218dcc35a19b10c0a3dc1dbbc78e2cb41b306`, workflow run `33202516767`, job `98955532165`: expected missing canonical Foundation contract.
- Complete runtime PostgreSQL RED: `0fc9de255394ba4ce1b919ad71ea47eeb3247e05`, workflow run `33204365030`, PostgreSQL job `98961526112`: 72 PASS / 6 expected FAIL on PostgreSQL 17.6.
- Runtime GREEN before final composition: `aea85c41268f62486a45e37ed6142cd684ad89df`, workflow run `33207651906`, PostgreSQL job `98972703970`: 78 PASS / 0 FAIL on PostgreSQL 17.6.
- Final composition generation `560eb1d30ad94986b9af3375735c3380b76d7070` passed Rust workspace `33208062237`, Architecture semantic audit `33208062163`, Merge authority audit `33208062198`, Agent governance `33208062254`, and Merge gate `33208062302`.
- Receipt-bound reconciliation RED was preserved at `4530617223e93970533eb15e5997c7c0296ce471`: Rust run `33209828606`, PostgreSQL job `98980086302`, 78 PASS / 1 expected FAIL in `runtime_reconciliation_requires_exact_replacement_receipt_binding`.
- Native exact-head review on `4530617223e93970533eb15e5997c7c0296ce471` reported two P1 findings: missing V2 final-revalidation/COMMIT progression and missing exact persisted replacement-receipt binding during reconciliation.
- V2 final-revalidation/COMMIT progression repair landed in `1a2d7e53168fb87d26fc2185be49e0b5f51ac592`.
- Exact persisted replacement-receipt reconciliation repair landed in `b88d8bf7ff0838b03b9462374b291119e1f947c8`.
- Whole-diff self-review found a further local projection defect: reconciled durable `Prepared` left the exact local attempt budget `Reserved`. RED head `ef0963b36696daa09475371d64ff16ab4383eb3b`, Merge gate run `33212142814`, Linux workspace job `98987719428`: build and strict Clippy PASS; workspace tests 188 PASS / 1 expected FAIL in `v2_reconciled_prepared_budget_regression_tests::reconciled_prepared_marks_the_attempt_prepared_in_the_local_budget`.
- Budget projection GREEN landed in `01d663035d62af87c8b4979b543b7d547bbdec32`. Rust workspace run `33236163526` completed SUCCESS; PostgreSQL 17.6 job `99057409787` verified the exact SHA and completed 79 PASS / 0 FAIL. Exact-head Merge authority audit `33236163535`, Agent governance `33236163536`, and Architecture semantic audit `33236163547` also completed SUCCESS.
- Protected `main@0135deb100109b910dada366d7a1b05484357e51` was reconciled non-destructively into the worker branch by merge commit `fc27d64f5803c15e70d78e6e7a0f9cd63980f89f`; no rebase, reset or force-push was used.
- A fresh review after reconciliation produced three P2 findings: exact receipt replay after losing the predecessor-lock race, one-snapshot V2 reconciliation across concurrent COMMIT, and full typed/session mirror validation before terminal outcome mapping.
- Race/structural reconciliation RED was preserved at `a88c0b78633e1999006d1bebb72790780d926752`: Rust run `33239070718`, PostgreSQL 17.6 job `99065141820`, exact checkout verified, 79 PASS / 4 expected FAIL. The four failures were exactly the newly added deterministic regressions; all prior tests stayed GREEN.
- The three P2 repairs landed in `d54d9cf5e70f4b6b081ef384afe4a4a70e270c76`: missing-predecessor exact receipt replay, shared transactional V1/V2 reconciliation, and full structural validation for every typed V2 outcome.
- Local final code validation on `d54d9cf5e70f4b6b081ef384afe4a4a70e270c76`: formatting PASS, strict affected-target Clippy PASS, game-server library 193 PASS / 0 FAIL, and isolated PostgreSQL 17.6 83 PASS / 0 FAIL.
- Hosted code GREEN on the same exact SHA: Rust run `33239666334`, PostgreSQL 17.6 job `99066708516`, exact checkout verified, 83 PASS / 0 FAIL. Architecture semantic audit `33239666333`, Merge authority audit `33239666378`, and Agent governance `33239666336` completed SUCCESS. All three P2 threads have exact repair replies and are resolved.
- Fresh native review on `99b1c13ebfe4edef120e5c89b3c3bf9dfe15114d` reported two P1 findings: the actor/epoch eight-attempt budget was reset across terminal GameSession replacement, and V2 final revalidation had no public way to supply actual mutable current authority facts.
- Attempt-budget RED was preserved at `c95962ce7debb28a35e137476397bffa38990ab6`: Rust run `33260072109`, PostgreSQL 17.6 job `99120370411`, exact checkout verified, 83 PASS / 1 expected FAIL in `runtime_terminal_replacement_preserves_actor_epoch_attempt_budget` (`attempt_count` 1 instead of 8).
- Current-facts RED was preserved at `2b51ebe64a83725036c993ca0bb862e632df5f4d`: Rust run `33260242289`, PostgreSQL job `99120815389`, exact checkout verified, expected compile failure `E0599` proving `ReconnectCurrentAuthorityV1::from_current_facts` was absent.
- Control-plane follow-up RED `9ec8342f7d9979f3b059d27059cf74338090a991` froze `runtime_terminal_replacement_rejects_exhausted_same_epoch_before_mutation`; its hosted PostgreSQL run `33260416224` / job `99121281797` remained blocked earlier by the already-proven current-facts `E0599`, so no separate runtime-failure claim is made for that test.
- Both P1 repairs and the fail-before-mutation ordering repair are code-GREEN at `3f8cf2ac48f3830f623b00dea901c74ea7b875f1`: the unchanged actor/epoch cap of eight is retained across session replacement, exhaustion rejects before predecessor/receipt/candidate mutation, and Foundation exposes an explicit current-facts snapshot constructor while keeping strict final comparison.
- Local validation on `3f8cf2ac48f3830f623b00dea901c74ea7b875f1`: game-server library 196 PASS / 0 FAIL, full Durability PostgreSQL target compiles, strict game-server all-target Clippy with `-D warnings` PASS, and `git diff --check` PASS.
- Hosted GREEN on exact `3f8cf2ac48f3830f623b00dea901c74ea7b875f1`: Rust run `33260542415`, PostgreSQL 17.6 job `99121611106`, exact checkout verified, 86 PASS / 0 FAIL including both attempt-budget regressions and `v2_final_revalidation_accepts_external_current_facts_and_rejects_changed_authority`; Architecture semantic audit `33260542379`, Merge authority audit `33260542309`, Agent governance `33260542389`, and Windows SIM golden job `99121611059` completed SUCCESS.
- Both fresh P1 review threads were replied to with exact GREEN evidence and resolved on `3f8cf2ac48f3830f623b00dea901c74ea7b875f1`; the integration-gate thread remains intentionally unresolved for the control plane.
- Remote Desktop was used only as an exception for an isolated temporary terminal checkout, precise edits and local Docker/PostgreSQL tests after container DNS and connector patch limitations blocked safer execution. No production/live system, database, secret or persistent user workspace was accessed.
- In the final P1 repair cycle, Remote Desktop was used only for the same isolated `%TEMP%` checkout, surgical Rust/metadata edits and local Rust compile/test/Clippy checks because the GitHub connector could not apply small patches to the large owned files. Local Docker was not started in this cycle; PostgreSQL 17.6 runtime proof came from hosted exact-head CI.

- Fresh native Codex review on metadata-complete head `b8579d26605b600fdb1031a8aa9fa05ca834d63f` reported two additional P1 findings: terminal replacement did not bind the candidate to the predecessor's current `ControlLossEpoch`/original grace deadline, and the actor-wide eight-attempt cap was not serialized across terminal predecessor and candidate GameSession rows.
- Fresh RED for both findings was preserved at `1cceaff4fdcc1d6a0ec2ef9355904c9de0043f8f`: Rust run `33261749626`, PostgreSQL 17.6 job `99124774176`, exact checkout verified, 86 PASS / 3 expected FAIL. The failures were exactly candidate loss-epoch mismatch, candidate original-grace mismatch, and delayed predecessor attempt 9 escaping the actor/epoch cap.
- Initial GREEN `f3f63855c5dd71883160e4e20f5481d433e0cade` added current continuity to the Foundation terminal snapshot/authorization, forbade V2 replacement from creating a missing continuity row, bound the locked predecessor row to that continuity, and introduced a shared actor/epoch `FOR UPDATE` attempt-budget lock/count for V1 and V2. Local game-server library tests were 199/199 PASS and strict all-target game-server Clippy passed.
- Hosted validation on `f3f63855c5dd71883160e4e20f5481d433e0cade` intentionally was not accepted as GREEN: Rust run `33262069313`, PostgreSQL job `99125600439` exposed one existing-regression failure where the shared lock helper classified a same-epoch changed-grace request as `InvalidStoredState` before the canonical V1 path could return `RejectedStaleAuthority`.
- Minimal regression repair `670e5028a5e11be3761c392c795717b324e853a0` keeps the same actor/epoch row lock and count but leaves authority/continuity classification to the existing V1/V2 paths. Exact hosted Rust run `33262693593` is SUCCESS; PostgreSQL 17.6 job `99127246281` verified the exact SHA and completed 89 PASS / 0 FAIL, including the two continuity regressions, delayed-predecessor attempt-cap regression, and `committed_session_accepts_a_later_non_reused_control_loss_epoch`. Architecture semantic audit `33262693580`, Merge authority audit `33262693572`, and Agent governance `33262693604` are SUCCESS; Merge gate generation `33262693577` is the matching exact-head generation.
- Whole-diff guard on `670e5028a5e11be3761c392c795717b324e853a0`: protected `main@138c5add957718bd26149820626e538068a35a58` is an ancestor (`behind_by=0`), the effective diff is exactly the 12 allocated paths, `git diff --check` passes, and the isolated working tree is clean.
- Both fresh P1 review threads received exact RED/GREEN evidence and are resolved on `670e5028a5e11be3761c392c795717b324e853a0`; the integration-gate thread remains intentionally unresolved for final metadata-head CI/self-review/Codex and control-plane pre-merge verification.

Any tracked commit moves the PR head and invalidates prior exact-head qualification as final integration proof. The metadata-complete head produced from this update is the next candidate freeze generation and requires fresh exact-head CI, whole-diff self-review and native Codex review.

## Required validation

### Focused RED -> GREEN

- Foundation exact tests in `admission_recovery_inner.rs` for terminal lifecycle, current scope, every predecessor/candidate constructor binding, generic V1 fallback, direct typed V2 replay, reconciliation, commit progression, local prepared projection and collision-only fresh-attempt eligibility.
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
last_progress: fresh Codex P1s from b8579d26605b600fdb1031a8aa9fa05ca834d63f were proven by exact RED 1cceaff4fdcc1d6a0ec2ef9355904c9de0043f8f and repaired through final code GREEN 670e5028a5e11be3761c392c795717b324e853a0 with hosted PostgreSQL 17.6 89/89 PASS
status: qualifying
integration_state: FINAL_EXACT_HEAD_QUALIFICATION_PENDING
branch: impl/game-terminal-session-replacement-250
admission_main_sha: a47e15fdc41373e32935b6fea19f51850f655cfc
integration_main_sha: 138c5add957718bd26149820626e538068a35a58
main_reconciliation_merge_sha: fc27d64f5803c15e70d78e6e7a0f9cd63980f89f
previous_code_green_head: 670e5028a5e11be3761c392c795717b324e853a0
head_sha: resolve_live_branch_after_this_metadata_commit
pr: 252
final_head_sha: resolve_live_branch_after_this_metadata_commit
final_head_frozen_at: 2026-08-29T18:25:29+02:00
ci_trigger_source: pull_request
ci_check_generation: metadata_complete_head_requires_fresh_exact_head_generation
ci_run_ids:
  final_review_p1_red_rust: 33261749626
  initial_final_review_green_rust: 33262069313
  final_code_green_rust: 33262693593
  final_code_green_architecture: 33262693580
  final_code_green_merge_authority: 33262693572
  final_code_green_governance: 33262693604
  final_code_green_merge_gate: 33262693577
ci_job_ids:
  final_review_p1_red_postgres: 99124774176
  initial_final_review_green_postgres_regression: 99125600439
  final_code_green_postgres: 99127246281
local_validation:
  game_server_lib: 199_pass_0_fail
  durability_postgres_compile: pass
  strict_game_server_clippy_all_targets_deny_warnings: pass
  diff_check: pass
hosted_green_validation:
  postgres_17_6: 89_pass_0_fail
  exact_checkout: pass
  architecture_semantic_audit: pass
  merge_authority_audit: pass
  agent_governance: pass
  merge_gate_generation: 33262693577
whole_diff:
  allocated_paths: 12_of_12
  behind_by: 0
  diff_check: pass
runner_assignment_state: hosted_exact_head_available
terminal_ci_wait_started_at: 2026-08-29T18:25:29+02:00
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 7
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: fresh exact-head CI on this metadata-complete candidate, exact-head whole-diff self-review, fresh native Codex review, and control-plane integration-gate reconciliation before squash merge
next_action: commit this metadata checkpoint, freeze/read back the live exact head, require all exact-head checks, perform whole-diff self-review, request fresh `@codex review`, repair any findings if needed, then issue `READY_FOR_INTEGRATION` for independent control-plane preflight and expected-head squash merge
```

## Runtime-scope final qualification checkpoint — 2026-08-30

This checkpoint supersedes the Context checkpoint above for the current final qualification generation. It records the last review-repair cycle before the metadata-complete exact-head freeze.

- Fresh native Codex review on `5d3d6f9cd4e7621a3f36619868f34cc2d6f6c10d` reported a P1 because terminal replacement authorization and final COMMIT revalidation could not represent and compare the actual current `RuntimeScopeRefV1` identity (Channel/Instance) independently of WorldId and scope generation.
- Initial test-only RED `85310158d0d46295cbb4bc10a764884a0583fdc3` was proven by Merge Gate Linux workspace job `99129278992`: exact-head all-target build failed with `E0599` because `ReconnectCurrentAuthorityV1::with_current_runtime_scope` did not exist.
- Executed two-boundary test-only RED `e0c981c750b8c5b80ff4d27d05cc2ff715cd2d66` was preserved in the hosted PostgreSQL target surface. Rust run `33265511987`, PostgreSQL 17.6 job `99134752795`: exact checkout was verified and compilation failed before GREEN, with both runtime-scope regressions present in `apps/game-server/src/durability/db.rs`.
- Production GREEN `613f9aef9926d5aee47a9a7c1c5fd20744c3a607` added explicit current runtime scope to `GameSessionAuthoritySnapshot` and `ReconnectCurrentAuthorityV1`, requires exact candidate/current scope identity for terminal replacement, and leaves final revalidation fail-closed through full current-authority equality.
- Lint/format-only follow-ups `79a366642eb29db74297963b2f607e04877d567f` and `1e189e6646a4c41370964146f6d740385fc47506` preserved the same production semantics.
- Exact hosted GREEN on `1e189e6646a4c41370964146f6d740385fc47506`: Rust run `33266293045`, PostgreSQL 17.6 job `99136831009`, exact checkout, 91 PASS / 0 FAIL, including `final_revalidation_can_supply_actual_current_runtime_scope_and_reject_drift` and `terminal_replacement_rejects_same_world_generation_different_current_runtime_scope`. Architecture semantic audit `33266293030`, Merge authority audit `33266293040`, Agent governance `33266293049`, Merge gate `33266293035`, and aggregate `game-gate` job `99137774423` all completed SUCCESS.
- Whole-diff pre-metadata guard on `1e189e6646a4c41370964146f6d740385fc47506`: protected `main@138c5add957718bd26149820626e538068a35a58` is the merge base, `behind_by=0`, and the effective diff is exactly the allocated 12 paths.

```yaml
last_progress: runtime-scope P1 from final review on 5d3d6f9cd4e7621a3f36619868f34cc2d6f6c10d was proven by hosted RED 85310158d0d46295cbb4bc10a764884a0583fdc3 and e0c981c750b8c5b80ff4d27d05cc2ff715cd2d66, repaired in 613f9aef9926d5aee47a9a7c1c5fd20744c3a607, and code-GREEN at 1e189e6646a4c41370964146f6d740385fc47506 with PostgreSQL 17.6 91/91 PASS
status: qualifying
integration_state: FINAL_EXACT_HEAD_QUALIFICATION_PENDING
branch: impl/game-terminal-session-replacement-250
integration_main_sha: 138c5add957718bd26149820626e538068a35a58
previous_code_green_head: 1e189e6646a4c41370964146f6d740385fc47506
head_sha: resolve_live_branch_after_this_metadata_commit
final_head_sha: resolve_live_branch_after_this_metadata_commit
final_head_frozen_at: 2026-08-30T00:24:39+02:00
runtime_scope_p1:
  reviewed_head: 5d3d6f9cd4e7621a3f36619868f34cc2d6f6c10d
  initial_red_head: 85310158d0d46295cbb4bc10a764884a0583fdc3
  initial_red_merge_gate_linux_job: 99129278992
  executed_red_head: e0c981c750b8c5b80ff4d27d05cc2ff715cd2d66
  executed_red_rust_run: 33265511987
  executed_red_postgres_job: 99134752795
  production_green_head: 613f9aef9926d5aee47a9a7c1c5fd20744c3a607
  final_code_green_head: 1e189e6646a4c41370964146f6d740385fc47506
final_code_green_ci:
  rust: 33266293045
  postgres_job: 99136831009
  postgres_result: 91_pass_0_fail
  architecture: 33266293030
  merge_authority: 33266293040
  governance: 33266293049
  merge_gate: 33266293035
  game_gate_job: 99137774423
whole_diff_pre_metadata:
  main_sha: 138c5add957718bd26149820626e538068a35a58
  behind_by: 0
  allocated_paths: 12_of_12
owner_action_required: null
blocker: fresh exact-head CI on the metadata-complete candidate, exact-head whole-diff self-review, fresh native Codex review, runtime-scope P1 resolution, READY_FOR_INTEGRATION handoff, and independent control-plane pre-merge verification
next_action: freeze/read back the live metadata-complete head, require all exact-head checks, perform whole-diff self-review, request fresh @codex review, repair any findings if needed, resolve the repaired runtime-scope P1, issue READY_FOR_INTEGRATION, then let Work independently verify and expected-head squash merge without bypass
```
