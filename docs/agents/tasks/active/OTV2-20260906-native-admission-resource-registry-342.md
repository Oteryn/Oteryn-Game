# OTV2-20260906-native-admission-resource-registry-342

```yaml
task_id: OTV2-20260906-native-admission-resource-registry-342
title: Register native admission resource envelopes
mode: CONTRACT
status: waiting
admission_state: NOT_ADMITTED
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/native-admission-resource-registry-342
issue: 342
pr: null
allocation_source_main_sha: 0d354091dfc3a144a9c83c31434dec2aff4fe0c4
admission_main_sha: NOT_ADMITTED
base_sha: NOT_ADMITTED
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: allocated registry worker
coordinator: Oteryn Work Delivery Coordinator
created_at: 2026-09-06
updated_at: 2026-09-06
execution_budget_minutes: 60
large_budget_reason: null
owned_paths:
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/agents/tasks/active/OTV2-20260906-native-admission-resource-registry-342.md
  - docs/superpowers/plans/2026-09-06-native-admission-resource-registry.md
public_contracts: [NATIVE-SOURCE-RESOURCE-ENVELOPE-V1, DUR-FRESH-RESOURCE-ENVELOPE-V1]
depends_on: [339, 341, protected_342_configuration_addendum_and_allocation]
blocks: [B329_resource_acceptance, native_source_resource_acceptance]
cross_repository_coordination_id: OTV2-NATIVE-SOURCE-EVIDENCE
external_repositories: []
```

## Outcome

Register every accepted independent resource dimension without changing its meaning or existing registry entries. This is an exact prospective serialized allocation, activated only after Work verifies source339, resource341, the configuration addendum and this allocation on protected main. The actual allocation merge becomes immutable admission. Work binds one exclusive writer/branch/window before mutation.

## Architecture and source of truth

Accepted source339 merge `c4099a5a626c5fb17cfe40c11cf8dd813b4550e7`; durable-resource341 acceptance requires actual protected readback. The two resource decisions and #342 explicit configuration addendum define caps, accounting and fixed first-slice ranges. Root/docs AGENTS, registry rules, plan and live Issue342 govern. Work162 remains unique control plane. Mapping drafts are preparation evidence only, never numeric authority.

## High-risk authority/recovery qualification

NOT_APPLICABLE to executable mutation: registry-only documentation performs no admission or SQL operation. Independent review is nevertheless required for resource/security semantics. Future implementation obligations must retain the full authority, size/count, ambiguity and restart matrix; a written test obligation is not executed proof.

## Acceptance criteria

- [ ] Every accepted cap/subdimension has an unambiguous unit, exact hard maximum and fixed minimum=maximum configuration range; exact-size semantic encodings stay exact-size.
- [ ] All original registry entry objects remain unchanged; no duplicate IDs, null values, invented minima or omitted coupled bounds. Preliminary 87-dimension inventory is a review aid, not a completeness oracle.
- [ ] Failure category, allocation impact, client visibility and max/max+1 obligations match actual internal semantics; no new wire error or production default.
- [ ] Full-copy, SQL result versus physical row/lock distinction, 64 pending/256 domain compatibility, complete cross-epoch predicate, shared slots and restart/ambiguity custody remain explicit.
- [ ] Semantic dimension review, JSON/governance and applicable architecture checks pass; genuinely independent exact-head whole-diff review has no unresolved material findings.
- [ ] Exact-head canonical CI and protected Merge Queue pass; Work verifies protected result, archives task and releases the registry lease.

## Excluded scope

No runtime, source/transport implementation, Cargo, migrations/SQL, validator or workflow/protection edits, stable protocol IDs, new architecture/resource values, historical deletion, Platform/Atlas/META, production/bootstrap/secret/live-data writes. B migration binary belongs to the separate exact B amendment, not this worker.

## Implementation / findings

NOT_STARTED. Preserve registry scope and validate all decision subdimensions independently rather than copying a draft mechanically.

## Validation

Focused: JSON parsing, unique/required fields, fixed ranges, original-row semantic equality and exhaustive decision mapping. Component: existing governance and applicable architecture checks. E2E: NOT_APPLICABLE because this package changes no executable runtime; future boundary_tests are obligations. Exact-head: canonical selected CI and protected Merge Queue required.

## Self-review

NOT_STARTED; inspect full mapping, complete diff and all acceptance criteria.

## Independent review

Required YES: resource/security semantics; non-author exact-head review against accepted decisions and preserved existing entries.

## PR and closeout

One implementation branch/PR after protected allocation; Work controls publication/integration/archive/release. No direct main, force, reset/rebase or no-op retrigger. No B/C/Server Seam readiness follows from registration alone.

## Context checkpoint

```yaml
last_progress: exact prospective registry allocation prepared with explicit configuration addendum
status: waiting
admission_state: NOT_ADMITTED
execution_window_number: 0
execution_windows_completed: 0
worker_rotations: 0
owner_action_required: null
blocker: protected_allocation_and_decision_readback
next_action: Work verifies protected allocation and both decisions then admits one bounded registry writer
```
