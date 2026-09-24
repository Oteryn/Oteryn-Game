# OTV2-20260924-nonshipping-movement-structural-fixture

```yaml
task_id: OTV2-20260924-nonshipping-movement-structural-fixture
issue: 162
title: Non-shipping structural Movement fixture
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/nonshipping-movement-structural-fixture-20260924
pr: null
base_sha: c516182255d3ea1724e91671c8d2187eb622e3df
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: delegated Sol 6 writer under coordinator #162
created_at: 2026-09-24
updated_at: 2026-09-24
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/foundation/runtime_actor_carrier.rs
  - apps/game-server/src/foundation/movement_static_kernel_structural_tests.rs
  - docs/agents/tasks/active/OTV2-20260924-nonshipping-movement-structural-fixture.md
  - docs/agents/evidence/OTV2-20260924-nonshipping-movement-structural-fixture.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and authority

- PROVEN allocation: [#162 comment 5823029665](https://github.com/Oteryn/Oteryn-Game/issues/162#issuecomment-5823029665), protected base above, four exact paths, one writer, independent exact-head Luna 6 review and coordinator-only integration.
- PROVEN existing seams: `content::reference_static_cell::ReferenceStaticCellIndex` is test-only; `foundation::runtime_actor_carrier` privately owns actor positions and compare-commit.
- The output is a test-only structural kernel and focused tests, not Reference parity, Content activation, production Movement wiring or MOVE-RL-03 proof.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this change creates only a `#[cfg(test)]` child and synthetic tests; it does not add or alter any production mutation, authority grant, controller, durable write or recovery consumer. The fixture nevertheless tests stale actor/owner and owner compare-commit rejection.

## Acceptance criteria

- [ ] Checked adjacent N/E/S/W target and one exact complete-key lookup per qualified attempt, with no scan/fallback.
- [ ] Synthetic WALKABLE commits only through the carrier owner; BLOCKED, absent, unqualified, conflict, scope/context mismatch and coordinate overflow reject without position or revision change.
- [ ] Stale actor/owner, stale snapshot, intervening write and replay reject; prelookup rejections perform zero cell lookups.
- [ ] Exact four-path delta, focused Rust tests, strict Clippy/fmt and full exact-head `game-gate` pass; independent Luna 6 review and coordinator-only Merge Queue remain pending until proven.

## Excluded scope

No diagonal, stairs/floor transition, pathfinding, dynamic occupancy, visibility, teleport, timing, production Movement, `lib.rs`, Content activation, Cargo, resource registry or protocol. Fixture bounds are not production maxima. Preserve the immutable July-28 Reference target and existing production evidence linkage; #139 production MOVE-RL-03 and Reference gates remain open.

## Implementation and validation

The test child delegates one candidate read to the existing Content index, then commits a proposal with the existing carrier compare-commit. Negative cases assert the owner's unchanged position/revision and lookup count. Evidence: `docs/agents/evidence/OTV2-20260924-nonshipping-movement-structural-fixture.md`.

- Focused: pending exact-head CI; local `cargo`/`rustfmt` unavailable in the worker workspace.
- Strict Clippy/fmt and full `game-gate`: pending exact-head CI.
- E2E: `NOT_APPLICABLE` for a non-shipping, uncomposed test-only fixture.
- Self-review: pending exact-head delta.
- Independent review: required, pending coordinator-arranged read-only Luna 6 review.
- PR, review threads, Merge Queue, merge_group and protected-main readback: pending coordinator lifecycle.

## Context checkpoint

```yaml
last_progress: Test-only fixture authoring on allocated branch
status: implementing
branch: agent/nonshipping-movement-structural-fixture-20260924
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
blocker: null
next_action: Publish four-path high-level API candidate, then freeze exact remote head
```
