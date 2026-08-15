# OTV2-20260815-tibia-reference-harvest

```yaml
task_id: OTV2-20260815-tibia-reference-harvest
title: Harvest canonical Tibia reference worldmap tooling
mode: MIGRATE
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: tools/OTV2-20260815-tibia-reference-harvest
pr: 283
base_sha: 0cfd8d8ee3ecf4fbb1cb76cbc9680b53a152e3c1
material_head_sha: 54165596f98b66c3164cccab881bb53f0655cb2b
final_head_sha: null
final_head_frozen_at: null
owner: chatgpt-github
created_at: 2026-08-15T23:24:00+02:00
updated_at: 2026-08-15T23:51:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
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
- [ ] Oteryn-v2 required exact-final-head merge-gate/governance/CodeQL checks pass after this lifecycle checkpoint commit.
- [x] Whole-diff self-review found no remaining material ownership, provenance or safety defect after four automated-review findings were repaired.
- [ ] Merge and task closeout are terminal before the Platform source research PRs are closed/deleted.

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
- exact head: `54165596f98b66c3164cccab881bb53f0655cb2b`
- result: PASS

### Component/integration

- command/run: deterministic synthetic worldmap compile, unit suite and CLI validation
- exact head: `54165596f98b66c3164cccab881bb53f0655cb2b`
- result: PASS

### E2E

- scenario: `NOT_APPLICABLE` — this migration does not execute a user/runtime journey; it relocates a deterministic offline reference tool with synthetic fixtures only.
- result: NOT_APPLICABLE

### Exact-head CI

- material head: `54165596f98b66c3164cccab881bb53f0655cb2b`
- observed PASS: Merge authority audit, merge-gate scope/governance/Rust policy/Rust supply chain/Linux workspace/dependency review, CodeQL actions/python, standalone CodeQL and focused worldmap reconstruction.
- remaining before final freeze: fresh pull-request generation for the lifecycle checkpoint commit, including Agent governance with the corrected current PR body and any repository-required Windows job.
- result: PENDING FINAL GENERATION

## Self-review

- exact head: `54165596f98b66c3164cccab881bb53f0655cb2b`
- method/reviewer: whole-diff implementing/coordinating self-review recorded as PR review `4944784918`
- material findings: none remaining after repair
- verdict: PASS

## Independent review

- required: NO — bounded migration of proprietary-data-free Python tooling and evidence; no runtime, protocol, credential, DDL or production behavior changes.
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- material findings: NOT_APPLICABLE
- verdict: NOT_APPLICABLE

## PR and closeout

- PR: #283
- changed-file review: PASS on material head after repair
- unresolved review threads: 0
- related/superseded PRs: Platform #1006 and #988 remain open until destination/Platform harvest is merged
- protected auto-merge: pending final exact-head generation
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: four automated-review findings repaired and material-head validation/self-review passed
status: validating
branch: tools/OTV2-20260815-tibia-reference-harvest
pr: 283
material_head_sha: 54165596f98b66c3164cccab881bb53f0655cb2b
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request
ci_check_generation: lifecycle-checkpoint-refresh
ci_checks_for_current_head: 1
ci_run_ids:
  - 31910093544
  - 31910093485
  - 31910093480
ci_job_ids:
  - 95073747757
  - 95073860301
  - 95073747527
runner_assignment_state: assigned
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 1
ci_recovery_actions_for_current_head: 1
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: Validate the fresh pull-request generation created by this task checkpoint commit; if all required checks pass and no new review finding exists, merge PR #283 and archive/release this task before Platform source PR #1006 is closed.
```
