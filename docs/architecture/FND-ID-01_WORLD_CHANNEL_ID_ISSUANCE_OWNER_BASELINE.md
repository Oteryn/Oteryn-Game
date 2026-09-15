# FND-ID-01 World and Channel Identifier Issuance Owner Baseline

- Status: Owner-accepted pre-contract baseline
- Date: 2026-08-07
- Decision owner: Oteryn project owner
- Gate: `FND-ID-01`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: World Registry, world/channel identity, topology control, GameNode assignment, runtime ownership, admission, recovery and future identifier contracts in `blakinio/Oteryn-v2` and coordinated `blakinio/Oteryn-Platform` contracts

## Purpose

Record the project owner's accepted authority boundary for issuance of canonical `WorldId` and `ChannelId` values before the complete `FND-ID-01` contract is drafted.

This document is canonical architecture input. It does not complete `FND-ID-01` and does not authorize protocol, runtime, admission, persistence, Platform, orchestrator or database implementation.

It refines the existing owner-accepted identifier, UUIDv7, GameNode and Platform/World Registry baselines without changing their semantic scopes.

## Accepted decision

The canonical authority and logical issuer for both `WorldId` and `ChannelId` is the **Platform World Registry / authoritative topology-control boundary**.

A GameNode, ChannelRuntime, container, host or deployment orchestrator does not independently mint or establish canonical world or channel identity.

The accepted ownership model is:

```text
Platform World Registry / topology authority
    owns canonical WorldId issuance
    owns canonical ChannelId issuance
    owns durable world/channel registry and assignment intent

External orchestrator
    may request capacity / process lifecycle actions
    may start, stop or replace GameNode processes
    does not establish canonical WorldId or ChannelId

GameNode
    receives an authorized WorldId + ChannelId assignment
    validates compatible revisions and current ownership fencing
    hosts the assigned ChannelRuntime
    does not redefine channel identity

ChannelRuntime
    owns authoritative simulation mutation for the assigned channel
    does not own durable topology identity issuance
```

## WorldId authority

Accepted rules:

- `WorldId` identifies one logical world and remains globally unique according to the accepted identifier contract;
- creation of a logical world is a topology/control-plane operation, not a side effect of starting a game-server process;
- the authoritative World Registry is the logical issuer and registry owner for `WorldId`;
- world display names, slugs, hostnames, deployment names and GameNode placement do not create or replace `WorldId`;
- restarting, relocating or scaling game-server processes never changes the world's canonical `WorldId`;
- Oteryn-v2 game runtime consumes the accepted `WorldId` and does not silently re-key it.

Where cross-repository representation is coordinated, the UUIDv7 durable-identity baseline remains the preferred representation direction. Issuer authority remains with the owning World Registry contract regardless of physical UUID generation library or deployment location.

## ChannelId authority

Accepted rules:

- canonical channel identity remains semantically `WorldId + ChannelId`;
- `ChannelId` is issued by the authoritative World Registry / topology-control boundary for exactly one owning `WorldId`;
- a GameNode does not create a new canonical `ChannelId` merely because it has spare capacity or starts a `ChannelRuntime`;
- an orchestrator may request that capacity be added or that a channel be started, but canonical identity is assigned through the registry/topology authority;
- equal or globally collision-resistant physical channel values do not remove the required `WorldId` scope;
- moving a channel to another GameNode preserves its `WorldId + ChannelId` semantic identity;
- stopping and later recovering the same channel does not create a new `ChannelId` merely because process placement changed;
- channel display numbers such as `Channel 1` remain labels and may be changed without changing identity.

## Dynamic channel creation flow

The accepted logical flow for a new channel is:

```text
capacity or product policy requests another channel
    -> authoritative topology control validates the request
    -> World Registry allocates or activates canonical WorldId + ChannelId
    -> placement/orchestrator selects or starts a suitable GameNode
    -> GameNode receives the authorized assignment plus compatible revisions and fencing context
    -> ChannelRuntime enters Starting/Warming/Ready lifecycle
    -> World Registry marks the channel routable only after readiness validation
    -> Game Gateway may route newly issued sessions to that channel
```

The exact API, transaction boundary, control-plane protocol, orchestrator product and rollout mechanism remain future contract work.

## Ownership generation and fencing

Canonical identity and current execution authority are separate concepts.

A recovered, restarted or relocated channel keeps the same:

```text
WorldId + ChannelId
```

while current mutation authority is additionally fenced by an explicit ownership generation or equivalent current-authority token owned by the later runtime/operations contract.

Accepted consequences:

- `ChannelId` is not incremented or replaced merely to fence a stale GameNode;
- a new ownership generation may be issued when authority is transferred or recovered;
- an old GameNode holding the correct `WorldId + ChannelId` but a stale generation has no current mutation authority;
- stale writers must be rejected by persistence, admission and externally visible ownership boundaries according to the later `FND-03`, `FND-04`, `DUR-02` and operations contracts;
- identity answers "which channel is this?" while the fencing value answers "which owner is currently authorized to mutate it?".

## Runtime ownership remains game-owned

This decision does not move gameplay authority into Platform.

The World Registry owns topology identity and routing/control-plane registry state. Once an authorized assignment is active, the `ChannelRuntime` remains the authoritative gameplay mutation owner for channel-local simulation under ADR-0001 and ADR-0009.

Platform World Registry must not become the owner of:

- combat;
- movement;
- creatures or AI state;
- channel-local ground state;
- local loot or transient effects;
- authoritative simulation ordering;
- arbitrary gameplay mutation.

Issuance authority for `WorldId`/`ChannelId` therefore does not imply gameplay-state ownership.

## Orchestrator boundary

The deployment orchestrator manages process/container lifecycle and placement, but canonical game topology remains an application-level contract.

Accepted rules:

- the orchestrator may start or replace GameNodes in response to requested capacity and health policy;
- orchestrator-specific pod/container/host identifiers are not `NodeId`, `WorldId` or `ChannelId`;
- an orchestrator must not derive a canonical `ChannelId` from a pod name, ordinal, replica index or host identifier;
- replacement of infrastructure preserves application identity and advances only the applicable lifecycle/fencing state;
- the game stack must remain portable across accepted deployment technologies without changing semantic world/channel identity.

## Failure and recovery consequence

When a GameNode fails:

- World Registry/topology state preserves the affected `WorldId + ChannelId` identity;
- routing to the failed assignment stops;
- stale ownership is fenced;
- a replacement GameNode may receive the same canonical channel identity with a newer accepted ownership generation;
- the channel becomes routable only after recovery and readiness validation;
- players are not silently reassigned to a different `ChannelId` merely because the original process failed.

This preserves the ADR-0009 requirement that channel recovery is not equivalent to hidden channel hopping.

## Cross-repository authority

ADR-0003 remains authoritative that Platform owns World Registry and route policy.

The complete `FND-ID-01` and later coordinated Platform contracts must therefore record at minimum:

- authoritative issuer/owner of `WorldId`;
- authoritative issuer/owner of `ChannelId`;
- canonical cross-language representation and validation rules;
- uniqueness and lifecycle rules;
- creation, activation, retirement and tombstone semantics;
- assignment and current-ownership metadata boundaries;
- what Oteryn-v2 may cache and how stale topology data is rejected;
- which operations require Platform/Registry availability and which existing runtime state may continue during bounded control-plane degradation.

Oteryn-v2 must not create a competing registry or alternate channel-ID namespace.

## Deliberately unresolved

This baseline does not decide:

- exact Registry API or service endpoint;
- exact database schema or table ownership inside Platform;
- exact UUIDv7 adoption/migration sequence for currently external identifiers;
- exact `ChannelId` textual or binary wire encoding;
- exact world/channel provisioning UI or administration workflow;
- whether a retired `ChannelId` remains permanently tombstoned or may ever be reactivated as the same semantic channel;
- exact ownership-generation width or representation beyond the accepted ordering/fencing class direction;
- exact allocator behavior during Registry partition or generator failure;
- exact orchestrator product;
- channel scaling thresholds, hysteresis or capacity limits;
- implementation of World Registry, GameNode, ChannelRuntime or admission paths.

These remain owned by `FND-ID-01`, `FND-02`, `FND-03`, `FND-04`, `PERF-01`, `OPS-CHANNEL-01` and coordinated Platform work as applicable.

## Rejected interpretations

### Let each GameNode generate ChannelId values

Rejected because GameNode lifecycle and placement are execution concerns, while `ChannelId` is durable topology identity. Independent generation would complicate split-brain prevention, recovery, registry consistency and channel relocation.

### Derive ChannelId from process/container ordinal

Rejected because infrastructure placement is replaceable and must not define semantic channel identity.

### Allocate a new ChannelId on every restart

Rejected because restart/recovery changes the current execution owner, not the semantic identity of the channel being recovered.

### Make World Registry the gameplay mutation owner

Rejected because topology/control-plane ownership does not replace the single authoritative simulation owner inside the active `ChannelRuntime`.

### Let the orchestrator become the canonical world registry

Rejected as an implicit coupling. Infrastructure may host and schedule processes, but application-level world/channel identity must remain stable across deployment technologies.

## Required application to later contracts

This decision is mandatory input to:

- `FND-ID-01` — owner/issuer matrix and lifecycle catalogue for `WorldId` and `ChannelId`;
- `FND-02` — protocol and routing fields that carry or derive world/channel scope;
- `FND-03` — ChannelRuntime assignment, ownership generation, lifecycle and recovery;
- `FND-04` — Game Session world/channel binding and stale-assignment rejection;
- `DUR-01`/`DUR-02` — durable representation and fencing references;
- `PERF-01` — capacity signals must not redefine topology identity;
- `OPS-CHANNEL-01` — dynamic channel provisioning, placement, recovery and orchestration;
- Platform World Registry/Game Gateway contracts — canonical issuance, routing and readiness behavior;
- `QA-E2E-01` — tests must prove stable channel identity across restart/relocation and rejection of stale owners.

## Programme effect

- `WorldId` issuance authority is fixed at the Platform World Registry / authoritative topology-control boundary.
- `ChannelId` issuance authority is fixed at the Platform World Registry / authoritative topology-control boundary.
- GameNodes consume assigned `WorldId + ChannelId`; they do not establish canonical topology identity.
- Orchestrators control infrastructure lifecycle but do not define application identity.
- Channel restart or relocation preserves `WorldId + ChannelId` and advances the applicable ownership fencing/generation instead of silently changing identity.
- `ChannelRuntime` remains the authoritative gameplay mutation owner after assignment.
- No implementation is authorized by this baseline.
