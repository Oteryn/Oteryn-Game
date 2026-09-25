# OTV2-20260925-engine-static-cell-carrier-504

```yaml
task_id: OTV2-20260925-engine-static-cell-carrier-504
title: Production Rust engineering static-cell carrier
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: agent/otv2-engine-static-cell-carrier-504
issue: 504
pr: null
base_sha: bfc8b54548a09c59e062873a5dfa48739c477420
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: allocated Sol 6 writer under #162
created_at: 2026-09-25T00:00:00Z
updated_at: 2026-09-25T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/mod.rs
  - apps/game-server/src/content/static_cell_engine.rs
  - apps/game-server/src/content/reference_playable.rs
  - docs/agents/tasks/active/OTV2-20260925-engine-static-cell-carrier-504.md
  - docs/agents/evidence/OTV2-20260925-engine-static-cell-carrier-504.json
public_contracts: []
depends_on: [504, 870]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

A versioned, reusable server-only production Rust carrier reads one exact addressed static cell from an immutable, bounded generation scope. Its constructors, codec and lookup accept typed **engineering** claims only. Reference target claim bindings remain empty and no Reference generation is activated.

## Architecture and source of truth

- PROVEN: #504 comments 5818657159 and 5828228571 separate accepted static-cell carrier direction from unresolved July-28 Reference collision admission.
- PROVEN: #162 comment 5828284797 grants this five-path task lease only.
- DERIVED: four cells and 2,743 encoded bytes are an exact candidate engineering envelope; they are not Reference corpus maxima or registry selection.
- UNKNOWN: exact target cell collision, #483 per-field provenance, and final resource population.

## High-risk authority/recovery qualification

NOT_APPLICABLE: no current actor mutation, activation, production authority grant, durable write or recovery interpretation. The caller supplies current active scope independently; this carrier cannot establish that a stored generation remains active.

## Acceptance criteria

- [ ] Exact WorldId, frame, revision, generation digest and Content Lock binding; direct one-address lookup.
- [ ] Fail closed missing, unresolved, conflict, duplicate and scope mismatch.
- [ ] Versioned bounded codec, measured 1/2/4 and max/max+1/overflow tests.
- [ ] Exact-head CI, review and protected integration performed by coordinator.

## Excluded scope

Reference collision admission, #483 continuity, activation, production Movement, registry, client legality and #642 wire/client. No changes to first-production or Reference Item artifacts.

## Validation

Focused Rust, fmt, strict Clippy and exact-head `game-gate`: pending repository CI. Host has no Rust. Separate inherited governance validator stale active packets #853/#869/#876 remain outside this lease.

## Self-review

Whole-diff and encoded boundary sweep pending frozen candidate.

## Independent review

Required on exact candidate; #162 coordinator routes Luna 6. This packet does not trigger metered review or merge.
