# OTV2-20260929-spell-cast-client-w3

```yaml
task_id: OTV2-20260929-spell-cast-client-w3
title: Session-layer spell cast path (SPELL cast contract section 9 step 3, session half)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
pr: pending
base_branch: main
branch: claude/spell-cast-client-w3
base_sha: 92cfc2fe
owner: "Oteryn: SPELL cast (native client)" (Claude Code)
execution_policy: continuous_progress
owned_paths:
  - crates/session/**
  - tools/dev-client/**
  - docs/agents/tasks/archive/OTV2-20260929-spell-cast-client-w3.md
public_contracts: []
depends_on: ["#1254"]
blocks: []
```

## Outcome

- `apps/client/**` is excluded: ADR-0020 section 1 gives `oteryn-client` no direct edge to `oteryn-protocol-oteryn`; the
  client-to-session edge is N5. The UI half waits for N5.
- `oteryn-session`: `Session::cast_spell` sends command type 3 and returns `CastOutcome` (typed
  `SpellCastDisposition`, status, sequence, optional vitals delta). `Rejected` (the closed server gate) is a normal
  outcome with no delta and the session stays usable. A `Cast` is followed by exactly one `ACTOR_VITALS` delta,
  validated and applied with the same domain, delta type, base revision and sequence discipline as step and use.
  `ACTOR_VITALS` (domain 3) is optional in the join snapshot and exposed by `Session::actor_vitals()`.
- `oteryn-dev-client`: thin `cast_spell` and `actor_vitals` delegation and the `ActorSpell` error mapping.
- Assumptions: a `Cast` always publishes one vitals delta (the cast pays a cost, SPELL-D3); vitals changes outside
  a command exchange (regeneration) are not read yet, as the session reads no unsolicited deltas.
- Not in `JoinSnapshot` or `CommandOutcome`, so `synthetic-client-harness` needs no change.

## Validation

`cargo test -p oteryn-session -p oteryn-dev-client`: pass (32 and 3). Workspace clippy, governance and repository
policy checks: see the PR. CI: Merge gate on the PR head.
