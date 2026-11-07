# Lag / Disconnect Protection Owner Baseline

- Status: Owner-accepted pre-contract baseline
- Date: 2026-08-07
- Decision owner: Oteryn project owner
- Owner acceptance checkpoint: 2026-08-07
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: server-authoritative connection-health detection, PvE disconnect protection, reconnect/re-entry safety, instanced-encounter recovery and later `FND-03` / `FND-04` / gameplay / analytics contracts

## Purpose

Record the owner-accepted gameplay baseline for protecting a character against genuine Internet loss, client crash or power loss without turning disconnect into a safe combat-reset primitive.

This document is architecture only. It does not authorize runtime implementation and does not freeze the final heartbeat format, wire representation, persistence schema, monster-AI data structures, PvP policy, sanction thresholds or orchestration details.

## Owner-accepted activation boundary

The gameplay-visible baseline is:

```text
0.0 s <= elapsed < 2.0 s  -> normal PvE monster attacks
elapsed >= 2.0 s          -> disconnect protection active
```

In player-facing terms, the first two full seconds after loss of sufficient server-authoritative control/liveness evidence remain normal combat. **From the beginning of the third second, all PvE monsters must stop actively attacking the disconnected/protected character.**

The activation boundary is exactly **2.0 seconds elapsed**, not 3.0 seconds elapsed. Saying "from the third second" describes the interval that begins after the first two complete seconds have passed.

The timer is measured from server-observed evidence, never from an untrusted client statement such as `I have lag` or `I disconnected`.

The exact heartbeat packets, evidence aggregation and low-level state names remain later `FND-03` / `FND-04` work.

## Relationship to gameplay ticks

The two-full-second boundary is a gameplay-policy boundary aligned with the intended Tibia-like two-second combat cadence for this class of behavior.

It does **not** require the authoritative Rust server main loop, scheduler, simulation or networking runtime to execute only once every two seconds. Internal processing may and should use finer-grained timing where appropriate.

Conceptually:

```text
last sufficient server-authoritative control/liveness evidence
        |
        | first 2 full seconds: normal PvE combat
        v
start of third second / elapsed >= 2.0 s
        |
        v
all PvE monsters stop actively attacking the protected actor
```

## Owner-accepted PvE monster behavior

From the beginning of the third second of continuous insufficient server-authoritative control/liveness evidence, **all PvE monsters must stop actively attacking the protected character**, including monsters that were already targeting or engaged with that actor.

While disconnect protection is active:

- monster AI emits no new offensive action against the protected actor;
- already-targeting monsters are suppressed from issuing further attacks against that actor;
- targeting/threat/aggro bookkeeping may remain preserved internally so protection does not become an aggro reset;
- the monster/encounter itself does not freeze and may continue acting against other legal targets.

Protection activation does not retroactively roll back authoritative gameplay committed before the 2.0-second boundary. Effects already committed may still resolve, including conceptually:

- an attack committed before the boundary;
- a projectile emitted before the boundary;
- damage-over-time already applied;
- an existing field or environmental hazard;
- an area effect already committed by the simulation.

The protection is therefore **not invulnerability**.

## State-preservation invariants

Disconnect protection must not itself:

- clear combat/PZ/logout lock;
- erase threat/aggro history to manufacture a clean state;
- heal HP, mana or other resources;
- clear damage-over-time, debuffs, cooldowns or exhaustion;
- teleport or reposition the character merely because connectivity was lost;
- reset encounter or instance state;
- roll back already committed actions;
- duplicate or recreate the character actor;
- admit another character from the same account while the protected actor still has unresolved mandatory world presence.

The same authoritative character state remains the subject of reconnect/recovery.

## Five-second stale-transport close

The owner accepted a short transport-cleanup boundary:

```text
stale_transport_close = 5 seconds
```

If connectivity/control has not recovered after five seconds, the stale concrete transport/connection may be forcibly closed or kicked.

This **does not mean the character loses disconnect protection at five seconds** and does not mean the logical gameplay state is discarded.

This five-second transport close must remain compatible with the separately accepted logical reconnect grace:

```text
stale transport close        = 5 seconds
logical GameSessionId grace = 15 seconds
```

Therefore:

- at five seconds the old transport can be considered terminal/stale for transport cleanup;
- the already accepted 15-second window may still allow the same logical `GameSessionId` to recover through a newer transport generation when all later `FND-04` checks succeed;
- five-second transport cleanup must not itself despawn, safe-log, heal, reset or expose a protected actor to avoidable death;
- stale packets/commands from the closed generation remain fenced.

Exact transport implementation and the precise relationship between close detection, reconnect credentials and the 15-second logical session window remain `FND-02` / `FND-04` contract work.

## Longer Internet, client-crash and power-loss recovery

The owner explicitly requires the design to protect genuine outages that last longer than five seconds, including:

- ISP/network interruption;
- router failure;
- client crash;
- computer crash/reboot;
- power loss.

The architecture therefore must not equate `offline longer than five seconds` with `character forfeits protection`.

A longer outage may place the character into an authoritative held/suspended recovery state. Working terminology may include `DISCONNECTED_HELD` and `SUSPENDED_CHARACTER`; the final state names and storage representation remain later contract work.

The accepted semantics are:

- authoritative character state is preserved rather than reset;
- mandatory combat/world obligations are not escaped by merely staying offline;
- a second character on the same account is not admitted while unresolved authoritative presence/obligation remains;
- the implementation need not keep an entire live GameNode actor resident indefinitely if durable suspension can preserve the required state correctly;
- later reconnect/re-entry resumes or resolves that preserved state rather than creating a fresh safe-state character.

Exact behavior after the already accepted 15-second `GameSessionId` grace expires remains to be formalized in `FND-04` and persistence/gameplay contracts, but it must preserve the above invariants.

## Owner-accepted re-entry protection

After a valid return to playable control, the owner accepts an initial PvE re-entry protection window:

```text
reentry_pve_protection = 4 seconds
```

This is intentionally approximately **two gameplay turns** for orientation and escape after recovering from a real outage.

During the four-second re-entry window:

- PvE monsters do not begin new attacks against the returning character;
- the player may use the window to reorient and escape;
- the character is the same authoritative character state, not a respawn/reset;
- no heal, resource refill, condition clear or other automatic gameplay reset is implied by re-entry protection.

The exact list of player actions that immediately cancel re-entry protection remains a later gameplay-contract decision. The design must prevent the four-second window from becoming a repeatable free offensive/loot/healing exploit.

## Instanced boss encounters

Disconnect protection must not hold an entire party encounter or boss instance open solely for one disconnected participant.

If one participant disconnects while the remaining group continues:

```text
party enters instance
        |
participant A disconnects
        |
A receives disconnect protection
        |
remaining party continues encounter
        |
boss may die normally
        |
encounter completes normally for the party
```

The encounter outcome is authoritative and is not rolled back because A was offline.

The architecture must retain durable participant/encounter evidence sufficient to determine at least conceptually:

- `EncounterId`;
- `CharacterId`;
- join/participation timing;
- disconnect timing;
- meaningful participation evidence across applicable roles;
- encounter result;
- attempt/cooldown consumption;
- personal reward eligibility/entitlement where the content model uses personal rewards.

The final participation formula must not be naïvely damage-only; tank, healer, support and other legitimate contribution models must be representable.

## Attempt, cooldown and reward consequences

Disconnect must not become a way to avoid an encounter attempt/cooldown or duplicate rewards.

Accepted semantic direction:

- a participant cannot disconnect to erase a legitimately consumed boss attempt/cooldown;
- encounter completion is recorded even if the participant is offline at boss death;
- where the game uses personal rewards, eligibility may be materialized as a durable character/encounter reward entitlement and claimed later exactly once;
- where the game uses shared physical corpse/container loot, disconnect does not create a private duplicate copy for the absent player;
- reward eligibility must be derived from the accepted encounter-participation policy, not merely from being online at the exact boss-death instant.

Exact reward thresholds, entitlement schema and claim transaction rules belong to later gameplay/content and `DUR-03` contracts.

## Re-entry after an instance has ended

If a disconnected player returns while the original instance still exists and the encounter contract permits resume, the same authoritative participant may resume according to that instance's recovery rules.

If the encounter has completed and the instance has already been destroyed, the player **must not be recreated inside a non-existent boss room** and must not cause the old instance to be resurrected.

Every such instance/content definition must provide a deterministic recovery destination conceptually equivalent to:

```text
entry_anchor
normal_exit_anchor
recovery_exit_anchor
```

When the original `InstanceId` no longer exists, the returning character resumes at the instance-defined `recovery_exit_anchor`.

The recovery destination:

- is predetermined by content/instance definition;
- is not selected opportunistically after seeing the disconnect outcome;
- must not become an arbitrary teleport-to-safety primitive;
- may be identical to the normal exit/lobby when appropriate;
- supersedes attempting to place the player back into a destroyed instance.

If that recovery exit is already a safe lobby/PZ, an additional four-second monster-protection window may be unnecessary there; exact composition with re-entry protection remains content/gameplay policy work.

## Anti-abuse telemetry and enforcement direction

The server cannot reliably distinguish every genuine ISP/power/client failure from a player deliberately cutting connectivity from a single event alone.

The owner therefore accepts **longitudinal telemetry and pattern analysis** as a primary anti-abuse mechanism.

Disconnect/re-entry evidence should support analysis of signals such as:

- disconnect frequency and duration;
- combat/PZ state at disconnect;
- HP/mana/resources at disconnect;
- recent incoming damage/risk;
- monster/boss/encounter context;
- repeat disconnects within the same encounter or combat episode;
- re-entry-protection usage;
- regional/ISP/GameNode outage correlation;
- comparison between combat disconnects and ordinary session disconnects.

A single disconnect is not proof of abuse. Repeated suspicious behavior may create an evidence-backed case and progressive enforcement path conceptually including warning and later suspension/ban if abuse continues and the required evidence threshold is met.

Consistent with ADR-0006, **Game Intelligence remains observational/investigative and must not autonomously ban players or mutate authoritative gameplay state**. Exact warning thresholds, sanction authority, review/appeal policy and temporary/permanent-ban progression require a separate enforcement-policy contract.

The architecture should distinguish individual suspicious disconnects from correlated infrastructure incidents so widespread ISP/GameNode/platform failures do not falsely accumulate abuse evidence against affected players.

## Explicitly unresolved

This baseline does not yet decide:

- exact liveness/heartbeat evidence required for the 2.0-second timer;
- final connection-health state names and transition mechanics;
- final representation/storage of long-lived held/suspended characters;
- exact post-15-second `GameSessionId` recovery/admission semantics;
- exact actions that cancel the four-second re-entry window;
- repeat-use/exhaustion/budget rules, if any are still needed after telemetry enforcement;
- PvP disconnect protection;
- precise boss/summon/environment/script actor classification under the PvE attack-suppression rule;
- exact encounter-participation/reward thresholds;
- final warning/ban thresholds and human-review requirements;
- mass-infrastructure-failure compensation policy;
- final telemetry retention/privacy schema.

## Required consumers

This baseline is mandatory input to later:

- `FND-03` runtime/timer/state-transition design;
- `FND-04` connection liveness, reconnect, transport cleanup, admission and session state machine;
- gameplay combat and monster-AI policy;
- instance/encounter/content contracts;
- `DUR-02` / `DUR-03` where suspended state, encounter outcome or reward entitlement becomes durable;
- QA/E2E disconnect/reconnect/instance-destruction scenarios;
- Game Intelligence telemetry and abuse-analysis design;
- later enforcement-policy design.

Tests must eventually prove at minimum that:

1. the first two full seconds retain normal PvE behavior;
2. from `elapsed >= 2.0 s`, every PvE monster stops emitting new attacks against the protected actor, including monsters already targeting it;
3. effects committed before protection activation are not rolled back;
4. stale transport is closed/fenced by the five-second cleanup policy without invalidating the separately accepted 15-second logical reconnect grace;
5. a genuine outage longer than five seconds does not silently reset or discard authoritative character state;
6. valid re-entry provides the configured four-second PvE orientation/escape window without respawn/heal/state reset;
7. an instance continues and can complete for remaining participants while one participant is disconnected;
8. disconnect cannot avoid a consumed encounter attempt/cooldown or duplicate a reward;
9. when the original instance is destroyed, later login uses the deterministic `recovery_exit_anchor` rather than resurrecting or re-entering the destroyed instance;
10. repeated disconnect evidence is auditable while Game Intelligence remains non-authoritative and non-banning.

## Programme effect

Accepted now:

```text
first 2 full seconds after loss of sufficient server-authoritative control/liveness evidence -> normal PvE attacks
from start of third second / elapsed >= 2.0 s -> all PvE monsters stop actively attacking the protected character
already-targeting monsters are also suppressed from issuing new attacks
already-committed effects are not rolled back
5 s without recovery -> stale concrete transport may be closed/kicked
5 s transport cleanup != end of 15 s logical GameSessionId reconnect grace
longer real outage -> authoritative state remains protected/held/suspendable rather than reset
valid re-entry -> initial 4 s / two-turn PvE orientation-and-escape protection
instance encounter continues for remaining party and may complete normally
attempt/cooldown and reward outcome are resolved from durable encounter participation
completed/destroyed instance -> returning character uses deterministic recovery_exit_anchor
repeated disconnect abuse -> telemetry/evidence -> separate progressive warning/sanction policy
Game Intelligence itself remains read-only/investigative and never autonomously bans
```

No runtime, protocol, persistence, database, client or Platform implementation is authorized by this baseline.
