# GAME-CHANNEL-01 — Pre-Decision Channel Product Policy Analysis

- Status: **PRE-DECISION ANALYSIS / NOT ACCEPTED**
- Date: 2026-08-11
- Gate: `GAME-CHANNEL-01`
- Coordination ID: `OTV2-GLOBAL-ARCHITECTURE`
- Decision owner: product owner
- Does not authorize: channel runtime implementation, dynamic scaling, numeric capacity/cooldown values, PvP policy, production routing or acceptance of recommendations below

## 1. Problem

Oteryn has already accepted the **systems architecture** for multichannel operation: one logical world may have many channels, each channel has one logical authoritative mutation owner, channel-local simulation is isolated, and selected character/economy/social state is shared at world scope.

What is not yet accepted is the **player-facing policy** that answers:

- why channels exist from the player's perspective;
- when a player is assigned or moved;
- how friends/parties stay together;
- when manual switching is allowed;
- how channel-local duplicated simulation interacts with scarce rewards and economy;
- how PvP and combat prevent channel-hopping abuse;
- how the system avoids making one logical world feel like unrelated copies.

A technically correct sharding system can still be a bad MMO product if players disappear from friends, rare content multiplies unintentionally or switching becomes an escape/farming tool.

This document prepares owner decisions. It does not make them.

## 2. Accepted constraints — do not silently redesign

### PROVEN

From ADR-0001/ADR-0009, `MULTICHANNEL_SYSTEM_SCOPE_MATRIX.md`, FND-03/FND-04 and the current architecture refinements:

- `WorldId` and `ChannelId` are distinct.
- One logical world may expose multiple channels.
- One logical authoritative writer owns each channel/instance mutation scope.
- Public map definition/ruleset/content revision is shared by all channels of a world.
- Creature/spawn runtime, ground items, corpses, local speech and ordinary PvP execution are channel-local unless a dedicated rule says otherwise.
- Character progression, inventory/equipment, character lease, market, bank, depot, guild membership, quest progress and selected social state are shared at character/world scope.
- Party membership may span channels, while shared-experience effect requires eligible co-location.
- A character has at most one active authoritative gameplay session.
- Game Session admission is bound to a selected channel.
- Channel change is already required to fail or wait during combat/PvP locks, direct trade, unresolved value transactions, unsafe instances/encounters, pending checkpoints, stale lease authority, incompatible revisions, unhealthy/draining/full targets and similar unsafe states.
- Boss/event runtime scope and reward eligibility must be explicit; independent channel copies do not automatically authorize multiple valuable reward claims.
- Recovery must not silently place a character into another live channel with different combat/spawn/loot state.
- Exact dynamic orchestration, capacity thresholds and production scaling remain later `OPS-CHANNEL-01`/`PERF-01` work.

`GAME-CHANNEL-01` must therefore define product policy **on top of** these boundaries rather than create competing ownership rules.

## 3. Decision timing test

### Must decide now?

**PARTIALLY YES.**

The minimal policy must be accepted before:

- `VSL-MULTICHANNEL-01` can be specified or called complete;
- multichannel is exposed as a real player-facing product feature;
- broad content creates high-value channel-local bosses/events/resources without eligibility policy;
- PvP/channel switching can be implemented without an abuse model;
- party/friend UX can promise co-location behavior.

It does **not** need to block single-channel `VSL-ADMISSION-01`, `VSL-MOVE-01`, `VSL-COMBAT-01`, `VSL-PERSISTENCE-01` or `VSL-RECOVERY-01` provided those slices use final `WorldId`/`ChannelId` identities and do not pretend multichannel behavior is complete.

### What becomes expensive if delayed?

- reward/content definitions may accidentally encode per-channel farming multiplicity;
- party/social UX may depend on implicit shard behavior;
- PvP escape/channel-hop behavior may become difficult to remove without player backlash;
- world events may be authored with incompatible assumptions about uniqueness;
- routing/recovery may silently normalize unsafe cross-channel relocation;
- client UI may expose channel concepts in ways that later product policy wants hidden.

### Evidence that could justify superseding later

- measured channel population and queue data;
- social co-location failure rates;
- player complaints about manual-switch friction;
- channel-hopping abuse telemetry;
- spawn/hunt availability and economy impact;
- PvP abuse evidence;
- capacity/failure measurements from `PERF-01`/`OPS-CHANNEL-01`;
- playtests showing channel visibility harms or improves world identity.

## 4. Comparative observations — non-normative

These external observations are used only to identify failure modes and useful patterns. They do not define Oteryn behavior.

### World of Warcraft layering/sharding

Blizzard has publicly described modern zone sharding as a capacity mechanism and separately explained that Classic's layer model was chosen to preserve more whole-world continuity. The same article notes that certain world-state/event semantics such as Wintergrasp were fundamentally difficult to preserve across layers/shards.

External evidence:

- Blizzard, *The Battle for Wintergrasp in Wrath Classic* — https://worldofwarcraft.blizzard.com/en-us/news/23833249/the-battle-for-wintergrasp-in-wrath-classic

### The Elder Scrolls Online phasing

ESO support documentation acknowledges the player-visible failure mode where friends standing in the same location cannot see one another because they are in different phases, and provides a `Travel to Player` mechanism to regroup when allowed.

External evidence:

- ESO Support, *What is phasing?* — https://help.elderscrollsonline.com/app/answers/detail/a_id/2566/~/what-is-phasing%3F

### DERIVED lesson for Oteryn

Capacity partitioning must not be allowed to become invisible social fragmentation. Oteryn should prefer explicit party/friend co-location rules, stable channel identity during risky activity and event definitions that declare whether their state/eligibility is channel-local or world-wide.

## 5. Key decision A — player-visible channel model

### Option A — fully hidden automatic shards

Players normally never see or choose a channel. Routing continually optimizes capacity and co-location.

**Benefits**

- simplest surface for casual users;
- system can optimize population dynamically;
- avoids players treating channels as farm-server selectors.

**Costs/risks**

- hard to explain why another player is not visible;
- silent relocation can damage persistent-world identity;
- debugging/support becomes opaque;
- party/friend exceptions become complex;
- dangerous around PvP, rare spawns, loot and active world-state transitions.

### Option B — explicit server/channel selection

Players choose a visible channel before or during gameplay.

**Benefits**

- clear mental model;
- easy to coordinate with friends manually;
- transparent support/debugging.

**Costs/risks**

- makes channel-hopping a primary gameplay tool;
- more friction at login;
- encourages farming/griefing around channel population;
- requires more anti-hopping restrictions.

### Option C — soft-visible sticky channels

The system automatically chooses and keeps a stable channel, but current channel identity is available in appropriate UI. Party/friend co-location is first-class, and manual `join/switch channel` is an explicit safe-state action rather than the normal way to play.

### RECOMMENDATION — owner decision required

**Option C: soft-visible, sticky channels.**

The player should experience one coherent logical world first. Channel identity exists and is explainable, but ordinary play should not require manual shard management. Manual switching remains available for legitimate social/capacity reasons under safety and anti-abuse gates.

## 6. Key decision B — initial assignment and social co-location

### Recommended policy — owner decision required

Use **sticky automatic assignment with deterministic social preference**:

1. healthy existing session/recovery stays bound to its existing `ChannelId`;
2. a party entering together should be routed to one suitable channel when safely possible;
3. joining a party/friend already in-world may offer an explicit `join their channel` action if the character is eligible to switch;
4. guild/friend presence may expose enough channel information to coordinate subject to privacy policy;
5. if the desired channel is full/draining/unhealthy, the client receives an explicit reason and safe alternatives rather than silently moving one player;
6. reconnect never uses social preference to override the accepted same-channel recovery semantics.

### Why not force parties together at any cost?

A full, unhealthy or unsafe target channel is a real authority/capacity boundary. Party co-location should be preferred, not allowed to bypass admission, capacity, encounter or lease safety.

## 7. Key decision C — manual switching and anti-hopping

### Option A — switch almost anywhere

Rejected as the recommended baseline because it turns channels into combat escape, spawn reroll and reward multiplication tools.

### Option B — relog-only switching

Safe but high friction and poor party UX.

### Option C — explicit safe-state switch with eligibility fences

### RECOMMENDATION — owner decision required

Use **Option C**.

A player may request a channel switch only when the server proves the character is in a safe switch state. Existing matrix gates remain mandatory, including:

- no combat/PvP lock;
- no active direct trade;
- no unresolved item/loot/economy transaction;
- no unsafe boss/raid eligibility state;
- no incompatible instance state;
- no pending checkpoint/lease ambiguity;
- compatible target ruleset/content revision;
- healthy target with capacity.

The product contract should additionally require a **world-scoped anti-hopping fence** for content where rapid channel changes can create repeated economic/reward advantage.

### Exact cooldown duration

**DEFER.**

Do not guess `30 s`, `5 min`, `10 min` or another fixed value in architecture. The contract should freeze **why** a cooldown/fence exists and what actions it protects. Exact values should be ruleset/content policy backed by playtest/abuse evidence and bounded by server-side minimum/maximum policy.

A switch cooldown must never become the sole security control. Reward eligibility, combat locks and transaction identities must remain authoritative independently of UI timing.

## 8. Key decision D — spawn and resource multiplication

Channel-local simulation naturally creates multiple copies of ordinary creatures/spawns/ground items. That is useful for hunt capacity but dangerous if every scarce/high-value system is multiplied without intent.

### Recommended classification — owner decision required

Each content definition that can materially affect progression/economy should declare two independent concepts:

```text
runtime encounter scope
-> channel-local | instance-local | explicitly world-coordinated

reward/eligibility scope
-> per encounter | per character/world reset | account/world reset | another explicit reviewed policy
```

Exact type names remain for later content/ruleset contracts.

### Recommended defaults

- **ordinary hunting spawns:** channel-local runtime and normal loot;
- **ordinary ground items/local interactions:** channel-local;
- **market/bank/depot/character progression:** world/character-shared as already accepted;
- **rare/high-value bosses, raids or world events:** no implicit default; definition must explicitly choose runtime uniqueness and world-scoped eligibility/repeat rules;
- **daily/weekly/unique rewards:** world/account/character eligibility must prevent farming the same logical reward through channel hopping where design intends uniqueness.

### Important distinction

Five boss copies on five channels may be technically valid runtime state while still allowing only one reward claim per reset. Runtime multiplicity and economic eligibility are separate decisions.

## 9. Key decision E — world bosses and global events

### Option A — every channel has an independent event

Scales participation but can multiply rarity/economy and fragment the social moment.

### Option B — one designated channel hosts the world event

Preserves uniqueness but creates queues/capacity hot spots and may exclude players.

### Option C — explicit event classes

Some events are channel-local; some are world-coordinated/unique; reward eligibility may be world-scoped even when multiple runtime encounters exist.

### RECOMMENDATION — owner decision required

Use **Option C** and require explicit scope on every event/raid/boss content definition. Do not define one universal boss rule at architecture level.

For marquee `world event` semantics, product design should default toward preserving one coherent world-level activation/eligibility story rather than multiplying the same rare event invisibly per channel. Implementation topology is deliberately not decided here.

This is especially important for future Echo Raid-style or living-world systems: `Area`/`Subarea` discovery may be world-visible while actual `EncounterZone` execution and reward uniqueness require an explicit policy.

## 10. Key decision F — PvP implications

### PROVEN baseline

PvP execution is channel-local; combat/skull/frag consequences are world/character durable where the accepted matrix says so; channel switching is blocked under combat/PvP lock.

### RECOMMENDATION — owner decision required

Treat channel switching as **never a valid immediate PvP escape mechanism**.

The later PvP contract should define:

- which PvP locks prevent switching;
- post-hostility switch fences;
- whether opponents can discover channel presence and under what privacy/anti-harassment limits;
- interaction with guild wars, skull/frag state and protection zones;
- whether any PvP-specific world/profile disables manual channel switching entirely in selected contexts.

Exact PvP formulas and timers remain outside this gate.

## 11. Key decision G — party, guild, friends and presence

### Recommended baseline — owner decision required

Preserve a **world-level social graph with channel-local physical play**:

- party membership: world-shared;
- party chat: world-shared;
- shared XP/combat contribution requiring co-location: channel-local eligibility;
- guild membership/chat: world-shared;
- private messages: world-shared;
- friend/guild presence: world-shared with privacy controls;
- local speech: channel/instance-local;
- party leader/member gets a safe `join channel` affordance when eligible;
- social systems must not implicitly teleport or transfer authority.

This reduces fragmentation while preserving channel-local simulation.

## 12. Key decision H — recovery and channel failure

### PROVEN baseline

Accepted FND-04 recovery preserves the logical channel/session authority semantics; recovery must not silently move a player into a different live channel with a different combat/spawn/loot state.

### RECOMMENDATION

`GAME-CHANNEL-01` should state this as a player-visible product promise:

> A transient network or GameNode failure never silently becomes a channel hop. Recovery either restores the same logical channel state under accepted fencing/recovery rules or produces an explicit recovery/unavailable outcome.

A later `OPS-CHANNEL-01` implementation may restart/replace the GameNode process hosting the **same logical ChannelId** under fencing. That is not the same as transferring the player to another existing channel.

## 13. Key decision I — channel creation/removal and capacity

Product policy needs semantics now; exact orchestration belongs later.

### Recommended product contract — owner decision required

- ordinary channels may be created/removed in response to measured demand;
- a new channel is not public-ready until compatible world/ruleset/content state and admission readiness are proven;
- an occupied channel enters an explicit draining state before removal;
- draining rejects new ordinary admission/switches but preserves safe completion/recovery rules for incumbents;
- players are not forcibly moved between healthy live channels merely to optimize utilization;
- low population should not trigger abrupt channel collapse while encounters/transactions/recovery are active;
- exact thresholds, hysteresis, minimum/maximum channel counts and orchestration implementation are `PERF-01`/`OPS-CHANNEL-01` decisions.

## 14. Recommended player UX

### RECOMMENDATION — not final UI design

The native client should make channel state understandable without making it the center of ordinary play:

- current world is always clear;
- current channel is available in world/session details and social/diagnostic surfaces;
- `Join party member's channel` is presented when useful and legal;
- switch failures are typed and understandable: combat lock, encounter eligibility, transaction pending, target full, target draining, incompatible revision, etc.;
- queue state is explicit when a requested social target/channel lacks capacity;
- reconnect UI says `reconnecting to Channel X` rather than hiding a potential authority transition;
- support/diagnostics can identify `WorldId`/`ChannelId` without exposing internal secrets.

Do not show backend GameNode/container topology as a player concept.

## 15. Recommended first multichannel proof

### RECOMMENDATION — feeds `VSL-MULTICHANNEL-01`, does not implement it

The first real proof should use exactly **two public channels of the same logical world** with one small representative area.

Minimum scenarios:

1. two players entering separately may land on different channels under capacity/routing policy;
2. party co-location request safely brings an eligible player to the party channel;
3. combat-locked character cannot switch;
4. character inventory/progression remains identical and fenced across a legal switch;
5. ground items and ordinary monsters remain channel-local;
6. world chat/guild/presence state crosses channels as declared;
7. one world-scoped reward cannot be duplicated by hopping;
8. one explicitly channel-local encounter can legitimately exist twice when its policy permits;
9. reconnect after transport loss restores the same channel semantics;
10. GameNode/channel failure does not silently place the character in the other active channel;
11. draining target rejects new switches/admissions while incumbents follow safe closeout policy;
12. logs/events identify world/channel/instance and preserve no process-global authority ambiguity.

A third/fourth/fifth channel adds little architectural evidence before these scenarios pass.

## 16. Blocking decisions versus deferred details

### Must be accepted before `VSL-MULTICHANNEL-01`

1. hidden/explicit/soft-visible player channel model;
2. default assignment and party/friend co-location semantics;
3. legal switch conditions and anti-hopping authority model;
4. default relationship between channel-local runtime and world-scoped reward eligibility;
5. event/boss definitions must declare scope rather than inherit accidental multiplicity;
6. PvP/combat cannot use channel switching as immediate escape;
7. same-channel recovery/no-silent-relocation promise;
8. draining/player-visible capacity semantics sufficient for deterministic E2E.

### May remain deferred

- exact numeric channel capacity;
- exact switch cooldown seconds;
- dynamic autoscaling thresholds/hysteresis;
- orchestrator technology;
- final channel naming/public numbering scheme;
- detailed queue prioritization;
- every boss/event-specific scope;
- PvP-specific timers/formulas;
- live migration between GameNodes;
- cross-region channel placement;
- final UI styling.

## 17. Owner decision packet

The minimum owner decisions required to turn this analysis into an accepted `GAME-CHANNEL-01` contract are:

1. **Player model:** soft-visible sticky channels (recommended), hidden shards or explicit manual channels.
2. **Assignment/co-location:** confirm automatic sticky assignment with party/friend join-channel preference.
3. **Manual switching:** confirm safe-state switching rather than anywhere/relog-only.
4. **Anti-hopping:** confirm world-scoped eligibility/fencing in addition to any UX cooldown.
5. **Ordinary spawn policy:** confirm channel-local ordinary creatures/loot as the capacity baseline.
6. **Rare events/bosses:** confirm explicit per-content runtime + reward scope, with no implicit multiplied rare rewards.
7. **PvP:** confirm channel switching is not an immediate PvP escape.
8. **Social continuity:** confirm world-level party/guild/chat/presence with channel-local physical simulation.
9. **Recovery:** confirm failure/reconnect never silently switches to another live channel.
10. **Capacity lifecycle:** confirm dynamic channels may exist later but healthy incumbents are not moved merely for utilization optimization.

No exact numeric capacity, cooldown or scaling threshold needs owner selection in this gate.

## 18. Recommended decision package

### RECOMMENDATION — NOT OWNER-ACCEPTED

```text
one logical world
-> multiple soft-visible sticky capacity channels
-> automatic safe assignment
-> party/friend co-location preference
-> manual join/switch only from a server-proven safe state
-> combat/PvP/value/encounter fences authoritative

ordinary simulation
-> monsters/spawns/ground items channel-local

world continuity
-> character durability + market + guild/chat/presence shared by declared world owners

rare/high-value content
-> explicit runtime scope
+ explicit world/account/character eligibility scope
-> never infer reward multiplicity from channel multiplicity

failure/reconnect
-> same logical channel semantics
-> no silent hop to another existing channel

capacity
-> dynamic create/drain later under PERF-01/OPS-CHANNEL-01
-> no forced healthy-player reshuffle merely for utilization
```

This package preserves Oteryn's multichannel scaling advantage while protecting social continuity, economy integrity, PvP fairness and the feeling of one persistent world.

## 19. Deliberately not decided here

- numeric capacity/player limits;
- exact cooldown durations;
- dynamic scaling algorithm or orchestrator;
- final queue priority rules;
- exact public channel labels;
- every event/boss policy;
- PvP formulas/timers;
- cross-region architecture;
- house-presence topology;
- live channel migration;
- final client UI layout.
