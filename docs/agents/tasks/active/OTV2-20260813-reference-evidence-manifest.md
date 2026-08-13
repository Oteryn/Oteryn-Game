# OTV2-20260813-reference-evidence-manifest

```yaml
task_id: OTV2-20260813-reference-evidence-manifest
title: Establish versioned Reference evidence/parity manifest
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/OTV2-20260813-reference-evidence-manifest
pr: 220
base_sha: 63dea4679dc0c0e3e7a34d8534791eec3e21c769
head_sha: db44a8d4e36932001a85af73b871fb92f5b13201
final_head_sha: null
final_head_frozen_at: null
owner: current coordinating agent/session
created_at: 2026-08-13T14:33:55Z
updated_at: 2026-08-13T14:55:00Z
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/agents/tasks/active/OTV2-20260813-reference-evidence-manifest.md
  - docs/architecture/REFERENCE_EVIDENCE_PARITY_MANIFEST_CONTRACT.md
  - docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json
  - docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.schema.json
public_contracts:
  - docs/architecture/REFERENCE_EVIDENCE_PARITY_MANIFEST_CONTRACT.md
  - docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json
  - docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.schema.json
depends_on:
  - GAME-VISION-01_FIRST_REFERENCE_BASELINE_OWNER_BASELINE.md
  - GAME-VISION-01_MINIMUM_OWNER_BASELINE.md
  - GAME-CHAR-01_STAGE_B_OWNER_BASELINE.md
  - GAME-ITEM-01_ITEM_MODEL_AND_EQUIPMENT_CONTRACT.md
  - SIM-DETERMINISM-01_AUTHORITATIVE_SIMULATION_CONTRACT.md
blocks:
  - evidence-backed Reference mechanic parity promotion
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Create the canonical paper-only contract and initial machine-readable registry
for the accepted immutable 2026-07-28 first Reference target, without promoting
unsupported mechanics or authorizing runtime work.

## Architecture and source of truth

- **PROVEN:** the accepted first target is Global Tibia production-observable
  behavior after the 2026-07-28 server-save/maintenance boundary.
- **PROVEN:** the accepted baseline requires a versioned evidence manifest and
  separates target evidence from Oteryn implementation/test parity.
- **PROVEN:** current programme status selects this manifest as the one next
  paper-only action.
- **UNKNOWN:** no mechanic-level case is complete in the initial registry;
  domain coverage therefore remains explicitly empty/fail-closed.
- **CONFLICT:** none introduced by this task. Historical pre-decision evidence
  remains non-authoritative; open PR #191 owns its bounded provenance correction.

## Acceptance criteria

- [x] Freeze the accepted target without inventing a later Reference revision.
- [x] Define evidence, implementation and parity axes independently.
- [x] Define source hierarchy, provenance, historical continuity and conflicts.
- [x] Add a parseable machine-readable initial manifest with no unsupported
      `PARITY_CONFIRMED` cases.
- [x] Define and add a normative machine-readable v1 schema for every future
      mechanic case, including fail-closed promotion conditions.
- [x] Include Economy in the explicit empty/fail-closed domain coverage inventory.
- [x] Define deterministic future identity via RFC 8785 JCS + SHA-256 without
      hand-claiming a digest before tooling exists.
- [x] Preserve security/integrity/legal overrides and OTS non-authority.
- [x] Inspect the full changed-file diff.
- [ ] Run applicable governance/document checks on the exact final head.
- [ ] Complete exact-head self-review and required review classification.
- [ ] Confirm zero unresolved material review threads.
- [ ] Squash merge unchanged accepted head and archive/release ownership.

## Excluded scope

No runtime, client, server, protocol, persistence, content compiler/loader,
database schema/migration, evidence-capture automation, official-client
automation, Platform/external-repository write, proprietary material,
production behavior, entitlement activation or new stable architecture gate.

## Implementation / findings

Issue #219 owns this package. Initial registry revision 1 intentionally contains
an empty mechanic-case array plus explicit domain coverage gaps. This is
preferable to importing pre-decision evidence as accepted truth.

The manifest digest remains `null` with an explicit meaning until accepted
tooling canonicalizes and verifies it. This task does not add a dependency or
claim executable digest validation.

## Validation

### Focused

- command/run: JSON generated and parsed with native JavaScript `JSON.parse`
  before connector write
- result: PASS

### Component/integration

- command/run: NOT_APPLICABLE — paper-only contract/manifest; no executable
  component changed
- result: NOT_APPLICABLE

### E2E

- scenario: NOT_APPLICABLE — no runtime or user-visible executable behavior
- result: NOT_APPLICABLE

### Exact-head CI

- final head: pending after task record/PR metadata freeze
- trigger source: pull request
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

- required: pending final risk classification
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending
- owner-funded Codex/OpenAI usage: FORBIDDEN / NOT INVOKED

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #162 and #191 inspected as non-overlapping; #191
  provenance correction must not be absorbed
- protected auto-merge: pending
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: Codex Review P2 feedback verified: v1 case schema and Economy fail-closed inventory are present
status: validating
branch: docs/OTV2-20260813-reference-evidence-manifest
head_sha: db44a8d4e36932001a85af73b871fb92f5b13201
pr: 220
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
repair_cycles_for_current_gate: 4
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: reply to resolved Codex Review P2 feedback and await fresh exact-head CI
```
