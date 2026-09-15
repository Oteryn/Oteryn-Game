# OTV2-20260805-protocol-canary-reference-only

```yaml
task_id: OTV2-20260805-protocol-canary-reference-only
title: Record protocol-canary as reference-only migration evidence
mode: CONTRACT
status: completed
repository: blakinio/Oteryn-v2
base_branch: main
branch: docs/adr-0008-protocol-canary-reference-only
pr: 37
base_sha: 52ef04882e13771829e0159b63410a7cd9e80150
head_sha: ce4b970174911d7b785bbb68f06849ae6241c167
merge_commit: 96c605c9fabc3266eca9dd7f0010c97e88fd057c
owner: GPT-5.6-Thinking-architecture-coordinator
created_at: 2026-08-05T18:59:00+02:00
updated_at: 2026-08-05T19:57:00+02:00
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/architecture/ADR-0008-protocol-canary-reference-only-migration-disposition.md
  - docs/architecture/GLOBAL_ARCHITECTURE_DECISION_REGISTER.md
  - docs/agents/prompts/OTV2_GLOBAL_ARCHITECTURE_DECISION_COORDINATOR.md
  - docs/agents/tasks/archive/OTV2-20260805-protocol-canary-reference-only.md
public_contracts:
  - docs/architecture/ADR-0008-protocol-canary-reference-only-migration-disposition.md
depends_on:
  - ADR-0001
  - ADR-0002
blocks:
  - FND-01 completion until the source-workspace inventory applies this fixed disposition
  - VSL-02 completion until protocol-canary is absent from the destination production runtime graph
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories:
  - blakinio/otclient
```

## Outcome

ADR-0008 fixes `protocol-canary = REFERENCE_ONLY`. Canary code, packet layouts, negotiation, adapters, fallback and translation are excluded from the Oteryn v2 production runtime graph. Any retained material is bounded migration evidence outside production workspace membership and release packaging with exact provenance and license treatment.

The global architecture coordinator now requires:

```text
FND-01
→ VSL-02
→ one atomic Oteryn-v2 client migration/root-workspace PR
→ source-only otclient marker
→ FND-ID-01 / FND-02 / FND-03 / FND-04
```

A separate destination workspace-bootstrap phase is prohibited.

## Acceptance criteria

- [x] ADR-0008 records the fixed reference-only disposition.
- [x] Production dependency, adapter, negotiation, fallback and translation paths are prohibited.
- [x] Provenance and license requirements are recorded for retained evidence.
- [x] The global register and coordinator prompt apply the decision to `FND-01`, `VSL-02` and `FND-02`.
- [x] No runtime code, external repository or production system was changed.
- [x] Full-diff architecture audit passed with zero material findings.
- [x] Exact-head required workflows passed.
- [x] PR #37 squash-merged and the result was verified.

## Validation

- focused changed-file and full-diff review: `PASS`
- component/integration: `NOT_APPLICABLE` — architecture-only package
- E2E: `NOT_APPLICABLE` — no executable product change
- exact head: `ce4b970174911d7b785bbb68f06849ae6241c167`
- Agent governance run `31032227692`: `PASS`
- Dependency review run `31032226364`: `PASS`
- CodeQL run `31032226152`: `PASS`
- unresolved review threads: `0`
- independent audit: `PASS`, zero material findings

## PR and closeout

- PR: `#37 — docs(architecture): record protocol and GameNode runtime baselines`
- merge result: squash-merged as `96c605c9fabc3266eca9dd7f0010c97e88fd057c`
- ownership release: complete
- external repositories changed: none
- production changes: none

## Context checkpoint

```yaml
last_progress: ADR-0008 and its coordinator/register updates were squash-merged to main and the task was archived.
status: completed
branch: docs/adr-0008-protocol-canary-reference-only
head_sha: ce4b970174911d7b785bbb68f06849ae6241c167
pr: 37
ci_check_generation: ce4b970174911d7b785bbb68f06849ae6241c167
ci_checks_for_current_head: 3
terminal_ci_wait_started_at: null
terminal_ci_checks_for_current_generation: 3
unchanged_state_checks: 0
identical_failure_retries: 0
repair_cycles_for_current_gate: 0
stall_warnings: 0
blocker: null
next_action: None; ownership released and later work continues under FND-01.
```
