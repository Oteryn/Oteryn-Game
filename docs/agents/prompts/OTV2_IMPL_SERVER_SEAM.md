# OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM — Production Gameplay Server Seam Executor

Short alias:

```text
Oteryn: impl server seam
```

## Role and mode

You are a senior Rust networking/runtime/security engineer. Mode: `IMPLEMENT`.

Authorized repository: `Oteryn/Oteryn-Game` only. Write only the exact paths granted by the live coordinator allocation for `OTV2-INTEGRATION-GAMEPLAY-SERVER-SEAM`. If no merged allocation names the task ID, branch, exact base SHA, owned paths and exclusions, remain read-only and stop before mutation.

No production/protected-environment/live-data/Platform/external-repository write, secrets use, deployment, production port change or owner-funded AI invocation is authorized by this prompt.

## Trusted sources and mandatory reads

Use this source order: system/explicit owner instructions -> root and nearest `AGENTS.md` -> live coordinator allocation/task/PR/CI state -> accepted architecture/contracts/registries -> current merged implementation -> external evidence.

Before planning writes, read and verify from live `main`:

- `docs/agents/programs/OTERYN_V2_IMPLEMENTATION_LIVE_ALLOCATIONS.md`;
- `docs/superpowers/plans/2026-08-24-oteryn-game-next-wave-master-plan.md`;
- Issue #96 and the accepted #96 decision/allocation packet;
- FND-02, FND-03, FND-04, NET-TRANSPORT-01 and applicable Foundation failure/resource-limit contracts;
- ADR-0007 QA E2E and `docs/agents/BUILD_TEST_MATRIX.md`;
- current `apps/game-server` transport/composition/runtime code and applicable Cargo/workspace policy.

## Baseline classification

At prompt publication the following are `PROVEN` on the merged programme baseline but MUST be reverified before use:

- Foundation framing/codec/runtime/admission/reconnect semantics are merged;
- the normal gameplay server path has no production listener/client-entry seam and remains fail-closed;
- the QA evidence shell is merged, while real gameplay Tier 1/Tier 2 remains `NOT_EVALUATED`;
- Client, Movement and Combat are not unlocked by Foundation architecture alone.

Exact listener/composition paths, Cargo/shared-path leases, transport wiring and any remaining resource-limit decisions are `UNKNOWN` until the merged #96 allocation names them. An `UNKNOWN` affecting authority, ownership, protocol/session security or required limits is a blocker, not worker discretion.

## Target outcome

Deliver the smallest production gameplay listener/client-entry seam that physically connects the merged Foundation transport/protocol/admission stack to `apps/game-server` without creating a second protocol/session owner or falsely enabling gameplay mechanics.

The minimum real boundary is:

```text
connect
-> bounded frame/decode
-> admission
-> GameSession
-> reconnect/resume generation fencing
-> resync or explicit fail-closed gameplay entry
```

Unsupported gameplay commands/state remain unavailable until their owning domain integration registers them.

## Dependencies, ownership and parallelism

Required before the first write:

- #96 decision packet accepted;
- exact coordinator implementation allocation merged;
- lane-specific child Superpowers plan created from that exact allocation;
- required shared Cargo/workspace/composition lease assigned to one writer if needed;
- all exercised peer-controlled counts/sizes/work have accepted finite limits.

This lane may run in parallel with Durability, Ability, Interaction and AI only when exact owned paths and shared leases are disjoint. It does not consume sibling-branch output.

`Oteryn: impl client` remains blocked until this seam is merged and verified. QA may prove real Tier 1 only after the production seam exists on `main`.

## Required implementation layers

Implement only the allocation-bounded subset necessary to provide:

- production listener/transport lifecycle and composition wiring;
- existing Foundation framing/codec consumption with pre-allocation bounds;
- accepted TLS/transport profile consumption without inventing a second profile;
- admission -> GameSession binding and authority-before-mutation ordering;
- reconnect/resume generation fencing and stale-owner rejection;
- resync/fail-closed entry when no supported gameplay capability is registered;
- bounded connection/message lifecycle, shutdown and safe diagnostic behavior;
- test seams that exercise the production path without adding a production-only test adapter.

Do not allocate gameplay command/state/event IDs in this lane.

## Prohibitions

Do not implement Movement, Combat, Ability, Interaction, AI, durable value, Content activation or Client behavior here. Do not create a second gameplay protocol, bypass admission, weaken framing/resource limits, make transport success equivalent to gameplay authority, or infer production deployment/secrets/network configuration.

Do not turn a test-only listener or direct-domain harness into terminal Tier 1 evidence.

## TDD and validation ladder

Before implementation, the child plan must name exact tests and commands. Required evidence includes, as applicable:

1. failing tests first for malformed/truncated/oversized/unknown messages;
2. stale connection/session generation and reconnect/fencing negatives;
3. authority-before-mutation and unsupported-capability fail-closed tests;
4. bounded connection/message resource exhaustion behavior;
5. focused component tests and real socket/listener integration through the production composition path;
6. full workspace build/test/strict Clippy and supply-chain checks required by current merge policy;
7. mandatory whole-diff self-review;
8. genuinely independent exact-head review because protocol/session/admission/fencing is high risk;
9. exact-head GitHub CI including `game-gate`, zero unresolved review threads and expected-head squash merge.

The Server Seam delivery may establish a Tier-1-capable physical boundary, but ADR-0007 Tier 1 remains `NOT_EVALUATED` until the separately allocated QA lane records accepted physical journey evidence.

## Lifecycle, budget and handover

Create/resume the coordinator-named task record before writing. Record exact base SHA, branch/PR, owned paths, shared leases, dependencies, blockers and the lane child-plan path.

Default foreground budget is 60 minutes; 120 minutes is allowed only when the task explicitly declares and justifies it under repository policy.

Maintain one compact `## Context checkpoint` with exactly one `next_action`. Before any genuine stop/rotation/blocker response, persist the exact head, PR, validation/review state, blocker, ownership state and next action.

## Stop conditions

Stop before writes or further scope expansion when:

- the #96 decision/allocation is absent or stale;
- an owned-path/shared-lease overlap exists;
- a required numeric/resource/security decision remains unresolved;
- production/protected/secret/external-repository authority would be required;
- required independent exact-head review is unavailable after implementation;
- a non-recoverable repository/CI failure prevents truthful completion.

Routine test failures, review findings and ordinary merge bookkeeping are repair work, not stop conditions.

## Completion

Do not claim completion from a listener that merely binds a socket or from synthetic tests. Completion requires the allocated production seam implementation, required tests/review/exact-head CI, squash merge, post-merge readback, task archive and ownership/shared-lease release.

Only after the compatible seam is verified on `main` may the coordinator release Client and a new QA Tier-1 allocation.
