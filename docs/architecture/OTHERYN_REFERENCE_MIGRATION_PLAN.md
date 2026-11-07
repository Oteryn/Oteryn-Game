# Otheryn reference and migration strategy

Status: planning baseline  
Date: 2026-08-05

## Objective

Build a new Rust game server without translating the C++ Otheryn runtime file by file.

Otheryn is treated as:

- a behavioural specification;
- a source of reusable content and data;
- a catalogue of gameplay capabilities;
- a producer of deterministic comparison scenarios;
- a temporary reference runtime during migration.

The target server is a native multichannel-first Rust architecture using only `protocol-oteryn`.

## What is rewritten

The following executable foundations are new Rust implementations:

- network transport and `protocol-oteryn`;
- authenticated Game Session admission;
- node, world, channel and instance lifecycle;
- authoritative scheduling and simulation loop;
- world/map runtime model;
- movement, visibility and reconciliation;
- creature, monster, NPC and summon runtime;
- combat, damage, conditions and cooldowns;
- spells, weapons and item use;
- inventory, equipment, containers and durable item transfers;
- loot, corpses and quick-loot;
- persistence, leases, revisions, recovery and idempotency;
- cross-channel world services and routing;
- administration, metrics, diagnostics and controlled rollout.

## What should be reused or converted

Subject to provenance and compatibility review:

- map data;
- item, monster, NPC, spell and vocation definitions;
- content packs and appearance metadata;
- formulas and gameplay parameters;
- database data and migration knowledge;
- quests, actions and event scripts;
- deterministic scenarios and behavioural fixtures;
- operational lessons from the existing Otheryn runtime.

Reusing data does not mean preserving the old in-memory ownership model or packet representation.

## What is intentionally removed from the target architecture

- `protocol-canary` and legacy packet adapters;
- Tibia 8.6/11.0/current wire compatibility profiles;
- build-specific packet branches;
- RSA/XTEA and historical framing retained only for external client compatibility;
- process-global mutable `Game` ownership;
- protocol classes that directly own gameplay semantics;
- assumptions that one process equals one world or one channel;
- unscoped global Lua state.

Historical gameplay versions remain possible through rulesets and content profiles over the same native protocol.

## Migration classification

Every Otheryn subsystem must receive exactly one primary classification:

| Classification | Meaning |
|---|---|
| `REWRITE_DOMAIN` | Reimplement behaviour in the new Rust domain architecture. |
| `REUSE_DATA` | Preserve/convert data or content without porting runtime ownership. |
| `REFERENCE_BEHAVIOUR` | Use as behavioural evidence and test oracle. |
| `ADAPT_SCRIPT` | Preserve intent but adapt scripts to the new scoped API. |
| `DROP_COMPATIBILITY` | Remove external-client/legacy-protocol compatibility. |
| `DEFER` | Explicitly outside the first supported release. |
| `REJECT` | Intentionally not part of Oteryn v2. |

## Recommended migration order

### Phase 0 — durable contracts

Before broad implementation:

- accept native Rust and multichannel ADRs;
- define world/channel/instance/node identities;
- define `protocol-oteryn`;
- define Game Session binding and character leases;
- define persistence fencing and item transaction semantics;
- complete the subsystem scope matrix;
- select scripting/content migration strategy.

### Phase 1 — minimum authoritative slice

Deliver one channel of one development world using the final multichannel abstractions:

```text
Platform login
→ character selection
→ Game Session bound to world and channel
→ enter map
→ movement
→ one creature encounter
→ loot pickup
→ durable save
→ logout and relog
```

Even with one active channel, all runtime identities and ownership rules must support several channels.

### Phase 2 — actual multichannel proof

Run at least two channels of the same world and prove:

- independent public map simulation;
- one active session per character;
- world chat/guild chat across channels;
- shared character progression and market/depot state;
- no ground-item visibility across channels;
- blocked channel change during protected states;
- no duplicate loot or reward through relog/channel hopping;
- channel failure does not corrupt the other channel.

### Phase 3 — core gameplay migration

Migrate, in dependency order:

- movement and pathfinding;
- combat and conditions;
- items, containers and equipment;
- spells and vocations;
- monsters and spawns;
- NPC and quest runtime;
- party, guild and communication integration;
- houses under a separately accepted ADR.

### Phase 4 — modern systems and profile breadth

Add selected 15+ systems and classic rulesets only after the shared domain foundations are stable.

A version profile must compose modules and policies; it must not fork the engine.

### Phase 5 — release hardening

- deterministic replay and regression comparison;
- fuzzing and parser limits;
- long-running multi-channel soak tests;
- crash recovery and stale-writer tests;
- rolling channel drain/restart;
- migration tooling and rollback;
- security and asset provenance review.

## Behavioural comparison strategy

For each migrated feature:

1. Identify the exact Otheryn source behaviour and relevant data.
2. Record observable inputs, outputs and edge cases.
3. Build project-owned deterministic fixtures.
4. Implement protocol-neutral Rust domain behaviour.
5. Compare both runtimes where lawful and technically practical.
6. Accept intentional differences through an explicit product decision.

The new server must not preserve an Otheryn bug merely because it exists in the reference runtime.

## Scripting strategy direction

Keeping Lua or another embedded scripting layer may significantly reduce content-rewrite cost, but the new API must be scoped and capability based.

A script must not obtain a default global game object. It receives a bounded context containing the world, optional channel/instance and actor/event identity.

Scripts may request domain actions; they do not directly mutate arbitrary server internals.

Candidate migration outcomes per script:

- run through a compatibility adapter over the new scoped API;
- mechanically convert data-only definitions;
- manually rewrite complex behaviours;
- retire obsolete scripts;
- replace critical logic with typed Rust domain code.

## Non-normative effort estimate

A production-quality MMORPG server remains a large greenfield implementation even after dropping Canary protocol compatibility.

Dropping external protocol compatibility removes substantial framing, mapper, packet-version and client-build complexity, but it does not remove the need to implement authoritative gameplay, persistence, recovery and content execution.

Planning estimates should be derived from the capability matrix rather than raw repository size. A future audit must classify every Otheryn subsystem and calculate implementation packages from observed behaviour, dependencies and acceptance scenarios.

## Required audit deliverable before implementation planning is final

Create an exact Otheryn capability inventory with at least:

- subsystem and source paths;
- current behavioural responsibility;
- data/content dependencies;
- scope in the new multichannel model;
- migration classification;
- security and consistency risks;
- target Rust owner/package;
- prerequisite contracts;
- deterministic acceptance scenarios;
- target milestone or explicit defer/reject decision.

This inventory becomes the basis for parallel agent packages and prevents accidental omission of gameplay systems.
