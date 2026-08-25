# Oteryn Game Ability Engine — Implementation Plan

**Goal:** Deliver the narrow, authoritative ability occurrence/effect-plan engine that proves deterministic typed damage/heal fixtures, not Reference combat parity.

**Scope and exact worker paths:**

- `apps/game-server/src/ability/mod.rs`
- `apps/game-server/src/ability/occurrence.rs`
- `apps/game-server/src/ability/intent.rs`
- `apps/game-server/src/ability/plan.rs`
- `apps/game-server/src/ability/commit.rs`
- `apps/game-server/src/ability/effects.rs`
- `apps/game-server/src/ability/tests.rs`
- `apps/game-server/tests/ability_engine.rs`
- `docs/agents/tasks/active/OTV2-20260825-impl-game-ability.md`

`apps/game-server/src/lib.rs` is excluded and held by the serialized coordinator lease. The worker uses a focused standalone harness until coordinator composition is published.

## Global constraints

- Start only from the allocation-recorded `main` SHA and write only the listed paths on one branch/PR for Issue #166.
- Consume the owner-accepted Ability whole-gate and the existing Simulation/Domain/Content seams. Bind every logical occurrence to its behavior-affecting revision.
- Use only registered limits: target candidates `2`, resolved targets `2`, effect-plan entries `2`, effect-plan bytes `4096`, calculation stages `8`. Every max+1 rejects before effects.
- Scope is occurrence/revision, normalized intent, deterministic plan/calculation order, idempotent commit/recovery, owner-scoped partial/sequential groups, and immediate stateless typed damage/heal fixtures.
- No formula/geometry invention; fixture behavior is non-shipping structural evidence and never Reference parity.
- Exclude unregistered geometry, retarget, multihit, channels, timers, cooldowns, conditions, reactions, cross-domain diagnostics, DUR scripts, item value, Movement, protocol IDs/events/UI, persistence, reference or production Content, registry/Cargo/workspace mutation.

## Task 1: Test the occurrence and bounded plan contract first

1. Add failing tests for deterministic occurrence/revision lineage, target normalization, duplicate no-double-commit recovery, partial/sequential commit groups, and stable calculation/effect order.
2. Add max/max+1 fixtures for the five limits and a test that no effect appears after a rejected plan.
3. Add negative tests proving proposal-only client/AI/script-style adapters cannot directly mutate effect state.
4. Add the standalone module compilation harness required while `lib.rs` stays coordinator-owned, and run the focused red suite.

## Task 2: Implement the minimal typed pipeline

1. Implement typed occurrence, intent, plan, commit and damage/heal effect modules in the allocated tree.
2. Make one logical occurrence idempotent across retry/lost response; a newer incompatible revision cannot reinterpret it.
3. Preserve deterministic Simulation-owned arithmetic/order seams without selecting Reference values or inventing future work semantics.
4. Run focused tests after each increment and reject any request beyond the allocated effect-plan model.

## Task 3: Validate and review

1. Run focused ability tests covering occurrence/revision, recovery, groups, all max/max+1 boundaries, order, and direct-mutation negatives.
2. Run the required component/workspace/governance checks for the exact worker head. Tier 1/Tier 2 E2E are `NOT_APPLICABLE` until a real server/client boundary exists.
3. Perform full-diff self-review and task-scoped independent review as required by the root policy. Fix material findings only within allocation.

## Task 4: Delivery and coordinator handoff

1. Freeze exact head; verify exact-head CI and zero unresolved review threads; merge with expected head only.
2. After protected-main readback, archive/release the worker task.
3. Coordinator alone serializes the `lib.rs` composition line. This delivery neither releases Movement/Combat nor creates protocol/event/Content authority.
