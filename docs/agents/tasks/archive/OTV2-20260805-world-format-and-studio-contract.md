# OTV2-20260805-world-format-and-studio-contract

```yaml
task_id: OTV2-20260805-world-format-and-studio-contract
title: Define the native world format and Oteryn Studio contract
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/world-format-studio-20260805
pr: 9
base_sha: 27f6f930285621e5714b8b12af03a684ac9f2e1b
validated_head_sha: a54c10fd183ec628f87d9d8d36e5e9becf759cca
merge_sha: 57afc45b0e55a33e14ab38681d3389ec6d7c7fd1
owner: released
created_at: 2026-08-05T11:32:00+02:00
completed_at: 2026-08-05T11:42:00+02:00
execution_budget_minutes: 60
owned_paths: []
public_contracts:
  - docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md
cross_repository_coordination_id: OTV2-NATIVE-WORLD-CONTENT
external_repositories:
  - blakinio/Otheryn
  - blakinio/otclient
  - opentibiabr/remeres-map-editor
  - beats-dh/Beats-Assets-Editor
```

## Outcome

Accepted and merged the project-owned native Oteryn world/content direction, integrated Oteryn Studio boundary and bounded legacy conversion strategy.

The accepted architecture now records:

- a greenfield native world/content format rather than OTBM or a hybrid extension;
- separate editable World Project, canonical model and deterministic runtime World Bundle representations;
- one integrated Oteryn Studio for maps, sprites/appearances, items and related content;
- stable namespaced content keys with revision-scoped compiled runtime IDs;
- authored static world definitions separated from authoritative dynamic PostgreSQL state;
- logical `Area`/`Subarea`/`Zone` geography separated from technical `Region`/`Chunk` partitioning;
- precise `EncounterZone`/`RaidCell`/`RaidAnchor` execution scopes for Echo-Raid-like mechanics;
- compile-time encounter eligibility data and bounded runtime placement;
- OTBM/OTB/appearance inputs handled through bounded importers, a Legacy Intermediate Representation and conversion diagnostics;
- Remere's Map Editor and Beats Assets Editor treated as reference/migration tools behind licensing and provenance gates.

## Delivered files

- `docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md`
- `docs/architecture/FOUNDATION_DECISION_BACKLOG.md`
- `docs/agents/REPOSITORY_MAP.md`
- `README.md`

## Acceptance criteria

- [x] Native editable project, canonical model and compiled bundle recorded.
- [x] Integrated Studio responsibilities recorded.
- [x] Stable content identity and runtime-ID boundary recorded.
- [x] Static authored content and dynamic PostgreSQL state separated.
- [x] Semantic, technical and encounter spatial layers defined.
- [x] Safe encounter eligibility and runtime placement policy defined.
- [x] Bounded legacy conversion and diagnostics defined.
- [x] External-tool licensing/provenance gates recorded.
- [x] Foundation backlog and repository routing updated.
- [x] Exact-head governance passed.
- [x] Full-diff independent audit passed with zero material findings.
- [x] PR #9 squash-merged.
- [x] Ownership released and task archived.

## Validation

### Focused and exact-head CI

- workflow: `Agent governance`
- exact head: `a54c10fd183ec628f87d9d8d36e5e9becf759cca`
- run: `30994255286`
- job: `92267420627`
- result: `PASS`

### Component/integration

- result: `NOT_APPLICABLE` — architecture-only task with no executable workspace

### E2E

- result: `NOT_APPLICABLE` — no runtime/editor implementation in this task

## Independent audit

- exact reviewed head: `a54c10fd183ec628f87d9d8d36e5e9becf759cca`
- method: adversarial full-diff architecture review against ADR-0001 through ADR-0004, repository governance, the foundation backlog and the owner's decisions
- changed paths: five intended paths only
- review threads/requested changes: none
- material findings: none open
- verdict: `PASS`

The audit confirmed that no runtime dependency on OTBM or external editors was introduced, unresolved implementation choices remain explicit follow-up contracts, and semantic geography is not coupled to technical or encounter execution boundaries.

## PR and closeout

- PR: `#9`
- merge method: squash
- merge result: `57afc45b0e55a33e14ab38681d3389ec6d7c7fd1`
- related/superseded PRs: none
- active task removed: yes
- ownership release: complete

## Follow-up

The next programme remains the ordered foundation backlog. ADR-0005 requires later contracts for concrete World Project and World Bundle encodings, spatial precedence, chunk benchmarks, Content Registry packaging, import fixtures, Studio renderer prototype, encounter runtime policy, scripting and asset provenance.

## Context checkpoint

```yaml
last_progress: PR #9 passed exact-head governance and audit, squash-merged as 57afc45b0e55a33e14ab38681d3389ec6d7c7fd1, and this task was archived with ownership released.
status: completed
branch: docs/world-format-studio-20260805
head_sha: a54c10fd183ec628f87d9d8d36e5e9becf759cca
pr: 9
ci_check_generation: 30994255286
ci_checks_for_current_head: 1
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 1
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
stall_warnings: 0
blocker: null
next_action: Continue the canonical foundation programme from docs/architecture/FOUNDATION_DECISION_BACKLOG.md.
```
