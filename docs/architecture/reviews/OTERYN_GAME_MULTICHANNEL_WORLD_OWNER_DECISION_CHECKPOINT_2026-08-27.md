# Oteryn Game — Multichannel World Owner Decision Checkpoint

- Date: 2026-08-27
- Issue: #220
- Admission protected main: `6e6e37852b7a050a1c7117ab2a9f316907d09daf`
- Status: `OWNER_SELECTED_DIRECTION / PENDING_CANONICAL_AMENDMENT`
- Mode: architecture analysis only
- Runtime implementation authority: `NONE`

## Purpose

Preserve the owner-selected direction reached through `OTV2_ARCHITECTURE_CONTINUATION_AGENT` without silently rewriting currently accepted `GAME-VISION-01` / `GAME-CHANNEL-01` contracts before the complete replacement model is analyzed and independently reviewed.

This is a durable continuation checkpoint, not a final superseding contract. Any clause that conflicts with current accepted architecture remains `CONFLICT_PENDING_EXPLICIT_SUPERSESSION` until a dedicated amendment identifies the exact superseded wording and passes the required architecture/governance review.

## Owner-selected decisions

### 1. Permanent World Project / World Bundle format

`DEFERRED`.

Do not freeze the permanent editable World Project or compiled World Bundle physical format from the synthetic format spike alone. Select only after representative real-world evidence covers at least:

`legacy/Crystal/OTBM input -> canonical Oteryn world -> Studio/edit -> deterministic compile -> server/client load/render`

and measures semantic-loss reporting, integrity/fail-closed behavior, authoring ergonomics, patchability and runtime performance.

Current format-spike candidates remain evidence, not canonical format authority.

### 2. World and Channel semantics

The intended product model is:

```text
WorldId
= one durable world/community/economy/progression boundary

ChannelId
= one parallel authoritative open-world simulation inside that WorldId
```

World-level durable facts include Character identity/progression, item/value ownership, economy and other explicitly world-scoped durable state. Channel-local runtime state includes ordinary open-world presence, movement, combat, creatures and other transient simulation unless a named world/instance owner exists.

### 3. Channel PvP mode belongs to Channel, not World identity

One World may expose multiple Channels with repeated or different PvP modes, for example:

```text
World: Oteryn-1
├─ Channel 1 — No PvP
├─ Channel 2 — No PvP
├─ Channel 3 — Optional PvP
└─ Channel 4 — Optional PvP
```

PvP mode is not unique per Channel and does not create a separate World/economy/Character namespace.

This intentionally conflicts with current accepted wording that different profile/ruleset families use distinct `WorldId` values and every Channel of a World inherits one profile family. Do not resolve that conflict by historical rewrite. A later amendment must decide whether PvP mode becomes a narrower channel-scoped policy dimension while other world/profile dimensions remain World-scoped.

### 4. Channel switch across PvP modes

Switching between No-PvP and Optional-PvP Channels of the same World is an ordinary fenced Channel switch of the same `CharacterId`, not character/world migration.

The same Character, inventory, bank, quest/progression and world economy continue across the switch. Destination Channel rules apply only after authoritative destination admission/ownership is committed. Existing hard locks, anti-hopping and value/authority fences remain applicable and may require refinement for cross-mode abuse prevention.

### 5. Channel PvP mode immutability

A `ChannelId` keeps one PvP mode for its semantic lifetime. Do not mutate an existing Channel from No-PvP to Optional-PvP or vice versa.

Capacity changes use new Channels plus ordinary drain/retirement lifecycle:

```text
old ChannelId -> drain/retire
new ChannelId -> new immutable mode assignment
```

### 6. First-generation economy parity across PvP modes

For the first generation, No-PvP and Optional-PvP Channels keep the same baseline XP, loot, spawn and progression economics.

PvP mode is initially a playstyle/risk preference, not a better-farm multiplier. A future PvP-risk reward bonus remains allowed only as a separate explicit economy/balance decision with abuse and world-supply analysis.

### 7. Player-controlled Channel selection

After login/character selection, the player sees available eligible Channels and chooses a concrete Channel. The system may highlight a non-binding recommendation but must not silently force a mode.

The Channel presentation should expose safe player-facing signals for at least:

- PvP mode;
- occupancy/capacity indication;
- runtime health/load indication distinct from occupancy;
- hosting region;
- player-relative latency/ping estimate;
- availability/draining/full state as applicable;
- a non-authoritative `Recommended` hint.

Recommendation may combine latency, runtime health/load, occupancy and optionally last-used Channel/allowed social hints, but final FND admission remains authoritative and the player may choose another eligible Channel.

### 8. Multi-region Channels inside one World

One `WorldId` may place different Channels in different geographic regions, for example EU and Brazil, while preserving one world/community/economy.

The critical constraint is that regional gameplay simulation must not synchronously traverse the WAN in the authoritative tick path. Movement/combat/AI/open-world simulation remain local to the ChannelRuntime owner.

### 9. First-generation durable topology

The first implementation target uses one World `DurableHomeRegion` and one authoritative PostgreSQL write domain for world-shared durable mutations.

Regional Channels may therefore incur WAN latency at explicit durable boundaries, but not on every gameplay tick. Do not introduce multi-master durable writes for Character/item/value/economy merely to reduce initial cross-region commit latency.

The design must keep semantic interfaces/fencing explicit enough that later evidence may justify regional durable partitioning without changing gameplay identity semantics.

## Housing — deliberately unresolved

`OPEN / DEEP_DIVE_REQUIRED`.

The discussion exposed that housing cannot be decided by simply copying a Tibia-style finite physical-house model or multiplying houses per Channel. Before any owner disposition, the continuation must analyze at least:

1. finite physical-house scarcity versus target populations such as 1k/2k+ players;
2. whether one physical address/HouseId is world-global, channel-local, or represented by another explicit model;
3. auction scope, bidding, settlement, rent and eviction under one World with many Channels;
4. house item placement/inventory authority, revisions, transactions, anti-duplication and concurrent access;
5. player presence/visibility inside a house across Channels;
6. guest lists, ACLs, co-ownership and guild ownership;
7. offline owner and transfer lifecycle;
8. cross-region latency, partition and recovery behavior;
9. prestige/scarcity consequences of physical houses;
10. apartments/residences/instanced or hybrid alternatives, evaluated rather than assumed;
11. migration/Reference compatibility and whether Reference housing is a product requirement or only a behavior source;
12. interaction with market/economy/value sinks and anti-abuse.

No housing topology, auction model, apartment system or cross-Channel house-presence behavior is accepted by this checkpoint.

## Required continuation order

The next `Oteryn: architektura` session must:

1. resolve fresh protected `main`, Issue #220 and the active/draft checkpoint PR;
2. re-read `OTV2_ARCHITECTURE_CONTINUATION_AGENT.md`, current `GAME-VISION-01`, `GAME-CHANNEL-01`, durability/value contracts and existing house/world/instance material;
3. classify the exact profile-family conflict introduced by Channel-scoped PvP mode;
4. perform the housing deep-dive above before asking the owner to choose a housing model;
5. ask one material owner question at a time;
6. after housing is resolved, continue through the remaining genuinely open architecture horizon rather than re-asking accepted decisions;
7. prepare explicit supersession/amendment text only after the replacement semantics are coherent enough to review as one bounded package.

## Non-goals / authority

No runtime/client/server/protocol/DDL/migration/deployment/production/Platform/Atlas mutation is authorized. No permanent World Project/Bundle choice is made. No current accepted ADR/contract is silently superseded by this checkpoint.