# OTV2-20260923-full-content-storage-structure-audit

```yaml
task_id: OTV2-20260923-full-content-storage-structure-audit
title: Full Tibia content storage structure and hierarchy audit
mode: AUDIT
status: investigating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/full-content-storage-structure-audit-20260923
pr: null
base_sha: c07240d50473b8697cbe10641028cd0e7eb2d1e4
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "single autonomous G0 content-structure auditor"
created_at: 2026-09-23T23:35:00+02:00
updated_at: 2026-09-23T23:35:00+02:00
execution_policy: continuous_progress
owned_paths:
  - docs/agents/tasks/active/OTV2-20260923-full-content-storage-structure-audit.md
  - docs/agents/evidence/OTV2-20260923-full-content-storage-structure-audit.md
  - docs/agents/evidence/OTV2-20260923-full-content-storage-structure-audit.json
  - tools/content-census/README.md
  - tools/reference-world-corridor-census/README.md
public_contracts: []
depends_on:
  - "PR #803 protected wiki-first Item census"
  - "WorldProject/v2 protected schema coverage"
blocks:
  - "G1 FULL_CONTENT_SOURCE_DISCOVERY_AND_FAMILY_CENSUS"
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Close G0 `FULL_CONTENT_STORAGE_STRUCTURE_AND_HIERARCHY_AUDIT`: prove whether the protected content architecture can represent the complete in-scope Tibia content universe, identify only evidence-backed representation/storage gaps, and establish the minimum scalable tooling hierarchy required before G1.

## Architecture and source of truth

- PROVEN: protected `main@c07240d50473b8697cbe10641028cd0e7eb2d1e4` contains PR #803.
- PROVEN: PR #805 remains open and owns only the Item-census lifecycle archive move; this task is path-disjoint.
- PROVEN: `docs/architecture/OTERYN_WORLD_PROJECT_SOURCE_PROFILE_V2_DECISION.md` and `apps/game-server/src/content/project/v2.rs` are the current v2 source-profile authority.
- PROVEN: existing wiki-wide coverage evidence reports 49 structured concept records, 19 domain groups and `unclassified=0`.
- PROVEN: the protected v2 family vocabulary and placement model distinguish reusable definitions, typed references, world placements and mutable runtime/durable state boundaries.

## High-risk authority/recovery qualification

```yaml
applicable: false
reason: "Documentation/tooling-organization G0 audit only; no production authority, protocol, persistence, session/fence or runtime mutation."
```

## Acceptance criteria

- [ ] All in-scope TibiaWiki navigation surfaces inventoried; hard exclusions absent.
- [ ] Every source surface mapped to an Oteryn owner or explicit non-definition disposition.
- [ ] Every Oteryn family storage location identified.
- [ ] Definition / relationship / placement / runtime-state separation verified.
- [ ] WorldProject/v2 physical layout evaluated with measured scale evidence and unknowns called out.
- [ ] Tooling/evidence/test hierarchy evaluated.
- [ ] Actual representation gaps identified; unnecessary schema redesign explicitly rejected.
- [ ] Minimal target hierarchy and ordered next slices produced.
- [ ] No bulk source corpus imported.
- [ ] Exact-head repository qualification completed before Merge Queue handoff.

## Excluded scope

- No bulk TibiaWiki crawl/import.
- No Item census restart.
- No `ProjectV3`, duplicate *V2 families, runtime promotion, identity minting or gameplay-semantic promotion.
- Completely exclude: Kalkulatory, Narzędzie do nasycania, Dostawca.
- No mass move of existing protected census tools.

## Implementation / findings

Read-only analysis in progress. Candidate repair scope is limited to retained G0 audit evidence and a new family-oriented convention for future G1+ census tooling; existing `reference-world-corridor-census` scripts remain in place.

## Validation

### Focused
- command/run: pending
- result: pending

### Component/integration
- command/run: pending
- result: pending

### E2E
- scenario: NOT_APPLICABLE — source-structure audit does not alter runtime behavior.
- result: NOT_APPLICABLE

### Exact-head CI
- final head: pending
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: docs/tooling organization
- result: pending

## Self-review

- exact head: pending
- method/reviewer: implementing/coordinating agent
- material findings: pending
- verdict: pending

## Independent review

- required: pending
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- changed-file review: pending
- unresolved review threads: pending
- related/superseded PRs: #803 #805
- protected auto-merge: forbidden substitute; native Merge Queue only
- merge commit/result: pending
- ownership release: pending

## Context checkpoint

```yaml
last_progress: protected main and v2 architecture refreshed; G0 read-only analysis underway
status: investigating
branch: agent/full-content-storage-structure-audit-20260923
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
next_action: write retained machine-readable and human-readable G0 audit evidence
```
