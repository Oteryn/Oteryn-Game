# Multichannel system scope matrix

Status: architecture baseline supporting ADR-0001  
Date: 2026-08-05

## Purpose

Every gameplay and platform subsystem must declare:

- its authoritative owner;
- its identity scope;
- its consistency requirement;
- whether it may cross channels;
- how it behaves during channel switching and failure.

The matrix prevents accidental process-global state, per-channel duplication of world data and ambiguous ownership.

## Scope vocabulary

| Scope | Meaning |
|---|---|
| Account | Shared by all characters owned by one account where product rules require it. |
| Character | Durable state of one character in one world. |
| World | Shared by every channel of one logical world. |
| Channel | One parallel public-world simulation. |
| Instance | One isolated runtime space associated with a channel or world service. |
| Node | Physical runtime placement only; never a product identity. |

## Consistency vocabulary

| Model | Meaning |
|---|---|
| Authoritative immediate | One ordered mutation owner; clients observe server decisions. |
| Strong durable | Commit/revision fence required before success is final. |
| Idempotent transactional | Retry-safe mutation with stable command/transaction identity. |
| Ordered per stream | Messages ordered within one room/entity stream. |
| Eventual | Temporary delay or stale read is acceptable. |
| Rebuildable | Runtime state may be reconstructed from durable source and deterministic content. |

## Baseline matrix

| System | Authoritative owner | Scope | Consistency | Cross-channel behaviour | Initial decision |
|---|---|---:|---|---|---|
| Public map definition | Content service/package | World | Immutable revision | Same revision on every channel | Shared immutable data |
| Public map runtime overlay | `ChannelRuntime` | Channel | Authoritative immediate | Never shared implicitly | Separate per channel |
| Creature and spawn runtime | `ChannelRuntime` | Channel | Authoritative immediate | Independent copies | Separate per channel |
| Player position | `ChannelRuntime` | Channel | Authoritative immediate | Changes only by new session/entry | Local |
| Character progression | Character persistence owner | Character | Strong durable | Same character state on every channel | Shared durable state |
| Character lease | World session/lease service | Character + World | Strong durable | Exactly one active channel | Mandatory fencing |
| Inventory and equipment | Character domain/persistence owner | Character | Idempotent transactional | Travels with character | Shared durable state |
| Ground items and corpses | `ChannelRuntime` | Channel | Authoritative immediate + durable boundary on pickup | Never visible in another channel | Local |
| Loot pickup transfer | Channel + character transaction boundary | Character + Channel | Idempotent transactional | Must not duplicate during relog/failure | Explicit transaction required |
| Local speech (`say`, `whisper`, `yell`) | `ChannelRuntime` | Channel/Instance | Authoritative immediate | Does not cross channels | Local visibility rules |
| World chat | World communication owner | World | Ordered per room | Reaches all channels of one world | Shared |
| Guild chat | World communication owner | World/Guild | Ordered per room | Reaches guild members on all channels | Shared |
| Private messages | World communication owner | World | Ordered per conversation where feasible | Reaches recipient on any channel | Shared |
| Presence | World presence owner | World | Eventual with lease truth | Shows online/channel state subject to privacy | Shared |
| Guild membership | Guild service/domain | World | Strong durable | Same guild on all channels | Shared |
| Party membership | Party service/domain | World | Strong/ordered | Membership and chat may cross channels | Shared membership |
| Shared experience | `ChannelRuntime` | Channel | Authoritative immediate | Only eligible colocated members | Local effect |
| Direct player trade | `ChannelRuntime` + durable transaction | Channel | Idempotent transactional | Same channel only | Local interaction |
| Market | Market service/domain | World | Strong durable | One economy across all channels | Shared |
| Bank | Character/account domain | Character or Account | Strong durable | Same balance on all channels | Shared |
| Depot | Character/world persistence | Character + World | Strong durable | Same contents on all channels | Shared |
| Mail/parcels | World service/domain | World | Idempotent transactional | Cross-channel delivery | Shared |
| Quest progress | Character persistence | Character | Strong durable | Same progress on all channels | Shared |
| NPC runtime/conversation | `ChannelRuntime` | Channel/Instance | Authoritative immediate | Independent NPC state unless explicitly global | Local by default |
| Daily/account rewards | Reward service/domain | Character or Account | Strong durable | Cannot repeat through channel hopping | Shared eligibility |
| Boss runtime | `ChannelRuntime` or explicit world event owner | Channel/World | Explicit per event | Must declare scope | No implicit default |
| Boss reward eligibility | Reward service/domain | Character/Account/World | Strong durable | Prevent repeated farming across channels | Shared eligibility |
| Raids and events | Event owner | World/Channel/Instance | Explicit per event | Scope declared in event definition | No implicit default |
| PvP combat execution | `ChannelRuntime` | Channel | Authoritative immediate | Local combat only | Local |
| Combat lock | Character/world policy | Character + World | Strong/lease-visible | Blocks channel change | Shared consequence |
| Skull and frag state | Character/world PvP domain | Character + World | Strong durable | Persists across channels | Shared |
| Guild war | Guild/world PvP domain | World | Strong durable | Applies on all channels | Shared |
| Houses: ownership/rent/access | House service/domain | World | Strong durable | One set across all channels | Shared, accepted |
| Houses: items/runtime topology | House service/domain | World | Strong durable | One authoritative state; topology deferred | Provisional |
| Houses: presence/entry | Future house ADR | World/Instance | Undecided | Must not enable channel switching | Deferred |
| Instanced dungeon | Instance runtime | Instance | Authoritative immediate | Isolated; origin channel explicit | Separate instance |
| Ruleset | Versioned configuration/content | World | Immutable revision | Identical on every channel | Shared |
| Item/monster/spell catalogues | Versioned content | World | Immutable revision | Identical on every channel | Shared |
| Scripts | Script runtime with explicit context | World/Channel/Instance | Depends on effect | No unscoped process globals | Mandatory context |
| Ranking | Ranking projection | World | Eventual | Aggregates all channels | Shared projection |
| Metrics | Observability system | Node/Channel/World labels | Eventual | Aggregated without becoming authority | Non-authoritative |
| Channel directory | World Registry | World | Health-driven/eventual with guarded admission | Lists and routes channels | Shared control plane |
| Game Session | Gateway/session issuer | Character + World + Channel | Strong, short-lived | Frozen to selected channel | Mandatory binding |

## Required identity envelope

Every runtime command, event and timer that can mutate or address gameplay state must carry enough identity to resolve its owner without process-global lookup.

Minimum envelope where applicable:

```rust
pub struct RuntimeScope {
    pub world_id: WorldId,
    pub channel_id: Option<ChannelId>,
    pub instance_id: Option<InstanceId>,
}
```

Durable character mutations additionally require:

```rust
pub struct CharacterMutationFence {
    pub character_id: CharacterId,
    pub game_session_id: GameSessionId,
    pub session_generation: SessionGeneration,
    pub command_id: CommandId,
}
```

## Channel change gates

A channel change must be rejected or delayed while any of these conditions is true:

- combat lock or protected PvP state;
- direct player trade;
- unresolved item/loot transaction;
- active house mutation or topology transition;
- protected boss/raid encounter where hopping changes eligibility;
- active instance that does not define a safe exit;
- pending character checkpoint;
- stale or unavailable lease service;
- incompatible target channel revisions;
- target channel is full, draining, unhealthy or offline.

## Reward anti-hopping contract

Every reward definition must declare both runtime encounter scope and eligibility scope.

Example:

```rust
pub struct RewardPolicy {
    pub encounter_scope: EncounterScope,
    pub eligibility_scope: EligibilityScope,
    pub reset_policy: ResetPolicy,
    pub idempotency_key_shape: RewardKeyShape,
}
```

The fact that five independent boss copies exist on five channels does not imply five reward claims are allowed.

## Script isolation rules

Script execution must receive an explicit context and may not discover a default world/channel from a global singleton.

```rust
pub struct ScriptContext {
    pub world_id: WorldId,
    pub channel_id: Option<ChannelId>,
    pub instance_id: Option<InstanceId>,
    pub actor_id: Option<EntityId>,
    pub event_id: EventId,
}
```

Process-global mutable script variables are prohibited unless owned by a named world service and accessed through a versioned API.

## House safety invariants pending dedicated ADR

The following are fixed even though final topology is deferred:

1. `HouseId` belongs to a world and does not include `ChannelId`.
2. Ownership, rent and access lists exist once per world.
3. House items have one authoritative state, not one copy per channel.
4. All item mutations are ordered, revisioned and retry-safe.
5. Entering or leaving a house cannot silently change the character's gameplay channel.
6. A stale channel cannot overwrite newer house or character state.
7. House service failure must fail closed for risky mutations.

## Open decisions requiring separate ADRs

- final house presence and topology model;
- exact party behaviour across channels beyond chat/membership;
- boss and raid default policies;
- whether selected social channels are available outside gameplay;
- exact inventory/ground-item transaction implementation;
- exact scripting engine and isolation mechanism;
- exact event journal/checkpoint strategy;
- channel drain and crash recovery timing;
- final database and messaging technologies.
