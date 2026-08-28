# OTV2-20260828-durability-provenance-recovery

```yaml
task_id: OTV2-20260828-durability-provenance-recovery
title: Recover #167 Durability branch provenance
mode: COORDINATE
status: ALLOCATION_PR_PENDING
repository: Oteryn/Oteryn-Game
base_branch: main
branch: coord/durability-provenance-recovery-240
issue: 240
parent_coordinator_issue: 162
affected_issue: 167
affected_pr: 212
admission_main_sha: 7c2da078596a7d2e27c3066ff74ac69b8b7f9af6
control_plane_profile: OTV2_WORK_DELIVERY_COORDINATOR
owner: Oteryn: work coordinator
successor_lane_id: OTV2-IMPL-DURABILITY-RECOVERY-240
successor_task_id: OTV2-20260828-impl-durability-successor
successor_branch: impl/game-durability-journal-recovery-240
successor_pr: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260828-durability-provenance-recovery.md
  - docs/agents/tasks/active/OTV2-20260828-impl-durability-successor.md
  - docs/superpowers/plans/2026-08-28-durability-provenance-recovery.md
runtime_write_authority: none
shared_lease: none
external_repositories: []
```

## Proven incident state

- `PROVEN`: protected `main` at recovery admission is `7c2da078596a7d2e27c3066ff74ac69b8b7f9af6`.
- `PROVEN`: the historical #167 branch is `impl/game-durability-journal`; PR #212 is Draft and must not qualify or merge.
- `PROVEN`: destructive commit `cd808d396018832b632be26911105a36f0cb7a20` crossed #167 ownership boundaries.
- `PROVEN`: restoration `73e17f418c63ec038f5aa7ef8f0888ac74b75aa2` was performed without the recovery allocation required by coordinator evidence on #162.
- `PROVEN`: later writes continued on that ancestry after `PAUSED_BRANCH_PROVENANCE_RECOVERY`; current observed #212 head at recovery design time is `fb30fba2a888835dfc7cbde27f940b79d7bfe05d`.
- `PROVEN`: `main...fb30fba2` is currently ownership-shaped to the ten historical #167 paths, but a clean final tree cannot retroactively repair missing mutation authority in published ancestry.
- `PROVEN`: Issue #240 requires a clean successor from protected main and preservation of #212 as immutable historical evidence.

## Allocation decision

Do not ratify the historical recovery. Do not force-push, reset, rebase, replace or delete the historical branch. The only authorized recovery is a new successor branch created after this allocation PR merges.

The successor may inspect/copy historical #212 file snapshots read-only, but it must not inherit commits, review verdicts, CI qualification or mutation authority from #212. It must produce its own TDD evidence, exact-head CI and required independent review.

## Successor allowlist

After this allocation merges and protected-main readback proves its merge SHA, `Oteryn: sol durability lead` may mutate only:

- `apps/game-server/build.rs`
- `apps/game-server/migrations/0001_admission_reconnect_journal.sql`
- `apps/game-server/src/bin/oteryn-game-migrate.rs`
- `apps/game-server/src/durability/admission_journal.rs`
- `apps/game-server/src/durability/db.rs`
- `apps/game-server/src/durability/mod.rs`
- `apps/game-server/src/durability/schema.rs`
- `apps/game-server/tests/durability_postgres.rs`
- `apps/game-server/tests/support/postgres.rs`
- `docs/agents/tasks/active/OTV2-20260828-impl-durability-successor.md`

No `Cargo.toml`, `Cargo.lock`, Foundation, Server Seam, workflow, registry, composition-root, governance, production, secret, live-data or external-repository write is included. `apps/game-server/src/durability/reconcile.rs` is not included because it is not part of the current ownership-shaped #212 candidate diff; any later need for it requires explicit control-plane reallocation before mutation.

## Recovery gates

1. Allocation PR must remain docs-only and path-exact.
2. Required governance/review/exact-head checks must pass on the unchanged allocation head.
3. Allocation must merge with expected-head protection; successor release occurs only after protected-main readback.
4. Successor branch must start from the allocation-recorded protected main, never from #212 ancestry.
5. Successor reconstruction copies only allowed file contents; no cherry-pick from #212.
6. Successor must independently re-run focused PostgreSQL/Rust evidence and current exact-head repository gates.
7. Required Codex review, if routed `CODEX_REQUIRED`, is requested by the allocated Durability lane lead under current policy, not by the Work coordinator.
8. PR #212 may be closed as superseded only after the successor PR exists and preserves links to #212/#240; its branch/history remain intact.

## Validation

### Allocation

- focused: `python tools/agents/validate_governance.py`
- runtime E2E: `NOT_APPLICABLE` — this task only establishes recovery authority and does not change runtime behavior.
- exact-head CI: required before allocation merge.

## Context checkpoint

```yaml
status: ALLOCATION_PR_PENDING
branch: coord/durability-provenance-recovery-240
head_sha: null
final_head_sha: null
pr: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260828-durability-provenance-recovery.md
  - docs/agents/tasks/active/OTV2-20260828-impl-durability-successor.md
  - docs/superpowers/plans/2026-08-28-durability-provenance-recovery.md
blocker: allocation_not_yet_merged
owner_action_required: null
next_action: publish the docs-only recovery allocation PR and qualify its exact head
```
