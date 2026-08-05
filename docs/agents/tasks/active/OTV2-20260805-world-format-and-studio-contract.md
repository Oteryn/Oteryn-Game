# OTV2-20260805-world-format-and-studio-contract

```yaml
task_id: OTV2-20260805-world-format-and-studio-contract
title: Define the native world format and Oteryn Studio contract
mode: CONTRACT
status: validating
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/world-format-studio-20260805
pr: 9
base_sha: 27f6f930285621e5714b8b12af03a684ac9f2e1b
head_sha: pending-final-validation-checkpoint
owner: chatgpt-github-agent
created_at: 2026-08-05T11:32:00+02:00
updated_at: 2026-08-05T11:44:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md
  - docs/architecture/FOUNDATION_DECISION_BACKLOG.md
  - docs/agents/REPOSITORY_MAP.md
  - README.md
  - docs/agents/tasks/active/OTV2-20260805-world-format-and-studio-contract.md
public_contracts:
  - docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md
depends_on:
  - ADR-0001 native Rust and multichannel baseline
  - ADR-0002 canonical repository ownership
blocks:
  - native map/content/editor implementation
  - broad OTBM and legacy content migration
cross_repository_coordination_id: OTV2-NATIVE-WORLD-CONTENT
external_repositories:
  - blakinio/Otheryn
  - blakinio/otclient
  - opentibiabr/remeres-map-editor
  - beats-dh/Beats-Assets-Editor
```

## Outcome

Persist the owner-approved direction for a greenfield Oteryn world/content format, one integrated Oteryn Studio application and legacy OTBM/content conversion boundaries.

The accepted architecture distinguishes player-facing spatial semantics from technical streaming partitions and provides a precise event-placement hierarchy suitable for Echo-Raid-like mechanics without making oversized subareas the runtime execution boundary.

## Architecture and source of truth

### PROVEN

- Oteryn v2 is a greenfield native Rust stack and does not use `protocol-canary` as its target runtime.
- The repository contains architecture/governance only and no implemented world format or editor at the task base SHA.
- The owner explicitly selected a native format from zero, a converter boundary for OTBM and one integrated editor for maps, sprites/appearances and items.

### DERIVED

- Remere's Map Editor is suitable as a behavioral/fixture oracle for legacy OTBM workflows, not as the canonical Oteryn editor.
- Beats Assets Editor is suitable as a modern asset/content workflow reference, not as a complete canonical Oteryn world editor.
- Exact external revisions, reusable code status, licenses and fixture provenance remain implementation-gate evidence and are not frozen by this task.

### ACCEPTED OWNER DIRECTION

- Build the native Oteryn world format from zero rather than evolving or hybridizing OTBM.
- Treat OTBM and other legacy formats as conversion inputs and, where practical, constrained export targets.
- Build one integrated Oteryn Studio for maps, sprites/appearances, items and related content.
- Use Remere's Map Editor and Beats Assets Editor as functional references, migration oracles and fixture producers; do not make them runtime dependencies or silently copy license-restricted code/assets.
- Model `Area` and `Subarea` as logical/player-facing geography independent of technical `Region` and `Chunk` boundaries.
- Add `EncounterZone`, `RaidCell` and `RaidAnchor` so dynamic encounters can target safe, precise areas within a large logical subarea.

## Acceptance criteria

- [x] ADR records the native editable project, canonical world model and compiled runtime bundle.
- [x] ADR records one integrated Oteryn Studio and its map, asset, content, validation and preview responsibilities.
- [x] ADR records stable content keys and compiled runtime IDs.
- [x] ADR separates static source content from dynamic PostgreSQL runtime state.
- [x] ADR defines `Area`, `Subarea`, `Zone`, `Region`, `Chunk`, `EncounterZone`, `RaidCell` and `RaidAnchor` responsibilities.
- [x] ADR defines safe encounter placement, compile-time eligibility masks and runtime selection policy.
- [x] ADR defines OTBM/legacy import through a bounded intermediate representation and conversion report.
- [x] ADR records licensing/provenance restrictions for reference tools and imported assets.
- [x] Foundation backlog reflects accepted world-format direction while leaving scripting specifics open.
- [x] README and repository map route future agents to the ADR.
- [ ] Governance/document validation passes on the exact final head.
- [x] Independent full-diff audit has no open material finding.
- [ ] PR is squash-merged and the task is archived.

## Excluded scope

- no Rust workspace or editor implementation;
- no parser, compiler, renderer or server runtime code;
- no writes to external repositories;
- no proprietary Tibia/CipSoft assets;
- no final scripting-language choice;
- no final chunk dimensions before benchmarks;
- no claim of lossless round-trip export for Oteryn-native features.

## Implementation / findings

- Added accepted ADR-0005 covering the greenfield source project, canonical model, runtime bundle, integrated Studio and legacy-conversion boundary.
- Defined stable namespaced content identity and separated authored definitions from dynamic PostgreSQL state.
- Separated semantic `Area`/`Subarea`/`Zone` geography from technical `Region`/`Chunk` partitioning.
- Added `EncounterZone`/`RaidCell`/`RaidAnchor`, compile-time eligibility data and bounded runtime selection for precise Echo-Raid-like mechanics.
- Recorded Remere's Map Editor and Beats Assets Editor as references/migration oracles with pinned-license/provenance gates before reuse.
- Updated the foundation backlog, repository map and README without creating runtime code or modifying external repositories.

## Validation

### Focused

- workflow: `Agent governance`
- reviewed head: `780ed23fea285906170bdc1257fc651700e3df58`
- run: `30994153030`
- result: `PASS`

### Component/integration

- result: `NOT_APPLICABLE` — architecture-only task with no executable workspace

### E2E

- result: `NOT_APPLICABLE` — no runtime/editor implementation in this task

### Exact-head CI

- head: pending this validation checkpoint commit
- workflow/run: `Agent governance`, pending automatic run
- result: pending

## Independent audit

- reviewed head: `780ed23fea285906170bdc1257fc651700e3df58`
- method/auditor: adversarial full-diff architecture review against ADR-0001 through ADR-0004, the foundation backlog, repository ownership rules and the owner's recorded decisions
- material findings: none open
- verdict: `PASS`

Audit checks included:

- exactly five intended owned paths;
- no runtime/code or external-repository mutation;
- no OTBM dependency introduced into the target runtime;
- editable source and compiled runtime representations remain distinct;
- semantic geography is not coupled to technical chunks;
- encounter mechanics do not use oversized subareas as execution units;
- stable keys are canonical while numeric runtime IDs remain revision-scoped mappings;
- dynamic state remains under runtime/PostgreSQL ownership;
- external code/assets remain behind license and provenance gates;
- unresolved encoding, chunk-size, scripting and renderer choices remain explicit follow-up contracts.

## PR and closeout

- PR: #9
- changed-file review: five intended paths, 693 additions and 19 deletions before this checkpoint
- unresolved review threads: none observed
- related/superseded PRs: none
- merge commit/result: pending exact-head governance
- ownership release: pending archive after merge

## Context checkpoint

```yaml
last_progress: ADR-0005 and all routing/backlog updates passed the first governance run and a full-diff architecture audit; the final checkpoint commit now requires exact-head governance.
status: validating
branch: docs/world-format-studio-20260805
head_sha: pending-final-validation-checkpoint
pr: 9
ci_check_generation: 30994153030
ci_checks_for_current_head: 0
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 0
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
stall_warnings: 0
blocker: null
next_action: Verify Agent governance on the final checkpoint commit, then mark PR #9 ready and squash-merge it.
```
