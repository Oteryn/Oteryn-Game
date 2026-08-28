# OTV2-20260815-tibia-reference-harvest

```yaml
task_id: OTV2-20260815-tibia-reference-harvest
title: Harvest canonical Tibia reference worldmap tooling
mode: MIGRATE
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: tools/OTV2-20260815-tibia-reference-harvest
pr: 283
closeout_pr: 284
base_sha: 0cfd8d8ee3ecf4fbb1cb76cbc9680b53a152e3c1
material_head_sha: 54165596f98b66c3164cccab881bb53f0655cb2b
final_head_sha: 5b3d16c9fdc0f50a58a52e30146456e6473180f0
final_head_frozen_at: 2026-08-15T23:50:00+02:00
delivery_merge_sha: 0c307db73832b824ccf50801e626671e0aeb38d1
owner: chatgpt-github
owner_state: released_after_closeout
created_at: 2026-08-15T23:24:00+02:00
updated_at: 2026-08-16T00:04:48+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths: []
historical_owned_paths:
  - docs/agents/tasks/active/OTV2-20260815-tibia-reference-harvest.md
  - docs/agents/evidence/OTV2-20260815-tibia-reference-harvest.md
  - tools/tibia-worldmap-reconstruction/**
  - .github/workflows/tibia-reference-tools.yml
public_contracts: []
depends_on:
  - blakinio/Oteryn-Platform#1006
  - blakinio/Oteryn-Platform#988
  - ADR-0002 repository ownership and native Rust client migration
blocks: []
cross_repository_coordination_id: OTER-CLIENT-REFERENCE-HARVEST-20260815
external_repositories:
  - blakinio/Oteryn-Platform
source_branch_disposition: auto_deleted_after_merge
source_branch_evidence: branch search after PR #283 merge returned no tools/OTV2-20260815-tibia-reference-harvest ref
```

## Outcome

Move the durable, proprietary-data-free worldmap normalization/comparison tool and the client/game semantic handoff from stale Platform research into the canonical native game/client repository, without importing Platform runner orchestration, credentials, screenshots, official binaries, or live-client control scaffolding.

## Architecture and source of truth

- **PROVEN:** `docs/architecture/ADR-0002-repository-ownership-and-client-migration.md` makes `blakinio/Oteryn-v2` canonical for the native Rust client/server/protocol and makes `blakinio/otclient` historical migration/reference evidence.
- **PROVEN:** Platform ADR 0041 assigns native Client, Game Server, `protocol-oteryn`, native game E2E mechanics, world/content tooling and bounded OTBM migration semantics to the Oteryn-Game lineage, whose current source lineage is `blakinio/Oteryn-v2`.
- **PROVEN:** Platform PR #1006 head `97f8df9e64e1e4f0520440073e497f24dad929ef` contains a proprietary-data-free `tools/tibia-worldmap-reconstruction/**` package with deterministic validation, merge, comparison and fail-closed OTBM export planning.
- **PROVEN:** Platform PR #988 head `f9ff34b37cf81c400a48f7ab9329393416ac304d` is an official-client offline/reference infrastructure task; its game/client conclusions belong here, while its host/isolation harness remains a Platform infrastructure concern.
- **DERIVED:** `blakinio/otclient` is not an eligible target for new harvested work after the accepted cutover.

## Acceptance criteria

- [x] Source ownership is reconciled against current Oteryn-v2 and Platform architecture authority.
- [x] The six safe worldmap reconstruction files are copied from Platform PR #1006 without proprietary assets or live-client material.
- [x] Durable evidence records what was and was not migrated from PRs #1006 and #988.
- [x] No Platform-specific GitHub runner workflow, credential path, login automation, VNC path, gdb/ptrace workflow, private-message action, screenshot/base64 evidence or proprietary binary is migrated.
- [x] Focused Python compile/unit/example validation passes on material head `54165596f98b66c3164cccab881bb53f0655cb2b`.
- [x] Oteryn-v2 required exact-final-head merge-gate/governance checks pass on final delivery head `5b3d16c9fdc0f50a58a52e30146456e6473180f0`.
- [x] Whole-diff self-review found no remaining material ownership, provenance or safety defect after four automated-review findings were repaired.
- [x] Delivery PR #283 merged as `0c307db73832b824ccf50801e626671e0aeb38d1` and its source branch was removed before Platform source research closeout.

## Excluded scope

- No official Tibia client execution, login, account/session data, gameplay automation or live research.
- No BattlEye modification/bypass, hooking, injection, ptrace/debugging or network interception.
- No implementation of `protocol-oteryn`, native runtime, OTBM binary writer or client/server behavior.
- No change to `apps/client/**` in this migration.
- No attempt to prove exact player XYZ, tile passability or collision-aware navigation; those remain separate future research questions.

## Implementation / findings

The migrated worldmap tool remains fail-closed: unobserved data is distinct from empty data, unknown client-to-server ID mappings remain `UNMAPPED`, and the OTBM export plan refuses readiness when observed static data lacks proven mapping or ground identity.

Automated review on initial head `a6ed5231913bb5f59a1f80c2accf69ae2fdb2e8b` found four material edge cases. Repair head `54165596f98b66c3164cccab881bb53f0655cb2b` now:

- skips explicit `observed:false` placeholders in OTBM export readiness;
- prevents an unobserved update from erasing previously observed evidence;
- rejects non-empty incoming `entities` until entity merge semantics are defined;
- rejects JSON booleans from integer coordinate/content fields.

Regression tests cover all four repairs and every corresponding review thread is resolved.

The current Platform-side official Linux reference harness is not copied by this task. It is classified as infrastructure/reference execution tooling rather than native product code. Only its durable client/game findings are recorded in the evidence document.

## Validation

### Focused

- command/run: `Tibia Reference Tooling` run `31910093544`, job `95073747757`
- exact material head: `54165596f98b66c3164cccab881bb53f0655cb2b`
- result: PASS

### Component/integration

- command/run: deterministic synthetic worldmap compile, unit suite and CLI validation
- exact material head: `54165596f98b66c3164cccab881bb53f0655cb2b`
- result: PASS

### E2E

- scenario: `NOT_APPLICABLE` — this migration does not execute a user/runtime journey; it relocates a deterministic offline reference tool with synthetic fixtures only.
- result: NOT_APPLICABLE

### Exact-head CI

Final delivery head: `5b3d16c9fdc0f50a58a52e30146456e6473180f0`.

- Agent governance run `31910514137`: PASS.
- Tibia Reference Tooling run `31910514134`: PASS.
- Merge authority audit run `31910514153`: PASS.
- Merge gate run `31910514148`: PASS, including Windows client validation and repository-selected gates.

Result: PASS.

## Self-review

- material implementation head: `54165596f98b66c3164cccab881bb53f0655cb2b`
- final delivery head: `5b3d16c9fdc0f50a58a52e30146456e6473180f0`
- method/reviewer: whole-diff implementing/coordinating self-review recorded as PR review `4944784918`; final docs-only lifecycle checkpoint was also inspected before merge
- material findings: none remaining after repair
- verdict: PASS

## Independent review

- required: NO — bounded migration of proprietary-data-free Python tooling and evidence; no runtime, protocol, credential, DDL or production behavior changes.
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- material findings: NOT_APPLICABLE
- verdict: NOT_APPLICABLE

## PR and closeout

- delivery PR: #283
- delivery merge: `0c307db73832b824ccf50801e626671e0aeb38d1`
- changed-file review: PASS after repair
- unresolved review threads: 0
- source branch: automatically deleted after merge; verified absent
- closeout PR: #284
- runtime/component/E2E for closeout: NOT_APPLICABLE — bookkeeping-only archive/ownership release
- ownership release: effective when #284 merges

## Context checkpoint

```yaml
last_progress: delivery PR 283 merged to main and source branch deletion verified
status: completed
branch: tools/OTV2-20260815-tibia-reference-harvest
pr: 283
closeout_pr: 284
material_head_sha: 54165596f98b66c3164cccab881bb53f0655cb2b
final_head_sha: 5b3d16c9fdc0f50a58a52e30146456e6473180f0
delivery_merge_sha: 0c307db73832b824ccf50801e626671e0aeb38d1
ci_trigger_source: pull_request
ci_check_generation: terminal-delivery
ci_checks_for_current_head: 1
ci_run_ids:
  - 31910514137
  - 31910514134
  - 31910514153
  - 31910514148
runner_assignment_state: completed
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 1
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Merge lifecycle-only closeout PR #284 after its governance checks pass; then this task is fully terminal and Platform may rely on Oteryn-v2 merge 0c307db73832b824ccf50801e626671e0aeb38d1 as durable destination evidence.
```

## Post-merge closeout

Delivery PR #283 is terminal merged. The canonical destination now exists on `main@0c307db73832b824ccf50801e626671e0aeb38d1`; the implementation source branch is absent; all initial automated-review findings are resolved; no production/live-client authority was expanded. This archive preserves the cross-repository provenance needed by Platform PRs #988 and #1006 without keeping historical client work in `blakinio/otclient`.
