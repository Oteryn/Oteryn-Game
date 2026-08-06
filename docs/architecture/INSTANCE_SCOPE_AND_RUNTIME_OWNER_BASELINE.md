# Oteryn v2 World-Scoped Instance and Runtime Ownership Baseline

- Status: Owner-accepted pre-contract baseline
- Date: 2026-08-06
- Decision owner: Oteryn project owner
- Related gates: `FND-ID-01`, `FND-03`, `FND-04`, `GAME-INSTANCES-01`
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: future identifier, runtime, admission, protocol, persistence, party, matchmaking and recovery contracts

## Purpose

Record the instance identity and simulation-ownership model explicitly accepted by the project owner before the complete identifier, runtime, admission and instance-lifecycle contracts are drafted.

This document is canonical architecture input. It does not complete any named gate and does not authorize runtime, protocol, persistence, matchmaking or gameplay implementation.

## Accepted topology

Channels remain the primary topology of one logical world. Instances are optional isolated gameplay contexts and do not replace channels, create independent worlds or divide the world's persistent community and economy.

A concrete instance may represent a dungeon, arena, boss room, quest scenario, event space or another bounded gameplay context.

## Accepted instance identity

The canonical semantic identity of a concrete instance is:

```text
WorldId + InstanceId
```

Accepted invariants:

- every instance belongs to exactly one logical `WorldId`;
- `InstanceId` is semantically scoped by `WorldId` even if its future technical representation is globally collision-resistant;
- a concrete instance is not semantically owned by its creator's or participants' source `ChannelId`;
- `WorldId + ChannelId + InstanceId` is not the canonical identity model;
- cross-world instances are forbidden;
- instance identity must remain independent from current GameNode, process, container, host and physical placement;
- names, activity labels, map coordinates and queue numbers are not canonical instance identity;
- future protocol, event, audit, persistence and recovery boundaries must carry or authoritatively derive the owning `WorldId`.

This decision freezes semantic scope. It does not select UUID, ULID, integer width, generation algorithm, database representation or wire encoding.

## Accepted cross-channel admission model

A concrete instance may admit eligible players whose authoritative sessions currently originate from different channels of the same logical world.

Admission must preserve the following invariants:

- every participant belongs to the same `WorldId` as the instance;
- admission is an explicit authoritative state transition, not a client-side teleport or coordinate change;
- source channels do not remain simultaneous gameplay owners of admitted characters;
- party membership alone does not authorize entry;
- later contracts must validate eligibility, readiness, capacity, session generation, activity rules and failure behavior;
- admission failure must leave the character under one unambiguous authoritative owner;
- partial group admission must be explicitly prevented or handled by an accepted activity policy;
- an instance cannot create cross-channel open-world combat, visibility, healing, experience, loot or proximity effects.

## Accepted authoritative runtime ownership

After successful admission, all participating characters and instance-local simulation are owned by one logical authoritative `InstanceRuntime`.

Accepted invariants:

- one concrete instance has one logical authoritative mutation owner at a time;
- all instance-local movement, visibility, combat, creatures, environmental state and activity progression are ordered by that owner;
- the owner may execute on a multithreaded GameNode, but parallel work must return through the authoritative ordering boundary;
- stale auxiliary results must be rejected using instance identity, ownership generation and relevant state revision;
- source `ChannelRuntime` owners must not mutate admitted characters' instance-local state;
- client state is never authoritative for admission, instance membership or gameplay results;
- changing physical placement does not silently change semantic instance identity;
- any future replacement or migration requires explicit fencing and recovery contracts.

The exact relationship between `InstanceRuntime`, `ChannelRuntime`, `WorldServices` and GameNode placement belongs to `FND-03` and later operations contracts.

## Accepted origin-channel binding

Every admitted player retains an authoritative `origin_channel_id` bound to the same `WorldId`.

`origin_channel_id` is routing and recovery metadata, not part of the canonical instance identity.

It is retained for:

- normal exit routing;
- reconnect and session-resume decisions;
- safe recovery after instance completion or failure;
- audit and diagnosis of ownership transitions;
- later policy decisions concerning return-channel capacity or availability.

Accepted invariants:

- `origin_channel_id` cannot authorize mutation after instance ownership has transferred;
- it must not be exposed as public presence information without the accepted privacy policy;
- it must be validated against the same `WorldId`;
- stale or unavailable origin-channel routing must not create duplicate sessions or dual writers;
- failure to return to the origin channel requires an explicit later fallback policy, not an implicit arbitrary-channel placement;
- changing the return destination must not rewrite the identity or history of the completed instance.

## Accepted simulation-ownership transition

Entering and leaving an instance are explicit simulation-ownership transitions.

The later admission and runtime contracts must define a fenced state machine equivalent in meaning to:

```text
ChannelRuntime authority
    -> admission prepared
    -> source authority fenced
    -> InstanceRuntime authority activated
    -> instance gameplay
    -> exit or recovery prepared
    -> instance authority fenced
    -> destination ChannelRuntime authority activated
```

Required safety properties:

- a character has at most one active authoritative simulation owner;
- every transition is generation-fenced and idempotent;
- retries cannot create a second character presence;
- disconnects and crashes cannot leave both source channel and instance authoritative;
- failure before ownership activation returns or preserves authority at the previous safe owner;
- failure after activation is recovered from the instance ownership record rather than client claims;
- durable character, inventory and reward mutations remain subject to later persistence and anti-duplication contracts.

This baseline does not select the exact transaction, lease or message sequence.

## Party and activity consequences

The accepted world-scoped party model remains unchanged:

- `WorldId + PartyId` owns party membership and organization;
- members may originate from different channels of the same world;
- open-world shared gameplay requires one common channel;
- instanced shared gameplay requires admission to one common concrete `WorldId + InstanceId`;
- remote party members outside the instance receive no instance-local combat, loot, experience, healing, visibility or proximity effects;
- leaving or failing admission does not automatically destroy the world-scoped party.

A future Party Finder may assemble eligible players across channels of one world, but it must consume the accepted admission, capacity, consent and failure contracts.

## Privacy consequences

Instance placement is non-public location information under the accepted social-presence baseline.

The client and public APIs must not reveal unauthorized `InstanceId`, GameNode placement, source channel or map position. Presence caches must fail toward less disclosure, and hidden placement must not be inferable through invitation, search, Party Finder, alternate-character or timing side channels.

## Required application to later contracts

This baseline is mandatory input to:

- `FND-ID-01` — `InstanceId` semantic scope and comparison rules;
- `FND-02` — instance admission, transition, snapshot and reconnect fields;
- `FND-03` — `InstanceRuntime` ownership, ordering, lifecycle and recovery;
- `FND-04` — Game Session binding, lease fencing and duplicate-session prevention;
- `DUR-01` and `DUR-02` — durable representation and ownership-transition evidence where required;
- `DUR-03` — item, loot, reward and currency safety during transitions;
- `ANL-01` — correlated transition and audit events;
- `QA-E2E-01` — deterministic cross-channel admission, crash and duplicate-owner scenarios;
- `GAME-INSTANCES-01` — lifecycle, matchmaking, capacity, lockouts, rewards and spectating;
- future party, Party Finder, privacy and operations contracts.

## Required deterministic acceptance scenarios

Later contracts must provide named evidence for at least:

1. two party members from different channels of one world enter one instance and receive one authoritative shared simulation;
2. a player from another world is rejected before ownership transfer;
3. duplicate admission commands do not create duplicate membership or presence;
4. one member fails eligibility or capacity validation without creating an ambiguous partial transition;
5. disconnect during admission resolves to exactly one authoritative owner;
6. source-channel crash after transfer cannot overwrite instance-owned state;
7. instance-runtime crash cannot reactivate a stale source-channel writer;
8. successful exit returns through validated origin-channel routing;
9. unavailable origin channel follows an explicit safe policy and never selects an arbitrary destination silently;
10. stale session generation, ownership generation or transition revision is rejected;
11. instance-local loot and rewards remain idempotent across retry and recovery;
12. unauthorized observers cannot obtain exact instance or source-channel placement.

## Deliberately unresolved

The following remain open:

- technical representation and generation of `InstanceId`;
- whether all instances are ephemeral or selected types may be durable;
- instance lifecycle states, timeout, idle shutdown and retention;
- admission protocol and exact ownership-transfer messages;
- capacity reservation, matchmaking, queues and partial-party policy;
- placement algorithm and relationship to channel-hosting GameNodes;
- whether active instance live migration is ever supported;
- checkpoint, replay, RPO, RTO and replacement behavior;
- return behavior when `origin_channel_id` is unavailable, full, draining or incompatible;
- reconnect windows and offline-member handling;
- lockouts, checkpoints, rewards, spectators and replay streams;
- persistence and audit granularity;
- exact privacy controls and user-facing presentation;
- concrete Rust types, crates, database schemas and protocol fields.

## Rejected interpretations

### Bind instance identity to the source channel

Rejected because a concrete instance may admit players from several channels of the same world and must retain identity independently from participant origin and physical placement.

### Allow several channel runtimes to co-own one instance

Rejected because authoritative simulation requires one logical mutation owner. Cross-channel admission transfers ownership; it does not merge channel writers.

### Treat admission as a client teleport

Rejected because coordinates do not establish authority, session fencing or failure recovery.

### Keep source channel authoritative during instance gameplay

Rejected because dual authority permits conflicting character, combat, inventory and recovery mutations.

### Use `origin_channel_id` as instance identity

Rejected because it is validated routing metadata and may differ between participants in the same concrete instance.

### Permit cross-world instances

Rejected because identity, character state, economy, ruleset and gameplay value remain world-scoped by default.

## Programme effect

- The canonical semantic instance identity is accepted as `WorldId + InstanceId`.
- Eligible players may enter one concrete instance from different channels of that world.
- One authoritative `InstanceRuntime` owns all admitted participants and instance-local simulation.
- Each participant retains validated `origin_channel_id` for exit, reconnect and recovery routing.
- Entry and exit are explicit fenced ownership transitions.
- Channels remain the primary world topology.
- Full identifier, runtime, admission and instance-lifecycle contracts remain unresolved and ordered by their registered gates.
- The source-only `blakinio/otclient` historical marker remains required before the complete `FND-ID-01` package begins.
- No implementation is authorized by this document.
