# OTV2-20260818-oteryn-game-transfer-readiness

```yaml
task_id: OTV2-20260818-oteryn-game-transfer-readiness
title: Prepare Oteryn-Game repository transfer and rename
mode: MIGRATE
status: investigating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/otv2-20260818-oteryn-game-transfer-readiness
pr: null
base_sha: 457df3772a7aaf648c1a048b2db2caa409fcf974
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: chat-github-20260818-oteryn-game-transfer-readiness
created_at: 2026-08-18T09:00:00Z
updated_at: 2026-08-18T09:00:00Z
execution_budget_minutes: 120
large_budget_reason: cross-owner repository transfer readiness requires current source/target inventory, Actions/package/release impact analysis, rollback, open-work coordination and exact post-transfer verification planning
owned_paths:
  - docs/agents/tasks/active/OTV2-20260818-oteryn-game-transfer-readiness.md
  - docs/architecture/migration/OTERYN_GAME_REPOSITORY_TRANSFER_READINESS.md
  - docs/architecture/migration/oteryn-game-repository-cutover-inventory.json
public_contracts: []
depends_on:
  - Oteryn/Oteryn ADR 0001 ecosystem topology authority
  - blakinio/Oteryn-Platform OTERYN_ECOSYSTEM_REPOSITORY_MIGRATION programme
blocks:
  - physical transfer and rename blakinio/Oteryn-v2 -> Oteryn/Oteryn-Game
cross_repository_coordination_id: OTERYN-GAME-TRANSFER-20260818
external_repositories:
  - Oteryn/Oteryn
  - Oteryn/Oteryn-Game
  - blakinio/Oteryn-Platform
  - blakinio/github-projects-control
```

## Outcome

Produce a fail-closed, current readiness decision for transferring the existing repository object `blakinio/Oteryn-v2` to organization `Oteryn` and renaming it to `Oteryn-Game` in the same GitHub transfer operation. Preserve history, issues, PRs, repository settings and normal Git/web redirects; identify every non-redirect-safe executable coordinate and package risk before any physical mutation.

## Architecture and source of truth

- **PROVEN:** `Oteryn/Oteryn` ADR 0001 is the canonical ecosystem topology authority and assigns the native Game product to `Oteryn/Oteryn-Game`.
- **PROVEN:** current source repository ID is `1323412342`, current coordinate is `blakinio/Oteryn-v2`, visibility is public, default branch is `main`, and connector permissions include admin/push/pull.
- **PROVEN:** current source `main` at admission is `457df3772a7aaf648c1a048b2db2caa409fcf974`.
- **PROVEN:** exact target `Oteryn/Oteryn-Game` returned 404 at admission.
- **PROVEN:** current Oteryn-v2 repository policy derives live repository identity from `GITHUB_REPOSITORY`; the repository-configuration workflow is same-repository/dynamic.
- **PROVEN:** current recursive source tree contains no `action.yml`, no `action.yaml`, no `Dockerfile`, and no `package.json`; repository code search contains no `workflow_call` and no `ghcr.io` reference.
- **PROVEN:** connected-repository code search found no `Oteryn-v2/.github/workflows` caller; public web search found no exact old-coordinate action/reusable-workflow references and no Oteryn-v2 GHCR references.
- **PROVEN:** current open source PRs at admission are draft PR #335 and draft PR #317.
- **UNKNOWN:** GitHub Packages inventory associated with the personal account/repository cannot yet be enumerated by the available connector; absence of package-producing source paths does not prove absence of manually/historically published packages.
- **UNKNOWN:** target-organization policy may restrict transfer/rollback even though source admin access and organization ownership are otherwise available.

## Acceptance criteria

- [x] Exact source repository identity/head/permissions recorded.
- [x] Exact target coordinate absence recorded.
- [x] Current open PR/work state recorded for cutover revalidation.
- [x] Repository-hosted Action/reusable-workflow provider surface revalidated on current tree.
- [x] Known connected/public caller searches executed and classified.
- [ ] Current GitHub Packages inventory is proven empty or exact package migration/linkage requirements are enumerated.
- [ ] Transfer/rename rollback feasibility is proven for the current owner and organization policy, or explicitly accepted as a residual risk by the owner.
- [ ] Exact preflight/cutover/post-cutover/rollback runbook is committed.
- [ ] Machine-readable cutover inventory classifies old-coordinate references and material unknowns.
- [ ] Full changed-file diff self-review passes with zero material findings.
- [ ] Exact-head required repository checks pass with clean review hygiene.
- [ ] Readiness PR is merged and task lifecycle is closed before any physical transfer is requested.

## Excluded scope

- Do not perform the physical transfer or rename from this readiness branch.
- Do not create a new empty `Oteryn/Oteryn-Game` repository; the target must be the transferred existing repository object so history/PRs/settings remain attached.
- Do not change runtime, protocol, persistence, gameplay, client behavior, production/deployment, secrets, packages or live game state.
- Do not modify Platform, META, Atlas, Otheryn, Canary, otclient or github-projects-control repositories in this task.

## Implementation / findings

Admission evidence materially narrows the previous Wave-1 Actions blocker: the current source is not an Action repository and exposes no reusable workflow through `workflow_call`. Therefore an external caller cannot legitimately depend on a repository-hosted action/reusable workflow that does not exist on the current source tree. Known connected/public searches also found no old-coordinate executable caller. This will be recorded as a bounded current-state proof, not an assertion about unknowable deleted historical content.

The remaining material pre-cutover unknowns are package association/migration state and exact owner/organization rollback permission. GitHub documents that a repository can be transferred to an organization and optionally renamed in the same transfer flow, while packages associated with a repository may transfer or lose their link depending on registry; package state therefore remains fail-closed until proven.

## Validation

### Focused

- current repository/target/access reads: PASS
- current recursive-tree Action/package-producer surface inspection: PASS
- connected/public old-coordinate executable caller search: PASS_WITH_BOUNDED_SCOPE

### Component/integration

- repository transfer simulation: NOT_APPLICABLE; GitHub exposes no non-mutating dry-run endpoint through the connector
- package linkage verification: BLOCKED pending exact package inventory evidence

### E2E

- scenario: NOT_APPLICABLE for readiness documentation; physical transfer is intentionally not executed by this task
- result: NOT_APPLICABLE

### Exact-head CI

- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: pending
- verdict: pending

## Independent review

- required: NO for the readiness document itself unless the final diff expands authority or weakens a safety gate; the physical transfer remains separately gated
- exact head: NOT_APPLICABLE
- method/auditor: NOT_APPLICABLE
- material findings: NOT_APPLICABLE
- verdict: NOT_APPLICABLE

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: source PR #335 and #317 are active unrelated work that must be revalidated at cutover, not closed by this task
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: created dedicated migration-readiness branch and durable task after current source/target/Actions/open-work preflight
status: investigating
branch: docs/otv2-20260818-oteryn-game-transfer-readiness
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
owner_action_required: prove current GitHub Packages state and confirm rollback transfer permission before physical cutover readiness can become CUTOVER_READY
blocker: package inventory and current organization transfer-back rollback policy are not yet proven
next_action: commit the exact readiness report and machine-readable inventory, then open a Draft PR and validate repository-required checks while preserving the two material unknowns fail-closed
```