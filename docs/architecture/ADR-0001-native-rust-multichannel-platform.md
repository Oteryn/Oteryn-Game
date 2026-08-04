# ADR-0001: Native Rust Oteryn stack and multichannel-first game server

- Status: Accepted foundation
- Date: 2026-08-05
- Decision owners: Oteryn project
- Applies to: `blakinio/Oteryn-v2`

## Context

Oteryn is moving away from a compatibility-first architecture based on a Canary-style gameplay protocol. The product direction is a project-owned platform composed of:

- the existing Oteryn web platform, Identity, Game Gateway and World Registry;
- a new authoritative game server written in Rust;
- a new native game client written in Rust;
- one project-owned gameplay protocol, `protocol-oteryn`.

The current C++ Otheryn server remains valuable as behavioural evidence, a data/content source and a migration oracle. It is not the target runtime of Oteryn v2.

The new server must support multiple gameplay channels for one logical world from its first architectural iteration. Multichannel support is not an optional module and must not be retrofitted after a singleton-world implementation exists.

## Decision

### 1. Native stack

The target stack is:

```text
Oteryn Platform
├── Web portal
├── Identity and OAuth/PKCE
├── Game Login Ticket
├── Game Gateway
├── World Registry and channel directory
└── Game Session issuance

Oteryn Game Server — Rust
├── authoritative world simulation
├── protocol-oteryn
├── channel runtimes
├── world-shared services
├── persistence and recovery
└── content/script runtime

Oteryn Client — Rust
├── renderer and assets
├── UI and input
├── protocol-oteryn
├── prediction and reconciliation
└── launcher/update lifecycle
```

No second login authority, password flow, ticket system or direct OAuth authentication to the game server may be introduced.

### 2. One gameplay protocol

Oteryn v2 uses only `protocol-oteryn` between the Rust client and Rust game server.

`protocol-canary`, legacy Tibia packet families and compatibility adapters are not part of the target runtime. Historical versions such as 7.6, 8.0 or 15+ are represented as game ruleset/content profiles, not as separate wire protocols.

The protocol must remain independent from gameplay rules. A classic and a modern world may use the same protocol while exposing different capabilities and data.

### 3. Otheryn is a reference, not the target runtime

The C++ Otheryn repository may be used for:

- behavioural comparison;
- formula and rule discovery;
- map, item, monster, NPC and content migration;
- deterministic fixtures and golden scenarios;
- database/schema migration evidence;
- compatibility and regression testing.

Oteryn v2 must not inherit Otheryn's global `Game` singleton, protocol-coupled domain design, mutable process-global state or build-specific packet behaviour.

### 4. Multichannel-first world model

A logical world may contain multiple parallel gameplay channels:

```text
World: Antica
├── Channel 1
├── Channel 2
├── Channel 3
├── Channel 4
└── Channel 5
```

The world is one product/economy/community boundary. A channel is one independent simulation of the public world map.

The following identifiers are distinct and must never be overloaded:

- `WorldId` — stable logical world, for example `antica`;
- `ChannelId` — stable gameplay channel within a world;
- `InstanceId` — dungeon, arena, house or other isolated runtime space;
- `ZoneId` — optional logical map partition;
- `NodeId` — physical process or host;
- `GameSessionId` — authenticated gameplay session.

A channel may move between nodes without changing its identity. A single node may initially host several channels.

### 5. One logical mutation owner per channel

Each `ChannelRuntime` has one logical owner of authoritative state mutation:

```text
network input
→ bounded typed command
→ channel command queue
→ deterministic authoritative execution
→ state revision
→ client deltas and persistence effects
```

This does not require one operating-system thread for all work. Pathfinding, persistence preparation, compression, asset loading and different channels may run concurrently. Authoritative ordering inside one channel must remain explicit and reproducible.

No mutable gameplay state may be global to the process without an explicit world-service owner and scope.

### 6. World rulesets

Profiles such as `classic-76`, `classic-80`, `modern-15` and `custom-oteryn` are world rulesets.

A ruleset may define:

- enabled gameplay modules;
- combat and progression formulas;
- professions/vocations;
- spell and item catalogues;
- attack speed, cooldown and exhaust rules;
- death and PvP policy;
- loot behaviour;
- enabled modern systems;
- map and content pack revisions.

Every channel of one world must run the same ruleset, map revision and content revision. Channels with different rulesets are separate worlds.

### 7. World-shared and channel-local state

World-shared state includes, subject to detailed subsystem contracts:

- character persistence and progression;
- guilds and guild chat;
- world chat across all channels of the same world;
- private messages and presence;
- market, bank and depot;
- rankings and world economy;
- account/character reward eligibility;
- house ownership and the provisional shared house state.

Channel-local state includes:

- online player positions;
- creatures, spawns and AI state;
- combat execution and transient effects;
- ground items, corpses and local loot;
- local NPC runtime;
- public-map tile overlays;
- local `say`, `whisper` and `yell` delivery;
- channel-local events and instances.

Every subsystem must declare its scope and consistency model. Scope must never be inferred from the process in which code happens to run.

### 8. Cross-channel communication inside one world

World chat, guild chat and private messages operate across all channels of the same world. They do not cross into other logical worlds unless a later product decision explicitly introduces that capability.

Local speech remains channel and map dependent because visibility depends on position, floor, range, instance and gameplay conditions.

Chat and presence contracts must allow later extraction into a dedicated realtime service. The first implementation may proxy shared chat through a game node, but channel-local code must not become the authoritative owner of world-wide rooms.

### 9. Character session lease

A character may have at most one active authoritative game session.

A world-level character lease must bind at least:

- `WorldId`;
- `CharacterId`;
- `ChannelId`;
- `GameSessionId`;
- lease/session generation;
- expiry and revocation state.

All persistence writes must be fenced by the current session generation. A stale or recovered channel may not overwrite state produced by a newer session.

### 10. Channel change

Channel change is a session transition, not an in-place teleport or adapter switch.

The initial policy is:

```text
safe gameplay exit/checkpoint
→ close current Game Session
→ release or advance character lease
→ choose/recommend another channel
→ issue a new Game Session
→ enter the new ChannelRuntime
```

A player must not change channel while a protected operation is active, including combat lock, direct trade, unresolved item mutation, protected encounter or instance ownership transition.

Reconnect may prefer the previous channel but must obtain a fresh valid session/lease decision.

### 11. Houses — provisional policy

Houses are intentionally treated as a sensitive unresolved subsystem.

The provisional product rule is:

> A house exists once per logical world and has one shared authoritative state across all channels.

The initial architecture must therefore prevent per-channel copies of ownership, access lists, containers or items.

However, this ADR does not freeze the final topology for house presence or entry. The implementation must hide the topology behind an explicit policy boundary and avoid irreversible assumptions about whether a house is:

- one shared world instance;
- a dedicated home channel;
- a synchronized projection;
- another future model.

Until a separate house ADR is accepted, only ownership, access, persistence and anti-duplication invariants are normative.

### 12. Consistency and exploit prevention

The architecture must make the following exploit classes impossible by construction or explicit transactional policy:

- one character active on two channels;
- stale session writes after relog/recovery;
- item duplication between channel ground state and character persistence;
- escaping combat/PvP consequences by switching channel;
- repeated account/character rewards through channel hopping;
- duplicate boss, chest or daily rewards when encounters exist on multiple channels;
- changing channel through a shared house or instance;
- inconsistent ruleset/content revisions between channels of one world.

### 13. Version and deployment fencing

Every running channel must advertise and validate a compatible set of revisions:

- protocol revision;
- ruleset revision;
- content revision;
- map revision;
- persistence schema revision;
- server build revision.

World Registry and Game Gateway must not route a session to an incompatible or draining channel.

### 14. Failure domains

Failure of a channel must not automatically corrupt shared world state or other channels.

Shared services require explicit degradation policy. Initial direction:

- chat/presence outage may degrade communication without stopping simulation;
- market outage disables market operations;
- lease outage blocks new admissions and channel changes;
- persistence outage blocks new admissions and risky durable mutations;
- house-service outage blocks house entry/mutation;
- Registry outage may allow bounded operation of existing sessions but not unsafe new routing.

## Consequences

### Positive

- Canary/Tibia wire compatibility does not constrain domain design.
- One protocol serves classic and modern worlds.
- The server can scale channels independently.
- Cross-channel world systems have explicit ownership.
- The design avoids a later rewrite from a singleton world.
- Otheryn content and behaviour can be migrated incrementally.

### Costs

- The foundation requires leases, revision fencing, routing and explicit subsystem scopes early.
- Shared-world services introduce distributed-systems concerns before full gameplay parity.
- Channel switching, loot, PvP, rewards and houses require stronger contracts than a traditional single-process server.
- The C++ runtime cannot simply be translated file by file.

## Non-goals of this ADR

This ADR does not select:

- the final database technology;
- the final serialization/IDL technology for `protocol-oteryn`;
- the final house topology;
- live migration of an active channel;
- multi-node partitioning of one channel;
- cross-world chat or guilds;
- exact crate names;
- final Lua or alternative scripting technology.

Separate ADRs or contracts must decide those subjects.

## Required follow-up contracts

Before gameplay implementation expands beyond a vertical slice, create and accept:

1. World/channel/instance/node identity and lifecycle contract.
2. Character lease, login, relog and recovery contract.
3. State ownership and consistency matrix for every subsystem.
4. `protocol-oteryn` transport, schema, sequencing and reconciliation contract.
5. Persistence transaction, idempotency and revision-fencing contract.
6. Channel-switching and anti-channel-hopping policy.
7. Rewards, bosses, raids and event scope policy.
8. PvP, skull, frag and combat-lock scope policy.
9. World communication and presence contract.
10. Dedicated house architecture ADR.
11. Otheryn content/data migration and scripting strategy.
12. Exact staged implementation programme for Platform, server and client.
