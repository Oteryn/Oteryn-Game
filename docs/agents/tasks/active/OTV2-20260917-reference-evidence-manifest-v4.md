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
pr: null
base_sha: b44fefe08f6aaf1b2c1c23dedd92bab0de87146e
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: REFERENCE_MANIFEST_V4_514
created_at: 2026-09-17T07:14:16Z
updated_at: 2026-09-17T07:14:16Z
execution_policy: continuous_progress
owned_paths:
  - docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json
  - docs/agents/evidence/OTV2-20260917-reference-manifest-v4-source-packet.md
  - docs/agents/tasks/active/OTV2-20260917-reference-evidence-manifest-v4.md
public_contracts:
  - docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json
depends_on:
  - "#162 comment 5710521667"
  - "#514"
  - "protected #576 evidence"
  - "protected #637 evidence"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Advance the accepted Reference evidence/parity registry exactly once from manifest revision 3 to 4. Preserve schema v1 byte-for-byte, retain all revision-3 cases/history, add only seven bounded `DERIVED` cases supported by the allocated evidence packet, preserve `UNKNOWN`/`CONFLICT` ceilings, and make no runtime or parity-confirmed claim.

## Architecture and source of truth

- `PROVEN`: #162 comment `5710521667` allocates this worker, branch, admission SHA, three writable paths, and the pinned SourceMeta validation mechanism.
- `PROVEN`: `docs/architecture/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1_OWNER_ACCEPTANCE.md` freezes schema v1, the nine-domain inventory, independent target/implementation/parity axes, fail-closed classifications, and digest policy.
- `PROVEN`: `docs/architecture/REFERENCE_EVIDENCE_PARITY_MANIFEST_CONTRACT.md` requires validation against the exact normative schema.
- `DERIVED`: seven bounded target claims are admitted at the field ceilings recorded in the public source packet.
- `UNKNOWN`: excluded formulas, mappings, timing edges, RNG/probabilities, identifiers, catalogues, and runtime behavior remain unregistered or explicitly outside each case.
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
- [x] Every declared difference names the accepted Oteryn Reference death contract.
- [x] Public-safe source packet records locators, dates, source types, provenance/legal dispositions, and uncertainty.
- [x] Exact pinned SourceMeta schema validation, JSON parse, governance validation, semantic audit, and diff checks pass.
- [ ] Exact-head CI and whole-diff review complete with zero unresolved material threads.

## Excluded scope

Schema mutation/versioning; runtime/client/server/Content/Cargo/workflow/registry/resource-limit/production changes; digest computation; Radiant threshold promotion without cleared exact source; starter template; exact XP thresholds/formulas; natural Rat loot probabilities; exact ten-second boundary scheduling; any OTS promotion; implementation fixtures; merge/enqueue.

## Implementation / findings

- Preserved the original revision-3 case objects without field edits.
- Used `WORLD_INTERACTION` for the `movement.*` evidence case under the accepted schema-v1 taxonomy; runtime ownership is unchanged.
- Omitted Radiant Skyhold because the recorded source refresh did not clear the exact official target-boundary source for `PROVEN`.
- Serialized target-near structured Rat evidence as schema-v1 `COMMUNITY_CORROBORATION`, retaining the investigator role `STRUCTURED_REFERENCE_DATA` in the source packet and classification `DERIVED`.
- Kept Oteryn implementation state `NOT_STARTED` and exact revision/test links empty for every added case.

## Validation

### Focused

- JSON parse: PASS — `python -m json.tool docs/contracts/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1.json`
- SourceMeta JSON Schema CLI `v16.3.0`: PASS — exact release asset SHA-256 verified as `d348714cfceeedf521cffecb13c199ba4b08fd865dc23c985f2de44fda39a9c4`; `jsonschema version`, `jsonschema metaschema <schema>`, and `jsonschema validate <schema> <manifest>` all passed from a temporary directory
- semantic/governance assertions: PASS — unique IDs, domain coverage, fail-closed mappings, declared-difference reference, zero parity confirmations, exact owned paths; `python tools/agents/validate_governance.py` passed 26 policy documents and 9 lanes
- schema byte-identity and revision-3 preservation comparison: PASS — schema Git blob unchanged; target and first four cases/first three history entries structurally identical to admission HEAD
- `git diff --check`: PASS

### Component/integration

- Repository Agent Governance / Architecture Semantic Audit / Merge Gate: pending exact-head CI.

### E2E

- `NOT_APPLICABLE`: documentation/evidence registry only; no runnable product behavior changes.

### Exact-head CI

- final head: pending
- trigger source: PR branch push
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing agent, whole diff against allocation and source ceilings
- material findings: pending
- verdict: pending

## Independent review

- required: YES; allocation requires whole-diff evidence/architecture review
- exact head: pending
- method/auditor: repository PR review/check surface
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: exactly one PR for this branch; pending creation/readback
- protected auto-merge: forbidden by this task; return `READY_FOR_INTEGRATION`
- merge commit/result: not requested
- ownership release: pending integration control-plane handoff

## Context checkpoint

```yaml
last_progress: pinned schema validation, governance validation, semantic invariants, and whole-diff checks passed
status: ready
branch: agent/reference-manifest-v4-514
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
next_action: commit and publish the exact candidate, open the one PR, and read exact-head CI
```
