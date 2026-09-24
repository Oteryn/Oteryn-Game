# OTV2-20260924-content-static-cell-fixture

```yaml
task_id: OTV2-20260924-content-static-cell-fixture
title: Content nonshipping single-cell fixture predecessor
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/content-static-cell-fixture-20260924
issue: 162
pr: null
base_sha: a9c72f5db14f48f428df9b12d200f9566ad36955
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: luna-6
created_at: 2026-09-24T13:01:43Z
updated_at: 2026-09-24T13:15:32Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/reference_static_cell.rs
  - apps/game-server/src/content/mod.rs
  - docs/agents/tasks/active/OTV2-20260924-content-static-cell-fixture.md
public_contracts: []
depends_on: []
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Add a test-only structural predecessor for a one-cell static collision lookup. It binds world, coordinate frame, generation, map revision, floor and exact coordinates, with fail-closed handling for missing, unqualified, conflicting and duplicate facts. It is not wired into Reference activation or any runtime consumer.

## Architecture and source of truth

- **PROVEN:** Live #162 allocation comment `5814505244` assigns these three paths exclusively to this worker and requires test-only construction, exact context binding, bounded fixture cardinality, walkable/blocked coverage and no scan.
- **PROVEN:** `docs/contracts/OTERYN_WORLD_SPATIAL_COORDINATE_PROFILE_V1.md` defines integer tile coordinates and explicit floor identity. The fixture uses checked conversions to those coordinate widths.
- **PROVEN:** live #483 requires official/owner evidence before Oteryn Reference promotion; OTS material is hypothesis and test-discovery input only. Its separate Reference spatial artifact/activation evidence gap remains open (#483/#486).
- **DERIVED:** A unit-test-only module and fixture constructor provide structural evidence for direct-key behavior without adding a production cell source or activation seam.
- **LIMIT:** These tests establish only the test-only structural predecessor. They do not close #483's production Reference spatial-evidence gap and cannot establish a production Reference cell or authorize activation.

## High-risk authority/recovery qualification

```yaml
applicable: false
reason: This test-only structural fixture performs no production mutation, authority decision, persistence operation, or recovery interpretation.
```

## Acceptance criteria

- [ ] The fixture code is compiled only under `cfg(test)` and cannot activate Reference Content.
- [ ] The full lookup key binds world, coordinate frame, generation, map revision, floor and coordinates.
- [ ] Exact-key lookup returns walkable and blocked facts through one ordered-map `get`; it does not scan or fall back to nearby cells.
- [ ] Missing, unqualified, conflicting and duplicate facts fail closed.
- [ ] Fixture max succeeds and max+1 fails; out-of-range coordinate conversion fails.
- [ ] No OTS or FIRST_PRODUCTION promotion, runtime integration, or production Reference cell claim is made.

## Excluded scope

Do not edit Reference artifact/compiler/project/world-runtime/protocol/lib/registry paths, or activate Reference Content. Do not infer target behavior from OTS sources, promote OTS evidence, or claim a production Reference cell. A separate Content-owned evidence-qualified spatial artifact/activation decision remains blocked on external Reference evidence.

## Implementation / findings

The `pub(crate)` module and fixture API are compiled only under `#[cfg(test)]`, so a later Movement unit test can consume the same structural predecessor without including it in production builds. Its index uses a complete typed key and a direct `BTreeMap::get`. Fixture construction checks a test-local four-record maximum and rejects duplicate keys. The fixture uses explicit walkable, blocked, unqualified and conflict states; no state is inferred from absence or collection order. This predecessor does not satisfy the independent official-evidence and production activation requirements tracked by #483/#486.

## Validation

### Focused

- command/run: focused `reference_static_cell` unit tests, strict workspace Clippy, and `cargo fmt --check` in the isolated remote Linux Rust 1.94 clone
- result: PASS — five focused tests, strict Clippy, and formatting check

### Component/integration

- command/run: `NOT_APPLICABLE` — no production path or consumer is changed
- result: not applicable

### E2E

- scenario: `NOT_APPLICABLE` — this is a test-only structural fixture predecessor
- result: not applicable

### Exact-head CI

- final head: pending coordinator atomic candidate publication
- trigger source: pending
- workflow/run/job: pending
- runner assignment: pending
- classification: pending
- result: pending

## Self-review

- exact head: pending
- method/reviewer: Luna 6 implementing agent
- material findings: pending
- verdict: pending

## Independent review

- required: pending coordinator risk-policy classification
- exact head: pending
- method/auditor: pending
- material findings: pending
- verdict: pending

## PR and closeout

- PR: pending
- Merge Queue: pending
- protected-main readback: pending
