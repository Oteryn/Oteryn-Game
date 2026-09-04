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
pr: 252
final_final_head_frozen_at: 2026-08-29T18:25:29+02:00
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
final_final_head_frozen_at: 2026-08-30T00:24:39+02:00
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

## AccountPresence final-revalidation repair checkpoint — 2026-09-03

This checkpoint supersedes every earlier Context/final-qualification checkpoint for current task status. Earlier SHAs and CI runs remain historical repair provenance only; in particular, `1e189e6646a4c41370964146f6d740385fc47506` is not the latest qualified candidate.

- Final exact-head Codex review on `d0106cda44cdffc758a93f669a7dfb9dee625b76` reported two P1 findings: current AccountPresence was not independently represented during direct final COMMIT or lost-COMMIT `Committed` reconciliation, and this active task checkpoint was stale.
- Fresh executable test-only RED is preserved at `e585c50b1eb5f9f64c8a32a6f189e4c989c658e9`. The focused game-server test command exited 101 because `AccountPresenceClaimV1` and the required AccountPresence argument to `ReconnectCurrentAuthorityV1::from_current_facts(...)` did not exist.
- Production GREEN is `906170dcabe68ab1fb329e7f3e36c717f6a04eae`. Foundation now exposes the typed `AccountPresenceClaimV1`, accepts the independently observed current claim as an optional current fact (`None` means released), and includes it in the complete exact current-authority comparison shared by direct V2 final COMMIT authorization and reconciled `Committed` controller installation.
- The focused regression proves that the exact current account/character claim authorizes, while a released claim and a claim reassigned to another CharacterId both fail closed even though the separate account-security generation remains unchanged. All previously required runtime-scope, connection, continuity, proof, FND-02, compatibility/security, session/controller and expiry checks remain in the same complete comparison/authorization paths.
- The repair remains inside the existing 12-path allocation. The code generation changed only four already allocated files; this metadata checkpoint changes only this allocated task record.

```yaml
status: qualifying
integration_state: FINAL_EXACT_HEAD_QUALIFICATION_PENDING
branch: impl/game-terminal-session-replacement-250
pr: 252
integration_main_sha: f5f8e3717a48e6854ac36595533046938ceec890
reviewed_parent_head: d0106cda44cdffc758a93f669a7dfb9dee625b76
account_presence_red_head: e585c50b1eb5f9f64c8a32a6f189e4c989c658e9
account_presence_code_green_head: 906170dcabe68ab1fb329e7f3e36c717f6a04eae
reviewed_exact_head: 615d5c4f12a734f7d2257af6f0455d30fbf6cb6e
checkpoint_commit_scope: docs_only_after_account_presence_code_green
focused_red:
  command: cargo +1.94.0 test --locked -p oteryn-game-server v2_final_authority_revalidation_requires_current_account_presence
  result: expected_compile_failure_exit_101
focused_green:
  command: cargo +1.94.0 test --locked -p oteryn-game-server v2_final_authority_revalidation_requires_current_account_presence
  result: pass
review_disposition:
  account_presence_p1: repaired_pending_fresh_exact_head_review
  stale_task_checkpoint_p1: repaired_by_this_checkpoint
  ready_for_integration: false
owner_action_required: null
blocker: fresh exact-head hosted CI, whole-diff self-review, fresh independent Codex review, and control-plane integration reconciliation remain required
next_action: publish this metadata-complete repair generation, verify the remote exact head, then let the coordinator run fresh exact-head hosted qualification and independent Codex review; do not mark READY_FOR_INTEGRATION or resolve review threads
```

## Final-review repair handoff checkpoint — 2026-09-03

This section supersedes all prior current-status checkpoints for continuation state; older checkpoints and their evidence remain historical provenance. The task is `REPAIR_REQUIRED`, is not ready for integration, and must be resumed from a fresh read of the live branch head because this docs-only checkpoint commit moves the PR head. It is not itself a reviewed or exact-head-qualified final candidate.

```yaml
status: REPAIR_REQUIRED
integration_state: NOT_READY_FOR_INTEGRATION
issue: 250
pr: 252
branch: impl/game-terminal-session-replacement-250
pr_state: open_draft_mergeable_unmerged
protected_main: f5f8e3717a48e6854ac36595533046938ceec890
reviewed_exact_head: 615d5c4f12a734f7d2257af6f0455d30fbf6cb6e
final_codex_review:
  submission: PRR_kwDOT8SzxM8AAAABMEC8gg
  reviewed_head: 615d5c4f12a734f7d2257af6f0455d30fbf6cb6e
account_presence_generation:
  red_head: e585c50b1eb5f9f64c8a32a6f189e4c989c658e9
  green_head: 906170dcabe68ab1fb329e7f3e36c717f6a04eae
  metadata_head_reviewed: 615d5c4f12a734f7d2257af6f0455d30fbf6cb6e
reviewed_head_qualification:
  rust_run: 33779042687_SUCCESS
  postgres_17_6_job: 100727892790_SUCCESS
  architecture_run: 33779042944_SUCCESS
  agent_governance_run: 33779042937_SUCCESS
  merge_gate_run: 33779043078_SUCCESS
changed_paths:
  - apps/game-server/build.rs
  - apps/game-server/migrations/0001_admission_reconnect_journal.sql
  - apps/game-server/src/bin/oteryn-game-migrate.rs
  - apps/game-server/src/durability/admission_journal.rs
  - apps/game-server/src/durability/db.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/durability/schema.rs
  - apps/game-server/src/foundation/admission_recovery_inner.rs
  - apps/game-server/src/lib.rs
  - apps/game-server/tests/durability_postgres.rs
  - apps/game-server/tests/support/postgres.rs
  - docs/agents/tasks/active/OTV2-20260828-terminal-session-replacement-repair.md
actionable_findings:
  - thread: PRRT_kwDOT8SzxM6e_-dm
    comment: 3926687258
    severity: P1
    requirement: independently current CharacterId-to-WorldId eligibility must flow through terminal-replacement PREPARE authority and final COMMIT/lost-COMMIT reconciliation
  - thread: PRRT_kwDOT8SzxM6e_-dt
    comment: 3926687267
    severity: P1
    requirement: current reconnect-candidate existence, liveness, expiry, attempt identity, and transport binding must be verified before COMMIT/reconciliation so stale, disappeared, or rebound candidate authority fails closed
  - thread: PRRT_kwDOT8SzxM6e_-dy
    comment: 3926687275
    severity: P1
    requirement: authoritative task metadata must identify the immutable reviewed/frozen SHA rather than movable HEAD semantics
execution_strategy: one primary high-reasoning lead owns the coherent current-authority design and all core mutations; parallel helpers may inspect, test, or review only
scope_guard: preserve the existing 12-path allocation; expected repairs stay within allocated Foundation/test surfaces; no Server Seam, gameplay, external repository, production/live data, resource maxima, Cargo/lockfile, or workflow changes
new_path_policy: SHARED_LEASE_REQUIRED_STOP_FOR_CONTROL_PLANE_EXPANSION
thread_policy: older technical threads remain intentionally unresolved until a fresh repaired generation is independently qualified; do not resolve them during continuation, and keep control-plane-owned integration gate PRRT_kwDOT8SzxM6dX3eH unresolved
checkpoint_commit_qualification: this docs-only commit advances the PR head and is not final exact-head qualification evidence
next_action: the next single lead agent creates fresh executable RED coverage for both coupled Foundation P1s, implements the minimal coherent GREEN within allocation, updates metadata accurately, native-publishes a new head, then reruns fresh exact-head hosted CI, whole-diff self-review, and independent Codex review; only zero unresolved actionable P0/P1/P2 can proceed to READY_FOR_INTEGRATION
```

## Coupled current-authority P1 repair checkpoint — 2026-09-03

This checkpoint supersedes the Final-review repair handoff checkpoint above for current execution state. It freezes the completed code generation by immutable SHA; it does not use movable `HEAD` semantics and it does not claim that the metadata commit containing this checkpoint has already been reviewed.

- The two coupled Foundation P1s from exact Codex review head `615d5c4f12a734f7d2257af6f0455d30fbf6cb6e` were treated as one current-authority repair: independently observed `CharacterId -> WorldId` eligibility plus independently observed live reconnect-candidate binding.
- Fresh executable RED is `14dc17d794a67af34226f3ec5dd95bf03ab744ca`; Rust run `33783771444`, Durability PostgreSQL harness job `100743493058` failed compilation as expected because `CharacterWorldEligibilityClaimV1`, `ReconnectCandidateBindingV1`, and their current-facts API did not yet exist.
- Production GREEN is `9c2fc39cc62aafd83083996524c554f3f34317f0`. `GameSessionAuthoritySnapshot` now independently requires current character/world eligibility for terminal-replacement authority, while `ReconnectCurrentAuthorityV1` independently carries both current character/world eligibility and the exact current candidate session/attempt/generation/transport/deadline binding. Direct final COMMIT and committed reconciliation share the complete fail-closed current-authority predicate.
- Test-helper/API fallout was repaired only inside already allocated paths: schema helper `89eafea6fba087349fcdfa8c7d5351a8f2f610ed`, durability helper `60abcc3d3c9d119a8df40a17a7f5b50171176301`, exact rustfmt-only commits `a9d1512976c890136a3b9630c79fbbc0c2305fae` and `c53b3d4b1b2e7157a5f3842e3d4e01ba82700665`, then the single downstream game-server test helper exposed by full workspace compilation was updated in `1da3beed55086aa375dbd47ef6cbab5fb23a8a1b` to supply the same two independent current facts.
- On immutable code-complete candidate `1da3beed55086aa375dbd47ef6cbab5fb23a8a1b`, Rust workspace run `33787753024` completed SUCCESS, including Durability PostgreSQL harness job `100756554709` and Windows SIM exact-head job `100756554993`. Architecture semantic audit `33787752822` and Agent governance `33787752902` completed SUCCESS. Merge Gate generation `33787752841` had already passed exact scope, formatting/policy/metadata, dependency review, governance, CodeQL actions/python, supply-chain, and Linux workspace build before this metadata freeze; its remaining long-running lanes are not reused as final proof because this metadata commit changes the exact head.
- Scope remains exactly the original 12 allocated paths. No force-push, rebase, reset, merge, Cargo/lockfile/workflow/resource-registry, gameplay, production/live-data, secret, external-repository, or resource-maximum change was introduced.

```yaml
status: qualifying
integration_state: FINAL_EXACT_HEAD_QUALIFICATION_PENDING
issue: 250
pr: 252
branch: impl/game-terminal-session-replacement-250
pr_state: open_draft_mergeable_unmerged
protected_main: f5f8e3717a48e6854ac36595533046938ceec890
reviewed_source_sha: 615d5c4f12a734f7d2257af6f0455d30fbf6cb6e
immutable_code_candidate_sha: 1da3beed55086aa375dbd47ef6cbab5fb23a8a1b
coupled_foundation_p1_red:
  head: 14dc17d794a67af34226f3ec5dd95bf03ab744ca
  rust_run: 33783771444
  postgres_job: 100743493058
  result: expected_compile_failure_exit_101
coupled_foundation_p1_green:
  production_head: 9c2fc39cc62aafd83083996524c554f3f34317f0
  schema_helper_head: 89eafea6fba087349fcdfa8c7d5351a8f2f610ed
  durability_helper_head: 60abcc3d3c9d119a8df40a17a7f5b50171176301
  db_fmt_head: a9d1512976c890136a3b9630c79fbbc0c2305fae
  mod_fmt_head: c53b3d4b1b2e7157a5f3842e3d4e01ba82700665
  code_complete_head: 1da3beed55086aa375dbd47ef6cbab5fb23a8a1b
code_complete_ci:
  rust_run: 33787753024_SUCCESS
  postgres_job: 100756554709_SUCCESS
  windows_sim_job: 100756554993_SUCCESS
  architecture_run: 33787752822_SUCCESS
  agent_governance_run: 33787752902_SUCCESS
  merge_gate_generation: 33787752841_PRE_METADATA_ONLY_NOT_FINAL_PROOF
whole_diff_pre_metadata:
  allocated_paths: 12_of_12
  protected_main: f5f8e3717a48e6854ac36595533046938ceec890
review_disposition:
  character_world_p1: code_repaired_pending_fresh_exact_head_codex
  live_candidate_p1: code_repaired_pending_fresh_exact_head_codex
  metadata_p1: repaired_by_immutable_code_candidate_record
  ready_for_integration: false
thread_policy: do not resolve technical review threads or the control-plane integration gate before metadata-complete exact-head CI, whole-diff self-review, and fresh exact-head Codex produce P0=0/P1=0/P2=0
owner_action_required: null
blocker: metadata-complete exact-head CI, whole-diff self-review, fresh independent exact-head Codex review, and zero actionable P0/P1/P2 are still required
next_action: read back the metadata commit SHA as the only final qualification candidate; run fresh exact-head hosted CI on that SHA, perform whole-diff self-review, request fresh native @codex review and require its reviewed commit to equal that SHA; repair and repeat on any P0/P1/P2; do not merge
```

## Authorization-observation deadline repair checkpoint — 2026-09-03

This docs-only checkpoint supersedes earlier current-status checkpoints for continuation state without rewriting or removing their historical evidence. The superseded independently reviewed metadata head was `8b3b218ce95a615d8804d8af6a7ddb87067dc3e7`. Codex review `PRR_kwDOT8SzxM8AAAABMEwVFg` reviewed commit `8b3b218ce9`; its latest P1 is thread `PRRT_kwDOT8SzxM6fBol3`, comment `3927341659`. The finding was that direct V2 `authorize_commit` checked the authorization deadline only against caller-supplied `now`, not the actual authority observation time `current.observed_at`.

- Fresh test-only RED is `35578c2cf048eae99f08f79f8ed64319e48d611e`. Merge Gate run `33791866970`, Linux workspace job `100770156188`, verified the exact checkout; build and strict Clippy passed, then workspace tests produced 214 PASS / 1 expected FAIL. The sole failure was `final_revalidation_rejects_authority_observed_after_authorization_deadline`: actual `Ok` with `authorization_deadline=105`, `current.observed_at=106`, and `now=104`, where `DeadlineExpired` was expected.
- Minimal production GREEN is `7da8a037a3870f858c4538c3909a8ba85c1875fb`. Its single semantic change is `if now > deadline || current.observed_at > deadline`.
- Exact code-GREEN Rust run `33792514763` completed SUCCESS; PostgreSQL job `100772249645` completed SUCCESS; Windows SIM job `100772249321` completed SUCCESS; Architecture run `33792514818` completed SUCCESS; Agent governance run `33792514823` completed SUCCESS.
- Protected `main` remains `f5f8e3717a48e6854ac36595533046938ceec890`. Scope remains exactly the 12 allocated paths. No force-push, rebase, reset, merge, or scope expansion occurred.

```yaml
status: qualifying
integration_state: FINAL_EXACT_HEAD_QUALIFICATION_PENDING
ready_for_integration: false
issue: 250
pr: 252
branch: impl/game-terminal-session-replacement-250
pr_state: open_draft_unmerged
protected_main: f5f8e3717a48e6854ac36595533046938ceec890
superseded_independently_reviewed_metadata_head: 8b3b218ce95a615d8804d8af6a7ddb87067dc3e7
codex_review:
  submission: PRR_kwDOT8SzxM8AAAABMEwVFg
  reviewed_commit: 8b3b218ce9
  latest_p1_thread: PRRT_kwDOT8SzxM6fBol3
  latest_p1_comment: 3927341659
deadline_red:
  head: 35578c2cf048eae99f08f79f8ed64319e48d611e
  merge_gate_run: 33791866970
  linux_workspace_job: 100770156188
  result: 214_pass_1_expected_fail
deadline_green:
  code_head: 7da8a037a3870f858c4538c3909a8ba85c1875fb
  semantic_change: now_gt_deadline_or_current_observed_at_gt_deadline
code_green_ci:
  rust_run: 33792514763_SUCCESS
  postgres_job: 100772249645_SUCCESS
  windows_sim_job: 100772249321_SUCCESS
  architecture_run: 33792514818_SUCCESS
  agent_governance_run: 33792514823_SUCCESS
allocated_paths: 12_of_12
history_operations: no_force_no_rebase_no_reset_no_merge
checkpoint_commit_qualification: the docs-only commit is not final qualification evidence; after publication its remote SHA must be read back and becomes the only final exact-head candidate
thread_policy: do not resolve threads, mark READY, change PR Draft state, or merge
next_action: read back the published docs-only remote SHA, then run fresh exact-head CI, whole-diff self-review, and fresh independent Codex review on that immutable SHA
```

## Terminal-replacement PREPARE AccountPresence repair checkpoint — 2026-09-03

Minimal GREEN adds `current_account_presence: Option<&AccountPresenceClaimV1>` to `TerminalGameSessionReplacementAuthorizationV1::from_current_authority`; `None`, account mismatch, or CharacterId reassignment fails closed; existing Character->World, runtime-scope, lease, connection, continuity and candidate checks remain unchanged; same-commit non-production edits are mechanical callsite/rustfmt changes within allocation.

```yaml
status: qualifying
integration_state: FINAL_EXACT_HEAD_QUALIFICATION_PENDING
ready_for_integration: false
reviewed_parent_head: 0fc9406476755acd73998d7465475b4d841678f8
p1_thread: PRRT_kwDOT8SzxM6fCnig
p1_comment: 3927729050
requirement: terminal-replacement PREPARE independently revalidates current AccountId-to-CharacterId AccountPresence
red_head: 57396c7027db0cf838510e0e950088c1ae78da48
red_rust_run: 33794233927
red_postgres_job: 100778335019
red_result: expected_E0061_E0308_compile_failure
code_green_head: 8e93a5c91a3c5b95d53ebeb8a6c244d228e1e83c
code_green_rust_run: 33794954483_SUCCESS
code_green_postgres_job: 100780292724_SUCCESS_103_pass_0_fail
code_green_windows_sim_job: 100780292535_SUCCESS
code_green_architecture_run: 33794954424_SUCCESS
code_green_governance_run: 33794954418_SUCCESS
code_green_merge_gate_run: 33794954547_SUCCESS
code_green_game_gate_job: 100783044583_SUCCESS
protected_main: f5f8e3717a48e6854ac36595533046938ceec890
allocated_paths: 12_of_12
history_operations: no_force_no_rebase_no_reset_no_merge
thread_policy: do_not_resolve_or_mark_READY_before_fresh_metadata_head_CI_self_review_and_exact_head_Codex_P0_P1_P2_zero
checkpoint_commit_qualification: remote SHA of this docs-only publication must be read back after push and becomes the sole final qualification candidate; do not put a self-referential SHA placeholder in this section
next_action: fresh exact-head hosted CI, whole-diff self-review, and independent exact-head @codex review on the read-back metadata-complete SHA; repair and repeat any P0/P1/P2; do not merge
```

## Final two-P1 repair checkpoint — 2026-09-03

This checkpoint supersedes all earlier current-status checkpoints for the live continuation state while retaining them as historical provenance. Fresh native Codex review on exact head `8a02d7825722a897ce0e52b63dc6238bd64ba1c9` produced two additional P1 findings: a historical terminal-replacement receipt permanently blocked a genuinely later ordinary reconnect on the replacement-created GameSession, and final current-authority revalidation did not independently carry the current `original_grace_deadline`.

- Receipt/future-epoch RED is `12c3111a4d8989c5eaa245bdfdb7fa8617eb8c22`; Rust run `33799400542`, PostgreSQL 17.6 job `100794820236`, exact checkout verified, 103 PASS / 1 expected FAIL. The sole failure was `replacement_created_session_can_reconnect_in_later_epoch_without_reusing_replacement_authorization`, returning `InvalidStoredState`.
- Current-original-grace RED is `8fa624810d8d3e5986ef6b8f0725d8451c82b834`; Rust run `33799957633`, PostgreSQL 17.6 job `100796763136`, exact checkout verified, expected compile failure `E0061` because `ReconnectCurrentAuthorityV1::from_current_facts(...)` had no explicit independently observed current original-grace-deadline argument.
- Minimal code GREEN is immutable `0ec5747a52b0ec961e8e7a7417ba7be23da0d9bb`. `AdmissionReconnectJournal::prepare_internal` now keeps the receipt-backed fail-closed guard on exact existing-attempt replay but no longer rejects a fresh unseen later attempt merely because the GameSession has historical replacement-receipt provenance. `ReconnectCurrentAuthorityV1` now carries an explicit positive `original_grace_deadline`; `from_current_facts(...)` receives it independently and `from_record(...)` supplies the persisted record value. The existing complete equality predicate therefore rejects deadline drift in direct COMMIT and committed reconciliation.
- Exact hosted GREEN on `0ec5747a52b0ec961e8e7a7417ba7be23da0d9bb`: Rust run `33800610471` SUCCESS; PostgreSQL 17.6 job `100798951680` verified exact checkout and completed 105 PASS / 0 FAIL; Windows SIM `100798951257` SUCCESS; Architecture `33800610652` SUCCESS; Agent governance `33800610784` SUCCESS; Merge gate `33800610201` SUCCESS; aggregate `game-gate` job `100801677693` SUCCESS.
- Whole-diff self-review on code GREEN `0ec5747a52b0ec961e8e7a7417ba7be23da0d9bb` against protected `main@f5f8e3717a48e6854ac36595533046938ceec890` found `merge_base=main`, `behind_by=0`, exactly the 12 allocated paths, and no P0/P1/P2 in the complete diff. The two latest production changes are minimal and preserve the prior negative-path regressions.
- No force-push, rebase, reset, merge, scope expansion, Cargo/lockfile/workflow/resource-registry, gameplay, production/live-data, secret, external-repository, or resource-maximum mutation occurred in this repair cycle.

```yaml
status: qualifying
integration_state: FINAL_EXACT_HEAD_QUALIFICATION_PENDING
ready_for_integration: false
issue: 250
pr: 252
branch: impl/game-terminal-session-replacement-250
pr_state: open_draft_mergeable_unmerged
protected_main: f5f8e3717a48e6854ac36595533046938ceec890
reviewed_head_with_latest_findings: 8a02d7825722a897ce0e52b63dc6238bd64ba1c9
latest_findings:
  later_reconnect_receipt_p1:
    thread: PRRT_kwDOT8SzxM6fDpe1
    comment: 3927601387
  current_original_grace_p1:
    thread: PRRT_kwDOT8SzxM6fDpe5
    comment: 3927601391
red_evidence:
  later_reconnect:
    head: 12c3111a4d8989c5eaa245bdfdb7fa8617eb8c22
    rust_run: 33799400542
    postgres_job: 100794820236
    result: 103_pass_1_expected_fail_invalid_stored_state
  current_original_grace:
    head: 8fa624810d8d3e5986ef6b8f0725d8451c82b834
    rust_run: 33799957633
    postgres_job: 100796763136
    result: expected_compile_failure_E0061
immutable_code_candidate_sha: 0ec5747a52b0ec961e8e7a7417ba7be23da0d9bb
code_green_ci:
  rust_run: 33800610471_SUCCESS
  postgres_job: 100798951680_SUCCESS_105_pass_0_fail
  windows_sim_job: 100798951257_SUCCESS
  architecture_run: 33800610652_SUCCESS
  agent_governance_run: 33800610784_SUCCESS
  merge_gate_run: 33800610201_SUCCESS
  game_gate_job: 100801677693_SUCCESS
whole_diff_pre_metadata:
  main_sha: f5f8e3717a48e6854ac36595533046938ceec890
  merge_base_sha: f5f8e3717a48e6854ac36595533046938ceec890
  behind_by: 0
  allocated_paths: 12_of_12
  self_review: P0_0_P1_0_P2_0
history_operations: no_force_no_rebase_no_reset_no_merge
thread_policy: resolve repaired technical threads only after the metadata-complete exact head has fresh CI and fresh exact-head Codex with zero P0/P1/P2; keep control-plane integration gate PRRT_kwDOT8SzxM6dX3eH unresolved for Work
owner_action_required: null
checkpoint_commit_qualification: this docs-only publication advances the PR head; its read-back remote SHA becomes the sole final qualification candidate
next_action: read back this metadata commit SHA; require fresh exact-head hosted CI, whole-diff self-review and fresh independent @codex review on that immutable SHA; if P0/P1/P2 remain zero, resolve repaired technical threads and issue READY_FOR_INTEGRATION without merging
```

## Later-reconnect replay/reconciliation receipt-scope repair checkpoint — 2026-09-04

This checkpoint supersedes the Final two-P1 repair checkpoint for live continuation state. A subsequent exact-head review found that the first later ordinary reconnect could be PREPARED after terminal replacement, but an exact replay of that later attempt and V2 reconciliation were still incorrectly treated as replacement-authorized solely because the same GameSession had a historical replacement receipt.

- Reviewed source head: `e697bdff241d2ca4a3d2f685ea3eb928c535d618`; actionable P1 thread `PRRT_kwDOT8SzxM6fE1OR`, top-level comment `3928606792`.
- Fresh executable RED is immutable `1bfb978ecdb4c9dba45bf6ab39fa86385ddb294a`. Rust run `33805721160`, PostgreSQL 17.6 job `100815584853` verified the exact checkout and ran 105 tests: 104 PASS / 1 expected FAIL. The sole failure was `replacement_created_session_can_reconnect_in_later_epoch_without_reusing_replacement_authorization`, with both `replay_ok=false` and `reconciliation_ok=false`.
- Minimal GREEN is split only across the two already allocated Durability paths that enforce the same receipt predicate: `fac91851cfdd8c3fe9e60eb4e27e2fd1c089382d` binds a replacement receipt to the exact record's candidate session plus predecessor connection/CharacterLease/scope generations; `6b2c3825e4aa9d26025cb75f573b6851bbfab81c` reuses that exact predicate in V2 reconciliation. `0ef16d77816a0ef11c6d5b2a1935865190728803` is rustfmt-only for the regression test.
- Exact hosted code GREEN on `0ef16d77816a0ef11c6d5b2a1935865190728803`: Rust workspace run `33809399549` SUCCESS; PostgreSQL 17.6 job `100827437620` verified exact checkout and completed 105 PASS / 0 FAIL; Windows SIM exact-head job `100827437842` SUCCESS; Architecture semantic audit `33809399600` SUCCESS; Agent governance `33809399473` SUCCESS; Merge gate `33809399411` SUCCESS.
- Scope remains exactly the existing 12 allocated paths. This checkpoint itself changes only the already allocated active task record. No force-push, rebase, reset, merge, scope expansion, Cargo/lockfile/workflow/resource-registry, gameplay, production/live-data, secret, external-repository, or resource-maximum mutation is introduced.

```yaml
status: qualifying
integration_state: FINAL_EXACT_HEAD_QUALIFICATION_PENDING
ready_for_integration: false
issue: 250
pr: 252
branch: impl/game-terminal-session-replacement-250
pr_state: open_draft_mergeable_unmerged
protected_main: f5f8e3717a48e6854ac36595533046938ceec890
reviewed_source_head: e697bdff241d2ca4a3d2f685ea3eb928c535d618
latest_p1:
  thread: PRRT_kwDOT8SzxM6fE1OR
  comment: 3928606792
  requirement: historical replacement receipt must apply only to the original replacement attempt and must not poison exact replay or V2 reconciliation of a genuinely later ordinary reconnect
red_evidence:
  head: 1bfb978ecdb4c9dba45bf6ab39fa86385ddb294a
  rust_run: 33805721160
  postgres_job: 100815584853
  result: 104_pass_1_expected_fail_replay_and_reconciliation_false
green_evidence:
  receipt_binding_head: fac91851cfdd8c3fe9e60eb4e27e2fd1c089382d
  reconciliation_scope_head: 6b2c3825e4aa9d26025cb75f573b6851bbfab81c
  code_complete_head: 0ef16d77816a0ef11c6d5b2a1935865190728803
  rust_run: 33809399549_SUCCESS
  postgres_job: 100827437620_SUCCESS_105_pass_0_fail
  windows_sim_job: 100827437842_SUCCESS
  architecture_run: 33809399600_SUCCESS
  agent_governance_run: 33809399473_SUCCESS
  merge_gate_run: 33809399411_SUCCESS
allocated_paths: 12_of_12
history_operations: no_force_no_rebase_no_reset_no_merge
checkpoint_commit_qualification: this docs-only publication advances the PR head; read back its immutable remote SHA and use only that SHA for final exact-head qualification
thread_policy: do_not_resolve_any_technical_thread_or_mark_READY_until_fresh_metadata_head_CI_whole_diff_self_review_and_exact_head_Codex_all_report_P0_0_P1_0_P2_0; keep control-plane integration gate PRRT_kwDOT8SzxM6dX3eH unresolved
owner_action_required: null
next_action: read back this metadata commit SHA; require fresh exact-head hosted CI on it, perform complete whole-diff self-review against current protected main, then request fresh independent native @codex review pinned to the same immutable SHA; repair and repeat any P0/P1/P2; do not merge
```

## Replacement-attempt receipt binding repair checkpoint — 2026-09-04

This checkpoint supersedes earlier current-status checkpoints for live continuation state while preserving them as historical provenance. Independent Codex review `5109280292` of parent `293449a1fdabbebbc725b1fb3ec5c4f6343bcf2b` produced P1 inline comment `3930961564`: the replacement receipt lacked the original candidate `ReconnectAttemptRef`, so permitted later fresh same-fence attempts could be misclassified as the original replacement attempt.

- Fresh RED is `891a1a166826abcf28deb92be29cef4ed53c2f15`. Rust run `33838542287`, PostgreSQL 17.6 job `100915905592`, failed at the expected build boundary with `terminal replacement receipt must persist the exact candidate reconnect attempt ref`.
- Minimal GREEN is `cf97f713ad452cf3a51118a34b247296749966f4`, commit `fix(durability): bind replacement attempt receipts`.
- GREEN Rust run `33850409050`, PostgreSQL 17.6 job `100951668541`, verified the exact checkout and completed `106 passed; 0 failed`, including `replacement_created_session_can_replay_fresh_same_fence_attempt_after_collision`.
- Windows SIM job `100951668289`: SUCCESS. Architecture semantic audit `33850408950`: SUCCESS. Agent governance `33850409016`: SUCCESS.
- Merge Gate run `33850409137`: SUCCESS; validate job `100953719857`: SUCCESS; canonical `game-gate` job `100953738250`: SUCCESS; Linux job `100951700436`: SUCCESS; Windows client job `100951700454`: SUCCESS; policy/metadata job `100951700548`: SUCCESS.
- Protected `main` remains `f5f8e3717a48e6854ac36595533046938ceec890`.
- The repair delta from RED to GREEN is exactly four allocated files: `apps/game-server/migrations/0001_admission_reconnect_journal.sql`, `apps/game-server/src/durability/admission_journal.rs`, `apps/game-server/src/durability/mod.rs`, and `apps/game-server/src/durability/schema.rs`. PR #252 remains exactly 12/12 allocated changed paths.
- No force, rebase, reset, or merge occurred. PR #252 remains Draft, open, and unmerged.
- After this docs-only publication, its remote SHA, established by authoritative readback, becomes the sole final exact-head qualification candidate. This document intentionally does not embed a self-referential SHA placeholder.

```yaml
status: qualifying
integration_state: FINAL_EXACT_HEAD_QUALIFICATION_PENDING
ready_for_integration: false
issue: 250
pr: 252
branch: impl/game-terminal-session-replacement-250
reviewed_parent: 293449a1fdabbebbc725b1fb3ec5c4f6343bcf2b
independent_codex_review: 5109280292
p1_inline_comment: 3930961564
red_head: 891a1a166826abcf28deb92be29cef4ed53c2f15
green_head: cf97f713ad452cf3a51118a34b247296749966f4
protected_main: f5f8e3717a48e6854ac36595533046938ceec890
allocated_paths: 12_of_12
history_operations: no_force_no_rebase_no_reset_no_merge
thread_policy: do_not_resolve_threads_or_declare_READY_before_fresh_exact_head_review_reports_P0_0_P1_0_P2_0
next_action: run fresh exact-head CI on the authoritative read-back docs head, then perform whole-diff self-review and independent read-only Codex review; repair and repeat only if a fresh P0/P1/P2 appears; do not resolve threads or declare READY before P0=P1=P2=0
```

## Final code-GREEN repair qualification checkpoint — 2026-09-04

This checkpoint supersedes earlier current-status checkpoints for live continuation state while preserving all older evidence as historical provenance. The immutable qualified code head is `fb1889002b61972f7de6ba5e3b91823e34b2016d`; the docs-only publication commit created from it becomes the sole final metadata candidate externally after authoritative remote readback. This document intentionally does not use `HEAD`, a placeholder, or its own unknown commit SHA as immutable evidence.

- Protected `main` is `f5f8e3717a48e6854ac36595533046938ceec890`.
- The prior metadata-complete candidate reviewed was `7d59bf6eaa701711046ccb1d3e94be6596db54e6`. Independent Codex review numeric ID `5110767899` reviewed that exact commit and produced two P1 threads: `PRRT_kwDOT8SzxM6fN5Vn` / comment `3932203686` for the production record-derived `ReconnectCurrentAuthorityV1::from_record` bypass, and `PRRT_kwDOT8SzxM6fN5Vt` / comment `3932203697` for historical terminal reconciliation incorrectly fenced by later scope advancement.
- Fresh test-only RED is `f21b31e0d68bbebacdf09fc2507ce8b6f4f2115c`. Rust run `33853739671`, PostgreSQL 17.6 job `100962147321`, verified the exact checkout and ran 108 tests: 106 PASS / exactly 2 expected FAIL, `v2_terminal_reconciliation_survives_scope_advance_but_committed_stays_fenced` and `record_derived_current_authority_is_test_only_but_current_facts_remain_public`. Windows SIM job `100962147504` was SUCCESS.
- Production GREEN is `5b553701a91c1ea904144cded983f983b5f06ba7`: `from_record` became `#[cfg(test)]`; reconciliation no longer globally rejects a historical terminal outcome on scope drift; `Committed` remains complete-current-authority fail-closed.
- The first GREEN hosted Rust run `33854385596` exposed only integration-test `E0599` because external harnesses still invoked the now-test-only `from_record`. This was not a semantic rollback, and `from_record` was not re-exposed.
- Mechanical test-harness repair `fb1889002b61972f7de6ba5e3b91823e34b2016d` replaces external test calls with exact-current helpers using public `from_current_facts`; production semantics are unchanged.
- Hosted exact code-GREEN on `fb1889002b61972f7de6ba5e3b91823e34b2016d`: Rust run `33855402001` SUCCESS; PostgreSQL 17.6 job `100967423940` verified the exact checkout and completed 108/108 PASS, including both fresh regressions; Windows SIM `100967423812` SUCCESS; Architecture `33855401993` SUCCESS; Agent governance `33855402041` SUCCESS; Merge Gate `33855402040` SUCCESS, including canonical `game-gate` job `100969639469` SUCCESS.
- Effective task scope remains exactly 12/12 allocated paths. There is no Cargo, lockfile, workflow, or registry expansion, and no merge, rebase, reset, or force operation occurred.

```yaml
status: qualifying
integration_state: FINAL_EXACT_HEAD_QUALIFICATION_PENDING
ready_for_integration: false
issue: 250
pr: 252
branch: impl/game-terminal-session-replacement-250
protected_main: f5f8e3717a48e6854ac36595533046938ceec890
prior_metadata_complete_candidate: 7d59bf6eaa701711046ccb1d3e94be6596db54e6
independent_codex_review: 5110767899
red_head: f21b31e0d68bbebacdf09fc2507ce8b6f4f2115c
production_green_head: 5b553701a91c1ea904144cded983f983b5f06ba7
qualified_code_head: fb1889002b61972f7de6ba5e3b91823e34b2016d
allocated_paths: 12_of_12
history_operations: no_force_no_rebase_no_reset_no_merge
metadata_commit_identity: established_by_authoritative_remote_readback_not_self_embedded
next_action: run fresh exact-head CI on the read-back metadata-complete SHA, perform a whole-diff self-review, and obtain an independent Codex review of that exact SHA; repair and repeat any P0/P1/P2 before READY_FOR_INTEGRATION
```

Disposition remains `FINAL_EXACT_HEAD_QUALIFICATION_PENDING`; this task is not ready for integration. Fresh exact-head CI, whole-diff self-review, and independent Codex review are still required on the externally read-back docs publication SHA before READY.

## V1 complete-current-authority reconciliation repair checkpoint — 2026-09-04

This checkpoint supersedes all earlier current-status checkpoints for live continuation state while preserving their historical evidence. The code generation is frozen at immutable GREEN `98561d780f4e266272965dcd074ad28d5f2c575b`; this docs-only publication must be identified by authoritative remote readback and is not self-referential.

- Protected `main` is `f5f8e3717a48e6854ac36595533046938ceec890`.
- Independent Codex review `5111393923` / `PRR_kwDOT8SzxM8AAAABMKmugw` reviewed exact metadata head `e9dfbe773fdd7a0bfd138972bf3f792521549ed7` and produced one actionable P1: thread `PRRT_kwDOT8SzxM6fPQmE`, comment `3932741432`, because public V1 committed reconciliation could restore a controller from scope generation alone without complete current authority.
- Fresh test-only RED is `61545bd4197e857f5ad88f7a751888abd121ceaa`. Rust run `33859990960`, PostgreSQL 17.6 job `100981982717`, verified exact checkout and failed at the intended V1 complete-current-authority boundary. Windows SIM job `100981982910` was SUCCESS.
- Minimal GREEN is `98561d780f4e266272965dcd074ad28d5f2c575b`. `ReconnectDurabilityFlowV1::accept_reconciliation` now consumes independently supplied `ReconnectCurrentAuthorityV1`; `Committed` requires exact candidate generation/transport, `current.observed_at <= record.authorization_deadline()`, and the same complete fail-closed `current_authority_matches_record` predicate used by V2 before `InstallController`. `Prepared` still transitions to final revalidation and historical `Terminal` remains historical without live-authority equality. Production `ReconnectCurrentAuthorityV1::from_record` remains test-only.
- Exact GREEN CI on `98561d780f4e266272965dcd074ad28d5f2c575b`: Rust `33860194846` SUCCESS; PostgreSQL 17.6 job `100982630189` exact checkout and 109/109 PASS; Windows SIM `100982630036` SUCCESS; Architecture `33860194875` SUCCESS; Merge authority `33860194840` SUCCESS; Agent governance `33860194878` SUCCESS; Merge Gate `33860194885` SUCCESS, including Windows client `100982758550`, Linux workspace `100982758578`, validate `100984359185`, and canonical `game-gate` `100984379832` all SUCCESS.
- Whole diff against current protected main remains `behind_by=0` and exactly 12/12 allocated paths. No Cargo/lock/workflow/registry or other scope expansion occurred. No force-push, rebase, reset, or merge occurred.

```yaml
status: qualifying
integration_state: FINAL_EXACT_HEAD_QUALIFICATION_PENDING
ready_for_integration: false
issue: 250
pr: 252
branch: impl/game-terminal-session-replacement-250
protected_main: f5f8e3717a48e6854ac36595533046938ceec890
reviewed_metadata_head: e9dfbe773fdd7a0bfd138972bf3f792521549ed7
independent_codex_review:
  numeric_id: 5111393923
  node_id: PRR_kwDOT8SzxM8AAAABMKmugw
  reviewed_commit: e9dfbe773fdd7a0bfd138972bf3f792521549ed7
  actionable_p1_thread: PRRT_kwDOT8SzxM6fPQmE
  actionable_p1_comment: 3932741432
red_head: 61545bd4197e857f5ad88f7a751888abd121ceaa
green_head: 98561d780f4e266272965dcd074ad28d5f2c575b
green_ci:
  rust_run: 33860194846_SUCCESS
  postgres_job: 100982630189_SUCCESS_109_pass_0_fail
  windows_sim_job: 100982630036_SUCCESS
  architecture_run: 33860194875_SUCCESS
  merge_authority_run: 33860194840_SUCCESS
  agent_governance_run: 33860194878_SUCCESS
  merge_gate_run: 33860194885_SUCCESS
  windows_client_job: 100982758550_SUCCESS
  linux_workspace_job: 100982758578_SUCCESS
  validate_job: 100984359185_SUCCESS
  game_gate_job: 100984379832_SUCCESS
whole_diff_pre_metadata:
  behind_by: 0
  allocated_paths: 12_of_12
history_operations: no_force_no_rebase_no_reset_no_merge
metadata_commit_identity: established_by_authoritative_remote_readback_not_self_embedded
thread_policy: after this docs-only SHA has fresh exact-head CI plus fresh whole-diff self-review and one independent exact-head Codex review with P0_0_P1_0_P2_0, resolve repaired lane-owned technical threads but keep control-plane integration gate PRRT_kwDOT8SzxM6dX3eH unresolved
next_action: read back this docs-only commit SHA; run fresh exact-head CI on it; perform fresh whole-diff self-review against current protected main; obtain one independent exact-head Codex review whose reviewed commit equals that SHA; repair and repeat any P0/P1/P2; only with P0=P1=P2=0 resolve repaired lane-owned technical threads except PRRT_kwDOT8SzxM6dX3eH and publish READY_FOR_INTEGRATION; do not merge
```

## Expired PREPARED and V1 observation-deadline repair qualification checkpoint — 2026-09-04

This checkpoint supersedes earlier current-status checkpoints for live qualification state while retaining them as historical provenance. Its docs-only commit identity is established by authoritative remote readback and is intentionally not self-embedded.

- Protected `main` is `f5f8e3717a48e6854ac36595533046938ceec890`; the prior metadata head reviewed was `f29b15065a3ff0b5e24b519efbae89720b310c5e`.
- Independent Codex review numeric ID `5112586391` reviewed exact `f29b15065a3ff0b5e24b519efbae89720b310c5e` and produced two actionable P1 findings: thread `PRRT_kwDOT8SzxM6fRtxw`, comment `3933702052`, because expired PREPARED reconciliation did not retire the durable incumbent; and thread `PRRT_kwDOT8SzxM6fRtx5`, comment `3933702061`, because V1 direct `authorize_commit` did not fence `current.observed_at` against the authorization deadline.
- Fresh test-only RED is `7e8266f251393ecf7d69bec87c473bb0e9bd3ca4`. Rust run `33870483323`, PostgreSQL 17.6 job `101015015095`, verified the exact checkout and completed 109 PASS / 1 expected FAIL for expired reconciliation. Merge Gate Linux job `101015043789` completed 223 PASS / 1 expected FAIL for `v1_final_revalidation_rejects_authority_observed_after_authorization_deadline`.
- Minimal production GREEN is `eb2225a7dfb99d7489a3d8eb8620ea326e441784`. Reconciliation uses the authoritative session `FOR UPDATE` boundary; an expired exact PREPARED attempt is terminalized with the existing terminalization semantics, its prepared anchor is cleared atomically, and the historical terminal snapshot is returned. Non-expired PREPARED, COMMITTED, and already-terminal semantics are preserved. V1 direct `authorize_commit` now rejects when `now > deadline || current.observed_at > deadline`.
- Exact-head GREEN CI: Rust `33870834866` SUCCESS; PostgreSQL 17.6 job `101016164225` exact checkout and 110/110 PASS; Windows SIM `101016163956` SUCCESS; Architecture `33870834822` SUCCESS; Agent governance `33870835950` SUCCESS; Merge Gate `33870834794` SUCCESS, including Linux workspace `101016280195` SUCCESS, Windows client `101016280333` SUCCESS, validate `101018645139` SUCCESS, and canonical `game-gate` `101018666111` SUCCESS.
- Whole diff against protected main is `behind_by=0` and exactly 12/12 allocated paths. There is no Cargo, lockfile, workflow, or registry expansion, and no force-push, rebase, reset, or merge occurred.

```yaml
status: qualifying
integration_state: FINAL_EXACT_HEAD_QUALIFICATION_PENDING
ready_for_integration: false
issue: 250
pr: 252
branch: impl/game-terminal-session-replacement-250
protected_main: f5f8e3717a48e6854ac36595533046938ceec890
prior_metadata_head_reviewed: f29b15065a3ff0b5e24b519efbae89720b310c5e
independent_codex_review: 5112586391
red_head: 7e8266f251393ecf7d69bec87c473bb0e9bd3ca4
green_head: eb2225a7dfb99d7489a3d8eb8620ea326e441784
whole_diff: behind_by_0_exactly_12_of_12_allocated_paths
history_operations: no_force_no_rebase_no_reset_no_merge
metadata_commit_identity: established_by_authoritative_remote_readback_not_self_embedded
thread_policy: only_if_P0_P1_P2_zero_resolve_repaired_lane_owned_technical_threads_except_control_plane_gate_PRRT_kwDOT8SzxM6dX3eH
next_action: run exact-head CI on the authoritative read-back docs SHA, perform a fresh whole-diff self-review, and obtain one independent exact-head Codex review; only if P0=P1=P2=0 resolve repaired lane-owned technical threads except control-plane gate PRRT_kwDOT8SzxM6dX3eH and publish READY_FOR_INTEGRATION; do not merge
```

## Future-authenticated-evidence repair checkpoint — 2026-09-04

This checkpoint supersedes earlier current-status checkpoints for live qualification while preserving their historical evidence. It records the bounded repair generation before the next material freeze; the immutable identity of this docs-bearing generation is established by authoritative GitHub readback after publication rather than self-embedded.

- Independent exact-head review `5113263511` reviewed frozen head `89ca402ae94b42044f74a1f866d4a23a6baed5e4` and produced two actionable P1 findings: technical comment `3934233107`, because future-dated authenticated `source_observed_at` values could extend COMMIT authority; and task-record comment `3934233118`, because the active record stopped before the generic-V1 repair and current-main merge-up.
- Fresh focused test-only RED is `e3ca7cd0c5c7560d4dcb83fdaa8c4029b98babd7`. Rust run `33876594852`, PostgreSQL 17.6 job `101034940507`, verified the exact checkout and completed 111 PASS / exactly 2 expected FAIL: `commit_authorization_rejects_future_authenticated_evidence_and_accepts_equal_timestamps` and `committed_reconciliation_rejects_authority_observed_before_authenticated_evidence`. The regressions cover direct V1 and V2 COMMIT, both authenticated evidence sources, current-authority chronology, committed reconciliation, and accepted equality boundaries.
- Minimal production GREEN is `54d9579279bd19718a698e16eb8577b84b2cbb37`. Both trusted caller `now` and independently observed current-authority time must be at or after both authenticated evidence source timestamps. Direct V1/V2 COMMIT returns the existing `StaleAuthority` class for future source provenance; the shared complete-current-authority predicate applies the same chronology to committed V1/V2 reconciliation, which retains the existing reconciliation mismatch class. All existing deadline, freshness, and authority checks remain intact.
- The later generic-V1 production repair `92ce1be30f05aba02f392069090200784530758d` is part of the published history and fences generic V1 terminal reconciliation from external production use. Protected-main reconciliation is `main@d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d`; merge-up `89ca402ae94b42044f74a1f866d4a23a6baed5e4` is the historical pre-repair generation. The effective task diff remains the existing 12/12 allocated paths.

```yaml
status: qualifying
integration_state: FINAL_EXACT_HEAD_QUALIFICATION_PENDING
ready_for_integration: false
issue: 250
pr: 252
branch: impl/game-terminal-session-replacement-250
protected_main: d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d
review:
  numeric_id: 5113263511
  reviewed_head: 89ca402ae94b42044f74a1f866d4a23a6baed5e4
  technical_p1_comment: 3934233107
  task_record_p1_comment: 3934233118
red_head: e3ca7cd0c5c7560d4dcb83fdaa8c4029b98babd7
red_rust_run: 33876594852_FAILURE
red_postgres_job: 101034940507_111_pass_2_expected_fail
green_head: 54d9579279bd19718a698e16eb8577b84b2cbb37
green_exact_head_ci: pending_on_metadata_complete_generation
current_main_reconciliation:
  integration_main_sha: d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d
  historical_pre_repair_merge_up: 89ca402ae94b42044f74a1f866d4a23a6baed5e4
allocated_paths: 12_of_12
metadata_commit_identity: established_by_authoritative_remote_readback_not_self_embedded
final_evidence_authority: final_SHA_CI_review_and_READY_are_GitHub_control_plane_evidence_under_current_TASK_TEMPLATE
next_action: read back the published metadata-complete SHA, run fresh exact-head hosted CI, perform whole-diff self-review, and obtain independent exact-head review; repair and repeat any P0/P1/P2 before READY_FOR_INTEGRATION; do not merge
```

## Historical COMMITTED reconciliation repair checkpoint — 2026-09-04

This checkpoint supersedes earlier current-status checkpoints for live qualification while retaining all earlier evidence as historical provenance. It records the final material repair generation before exact-head qualification; the docs-only checkpoint commit identity is established by authoritative remote readback and is intentionally not self-embedded.

- Exact-head review `5113483686` on frozen head `532b1adefcf5937892bc54e3b1591fcced852f16` produced P2 comment `3934420221`: a valid earlier COMMITTED winner became unreconcilable after the same GameSession advanced into a later `ControlLossEpoch` and current projection.
- Fresh executed test-only RED is `6dc0f1c05487562e10822adbc85c28975ab92380`. Rust run `33878520064`, PostgreSQL 17.6 job `101041281811`, verified the exact checkout and completed 113 PASS / exactly 1 expected FAIL, solely `committed_reconciliation_remains_historical_after_later_epoch_opens`, which returned `InvalidStoredState` instead of historical committed evidence.
- The repair generation comprises initial semantic GREEN `6788cdde22899e3fe8700088eebf905bf175f422` plus exact projection-validation correction `d3ce7462bf1442f6a852981a8a9065a125bb1ddf`. COMMITTED reconciliation retains full record/typed-attempt/recovery-grant mirror validation. It accepts historical COMMITTED evidence only when the session has advanced to a strictly later epoch with a structurally valid current PREPARED or COMMITTED projection; the unchanged complete Foundation current-authority gate remains solely responsible for refusing stale `InstallController` projection.
- Exact code-GREEN Rust run `33879231616` is SUCCESS. PostgreSQL 17.6 job `101043522525` verified exact `d3ce7462bf1442f6a852981a8a9065a125bb1ddf` and completed 114/114 PASS, including the new historical reconciliation regression. Architecture `33879231613`, Agent governance `33879231742`, and Merge authority `33879228974` are SUCCESS.
- Protected `main@d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d` remains the merge base, GitHub compare reports `behind_by=0`, and the effective PR diff remains exactly the allocated 12/12 paths. No scope expansion, force-push, rebase, reset, or merge occurred.

```yaml
status: qualifying
integration_state: FINAL_EXACT_HEAD_QUALIFICATION_PENDING
ready_for_integration: false
issue: 250
pr: 252
branch: impl/game-terminal-session-replacement-250
protected_main: d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d
review: 5113483686
p2_comment: 3934420221
reviewed_head: 532b1adefcf5937892bc54e3b1591fcced852f16
red_head: 6dc0f1c05487562e10822adbc85c28975ab92380
red_rust_run: 33878520064_FAILURE
red_postgres_job: 101041281811_113_pass_1_expected_fail
green_head: d3ce7462bf1442f6a852981a8a9065a125bb1ddf
green_rust_run: 33879231616_SUCCESS
green_postgres_job: 101043522525_SUCCESS_114_pass_0_fail
whole_diff: behind_by_0_exactly_12_of_12_allocated_paths
metadata_commit_identity: established_by_authoritative_remote_readback_not_self_embedded
next_action: run fresh exact-head CI, whole-diff self-review, and independent exact-head review on the authoritative read-back checkpoint SHA; keep FINAL_EXACT_HEAD_QUALIFICATION_PENDING and do not resolve threads, mark READY_FOR_INTEGRATION, or merge
```

## Later PREPARED structural-binding repair checkpoint — 2026-09-04

This single superseding pre-freeze checkpoint replaces earlier current-status checkpoints for continuation state while retaining all earlier RED/GREEN evidence as historical provenance. Its own commit identity is established by authoritative remote readback and is intentionally not self-embedded.

- Independent exact-head review `5113765290` on frozen head `d08677b1efec188ae4410699f22c8dfb8664150d` produced P2 comment `3934639680`: historical COMMITTED reconciliation accepted a later PREPARED projection after checking only epoch/state and actor/scope identity, without validating its complete session, canonical attempt, transport, protection, and FND-02 bindings.
- Fresh executed test-only RED is `6a11c453e2fb2a39cf50c8ae39be3a3aab58f682`. Rust run `33880701069`, PostgreSQL 17.6 job `101048371153`, verified the exact checkout and completed 114 PASS / exactly 1 expected FAIL, solely `historical_committed_reconciliation_rejects_corrupt_later_prepared_projection`; corrupt later PREPARED state was accepted as historical COMMITTED evidence instead of returning `InvalidStoredState`.
- Minimal production GREEN is `1aaa8a714256ed64baec9c3eae78c3578ad1b1ee`, refined by `55e95945d91c95ce5874a10758b89c130c5d20b4` to preserve valid unconsumed reauthenticated PREPARED proof semantics. The later PREPARED validator now requires the complete current session generation/state/anchor binding, canonical attempt-to-typed mirrors, exact runtime scope and authority generations, transport reservation, precommit protection continuity, FND-02 mirrors, compatibility evidence, and proof shape from the later attempt's own durable canonical record. The old COMMITTED record is not required to equal the later projection; ACTIVE later-projection validation and the complete Foundation current-authority gate are unchanged.
- Final regression correction `1355cb61fcf6c5d02dd8404268afd0963348b3b9` makes the canonical-attempt corruption target the transport mirror while preserving the same RED assertion set. Rust run `33881800738`, PostgreSQL 17.6 job `101051972439`, verified that exact head and completed 115/115 PASS, including the valid later-PREPARED historical replay and all five corrupt later-PREPARED binding cases.
- Local formatting, strict game-server all-target Clippy, focused regression, game-server library 227/227, and `git diff --check` passed. Protected `main@d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d` remains the merge base; GitHub compare reports `behind_by=0`; the effective PR diff remains exactly the allocated 12/12 paths. No scope expansion, force-push, rebase, reset, or merge occurred.

```yaml
status: qualifying
integration_state: FINAL_EXACT_HEAD_QUALIFICATION_PENDING
ready_for_integration: false
issue: 250
pr: 252
branch: impl/game-terminal-session-replacement-250
protected_main: d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d
review: 5113765290
p2_comment: 3934639680
reviewed_head: d08677b1efec188ae4410699f22c8dfb8664150d
red_head: 6a11c453e2fb2a39cf50c8ae39be3a3aab58f682
red_rust_run: 33880701069_FAILURE
red_postgres_job: 101048371153_114_pass_1_expected_fail
green_head: 55e95945d91c95ce5874a10758b89c130c5d20b4
final_regression_head: 1355cb61fcf6c5d02dd8404268afd0963348b3b9
final_rust_run: 33881800738_SUCCESS
final_postgres_job: 101051972439_SUCCESS_115_pass_0_fail
whole_diff: behind_by_0_exactly_12_of_12_allocated_paths
metadata_commit_identity: established_by_authoritative_remote_readback_not_self_embedded
next_action: run fresh exact-head hosted CI on the authoritative read-back checkpoint SHA, perform whole-diff self-review, and obtain one independent exact-head review; repair and repeat any P0/P1/P2 before READY_FOR_INTEGRATION; do not resolve threads, mark Ready, enqueue, or merge
```

## Later PREPARED candidate-generation regression completion checkpoint — 2026-09-04

This single superseding pre-freeze checkpoint replaces earlier current-status checkpoints for continuation state while retaining all earlier evidence as historical provenance. Its own commit identity is intentionally not self-embedded.

- Review `5113765290`, P2 comment `3934639680`, required complete validation of a later PREPARED projection. Original executable RED `6a11c453e2fb2a39cf50c8ae39be3a3aab58f682` and the hosted intermediate failure on `55e95945d91c95ce5874a10758b89c130c5d20b4` demonstrated that candidate-generation corruption was accepted.
- Production completion `825f5cbf05c6217ae4144caf52d9daa333d43d5d` requires the canonical candidate generation to equal the predecessor generation plus one.
- Regression restoration commit `aa417c90d809239d70185e415ed01d1c1aea90ba` retains the existing five corruption classes and adds a distinct canonical-record candidate-generation corruption (`10` for predecessor `7` / valid candidate `8`) that must fail closed with `InvalidStoredState`; the transport corruption case remains unchanged.
- Commit `1355cb61fcf6c5d02dd8404268afd0963348b3b9` and its PostgreSQL 17.6 evidence (`115/115`) remain pre-restoration evidence only because that generation replaced, rather than retained, the candidate-generation corruption case.
- Protected `main@d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d` remains the integration base, and the effective PR scope remains exactly the allocated 12/12 paths.

```yaml
status: qualifying
integration_state: FINAL_EXACT_HEAD_QUALIFICATION_PENDING
ready_for_integration: false
review: 5113765290
p2_comment: 3934639680
original_red: 6a11c453e2fb2a39cf50c8ae39be3a3aab58f682
hosted_intermediate_failure: 55e95945d91c95ce5874a10758b89c130c5d20b4
production_completion: 825f5cbf05c6217ae4144caf52d9daa333d43d5d
restored_regression_commit: aa417c90d809239d70185e415ed01d1c1aea90ba
pre_restoration_evidence: 1355cb61fcf6c5d02dd8404268afd0963348b3b9_115_pass_0_fail
protected_main: d8e6233fa6b6b06f9ef643d5fdd9083d7bb3314d
allocated_paths: 12_of_12
final_evidence_authority: final_exact_SHA_CI_review_and_READY_must_live_in_immutable_GitHub_evidence_after_freeze
next_action: coordinator performs final exact-head CI, whole-diff self-review, independent review, thread reconciliation, and READY handoff after this pre-freeze checkpoint; do not merge
```
