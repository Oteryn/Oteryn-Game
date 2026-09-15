# OTV2-20260822-impl-qa-e2e

```yaml
task_id: OTV2-20260822-impl-qa-e2e
title: Implement native QA E2E evidence platform shell
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-impl-qa-e2e-01
issue: 91
pr: 98
base_sha: fd39c6aa026e82062a8b29af24811d467c115f19
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
head_sha: b008c6438f5d902a98ac9dd7c3318a4d72dd822f
final_head_sha: null
final_head_frozen_at: null
owner: chat-github-20260822-qa-e2e
created_at: 2026-08-22T18:11:00+02:00
updated_at: 2026-08-24T16:29:44+02:00
execution_budget_minutes: 60
owned_paths:
  - apps/game-server/tests/**
  - docs/agents/tasks/active/OTV2-20260822-impl-qa-e2e.md
public_contracts:
  - docs/architecture/ADR-0007-native-end-to-end-test-platform.md
  - docs/agents/BUILD_TEST_MATRIX.md
  - docs/architecture/FND-02_PROTOCOL_OTERYN_V1_CONTRACT.md
  - docs/architecture/FND-03_RUNTIME_EXECUTION_CONTRACT.md
  - docs/architecture/FND-04_IDENTITY_GAME_SESSION_ADMISSION_CHARACTER_LEASE_CONTRACT.md
depends_on:
  - Oteryn-Game#45
  - Oteryn-Game#46
blocks:
  - OTV2-IMPL-MOVE
  - OTV2-IMPL-COMBAT
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Deliver the smallest reusable evidence/test shell that classifies real-boundary journeys truthfully and cannot treat mocks, direct state mutation, incomplete evidence or environment startup as terminal E2E proof.

## Architecture and source of truth

- `PROVEN`: ADR-0007 defines Tier 1/2/3, canonical phases, cleanup and repeated-run evidence semantics.
- `PROVEN`: `BUILD_TEST_MATRIX.md` requires path-proportional validation and forbids speculative tests for product seams that do not exist.
- `PROVEN`: the live allocation grants writes only to `apps/game-server/tests/**` and this active task file; no shared mutation lease is assigned.
- `PROVEN`: merged product evidence remains fail-closed for gameplay entry; a real Platform-to-server gameplay Tier 1 and native-client Tier 2 journey are not yet executable from the supported product path.
- `DERIVED`: the lawful deliverable at this stage is the reusable evidence shell plus truthful `NOT_EVALUATED` terminal-journey status, not fabricated physical E2E proof.

## Acceptance criteria

- [x] Deterministic comparison-cell evidence includes exact client/server/build/platform/protocol/ruleset/content/World Bundle/migration/OS/target/features/seed/clock/topology/fault fields.
- [x] Stable population classification distinguishes `PASS / UNSTABLE / FAIL / BLOCKED / NOT_EVALUATED`.
- [x] All 14 canonical ADR-0007 phases are required in exact order and first divergence/failure class cannot contradict the attempt outcome.
- [x] Duplicate physical attempt IDs cannot satisfy a repeated-run population.
- [x] Tier 1 rejects native-client mislabelling, mock transport and direct authoritative-domain shortcuts.
- [x] Tier 2 requires a real native-client boundary, normal networking and client-presentation evidence.
- [x] Tier 3 requires production-default native artifacts and rejects a Tier 2/test-adapter boundary as release-binary proof.
- [x] Cleanup summary and cleanup phase must agree; incomplete or unknown cleanup cannot yield a clean pass.
- [x] Tests compile only against actually merged product seams; unavailable real journeys remain explicit `NOT_EVALUATED` blockers.

## Excluded scope

No production-default test adapter, production listener/client-entry implementation, live account/data, synthetic-client-as-Tier2 claim, direct-domain-as-Tier1 shortcut, schema/migration change, shared workspace mutation or product behavior invented to make scenarios green.

## Implementation / findings

- Existing branch shell began with 8 green evidence-boundary tests and no delivery PR.
- TDD review reproduced five shell gaps first: missing `build_features`, partial phase lists, failed attempts without failure evidence, duplicate physical attempt IDs and Tier 3 accepting a generic Tier 2 boundary.
- A second RED pass reproduced three additional gaps: Tier 1 accepting a native-client-labelled attempt, Tier 2 accepting `client-presentation = NOT_APPLICABLE`, and cleanup summary `Complete` contradicting a failed cleanup phase.
- All eight reproduced gaps are now fail-closed. Focused suite is 17 tests.
- Issue #91 records the delivery goal and PR #98 is the single delivery PR for this allocation.
- The implementation does not convert synthetic shell validation into terminal gameplay E2E evidence.

## Validation

### Focused

- command/run: `cargo +1.94.0 test -p oteryn-game-server --test evidence_shell --locked`
- result: `PASS` — 17 passed, 0 failed.

### Component/integration

- command/run: `rustfmt +1.94.0 --check apps/game-server/tests/evidence_shell.rs apps/game-server/tests/support/evidence.rs`
- result: `PASS` on the allocated Rust files.
- command/run: `cargo +1.94.0 clippy -p oteryn-game-server --tests --locked -- -D warnings`
- result: `PASS`.
- command/run: `cargo +1.94.0 test -p oteryn-game-server --tests --locked`
- result: `PASS` on worker branch — 3 existing package tests + 17 evidence-shell tests, 0 failed.
- prospective integration snapshot: `main@19329df11eb5c605e338a472c277ac023a8d7c43` plus the QA patch.
- prospective integration result: `PASS` — strict game-server Clippy; 129 game-server tests + 17 evidence-shell tests; `cargo +1.94.0 test --locked --workspace` completed with exit code 0 including client, renderer, security and doctests.
- caveat: `main` advanced after that local prospective snapshot; PR #98 exact-head/merge-ref CI is authoritative for final integration.
- Windows full-workspace format note: local `core.autocrlf=true` conflicts with repository `rustfmt.toml` `newline_style = "Unix"` across many unchanged files, so local `cargo fmt --all --check` is not used as terminal evidence. Exact-head Linux CI remains authoritative.

### E2E

- scenario: supported Platform -> Gateway -> gameplay server login/relog Tier 1; native-client login/relog Tier 2.
- result: `NOT_EVALUATED` — validated integration snapshot still passed fail-closed product tests including `bootstrap_is_explicitly_gameplay_unavailable` and client `gameplay_entry_fails_before_any_route_or_credential`; no supported production gameplay listener/client-entry seam was available to count a real attempt.

### Exact-head CI

- final head: pending after this task-record commit; PR #98 is authoritative.
- trigger source: pull request #98.
- workflow/run/job: pending.
- runner assignment: pending.
- classification: pending.
- result: pending.

## Self-review

- exact implementation head reviewed: `b008c6438f5d902a98ac9dd7c3318a4d72dd822f`.
- method/reviewer: implementing QA agent; full allocated-code diff review, ADR-0007/BUILD_TEST_MATRIX cross-check and RED/GREEN regression review.
- material findings: 8 shell-level false-positive/incomplete-evidence gaps reproduced and repaired; no production/runtime mutation present.
- verdict: `PASS` for the implementation diff; final PR head still requires exact-head CI after task-record binding.

## Independent review

- required: `NO` — this delivery mutates test-side evidence validation only and does not alter production authority, trust boundaries, authentication/session semantics, durable state, schema or migration behavior.
- exact head: `NOT_APPLICABLE`.
- method/auditor: `NOT_APPLICABLE`.
- material findings: `NOT_APPLICABLE`.
- verdict: `NOT_APPLICABLE`; ordinary PR review and protected exact-head checks remain required.

## PR and closeout

- issue: #91.
- delivery PR: #98.
- changed-file review: `PASS` — four files, all inside the live allocation.
- unresolved review threads: pending exact-head PR review.
- related/superseded PRs: allocation #45; exact-base #46; none superseded.
- protected auto-merge: pending.
- merge commit/result: pending.
- ownership release: coordinator-owned after terminal delivery; not authorized by this worker allocation.

## Context checkpoint

```yaml
last_progress: QA evidence shell hardened from 8 to 17 focused tests; eight reproduced false-positive/incomplete-evidence gaps are fail-closed, code head b008c6438f5d902a98ac9dd7c3318a4d72dd822f is pushed, Issue #91 and delivery PR #98 exist, and exact-head PR validation is the remaining delivery gate.
status: validating
branch: agent/otv2-impl-qa-e2e-01
head_sha: b008c6438f5d902a98ac9dd7c3318a4d72dd822f
pr: 98
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request_98
ci_check_generation: pending_after_task_record_bind
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
blocker: real Tier 1/Tier 2 gameplay journeys remain NOT_EVALUATED until the supported production gameplay listener/client-entry path exists; this does not block merging the truthful evidence shell.
next_action: observe exact-head PR #98 checks and repair only a verified current-head failure before merge.
```

## Lifecycle disposition (v3.10 audit)

Archived by the v3.10 Doc/Agent IA lifecycle audit because Issue #91 is closed completed and PR #98 is squash-merged as dc22e0da8efcc6f4458416191261063b295af5b4. The historical status fields above are preserved as the final pre-close snapshot and are not current authority.
