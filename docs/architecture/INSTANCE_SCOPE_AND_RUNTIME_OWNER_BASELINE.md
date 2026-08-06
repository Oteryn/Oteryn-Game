# Oteryn v2 World-Scoped Instance, Map Runtime and Activity Admission Baseline

- Status: Owner-accepted pre-contract baseline
- Date: 2026-08-06
- Decision owner: Oteryn project owner
- Related gates: `FND-ID-01`, `FND-02`, `FND-03`, `FND-04`, `DUR-03`, `GAME-INSTANCES-01`, future Party Finder contract
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: future identifier, runtime, map/content, admission, protocol, persistence, party, matchmaking, reward and recovery contracts

## Purpose

Record the instance identity, map representation, activity admission and simulation-ownership model explicitly accepted by the project owner before the complete runtime and protocol contracts are drafted.

This document is canonical architecture input. It does not complete any named gate and does not authorize runtime, protocol, persistence, Party Finder or gameplay implementation.

Where the earlier `FND-ID-01_OWNER_ACCEPTED_BASELINE.md` left concrete instance scope, cross-channel membership and return semantics unresolved, this later owner decision supersedes those specific unresolved placeholders. Representation, wire format and implementation details remain unresolved.

## Accepted topology

Channels remain the primary topology of one logical world. Instances are optional isolated gameplay contexts and do not replace channels, create independent worlds or divide the world's persistent community, characters or economy.

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
- instance identity remains independent from current GameNode, process, container, host and physical placement;
- names, activity labels, map coordinates and queue numbers are not canonical instance identity;
- future protocol, event, audit, persistence and recovery boundaries carry or authoritatively derive the owning `WorldId`.

This decision freezes semantic scope. It does not select UUID, ULID, integer width, generation algorithm, database representation or wire encoding.

## Accepted map and spatial model

Instanced gameplay is not represented as an ordinary teleport to another globally shared portion of one channel map.

The accepted model separates:

```text
Channel map
    persistent/open-world map and physical activity triggers

Activity map template
    immutable or revisioned geometry and content definition

Concrete InstanceRuntime
    private mutable state for one WorldId + InstanceId
```

### Channel-side entry space

A physical boss entrance may contain:

- a lever or another interaction trigger;
- required player tiles or an entrance volume;
- visible environmental state;
- activity-specific access rules;
- an exit or return anchor.

The entrance remains ordinary channel-owned map state. Pulling the lever does not directly mutate player coordinates into an instance.

### Activity map template

A boss arena or dungeon is defined by a revisioned content template, conceptually containing:

```text
ActivityMapTemplate {
    template_id
    content_revision
    geometry
    collision
    static_decorations
    entry_anchors
    creature_spawn_definitions
    scripted_triggers
    exit_anchors
    activity_limits
}
```

The exact source format remains unresolved and may later be represented by the project's native content format and World Bundle pipeline.

Immutable geometry, collision and static content may be shared across many active instances. A new instance does not require a deep copy of all static map assets.

### Instance-local mutable overlay

Each `WorldId + InstanceId` owns an isolated mutable overlay including, where applicable:

- admitted players;
- bosses and creatures;
- doors and destructible objects;
- fields, hazards and temporary effects;
- corpses and transient items;
- timers and encounter phases;
- scripted state and objectives;
- checkpoint and completion state;
- reward eligibility and settlement state.

No instance-local mutation is visible to another instance using the same template unless a later explicit cross-instance service contract allows a specific durable effect.

### Spatial identity

A position is meaningful only inside its owning spatial context. The accepted semantic distinction is equivalent to:

```text
ChannelSpace {
    WorldId,
    ChannelId,
    Position
}

InstanceSpace {
    WorldId,
    InstanceId,
    Position
}
```

Two instances may use identical local coordinates without sharing visibility, collision, creatures or mutable state. Raw `x, y, z` values are never sufficient to establish spatial identity or authority across runtime boundaries.

## Accepted trigger-neutral activity admission

Physical levers, Party Finder, quests, scheduled events, arena queues and authorized administrative operations must not implement separate instance-creation engines.

They are different admission sources that consume one common authoritative activity-admission contract, conceptually equivalent to:

```text
CreateActivityInstance {
    activity_id
    activity_template_revision
    participants
    admission_source
    return_policy
    request_id
}
```

Possible `admission_source` classes include:

- `physical_trigger`;
- `party_finder`;
- `quest`;
- `scheduled_event`;
- `arena_queue`;
- `authorized_operation`.

All sources converge on the same capacity reservation, eligibility validation, instance allocation, ownership transfer, snapshot, reward and recovery boundaries. Source-specific validation may be added, but it must not bypass the shared safety model.

## Accepted physical-trigger flow

For an activity such as a five-player Vladruk boss encounter, the accepted flow is:

1. Five players occupy the configured entrance tiles or entrance volume on one channel.
2. An authorized player activates the lever.
3. The `ChannelRuntime` treats this as an admission request, not as an immediate teleport.
4. The server atomically validates the complete participant set.
5. Capacity and one concrete instance are reserved.
6. The activity map template and private mutable overlay are prepared.
7. Simulation ownership of all accepted participants is transferred to one `InstanceRuntime`.
8. Clients receive the committed instance context and a full authoritative snapshot.
9. The entrance is released for later groups according to activity policy.

The validation boundary includes at least:

- exact required participant count or an accepted activity-specific range;
- distinct characters and valid active sessions;
- required source location for physical triggers;
- same `WorldId`;
- level, quest, item, cooldown, lockout and other activity requirements;
- absence of incompatible character states such as another transfer;
- instance and infrastructure capacity;
- current activity/template revision compatibility;
- duplicate-request and replay protection.

Unless an activity explicitly adopts another accepted policy, a failure for one required participant fails the whole admission attempt before ownership transfer. No participant is silently left in an ambiguous partial state.

## Accepted Party Finder flow

Party Finder uses the same instance and admission engine. It differs only in how participants are assembled and how readiness is established.

The accepted flow is:

1. Party Finder matches or assembles eligible characters from one logical world.
2. Participants may currently be owned by different channels of that world.
3. Every required participant accepts the activity and enters a bounded readiness state.
4. The server validates the complete participant set and reserves capacity.
5. One `WorldId + InstanceId` and one authoritative `InstanceRuntime` are prepared from the same activity template used by physical entry.
6. Each participant is transferred directly from their source `ChannelRuntime` to that common `InstanceRuntime`.
7. The players do not need to be moved to one temporary common channel before entering the activity.
8. The shared encounter begins only after the accepted admission barrier succeeds.

Party Finder therefore does not create a parallel boss system, parallel arena maps or a weaker transfer path. It is a remote admission source for the same activity definition and runtime contract.

Party Finder adds source-specific requirements such as:

- accepted roles or composition where the activity requires them;
- explicit ready confirmation and timeout;
- revalidation immediately before commit;
- disconnect and decline behavior;
- queue reservation and cancellation behavior;
- abuse, repeated-decline and AFK controls;
- policy for replacing a participant before the transfer commits.

The exact matchmaking algorithm, role model, queue policy and penalties remain unresolved.

## Accepted cross-channel admission model

A concrete instance may admit eligible players whose authoritative sessions currently originate from different channels of the same logical world.

Admission preserves the following invariants:

- every participant belongs to the same `WorldId` as the instance;
- admission is an explicit authoritative state transition, not a client-side teleport or coordinate change;
- source channels do not remain simultaneous gameplay owners of admitted characters;
- party membership alone does not authorize entry;
- eligibility, readiness, capacity, session generation, activity revision and failure behavior are authoritatively validated;
- admission failure leaves each character under one unambiguous authoritative owner;
- partial group admission is prevented unless an activity has an explicit accepted partial-admission policy;
- an instance cannot create cross-channel open-world combat, visibility, healing, experience, loot or proximity effects.

## Accepted authoritative runtime ownership

After successful admission, all participating characters and instance-local simulation are owned by one logical authoritative `InstanceRuntime`.

Accepted invariants:

- one concrete instance has one logical authoritative mutation owner at a time;
- all instance-local movement, visibility, combat, creatures, environmental state and activity progression are ordered by that owner;
- the owner may execute on a multithreaded GameNode, but parallel work returns through the authoritative ordering boundary;
- stale auxiliary results are rejected using instance identity, ownership generation and relevant state revision;
- source `ChannelRuntime` owners do not mutate admitted characters' instance-local state;
- client state is never authoritative for admission, instance membership or gameplay results;
- changing physical placement does not silently change semantic instance identity;
- any future replacement or migration requires explicit fencing and recovery contracts.

The exact relationship between `InstanceRuntime`, `ChannelRuntime`, `WorldServices` and GameNode placement belongs to `FND-03` and later operations contracts.

## Accepted origin-channel and return binding

Every admitted player retains authoritative origin metadata bound to the same `WorldId`, including at least `origin_channel_id` and an accepted return destination or return-policy reference.

Origin metadata is routing and recovery metadata, not part of canonical instance identity.

It is retained for:

- normal exit routing;
- reconnect and session-resume decisions;
- safe recovery after instance completion or failure;
- audit and diagnosis of ownership transitions;
- later policy decisions concerning return-channel capacity or availability.

Accepted invariants:

- origin metadata cannot authorize mutation after instance ownership has transferred;
- it is not exposed as public presence information without the accepted privacy policy;
- it is validated against the same `WorldId`;
- stale or unavailable origin routing cannot create duplicate sessions or dual writers;
- failure to return to the origin channel follows an explicit fallback policy rather than implicit arbitrary-channel placement;
- changing the eventual return destination does not rewrite the identity or history of the completed instance;
- a configured safe exit anchor is preferred over blindly restoring an entrance tile that may be occupied or unsafe.

Party Finder participants may each retain a different `origin_channel_id`. After activity completion, each participant normally returns through their own validated origin routing unless the activity explicitly defines another accepted same-world destination policy.

## Accepted seamless no-relogin handoff

Entering an instance must not require account re-authentication, character-list selection or a user-visible relog.

The accepted user experience is a seamless context transition, potentially covered by a brief loading or teleport presentation.

The transport and authority model is make-before-break when the destination runtime requires another gameplay connection:

1. The destination instance and participant slots are reserved.
2. The source owner prepares a bounded transfer and temporarily gates incompatible new commands.
3. The currently authenticated session authorizes short-lived, single-use instance admission material.
4. The client may establish a destination gameplay connection in the background while the source connection remains available for safe abort.
5. The source owner produces a consistent transfer package and current fencing context.
6. One authoritative commit transfers simulation ownership from source to destination.
7. The destination sends a full authoritative instance snapshot.
8. The client activates the destination context.
9. The stale source gameplay path is retired and can no longer mutate the character.

If source and destination execute on the same GameNode and protocol connection reuse is safe, the implementation may optimize the physical transport by rebinding the existing connection. This is only an optimization; it must preserve the same admission, fencing, snapshot, idempotency and recovery semantics.

The exact choice between a fresh invisible Game Session, a Game Session continuation, and a dedicated instance-admission grant is left to `FND-04`. Any accepted form must be short-lived, scoped to the intended character/world/instance/transfer generation, single-use or equivalently replay-safe, and must not become a reusable login credential.

The Game Gateway remains a control-plane admission authority under ADR-0003 and does not become a permanent gameplay traffic proxy merely to hide the handoff.

## Accepted ownership-transition state machine

Entering and leaving an instance are explicit simulation-ownership transitions.

The later admission and runtime contracts define a fenced state machine equivalent in meaning to:

```text
ChannelRuntime authority
    -> admission validated
    -> destination reserved and prepared
    -> handoff connection/context prepared
    -> source authority fenced
    -> InstanceRuntime authority activated
    -> full instance snapshot accepted
    -> source gameplay path retired
    -> instance gameplay
    -> exit or recovery prepared
    -> instance authority fenced
    -> destination ChannelRuntime authority activated
```

Required safety properties:

- a character has at most one active authoritative simulation owner;
- every transition has a unique transfer identity and is generation-fenced and idempotent;
- retries cannot create a second character presence;
- disconnects and crashes cannot leave both source channel and instance authoritative;
- failure before ownership activation preserves or restores authority at the previous safe owner;
- failure after activation is recovered from the destination ownership record rather than client claims;
- a stale source connection is unable to resume mutation after commit;
- durable character, inventory and reward mutations remain subject to persistence and anti-duplication contracts.

This baseline does not select the exact transaction, lease, token or message schema.

## Accepted group admission barrier

Activities that require the complete group, such as a five-player boss encounter, use a coordinated admission barrier.

Before commit, the group may be in prepared/reserved states, but encounter gameplay does not begin until all required participants have passed validation and destination readiness requirements.

The later activity contract must state whether the final transition is:

- strict all-or-nothing;
- bounded partial start with explicit minimum composition;
- replacement-capable before commit;
- delayed until a disconnected participant reconnects within a defined window.

The default for fixed-group boss activities is strict all-or-nothing admission. Exact timeout and replacement policy remain activity-specific.

## Accepted activity completion, reward and cleanup flow

On success, failure, abandonment or timeout:

1. The `InstanceRuntime` freezes the terminal encounter result.
2. Eligibility and rewards are calculated from authoritative instance state.
3. Durable reward, lockout and inventory effects are committed idempotently under later `DUR-03` contracts.
4. Return destinations are validated and reserved.
5. Simulation ownership transfers from the instance to destination channel runtimes.
6. Clients receive full destination snapshots or an equivalent accepted resynchronization boundary.
7. The completed instance enters cleanup/retention state and is destroyed only after required durable and audit evidence is safe.

A physical entrance normally becomes available for another group after the prior group has committed entry, subject to activity-specific concurrency, cooldown and capacity policy. The entrance is not automatically blocked for the full duration of another group's private encounter.

## Party and activity consequences

The accepted world-scoped party model remains unchanged:

- `WorldId + PartyId` owns party membership and organization;
- members may originate from different channels of the same world;
- open-world shared gameplay requires one common channel;
- instanced shared gameplay requires admission to one common concrete `WorldId + InstanceId`;
- remote party members outside the instance receive no instance-local combat, loot, experience, healing, visibility or proximity effects;
- leaving or failing admission does not automatically destroy the world-scoped party.

A Party Finder may assemble eligible players across channels of one world, but it consumes the accepted activity-admission, capacity, consent, transfer and failure contracts.

## Privacy consequences

Instance placement is non-public location information under the accepted social-presence baseline.

The client and public APIs must not reveal unauthorized `InstanceId`, GameNode placement, source channel or map position. Presence caches fail toward less disclosure, and hidden placement must not be inferable through invitation, search, Party Finder, alternate-character or timing side channels.

## Required application to later contracts

This baseline is mandatory input to:

- `FND-ID-01` — `InstanceId` semantic scope and comparison rules;
- `FND-02` — admission, transfer, snapshot, reconnect and context-transition fields;
- `FND-03` — `InstanceRuntime` ownership, ordering, lifecycle, placement and recovery;
- `FND-04` — Game Session binding, admission material, lease fencing and duplicate-session prevention;
- `DUR-01` and `DUR-02` — durable representation and ownership-transition evidence where required;
- `DUR-03` — item, loot, reward, lockout and currency safety during transition and settlement;
- `DUR-04` — activity templates, map/content revisions and World Bundle bindings;
- `ANL-01` — correlated admission, transition, encounter and audit events;
- `QA-E2E-01` — deterministic cross-channel admission, crash and duplicate-owner scenarios;
- `GAME-INSTANCES-01` — lifecycle, matchmaking, capacity, lockouts, rewards, spectating and retention;
- future party, Party Finder, map, content, privacy and operations contracts.

## Required deterministic acceptance scenarios

Later contracts must provide named evidence for at least:

1. five eligible players on one channel activate a physical lever and enter one isolated boss instance;
2. an ineligible player causes a strict fixed-group physical admission to fail before any ownership transfer;
3. two groups use the same activity map template concurrently without sharing mutable state;
4. identical local map coordinates in two instances do not create visibility or collision between them;
5. Party Finder participants from three channels of one world enter one shared authoritative instance without first moving to a common channel;
6. a player from another world is rejected before ownership transfer;
7. duplicate lever, Party Finder or admission commands do not create duplicate instances, membership or presence;
8. one participant disconnects during preparation and the activity follows its explicit barrier policy;
9. disconnect during ownership commit resolves to exactly one authoritative owner;
10. source-channel crash after transfer cannot overwrite instance-owned state;
11. instance-runtime crash cannot reactivate a stale source-channel writer;
12. a cross-GameNode handoff completes without account relog or character reselection;
13. failure before commit leaves the player safely on the source channel;
14. failure after commit reconnects from the authoritative instance ownership record;
15. successful exit returns each participant through validated origin-channel routing and a safe exit anchor;
16. unavailable origin channel follows an explicit safe policy and never selects an arbitrary destination silently;
17. stale session generation, ownership generation, template revision or transfer revision is rejected;
18. instance-local loot, lockouts and rewards remain idempotent across retry and recovery;
19. cleanup cannot destroy required reward, audit or recovery evidence prematurely;
20. unauthorized observers cannot obtain exact instance, source-channel or GameNode placement.

## Deliberately unresolved

The following remain open:

- technical representation and generation of `InstanceId`;
- concrete native source format for activity map templates;
- exact immutable/mutable map data structures and copy-on-write strategy;
- whether all instances are ephemeral or selected types may be durable;
- full instance lifecycle states, timeout, idle shutdown and retention;
- exact protocol messages and transport framing for seamless handoff;
- whether `FND-04` uses a fresh invisible Game Session, continuation or dedicated admission grant;
- exact issuer, signing, validation and expiry format of admission material;
- capacity algorithm, queue scheduling and resource placement policy;
- Party Finder matching, roles, penalties, replacement and cancellation algorithms;
- activity-specific partial-party and reconnect policy;
- relationship between instance placement and channel-hosting GameNodes;
- whether active instance live migration is ever supported;
- checkpoint, replay, RPO, RTO and replacement behavior;
- fallback return behavior when `origin_channel_id` is unavailable, full, draining or incompatible;
- lockouts, checkpoints, spectators and replay streams;
- persistence and audit granularity;
- exact privacy controls and user-facing presentation;
- concrete Rust types, crates, database schemas and wire fields.

## Rejected interpretations

### Bind instance identity to the source channel

Rejected because a concrete instance may admit players from several channels of the same world and retains identity independently from participant origin and physical placement.

### Allow several channel runtimes to co-own one instance

Rejected because authoritative simulation requires one logical mutation owner. Cross-channel admission transfers ownership; it does not merge channel writers.

### Treat admission as a client teleport

Rejected because coordinates do not establish authority, session fencing, capacity reservation or failure recovery.

### Keep source channel authoritative during instance gameplay

Rejected because dual authority permits conflicting character, combat, inventory and recovery mutations.

### Use `origin_channel_id` as instance identity

Rejected because it is validated routing metadata and may differ between participants in the same concrete instance.

### Copy the entire static map for every instance

Rejected as the mandatory model because immutable revisioned geometry and assets may be shared while private mutable overlays remain isolated. A later implementation may materialize selected data where profiling proves it useful, but semantic isolation must not depend on wasteful full duplication.

### Implement separate instance engines for levers and Party Finder

Rejected because duplicated admission, transfer, reward and recovery paths would drift and create exploit surfaces. Both are admission sources for one shared activity-instance system.

### Move Party Finder groups to a temporary common channel first

Rejected as the normal instanced-activity path because the common `InstanceRuntime` is already the shared simulation boundary. An extra channel transition adds failure, capacity and duplication risks without creating needed authority.

### Require a visible relog for instance entry

Rejected because an authenticated session can authorize a fenced gameplay-context handoff. A transport reconnect may occur invisibly, but account authentication and character selection are not repeated.

### Make the Gateway a permanent gameplay proxy

Rejected because ADR-0003 keeps the Gateway in the control plane and gameplay traffic connects directly to the selected game runtime.

### Permit cross-world instances

Rejected because identity, character state, economy, ruleset and gameplay value remain world-scoped by default.

## Programme effect

- The canonical semantic instance identity is `WorldId + InstanceId`.
- Activity maps use revisioned templates plus isolated instance-local mutable overlays.
- Positions are scoped by channel or instance context rather than raw coordinates alone.
- Physical triggers and Party Finder consume one shared activity-admission and instance-runtime system.
- Eligible players may enter one concrete instance from different channels of the same world.
- One authoritative `InstanceRuntime` owns all admitted participants and instance-local simulation.
- Each participant retains validated origin routing for exit, reconnect and recovery.
- Entry and exit are explicit fenced ownership transitions.
- Cross-GameNode transitions use a seamless make-before-break handoff without user-visible relog; same-node transport reuse is only an optimization.
- Fixed-group boss activities default to strict all-or-nothing admission unless their contract states otherwise.
- Channels remain the primary world topology.
- Full identifier, protocol, runtime, map format, Party Finder, admission and instance-lifecycle contracts remain ordered future work.
- The source-only `blakinio/otclient` historical marker remains required before the complete `FND-ID-01` package begins.
- No implementation is authorized by this document.
