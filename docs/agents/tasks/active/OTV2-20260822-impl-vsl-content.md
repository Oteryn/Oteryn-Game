# OTV2-20260822-impl-vsl-content

```yaml
task_id: OTV2-20260822-impl-vsl-content
title: Implement minimal native VSL content compiler loader seam
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-impl-vsl-content-01
pr: null
base_sha: fd39c6aa026e82062a8b29af24811d467c115f19
allocation_merge_sha: 33cec30b8075c73290d7d76e9f59df4701771650
owner: chat-github-20260822-vsl-content
created_at: 2026-08-22T18:11:00+02:00
updated_at: 2026-08-22T18:11:00+02:00
execution_budget_minutes: 60
owned_paths:
  - apps/game-server/src/content/**
  - docs/agents/tasks/active/OTV2-20260822-impl-vsl-content.md
public_contracts:
  - docs/architecture/ADR-0005-native-world-format-and-oteryn-studio.md
  - docs/architecture/DUR-04_CONTENT_WORLD_AND_SCRIPTING_CONTRACT.md
  - docs/architecture/VSL-CONTENT-01_MINIMAL_NATIVE_CONTENT_SLICE_CONTRACT_CANDIDATE.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
```depends_on:
  - Oteryn-Game#45
  - Oteryn-Game#46
  - OTV2-20260818-impl-simulation
blocks:
  - OTV2-IMPL-ABILITY
  - OTV2-IMPL-INTERACTION
  - OTV2-IMPL-AI
  - OTV2-CONTENT-FORMAT-SPIKE
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
external_repositories: []
```

## Outcome

Deliver the minimum typed canonical graph plus deterministic compiler/projection/loader evidence seam needed by the first movement/combat slice, without freezing the permanent World Project/Bundle representation.

## Source facts

- `PROVEN`: VSL fixture/evidence physical representation is explicitly non-production and replaceable.
- `PROVEN`: final world/bundle encoding, chunk packing, compression and Studio source representation remain undecided.
- `PROVEN`: client projection is allowlisted and non-authoritative; server-only fields must not leak.
- `UNKNOWN` Reference formulas/content values remain test-only fixtures and cannot establish parity.

## Acceptance criteria

- [ ] TDD-first stable namespaced keys/revisions and canonical typed graph validation.
- [ ] Deterministic compilation is independent of source enumeration order.
- [ ] Separate server and client-safe projections with negative leakage tests.
- [ ] Non-production evidence artifact has explicit version, manifest/revision provenance, bounded sections and digest/integrity checks.
- [ ] Corrupt/truncated/oversized/missing-reference/incompatible artifacts fail before activation.
- [ ] Staging is separate from activation and valid activation is all-or-nothing.
## Implementation plan

1. RED: add focused tests for duplicate/missing keys, deterministic graph ordering, projection leakage and invalid artifact activation.
2. GREEN: implement minimal typed graph, projection and disposable evidence artifact behind clear semantic interfaces.
3. Add bounded parser/loader validation using existing accepted resource dimensions; do not invent permanent product capacities.
4. Keep shared composition untouched until coordinator lease advances; the module remains independently reviewable in its allocated path.
5. Integrate after prerequisite seams merge, run workspace validation, self-review and closeout without claiming final format selection.

## Excluded scope

No permanent `.omap`/`.owb` contract, compression/chunk/CDN/signing decision, Studio UI, proprietary assets, broad content set, production distribution or Reference-parity claim.

## Validation

### Focused
- command/run: pending until first RED test is created
- result: pending

### Component/integration
- command/run: `cargo test --workspace` after lawful composition integration
- result: baseline PASS; lane result pending

### E2E
- scenario: content compile/load/activate evidence consumed by later Movement/Combat; currently `NOT_EVALUATED`
- result: pending

## Context checkpoint

```yaml
last_progress: exact-base bind #46 merged; isolated content worktree created from post-bind main; permanent physical-format decisions remain explicitly excluded.
status: implementing
branch: agent/otv2-impl-vsl-content-01
head_sha: fd39c6aa026e82062a8b29af24811d467c115f19
pr: null
blocker: shared composition lease is intentionally held by FOUNDATION
owner_action_required: null
next_action: write the first failing canonical-graph/projection/artifact tests inside apps/game-server/src/content.
```
