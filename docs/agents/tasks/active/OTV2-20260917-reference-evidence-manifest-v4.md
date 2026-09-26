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
base_sha: b44fefe08f6aaf1b2c1c23dedd92bab0de87146e
head_sha: f41ea1820bd9427384fece539d9dee5233fe2c24
final_head_sha: null
final_head_frozen_at: null
owner: REFERENCE_MANIFEST_V4_514
created_at: 2026-09-17T07:14:16Z
updated_at: 2026-09-17T08:10:12Z
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

Advance the accepted Reference evidence/parity registry exactly once from manifest revision 3 to 4. Preserve schema v1 byte-for-byte, retain all revision-3 cases/history, add only seven bounded `DERIVED` cases supported by the allocated evidence packet, preserve `UNKNOWN`/`CONFLICT` ceilings, and make no runtime or parity-confirmed claim.

## Architecture and source of truth

- `PROVEN`: #162 comment `5710521667` allocates this worker, branch, admission SHA, three writable paths, and the pinned SourceMeta validation mechanism.
- `PROVEN`: #162 comment `5711026915` selects the bounded schema-v1 serialization `STRUCTURED_REFERENCE_DATA -> COMMUNITY_CORROBORATION` for external structured Rat wiki locators without evidence promotion.
- `PROVEN`: `docs/architecture/REFERENCE_EVIDENCE_PARITY_MANIFEST_V1_OWNER_ACCEPTANCE.md` freezes schema v1, the nine-domain inventory, independent target/implementation/parity axes, fail-closed classifications, and digest policy.
- `PROVEN`: `docs/architecture/REFERENCE_EVIDENCE_PARITY_MANIFEST_CONTRACT.md` requires validation against the exact normative schema.
- `DERIVED`: seven bounded target claims are admitted at the field ceilings recorded in the public source packet.
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
- [ ] Exact repaired generation passes pinned SourceMeta schema validation, JSON parse, governance validation, semantic audit, and diff checks.
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
- Repair content heads: source packet `ad9ef95d0812a5f332c463795c2782da5e02c561`; manifest `f41ea1820bd9427384fece539d9dee5233fe2c24`.

## Validation

### Focused

Historical validation through first repair head `ee79acaa0a7d4b65f148eadddd87d3f6eac78bf8` passed pinned SourceMeta Draft 2020-12 validation, JSON parse, governance validation, semantic audit and `git diff --check`, but that evidence is stale for the second repair generation.

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

- Repository Agent Governance / Architecture Semantic Audit / Merge Gate: pending fresh exact-head CI after second repair generation.

### E2E

- `NOT_APPLICABLE`: documentation/evidence registry only; no runnable product behavior changes.

### Exact-head CI

- final head: pending readback after task-record publication
- trigger source: PR branch push
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending readback after task-record publication
- method/reviewer: whole diff against #162 allocation, manifest owner pin, schema-v1 contract, and independent-review findings
- material findings closed by second repair generation: progression-scope P1; artifact-anchor P2; retrieval-time P2; lifecycle-generation P2
- verdict: pending fresh exact-head readback

## Independent review

- required: YES; allocation requires fresh whole-diff evidence/architecture review on the exact second-repair head
- exact head: pending
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
last_progress: second same-branch repair generation published source-locator/timestamp fixes and progression-only declared-difference semantics
status: validating
branch: agent/reference-manifest-v4-514
head_sha: f41ea1820bd9427384fece539d9dee5233fe2c24
pr: 640
final_head_sha: null
final_head_frozen_at: null
ci_trigger_source: pull_request_branch_push
ci_check_generation: pending_exact_head_readback
ci_checks_for_current_head: 0
ci_run_ids: []
ci_job_ids: []
runner_assignment_state: unknown
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 2
ci_recovery_actions_for_current_head: 0
stall_warnings: 0
owner_action_required: null
blocker: null
next_action: fresh-read PR #640 exact head, run/consume second-generation validation and hosted CI, then request one fresh independent whole-diff review on that exact head
```
