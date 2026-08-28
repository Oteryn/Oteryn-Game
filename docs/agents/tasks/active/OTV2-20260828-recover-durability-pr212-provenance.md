# OTV2-20260828-recover-durability-pr212-provenance

```yaml
task_id: OTV2-20260828-recover-durability-pr212-provenance
title: Prospectively reconstruct the Durability candidate after PR #212 provenance incident
mode: COORDINATE
recovery_class: PROSPECTIVE_SUCCESSOR_RECONSTRUCTION
status: ALLOCATION_PENDING_PROTECTED_MAIN_MERGE
repository: Oteryn/Oteryn-Game
base_branch: main
branch: coord/durability-212-recovery-allocation
pr: 242
base_sha: 7c2da078596a7d2e27c3066ff74ac69b8b7f9af6
head_sha: external_pr_evidence
final_head_sha: external_pr_evidence
head_evidence_surface: "PR #242 live head plus immutable PR comments/reviews/workflow runs"
successor_branch: recovery/durability-212-owned-successor
issue: 240
parent_issue: 162
implementation_issue: 167
source_pr: 212
allocation_base_sha: 7c2da078596a7d2e27c3066ff74ac69b8b7f9af6
prior_paused_head_evidence: a4d1d5c475e8da49d14707f64e99419010cd7bd6
source_head_evidence_only: fb30fba2a888835dfc7cbde27f940b79d7bfe05d
source_manifest: Oteryn/Oteryn-Game#240 comment 5453015299
last_pre_incident_ownership_correct_head: 8f6f1de4e363eb707db655a8af416c3fcb998e89
compromised_ancestors:
  - cd808d396018832b632be26911105a36f0cb7a20
  - 73e17f418c63ec038f5aa7ef8f0888ac74b75aa2
owner_before_allocation_merge: Oteryn: work coordinator
owner_after_allocation_merge: Oteryn: sol durability lead
owner: Oteryn: work coordinator
created_at: 2026-08-28T13:04:06Z
updated_at: 2026-08-28T13:28:43Z
execution_budget_minutes: 120
large_budget_reason: the successor must reproduce a high-risk PostgreSQL durability candidate, including fresh exact-head E2E, review and CI after an evidence-preserving provenance recovery
owned_paths:
  - docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md
  - docs/agents/tasks/active/OTV2-20260825-work-delivery-coordinator.md
  - docs/agents/tasks/active/OTV2-20260825-impl-durability.md
  - docs/agents/tasks/active/OTV2-20260828-recover-durability-pr212-provenance.md
  - docs/superpowers/plans/2026-08-28-oteryn-game-durability-branch-provenance-recovery.md
owned_paths_after_allocation_merge:
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/durability/db.rs
  - apps/game-server/src/durability/schema.rs
  - apps/game-server/src/durability/admission_journal.rs
  - apps/game-server/src/durability/reconcile.rs
  - apps/game-server/src/bin/oteryn-game-migrate.rs
  - apps/game-server/migrations/0001_admission_reconnect_journal.sql
  - apps/game-server/build.rs
  - apps/game-server/tests/durability_postgres.rs
  - apps/game-server/tests/support/postgres.rs
  - docs/agents/tasks/active/OTV2-20260828-recover-durability-pr212-provenance.md
source_blob_manifest_evidence_only:
  apps/game-server/build.rs: 3a8149ef075f6896a7435c716cb8a4de5d94606b
  apps/game-server/migrations/0001_admission_reconnect_journal.sql: 52aa1931550df3be6ab97d8b5a6814559f4ae494
  apps/game-server/src/bin/oteryn-game-migrate.rs: 80e72fcdeeb70359986a5f93fe287362c0d205a1
  apps/game-server/src/durability/admission_journal.rs: 336fbf4ed5f2cd740ab954261b924011030c272d
  apps/game-server/src/durability/db.rs: 48746007625646dee9d8a44972005cacb2a97c73
  apps/game-server/src/durability/mod.rs: f37fd5e1d8ae50e8b71391a85da73369ac25fcb5
  apps/game-server/src/durability/schema.rs: 8c92e301bd420a386f8684025ba429903b1b6e91
  apps/game-server/tests/durability_postgres.rs: 460ad5888d8e870bbeda50a3dc8f64b24a30c1cb
  apps/game-server/tests/support/postgres.rs: bcb243f6c4823a14ec8116b72439c2c79c115d94
public_contracts:
  - DUR-RECONNECT-AUTHORITY-V1
  - DUR-RECONNECT-TRANSPORT-REF-UNIQUENESS-V1
  - FND-04B_RECONNECT_RECOVERY_CONTINUITY_CONTRACT
depends_on:
  - issue:162 active Work control plane
  - issue:167 original Durability scope
  - issue:240 recovery lifecycle
  - protected-main allocation merge from this task branch
blocks:
  - Server Seam remains WAITING_DEPENDENCY until this successor is independently qualified and merged
write_authority: coordinator_docs_only_until_this_allocation_merges
excluded_scope:
  - Cargo.toml
  - Cargo.lock
  - apps/game-server/Cargo.toml
  - apps/game-server/src/lib.rs
  - Foundation paths
  - Server Seam paths
  - workflow/governance/registry paths
  - production database/configuration/secrets
  - Platform/Atlas/META/external repositories
external_repositories: []
```

## Purpose

Recover the Durability delivery lifecycle without rewriting history or claiming that the existing restoration commit had prior authority. This is an allocation only. It creates no runtime candidate and grants no worker write authority until the allocation is merged to protected `main`.

## Architecture and source of truth

- `PROVEN`: `AGENTS.md`, `docs/agents/AGENTS.md`, `docs/agents/prompts/OTV2_WORK_DELIVERY_COORDINATOR.md`, `docs/agents/SESSION_RECOVERY_AND_ORPHANED_EXECUTION.md`, `docs/agents/GITHUB_ONLY_EXECUTION.md`, and `docs/agents/CODEX_REVIEW_POLICY.json` govern this allocation.
- `PROVEN`: protected `main@7c2da078596a7d2e27c3066ff74ac69b8b7f9af6` is the allocation base read during the recovery preflight; the coordinator must refresh it before publishing this allocation and classify any advance before write.
- `PROVEN`: Issue #162 is the active Work control plane, Issue #167 is the original Durability scope, and Issue #240 is the recovery lifecycle. No unmerged document expands authority.
- `PROVEN`: Issue #240 comment `5453015299` binds `fb30fba2a888835dfc7cbde27f940b79d7bfe05d` and its nine listed blobs as the sole historical source manifest. It is evidence only, not historical authorization or qualification.
- `PROVEN`: `a4d1d5c475e8da49d14707f64e99419010cd7bd6` is the earlier paused-head evidence and the source of the unresolved reservation-through-COMMIT Codex P2. It remains a required successor acceptance predicate even though the later bound source manifest supplies content evidence.
- `PROVEN`: the named owner has no runtime write authority before this allocation merges. The coordinator owns only the five documentation paths listed above, including the bounded status/provenance correction in the existing Durability packet.
- `DERIVED`: the successor must be a new, clean history rooted at this allocation's protected-main merge SHA. The old PR/tree supplies read-only content evidence only, never authority or commit ancestry.

## Proven facts

- Draft PR #212 currently exposes the Issue #240-bound source `fb30fba2a888835dfc7cbde27f940b79d7bfe05d`, but it retains destructive cross-scope commit `cd808d396018832b632be26911105a36f0cb7a20` and unallocated restoration `73e17f418c63ec038f5aa7ef8f0888ac74b75aa2` in its ancestry.
- A later clean tree, CI, task update or allocation cannot retroactively make either historical mutation authorized.
- The existing branch and Draft PR are required forensic/TDD/review evidence. They must remain unchanged until the successor exists; no force-push, reset, rebase, deletion, review qualification or merge is authorized.
- The prospective successor is independently required by a real provenance incident; it is not a duplicate branch/PR created to escape CI or a lost worker session.

## Allocation outcome after protected-main merge

The named Sol Durability lead receives one branch and one writable worktree from the exact allocation merge SHA. It may inspect the old PR/tree/commits as read-only evidence, but it must not merge, cherry-pick, rebase, or otherwise import the compromised ancestry. It reconstructs only the listed owned path blobs/diff in the clean successor history, treats every prior CI/review as historical, and re-establishes its own TDD, isolated PostgreSQL evidence, exact-head CI and independent review.

The current open Codex P2 on PR #212 is not disposed by this allocation. The successor must independently reproduce or resolve its contract-level acceptance predicate within the listed paths before candidate freeze.

## Acceptance criteria

- [ ] This allocation changes only its five coordinator-owned documentation paths; the existing Durability packet changes only status, provenance, blocker, no-write and next-action fields under the recorded correction authority.
- [ ] It records Issue #240, the exact pre-incident/source/compromised SHAs, the no-ratification finding, and the immutable-evidence status of PR #212.
- [ ] It grants no runtime, shared-path, secret, production, workflow, Cargo or external-repository write authority before the protected-main allocation merge.
- [ ] It transfers exactly the listed successor paths to one named Sol Durability lead only after the allocation merge, and makes the successor branch/root rule explicit.
- [ ] It preserves the CodeX P2 reservation-race finding as an unclosed successor acceptance condition.
- [ ] `git diff --check`, governance validation and repository-policy validation pass; runtime and PostgreSQL E2E are `NOT_APPLICABLE` for this allocation-only PR.
- [ ] An exact-head independent review is complete under `CODEX_REVIEW_POLICY.json` before protected-main merge; all required findings are resolved in the authorized scope.

## Excluded scope

This allocation does not implement, repair, merge, cherry-pick, rebase, force-push, delete or otherwise mutate the Durability runtime candidate. It does not grant ownership of Cargo/workspace files, `apps/game-server/Cargo.toml`, `apps/game-server/src/lib.rs`, Foundation, Server Seam, workflows, registries, contracts, production databases/configuration/secrets, external repositories, accounts or environments. It does not close or qualify PR #212.

## Required successor lifecycle

1. Read fresh GitHub `main`, #162, #167, #240, PR #212, this task and the Durability contracts. Confirm that this allocation is merged, the new branch starts from the recorded allocation merge SHA, and the source commit/blobs still exactly equal the Issue #240 manifest.
2. Preserve the original branch/PR untouched. Record the bound source commit/tree/blob identities and the earlier paused `a4d1...` evidence in the successor PR/task as evidence only.
3. Reconstruct only the exact owned paths in `recovery/durability-212-owned-successor`. Do not use compromised commit ancestry and do not introduce a shared-path edit.
4. Create/re-run focused RED/GREEN coverage for every recovered semantic change. In particular, the retained transport reservation must remain protected through COMMIT so a concurrent delete/reassignment cannot publish an ACTIVE controller from incomplete durable evidence.
5. Run the current task-required PostgreSQL migration/fencing/reconciliation E2E plus full exact-head repository validation. The old PR's green workflow results are not successor qualification.
6. Freeze the successor exact head. The allocated lane lead requests fresh native Codex review under `CODEX_REVIEW_POLICY.json`, resolves all required findings in the successor allocation, and obtains fresh review/CI evidence.
7. Only after a successor PR exists may the control plane close #212 as superseded while retaining its branch/history as immutable evidence. The normal integration/merge/issue/archive lifecycle then applies to the successor.

## Implementation / findings

The coordinator opened Issue #240 and paused PR #212 before this allocation write. The independent audit identified the historical destructive commit and unauthorized restoration as a P0 provenance failure. This allocation is the only prospective remedy: it preserves the original branch and Draft PR for audit/TDD evidence, creates one clean successor after protected-main merge, and requires fresh validation/review. It does not amend the historical record.

## Validation

### Focused

- command: `python3 tools/agents/validate_governance.py`
- result: pending exact-head allocation validation

### Repository policy

- command: `python3 tools/repository/validate_repository_policy.py`
- result: pending exact-head allocation validation

### Runtime / E2E

- scenario: `NOT_APPLICABLE` — this PR allocates documentation authority only and changes no runtime behavior.
- result: `NOT_APPLICABLE`

### Exact-head CI

- initial published head: `e5bb7137e816df51be3ebca728af6077a35bf471`
- final exact-head: external PR #242 evidence surface
- required workflows: governance, architecture semantic audit, merge authority audit, Rust workspace and merge gate on the final PR head
- result: pending

## Self-review

- initial reviewed head: `e5bb7137e816df51be3ebca728af6077a35bf471`
- final reviewed head: external PR #242 evidence surface
- method: coordinator checks the full staged/PR diff against Issue #240, the parent authority and exact owned paths
- verdict: pending

## Independent review

- required: `CODEX_REQUIRED` — this is a governance/authority allocation and is outside the ordinary docs-only automatic exemption because it changes `docs/agents` paths.
- method: native exact-head Codex review plus independent coordinator/audit evidence; the future Durability lane candidate must obtain its own separate exact-head review.
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- protected auto-merge: not used
- merge commit/result: pending protected-main exact-head validation and review
- ownership release: none until merge; after merge, only the named successor owns the listed runtime paths for one successor branch/PR.

## Context checkpoint

```yaml
last_progress: PR #242 opened Draft at e5bb7137e816df51be3ebca728af6077a35bf471; Issue #240 comment 5453015299 binds fb30fba2a888835dfc7cbde27f940b79d7bfe05d and nine exact source blobs as evidence only; a4d1 remains earlier paused evidence with the unresolved COMMIT-reservation P2
status: ALLOCATION_PENDING_PROTECTED_MAIN_MERGE
branch: coord/durability-212-recovery-allocation
head_sha: external_pr_evidence
pr: 242
final_head_sha: external_pr_evidence
head_evidence_surface: "PR #242 live head plus immutable PR comments/reviews/workflow runs"
owner_action_required: null
blocker: allocation not yet merged; original PR #212 must remain evidence-only
next_action: qualify and merge this docs-only allocation, then create one clean successor branch from its protected-main merge SHA
```
