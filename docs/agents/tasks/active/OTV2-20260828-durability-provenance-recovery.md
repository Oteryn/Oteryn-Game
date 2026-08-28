# OTV2-20260828-durability-provenance-recovery

```yaml
task_id: OTV2-20260828-durability-provenance-recovery
title: Recover #167 Durability branch provenance
mode: COORDINATE
status: REVIEW_RECONCILIATION_REQUIRED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: coord/durability-provenance-recovery-240
issue: 240
pr: 241
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
source_snapshot_head: fb30fba2a888835dfc7cbde27f940b79d7bfe05d
exact_final_head_evidence: immutable PR #241 review/check evidence after the final tracked-file commit; a commit cannot contain its own SHA
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
- `PROVEN`: recovery allocation is published as PR #241 from `coord/durability-provenance-recovery-240`.
- `PROVEN`: the historical #167 branch is `impl/game-durability-journal`; PR #212 is Draft and must not qualify or merge.
- `PROVEN`: destructive commit `cd808d396018832b632be26911105a36f0cb7a20` crossed #167 ownership boundaries.
- `PROVEN`: restoration `73e17f418c63ec038f5aa7ef8f0888ac74b75aa2` was performed without the recovery allocation required by coordinator evidence on #162.
- `PROVEN`: later writes continued on that ancestry after `PAUSED_BRANCH_PROVENANCE_RECOVERY`; current observed #212 head at recovery design time is `fb30fba2a888835dfc7cbde27f940b79d7bfe05d`.
- `PROVEN`: `main...fb30fba2` is currently ownership-shaped to the ten historical #167 paths, but a clean final tree cannot retroactively repair missing mutation authority in published ancestry.
- `PROVEN`: Issue #240 requires a clean successor from protected main and preservation of #212 as immutable historical evidence.
- `PROVEN`: repository closeout policy forbids a self-referential follow-up commit merely to populate the commit's own SHA; after the last material tracked-file change, exact current head is recorded in immutable PR #241 review/check evidence.

## Allocation decision

Do not ratify the historical recovery. Do not force-push, reset, rebase, replace or delete the historical branch. The only authorized recovery is a new successor branch created after this allocation PR merges.

The successor may inspect/copy file contents only from exact historical source snapshot `fb30fba2a888835dfc7cbde27f940b79d7bfe05d`. Any later #212 head is not automatically admissible source material and requires a new durable control-plane source-admission decision before any bytes are copied from it. The successor must not inherit commits, review verdicts, CI qualification or mutation authority from #212 and must produce its own TDD evidence, exact-head CI and required independent review.

## Mandatory successor-owned TDD provenance

The clean successor may not reconstruct tests and final implementation in one generation.

1. After this allocation is canonical, create the successor branch from the allocation-recorded protected-main SHA.
2. First copy only the frozen test blobs:
   - `apps/game-server/tests/durability_postgres.rs` = `460ad5888d8e870bbeda50a3dc8f64b24a30c1cb`;
   - `apps/game-server/tests/support/postgres.rs` = `bcb243f6c4823a14ec8116b72439c2c79c115d94`.
3. Publish a Draft successor PR and run the focused Durability target on that exact test-only head. It must visibly **FAIL** because the production Durability module is absent. Skipped/not-run is not RED.
4. Preserve exact RED head/run evidence before adding production bytes.
5. Only then copy the seven frozen implementation/migration/build blobs from `fb30fba2...` and rerun the same target to **GREEN**.
6. Historical #212 test results cannot satisfy either the RED or GREEN successor gate.

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
5. Successor reconstruction copies only allowed file contents from exact source snapshot `fb30fba2a888835dfc7cbde27f940b79d7bfe05d`; no cherry-pick from #212 and no later source head without separate admission.
6. Successor must prove its own visible test-only RED generation before any implementation blob restore, then prove GREEN after exact implementation reconstruction.
7. Successor must independently re-run focused PostgreSQL/Rust evidence and current exact-head repository gates.
8. Required Codex review, if routed `CODEX_REQUIRED`, is requested by the allocated Durability lane lead under current policy, not by the Work coordinator.
9. PR #212 may be closed as superseded only after the successor PR exists and preserves links to #212/#240; its branch/history remain intact.

## Validation

### Allocation

- focused: `python tools/agents/validate_governance.py`
- runtime E2E: `NOT_APPLICABLE` — this task only establishes recovery authority and does not change runtime behavior.
- exact-head CI: required before allocation merge.

## Context checkpoint

```yaml
status: REVIEW_RECONCILIATION_REQUIRED
branch: coord/durability-provenance-recovery-240
pr: 241
head_sha: null
final_head_sha: null
exact_head_evidence: record the exact post-repair PR #241 head in immutable PR review/check evidence; do not create a self-referential status commit
owned_paths:
  - docs/agents/tasks/active/OTV2-20260828-durability-provenance-recovery.md
  - docs/agents/tasks/active/OTV2-20260828-impl-durability-successor.md
  - docs/superpowers/plans/2026-08-28-durability-provenance-recovery.md
blocker: fresh_exact_head_review_and_ci_required_after_task_identity_repair
owner_action_required: null
next_action: record the new exact PR #241 head in immutable evidence, run fresh whole-diff self-review/Codex/CI, then expected-head merge only if all gates are clean
```
