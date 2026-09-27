# OTV2-20260917-reference-evidence-manifest-v4

```yaml
task_id: OTV2-20260917-reference-evidence-manifest-v4
title: Register bounded Reference evidence manifest revision 4
mode: CONTRACT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/reference-manifest-v4-514
issue: 514
pr: 640
base_sha: 0e48ae518842fafc8b9e0867eb8ce041dc1a4332
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: REFERENCE_MANIFEST_V4_514
created_at: 2026-09-17T07:14:16Z
updated_at: 2026-09-27T18:39:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json
  - docs/agents/evidence/OTV2-20260917-reference-manifest-v4-source-packet.md
  - docs/agents/tasks/active/OTV2-20260917-reference-evidence-manifest-v4.md
public_contracts:
  - docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json
depends_on:
  - "#162 comment 5710521667"
  - "#162 comment 5711026915"
  - "#514"
  - "protected #576 evidence"
  - "protected #637 evidence"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Advance the accepted Reference evidence/parity registry exactly once from manifest revision 3 to 4. Preserve schema v1 byte-for-byte, retain all revision-3 cases/history, add six bounded `DERIVED` cases and one fail-closed `UNKNOWN` current-observation case from the allocated evidence packet, preserve `UNKNOWN`/`CONFLICT` ceilings, and make no runtime or parity-confirmed claim.

## Architecture and source of truth

- `PROVEN`: #162 comment `5710521667` allocates this worker, branch, admission SHA, three writable paths, and the pinned SourceMeta validation mechanism.
- `PROVEN`: #162 comment `5711026915` selects the bounded schema-v1 serialization `STRUCTURED_REFERENCE_DATA -> COMMUNITY_CORROBORATION` for external structured Rat wiki locators without evidence promotion.
- `PROVEN`: `docs/architecture/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1_OWNER_ACCEPTANCE.md` freezes schema v1, the nine-domain inventory, independent target/implementation/parity axes, fail-closed classifications, and digest policy.
- `PROVEN`: `docs/architecture/REFERENCE_EVIDENCE_PARITY_MANIFEST_CONTRACT.md` requires validation against the exact normative schema.
- `DERIVED`: six bounded target claims are admitted at the field ceilings recorded in the public source packet.
- `UNKNOWN`: Goblin Intruder retaliation is retained only as a current post-target observation because target-cut continuity is unproven.
- `UNKNOWN`: excluded formulas, mappings, timing edges, RNG/probabilities, identifiers, catalogues, exact retrieval instants not captured by protected evidence, and runtime behavior remain unregistered or explicitly outside each case.
- `CONFLICT`: existing conflicts are not rewritten or resolved by this task.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: this is a paper evidence/manifest revision. It performs no production mutation, authority-bearing session operation, persistence recovery interpretation, PREPARE/COMMIT operation, controller installation, or live-account action.

## Acceptance criteria

- [x] Exact branch reused from admission `main@b44fefe08f6aaf1b2c1c23dedd92bab0de87146e`.
- [x] Only the three allocated paths are changed.
- [x] Schema v1 is byte-identical.
- [x] `schema_version=1`, `manifest_revision=4`, target ID unchanged, and `canonical_digest=null`.
- [x] All four revision-3 cases and history entries are unchanged.
- [x] Seven cases are admitted only at supported field/classification ceilings; none is `PARITY_CONFIRMED`.
- [x] Every declared difference names the accepted Oteryn Reference death contract and the declared-difference case is scoped only to progression loss.
- [x] Public-safe source packet records exact locators/source types, explicit unique evidence anchors, truthful date/timestamp precision, provenance/legal dispositions, and uncertainty.
- [x] Exact repaired manifest/source generation passes pinned SourceMeta schema validation, JSON parse, semantic invariants, and diff checks; successor repository governance remains exact-head gated.
- [ ] Exact-head hosted CI and whole-diff review complete with zero unresolved material threads.

## Excluded scope

Schema mutation/versioning; runtime/client/server/Content/Cargo/workflow/registry/resource-limit/production changes; digest computation; Radiant threshold promotion without cleared exact source; starter template; exact XP thresholds/formulas; natural Rat loot probabilities; exact ten-second boundary scheduling; any OTS promotion; implementation fixtures; merge/enqueue.

## Implementation / findings

- Preserved the original revision-3 case objects without field edits.
- Used `WORLD_INTERACTION` for the `movement.*` evidence case under the accepted schema-v1 taxonomy; runtime ownership is unchanged.
- Omitted Radiant Skyhold because the recorded source refresh did not clear the exact official target-boundary source for `PROVEN`.
- Serialized target-near structured Rat evidence as schema-v1 `COMMUNITY_CORROBORATION` under #162 comment `5711026915`, retaining the investigator role `STRUCTURED_REFERENCE_DATA` in the source packet and classification `DERIVED`.
- Kept Oteryn implementation state `NOT_STARTED` and exact revision/test links empty for every added case.
- Second repair generation narrowed `character.death.low_level_base_xp_skill_loss.v1` to Global progression-loss semantics only; the Oteryn difference remains parity-side through the accepted difference reference.
- Second repair generation added explicit unique anchors for every manifest `artifact_id` and replaced unsupported synthetic midnight retrieval timestamps with `null` plus truthful date-precision notes.
- Third repair generation added the official 2009 death-history continuity source and lowered Goblin Intruder retaliation to fail-closed UNKNOWN after exact-head Luna review found two P1 evidence-classification gaps.
- Current-gate reconciliation merged protected main `0e48ae518842fafc8b9e0867eb8ce041dc1a4332` normally into the task branch because the original merge base predated the required routing validator. Reconciled predecessor `3708e5b01d06d083e2eb7ff12a6ca3514dc96414` preserved all three owned content blobs and introduced no additional PR paths.
- The exact candidate containing this packet is bound by live branch/PR readback and the #162 freeze comment after publication; this packet intentionally does not self-assert its own commit ID or copy transient run state.

## Validation

### Focused

Exact reconciled predecessor `3708e5b01d06d083e2eb7ff12a6ca3514dc96414` passed pinned SourceMeta Draft 2020-12 metaschema/instance validation on exact Git blobs, JSON parse, semantic invariants, LF/no-CR byte checks and `git diff --check`. The task-checkpoint repair changes no manifest, schema or source-packet byte; its successor still requires fresh exact-head repository governance, hosted CI and independent review.

Required fresh checks for the resulting exact PR head:

- pinned SourceMeta JSON Schema CLI `v16.3.0`, exact release asset SHA-256 `d348714cfceeedf521cffecb13c199ba4b08fd865dc23c985f2de44fda39a9c4`;
- `jsonschema metaschema docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.schema.json`;
- `jsonschema validate docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.schema.json docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json`;
- `python -m json.tool docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json`;
- semantic invariants: schema identity, target identity, revision-3 preservation, unique case IDs, domain coverage, progression-only declared-difference scope, accepted difference reference, explicit artifact anchors, truthful retrieval precision, no parity confirmations, exact owned paths;
- `python tools/agents/validate_governance.py`;
- `python tools/architecture/semantic_contract_audit.py` against admission main and exact candidate;
- `git diff --check`.

### Component/integration

- Repository Agent Governance / Architecture Semantic Audit / Merge Gate: use the live exact-head checks on PR #640; every successor generation requires fresh qualification.

### E2E

- `NOT_APPLICABLE`: documentation/evidence registry only; no runnable product behavior changes.

### Exact-head CI

- final head: authoritative in live PR #640 and the corresponding #162 freeze comment after publication; intentionally not self-referential here
- trigger source: PR branch mutation or metadata event as recorded by GitHub
- workflow/run/job: authoritative in live exact-head checks
- runner assignment: authoritative in live exact-head checks
- classification: authoritative in live exact-head checks
- result: pending successor qualification at task-checkpoint authoring time; use live checks for current status

## Self-review

- exact head: authoritative in live PR #640 and the corresponding #162 freeze comment after publication
- method/reviewer: whole diff against #162 allocation, manifest owner pin, schema-v1 contract, and independent-review findings
- material findings closed before the next freeze: progression-scope P1; artifact-anchor P2; retrieval-time P2; lifecycle-generation P2; missing death-continuity source P1; unsupported Goblin target continuity P1; stale checkpoint P2
- verdict: pending fresh exact-head review of the successor containing this checkpoint repair

## Independent review

- required: YES; allocation requires fresh whole-diff evidence/architecture review on every exact successor head
- exact head: authoritative in live PR #640 and the corresponding #162 freeze comment after publication
- method/auditor: repository PR review/check surface
- material findings: prior generation findings are stale after repair
- verdict: pending

## PR and closeout

- changed-file review: exactly the three allocated paths; pending exact-head readback
- unresolved review threads: prior threads must be resolved only after exact-head evidence proves their findings repaired
- related/superseded PRs: PR #640 is the one existing PR for this branch
- protected auto-merge: forbidden by this task; return `READY_FOR_INTEGRATION`
- merge commit/result: not requested
- ownership release: pending integration control-plane handoff

## Context checkpoint

```yaml
last_progress: third evidence repair and current-gate reconciliation are complete; this packet routes subsequent exact-head lifecycle state to live PR/check/Issue authority
status: validating
branch: agent/reference-manifest-v4-514
head_sha: null  # live PR #640 and #162 freeze comment after publication
pr: 640
final_head_sha: null  # intentionally not self-referential
final_head_frozen_at: null  # authoritative in #162 freeze comment
ci_trigger_source: live_github_pr_events
ci_check_generation: live_pr_exact_head
ci_checks_for_current_head: live_pr_authoritative
ci_run_ids:
  - live_pr_640
ci_job_ids:
  - live_pr_640
runner_assignment_state: live_pr_authoritative
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: live_pr_authoritative
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 5
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: follow live PR #640 exact-head qualification and independent review through READY_FOR_INTEGRATION; do not amend this packet solely to copy transient SHA/run status
```
