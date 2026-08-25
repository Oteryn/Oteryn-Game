# Oteryn Game Interaction Lifecycle — Implementation Plan

**Goal:** Deliver a bounded, deterministic interaction workflow engine without granting it Movement, Ability, durability, transport, client, registry, or workspace authority.

**Scope and exact worker paths:**

- `apps/game-server/src/interaction/mod.rs`
- `apps/game-server/src/interaction/identity.rs`
- `apps/game-server/src/interaction/plan.rs`
- `apps/game-server/src/interaction/lifecycle.rs`
- `apps/game-server/src/interaction/dispatch.rs`
- `apps/game-server/src/interaction/tests.rs`
- `apps/game-server/tests/interaction_workflow.rs`
- `docs/agents/tasks/active/OTV2-20260825-impl-game-interaction.md`

`apps/game-server/src/lib.rs` is excluded: its single module-composition line is a serialized coordinator mutation after the worker delivery has passed its own review. The worker must provide a focused, standalone compilation/test harness for its owned module tree and must not edit a shared surface.

## Global constraints

- Start only from the allocation-recorded `main` SHA and write only the listed paths on one branch/PR for Issue #165.
- Use the accepted interaction successor-child identity/retry contract, existing Foundation/SIM/Domain/Content seams, and registry values: cascade depth `2`, child fan-out `8`, root work `8`, trigger candidates `16`, retained child lifecycles `8`.
- `PENDING`, `COMMITTED`, and `REJECTED` must be truthful and retry-safe. The same logical child identity reconciles ambiguous owner completion; duplicate triggers may not double-effect.
- Use deterministic ordering and purpose-isolated RNG where ordering randomisation is exercised. Max+1 rejects before an effect.
- Movement owns position commit; Ability owns effects; DUR-03 owns durable value; Foundation owns session/handoff. Interaction uses typed adapters and never writes foreign owner state or provides generic distributed atomicity.
- No movement/handoff, ability effects, FND protocol/event IDs, transport, persistence, client, Cargo/workspace/registry/stable-ID mutation, or unregistered dimension.

## Task 1: Write characterization tests before behavior

1. Add failing focused tests for stable parent-to-child identity, canonical order/RNG, duplicate trigger no-double-effect, `PENDING` reconciliation, stale/cancel handling, and foreign-owner direct-write rejection.
2. Add each resource boundary and max+1 rejection fixture for all five allocated limits.
3. Add the standalone module compilation harness needed while `lib.rs` remains coordinator-owned.
4. Run the smallest relevant test command and record the expected failing behavior in the task packet.

## Task 2: Implement the bounded pure lifecycle core

1. Implement typed identity, validated plan construction, and lifecycle state transitions in the owned module tree.
2. Implement deterministic dispatch plus a proposal/reconciliation adapter boundary with no direct foreign-state mutation.
3. Preserve failure visibility: unknown/ambiguous results remain `PENDING` until the same child occurrence is reconciled; no hidden partial success.
4. Run focused tests after each behavior increment; do not add unallocated event bus or persistence abstractions.

## Task 3: Prove boundaries and integration seams

1. Run the interaction-focused suite and its max/max+1, retry, order/RNG, duplicate and negative-authority fixtures.
2. Exercise only typed fixture adapters to Foundation/Ability/DUR boundaries; label physical Tier 1/Tier 2 E2E `NOT_APPLICABLE` because no real listener/client exists.
3. Run the exact repository-required Rust/governance checks for the worker head and perform full-diff self-review.
4. Request the required task review. Fix all material findings within the worker scope; return `BLOCKED` rather than broadening paths.

## Task 4: Delivery and coordinator handoff

1. Freeze exact worker head, confirm zero unresolved review threads and required CI for that head, then request expected-head merge.
2. Post-merge, archive/release the worker task only after coordinator readback.
3. Coordinator alone serializes the `lib.rs` publication line and validates composed crate behavior in a separate exact-head lifecycle. This plan does not authorize Movement or make Movement ready.
