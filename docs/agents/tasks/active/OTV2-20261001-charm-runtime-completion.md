# OTV2-20261001-charm-runtime-completion

```yaml
task_id: OTV2-20261001-charm-runtime-completion
title: Complete the owner-directed Charm implementation table through existing systems
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/charm-cleanse-owner-store-20261001
pr: null
issue: 162
base_sha: 326742be1566e596dfd3cca9d8caf631f1aff4a0
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Codex root, single publisher; subagents prepare disjoint source lanes
created_at: 2026-10-01
updated_at: 2026-10-01
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/ability/condition.rs
  - apps/game-server/src/ability/condition_tests.rs
  - docs/agents/tasks/active/OTV2-20261001-charm-runtime-completion.md
public_contracts: []
depends_on: [CONDITIONS-0, CHARM-0]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome and authority

The owner explicitly directs completion of the original implementation column, not closure
of only the D186 repair. Full scope and coordinated owning inputs are recorded on #162 in
comment5937505854. Root is the only publisher; five source preparers cover the table's lanes.
PR1476 closes the separate D186 repair only; this multi-PR implementation task remains active.

The first bounded native part implements Cleanse removal and immunity in the existing
ConditionStore. It does not create a parallel condition store or claim actor/gameplay activation.
Future code uses the existing owning combat, movement, death, loot, Item and transport systems;
shared-path/migration/wire allocations are reconciled with the programme control plane.

## Architecture and source of truth

- PROVEN: CONDITIONS-0 §5 defines one eligible negative selection, drowning exclusion, removal
  and 11-second immunity to the conflict key. Haste/paralysis share Speed under §3; that accepted
  behavior is retained explicitly and is not claimed as measured current Global behavior.
- PROVEN: CHARM-0 and D186 govern source/target eligibility and generated-damage no-chain.
- PROVEN: full source/reference preparation is retained in #1434, head
  bcaf0849ad71e800e2b7d43bdbd34a8c780537b4; related OTS code is reference, not Global proof.
- Bound META3.1.0: 1bfb5ff98c8aa156e73669a14e083a1d464c29fb.
- Publication: ordinary high-level API authoring on one new isolated branch, with fresh live
  head reads before each write. No material local Git candidate exists or is reconstructed.

## High-risk authority/recovery qualification

NOT_APPLICABLE to this bounded part: pure actor-local store transitions, not a production
session/lease authority grant, durable write, migration or protocol change. Prepared Cleanse
plans are not actor authority; the future composition owner must bind and fence the exact actor.
No immutable plan supplies current authority. Sequence exhaustion now refuses before mutation,
preventing a stale retained plan from aliasing a later instance after sequence wrap.

## Full-table acceptance (task stays open until all required consumers are delivered)

- [ ] Nine damage effects and Carnage are connected to authoritative hit/death commits.
- [ ] Cripple, Numb, Adrenaline and Cleanse have actual actor/movement/removal consumers.
- [ ] Dodge, Parry and Void Inversion have accepted incoming HP/mana application and ordering.
- [ ] Critical/leech have full formulas, equipment inputs and D186/no-chain coverage.
- [ ] Gut, Scavenge, Bless and Fatal Hold have actual loot/tools/death/flee consumers.
- [ ] Promotion/Premium/Expansion come from current authoritative facts with limit tests.
- [ ] Native server/client communication and Cyclopedia use production state.
- [ ] Unassign charges its accepted fee atomically with assignment removal.
- [ ] Consumer-level replay, negative authority, reload and final integration verification pass.

## Implementation and validation

- Authored first part: deterministic prepare/commit Cleanse, expired/drowning exclusions,
  replay-safe removal, retained key immunity, death clearing and checked sequence exhaustion.
- Eight new focused tests exercise those store transitions and boundary refusals.
- Root self-review corrected the prepared test's millisecond arithmetic; proposed type-only
  immunity was deferred because it conflicts with the accepted conflict-key wording.
- Local Rust/RED/GREEN/fmt/clippy execution: UNAVAILABLE_EXECUTOR. Managed workspace failed;
  only source authoring/readback is proven. Repository CI must compile and execute this part.
- No PostgreSQL/client/E2E or gameplay activation is claimed by the store tests.
- Final exact head, current CI and independent review belong in the external FREEZE packet.
  A commit cannot contain its own SHA; the self-referential fields remain null.
- Protected integration and required external review dispatch are programme control-plane owned.

## Context checkpoint

```yaml
last_progress: authored retained Cleanse state transitions and eight boundary tests
status: implementing
branch: codex/charm-cleanse-owner-store-20261001
pr: null
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner_action_required: null
blocker: executor unavailable; full consumers require recorded owning inputs and leases
next_action: qualify first bounded part through exact-head CI and continue native consumers
```
