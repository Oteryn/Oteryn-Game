# OTV2-20260828-supply-chain-chacha20

```yaml
task_id: OTV2-20260828-supply-chain-chacha20
title: Replace yanked chacha20 lockfile resolution
mode: REPAIR
status: waiting_allocation_merge
repository: Oteryn/Oteryn-Game
base_branch: main
branch: fix/supply-chain-yanked-chacha20-231
pr: null
issue: 231
admission_main_sha: 4b6656f688868aa2fb59c18392c2f859f1c5a1c7
base_sha: 4b6656f688868aa2fb59c18392c2f859f1c5a1c7
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: GPT-5.6 Sol supply-chain repair session
created_at: 2026-08-28T05:40:00Z
updated_at: 2026-08-28T05:40:00Z
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - Cargo.lock
  - docs/agents/tasks/active/OTV2-20260828-supply-chain-chacha20.md
shared_paths:
  - Cargo.lock
shared_cargo_lease: allocated_only_after_this_allocation_reaches_protected_main
public_contracts: []
depends_on:
  - owner authorization recorded on Oteryn/Oteryn-Game#231 comment 5448901431
blocks:
  - Oteryn/Oteryn-Game#230 final merge qualification while its required merge gate fails on the baseline yanked resolution
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Restore the repository supply-chain gate by replacing the yanked `chacha20 0.10.1` lockfile resolution with the smallest compatible non-yanked resolution proven by the current dependency graph, without changing runtime, SQLx semantics, manifests, workflows or policy.

## Architecture and source of truth

- `PROVEN`: protected admission `main` is `4b6656f688868aa2fb59c18392c2f859f1c5a1c7`.
- `PROVEN`: `Cargo.lock` on that protected main contains `chacha20 0.10.1`.
- `PROVEN`: PR #230 exact-head `cargo deny check` failed because crates.io reports `chacha20 0.10.1` as yanked.
- `PROVEN`: the failing dependency path is transitive through `rand -> sqlx-postgres -> sqlx -> oteryn-game-server`; PR #230 itself does not modify Cargo surfaces.
- `PROVEN`: active Durability PR #212 does not modify Cargo manifests or `Cargo.lock`; its task declares `shared_paths: none`.
- `PROVEN`: owner authorization is durably recorded in Issue #231 comment `5448901431` and is explicitly `Cargo.lock`-only.
- `DERIVED`: this repair must hold the serialized shared-Cargo lease until its implementation PR merges and ownership is released.

## Acceptance criteria

- [ ] This allocation reaches protected `main` before any `Cargo.lock` mutation for #231.
- [ ] Implementation changes exactly `Cargo.lock` plus this task record; no manifest/workflow/runtime/source change.
- [ ] A pre-fix reproduction proves `cargo deny check` fails on the yanked `chacha20 0.10.1` baseline.
- [ ] Dependency resolution selects the smallest compatible non-yanked `chacha20` version accepted by current registry metadata; no unrelated dependency refresh.
- [ ] `cargo metadata --locked` succeeds on the repaired exact head.
- [ ] Required workspace build/test/clippy and supply-chain checks succeed on the repaired exact head.
- [ ] Full PR diff is reviewed, exact-head CI is complete, unresolved required threads are zero, and expected-head merge is used.
- [ ] After merge, the shared `Cargo.lock` lease is released and #230 is requalified against the new protected main.

## Excluded scope

No `Cargo.toml`, `deny.toml`, workflow, Rust source, SQLx version/feature/API/behavior, database schema/migration, protocol, runtime, production, secret, live-data, governance weakening or external-repository mutation. Do not allowlist, suppress, skip or disable yanked-crate checking. Do not perform unrelated dependency updates.

## Implementation / findings

This record is an allocation only until merged. It grants no `Cargo.lock` write authority from the branch itself. The implementation branch must be created from the protected main that contains this allocation, then perform one minimal resolver change and verify the resulting lockfile diff.

## Validation

### Focused

- command/run: pre-fix `cargo deny check --all-features` (or repository-equivalent exact command) on the allocated implementation base
- result: pending

### Component/integration

- command/run: `cargo metadata --locked`; repository-required build/test/clippy for Cargo-lock changes
- result: pending

### E2E

- scenario: `NOT_APPLICABLE` — lockfile-only supply-chain maintenance with no runtime semantic change
- result: `NOT_APPLICABLE`

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: shared Cargo / Rust workspace validation required
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: pending
- verdict: pending

## Independent review

- required: pending; resolve from protected-main risk policy after allocation merge and before implementation PR merge
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #230 blocked by baseline supply-chain failure; #212 is non-overlapping
- protected auto-merge: disabled unless current policy proves eligible
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: owner authorized bounded Cargo.lock-only repair in Issue #231 and exact shared-surface allocation is being created from protected main 4b6656f688868aa2fb59c18392c2f859f1c5a1c7
status: waiting_allocation_merge
branch: fix/supply-chain-yanked-chacha20-231
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
blocker: allocation not yet merged to protected main
next_action: merge the docs-only exact shared-Cargo allocation after its required exact-head CI passes
```
