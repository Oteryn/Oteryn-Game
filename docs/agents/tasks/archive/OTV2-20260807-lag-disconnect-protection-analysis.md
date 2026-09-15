# OTV2-20260807-lag-disconnect-protection-analysis

```yaml
task_id: OTV2-20260807-lag-disconnect-protection-analysis
title: Continue Lag / Disconnect Protection architecture analysis
mode: ARCHITECTURE_ANALYSIS_ONLY
status: owner_accepted_checkpoint_current
repository: Oteryn/Oteryn-Game
base_branch: main
owner: Oteryn project owner
updated_at: 2026-08-07T18:01:00+02:00
cross_repository_coordination_id: OTV2-NATIVE-FOUNDATION
canonical_contracts:
  - docs/architecture/FND-ID-01_ACCOUNT_SINGLE_ONLINE_CHARACTER_OWNER_BASELINE.md
  - docs/architecture/FND-ID-01_GAME_SESSION_RECONNECT_GENERATION_OWNER_BASELINE.md
  - docs/architecture/LAG_DISCONNECT_PROTECTION_OWNER_BASELINE.md
  - docs/architecture/LAG_DISCONNECT_REENTRY_ACTION_POLICY_OWNER_BASELINE.md
accepted_prs:
  - PR #72 / merge 97b29e5c927f319ed03fb5583614d5fe0366d134
  - PR #73 / merge af094d4d75d3a19db63714810f263059c78f7b3a
  - PR #75 / merge 4434f3f16ba5d313b31cb6959ca68fb934b0572a
  - PR #76 / merge b457fb70954f46df2f5f11f6b7a2b2e056b79b03
```

## Purpose

Preserve the complete current owner-accepted Lag / Disconnect Protection architecture so a later agent can continue without reconstructing the discussion from chat history.

This is an architecture checkpoint only. Do not implement runtime, protocol, persistence, database, client, Platform or gameplay code unless the owner explicitly authorizes implementation.

The canonical semantic sources are the architecture baseline files listed above. This task is the current discussion/handover overlay and must not contradict them.

## Accepted prerequisites

### One authoritative online character per account

At most one character per account may hold authoritative gameplay permission at a time.

A second authenticated client cannot kick, revoke or steal a healthy incumbent character while that character has combat/PZ/logout mandatory world presence.

Same-character recovery may reconnect to the same actor when the incumbent transport is genuinely unavailable. A different character remains blocked while unresolved authoritative presence exists.

### Logical Game Session versus transport

A bounded reconnect after transient transport loss may preserve the same logical `GameSessionId` while establishing a newer transport/connection generation.

Older transport generations fail closed and delayed/stale packets cannot regain gameplay authority.

### Logical reconnect grace

Owner-accepted initial policy:

```text
reconnect_grace_window = 15 seconds
```

The same 15-second window applies in and out of combat/PZ/logout lock.

This is a tunable server policy, not an immutable wire constant. Expiry of the 15-second logical reconnect grace must not by itself create a safe logout, clear combat state or remove a character that still has mandatory world presence.

## Owner-accepted disconnect protection activation

The gameplay-visible PvE rule is:

```text
0.0 s <= elapsed < 2.0 s  -> normal PvE monster attacks
elapsed >= 2.0 s          -> disconnect protection active
```

Player-facing wording: the first two full seconds remain normal combat; **from the beginning of the third second** all PvE monsters stop actively attacking the disconnected/protected character.

The boundary is exactly `elapsed >= 2.0 s`, not 3.0 seconds elapsed.

The timer must be based on server-authoritative liveness/control evidence, never on a client claim that it is lagging or disconnected.

This gameplay boundary does not require a two-second main server loop. Networking, simulation and liveness processing may run at a finer cadence.

## Owner-accepted PvE behavior while disconnected

Once disconnect protection is active:

- all PvE monsters stop issuing new offensive actions against the protected actor;
- this includes monsters that already had the character targeted or engaged;
- preserved aggro/threat may remain internally so disconnect does not become an aggro reset;
- monsters and encounters do not freeze and may continue acting against other legal targets;
- actions/effects already authoritatively committed before the two-second boundary are not rolled back;
- existing DoT, fields, hazards, projectiles or already committed AoE may still resolve;
- disconnect protection is therefore not invulnerability.

Disconnect protection must not automatically:

- heal or refill HP/mana/resources;
- clear conditions, debuffs, cooldowns or exhaustion;
- clear combat/PZ/logout lock;
- erase aggro/combat attribution;
- teleport/reposition the character merely because connectivity was lost;
- reset an encounter or instance;
- roll back committed gameplay;
- duplicate/recreate the actor;
- admit a second character from the same account while unresolved authoritative presence remains.

## Five-second stale transport cleanup

Owner-accepted transport cleanup target:

```text
stale_transport_close = 5 seconds
```

If control/connectivity has not recovered after five seconds, the old concrete transport may be forcibly closed/kicked and remains fenced.

Critical distinction:

```text
5 s  = stale concrete transport cleanup
15 s = logical GameSessionId reconnect grace
```

The five-second transport cleanup does **not** end the character's disconnect protection, does not despawn the actor, and does not shorten the already accepted 15-second logical reconnect grace.

## Longer Internet outage, client crash or power loss

The design must protect genuine failures lasting longer than five seconds, including ISP/router failure, client crash, computer crash/reboot and power loss.

A longer outage may move the character into an authoritative held/suspended recovery state. Working names such as `DISCONNECTED_HELD` or `SUSPENDED_CHARACTER` are not final identifiers.

Accepted semantics:

- authoritative character state is preserved rather than reset;
- unresolved combat/world obligations are not escaped merely by staying offline;
- a different character from the same account remains blocked while authoritative presence/obligation is unresolved;
- an implementation need not keep the full live actor resident indefinitely if later durability contracts can suspend it safely;
- later recovery resolves the preserved state rather than creating a fresh safe-state character.

Exact post-15-second admission/recovery semantics and storage representation remain unresolved and belong to later FND-04/persistence/gameplay contracts.

## Owner-accepted four-second re-entry protection

After a valid return to playable control:

```text
reentry_pve_protection = 4 seconds
```

This represents approximately two gameplay turns for recovery and escape.

During the full four-second PvE re-entry window:

### Allowed

- movement and direction change;
- escape/repositioning through normal movement rules;
- ordinary self-healing;
- health potion use;
- mana/resource potion use required for recovery;
- all allowed recovery actions still pay their normal resource/item costs and obey normal cooldowns.

### Suspended

- basic/auto attack initiation;
- explicit attack-target selection that would start new offensive combat;
- offensive spells, runes, abilities or other actions later classified as offensive.

Offensive input attempted during the protection window must **not be queued or buffered** for automatic execution after protection expires.

The protection window itself does not automatically heal, refill, clear conditions, remove combat/PZ/logout obligations, reset aggro or reset encounter state.

When the four seconds expire, normal offensive authority and normal PvE targeting rules resume.

## Instanced boss / party encounter recovery

A disconnected participant must not freeze or indefinitely hold open a boss instance for the rest of the party.

If participant A disconnects:

```text
A receives disconnect protection
remaining party continues encounter
boss may die normally
encounter completes normally
```

The encounter result is authoritative and is not rolled back because A was offline.

The architecture must retain durable participant/encounter evidence sufficient to later determine:

- `EncounterId` and `CharacterId`;
- join/participation timing;
- disconnect timing;
- meaningful participation across roles, not damage only;
- encounter outcome;
- attempt/cooldown consumption;
- personal reward eligibility/entitlement when applicable.

Disconnect cannot be used to erase a consumed boss attempt/cooldown or duplicate a reward.

Personal rewards may later be materialized as durable exactly-once entitlements. Shared physical corpse/container loot is not duplicated for the absent player.

Exact participation/reward thresholds remain unresolved.

## Re-entry after an instance has ended

If the original instance still exists and the encounter contract permits resume, the same authoritative participant may resume according to that instance's recovery rules.

If the boss encounter has completed and the original `InstanceId` has been destroyed, the returning player must not be recreated inside the vanished boss room and the instance must not be resurrected.

Every such instance/content definition must provide deterministic recovery destinations conceptually equivalent to:

```text
entry_anchor
normal_exit_anchor
recovery_exit_anchor
```

When the old instance no longer exists, the character resumes at its predefined `recovery_exit_anchor`.

The recovery exit is content-defined in advance and cannot be chosen opportunistically after seeing the disconnect outcome. It may equal the normal exit/lobby when appropriate.

If the recovery exit is already a safe lobby/PZ, an extra four-second monster-protection window may be unnecessary there; exact composition remains later content/gameplay policy.

## Owner-accepted anti-abuse direction

The server cannot reliably distinguish a single genuine ISP/power/client failure from deliberate cable-pulling.

Therefore the project accepts **longitudinal telemetry and pattern analysis** as the primary anti-abuse evidence path rather than treating a single disconnect as proof.

Disconnect/re-entry telemetry should support analysis of at least:

- disconnect frequency and duration;
- combat/PZ state;
- HP/mana/resources at disconnect;
- recent incoming damage/risk;
- monster/boss/encounter context;
- repeat disconnects inside the same encounter/combat episode;
- use of four-second re-entry protection;
- repeated low-HP disconnect -> protected reconnect -> heal/escape cycles;
- regional/ISP/GameNode/platform outage correlation;
- comparison of combat disconnects with ordinary session disconnects.

One event is not proof of abuse.

Repeated evidence-backed suspicious behavior may feed a progressive enforcement path such as warning -> stronger warning/review -> temporary suspension/ban -> stronger sanction for continued proven abuse.

Consistent with ADR-0006, **Game Intelligence remains observational/investigative**. It does not autonomously ban players and does not mutate authoritative gameplay state. Exact enforcement authority, thresholds, review/appeal rules and temporary/permanent-ban progression require a separate policy contract.

Correlated mass infrastructure incidents must be distinguishable from individual suspicious patterns so genuine ISP/GameNode/platform failures do not falsely accumulate abuse evidence.

## Explicitly not accepted / still unresolved

Do not silently promote any of the following into implementation:

- exact heartbeat/liveness evidence and state-transition names;
- exact persistence/storage model for held/suspended characters;
- exact post-15-second logical-session recovery/admission semantics;
- PvP disconnect/re-entry behavior;
- exact classification of support spells, support items, class-specific defensive abilities and healing of other players during the four-second window;
- loot, container use, push, lever/switch, NPC and other non-combat interaction rules during re-entry protection;
- voluntary early cancellation of re-entry protection to regain offensive authority;
- exact boss/summon/environment/script-actor classification under PvE attack suppression;
- boss-mechanic commitment rules when a player disconnects during a targeted mechanic;
- exact encounter participation/reward thresholds;
- `Disconnect Protection Exhaustion` or a regenerating protection budget;
- Emergency Character Controller / server-side auto-heal or auto-play;
- exact warning/sanction thresholds, human review and appeal policy;
- mass-outage compensation policy;
- telemetry retention/privacy schema.

`Emergency Character Controller` remains **NOT ACCEPTED**. The current accepted model protects and preserves the actor but does not make the server play the character during disconnect.

## Required future consumers

This checkpoint and its canonical baseline documents are mandatory input to later:

- `FND-02` protocol fields/results where needed;
- `FND-03` runtime timers, command scheduling, stale-input and actor suspension mechanics;
- `FND-04` liveness, reconnect, transport cleanup, admission and lease state machine;
- gameplay combat/action classification;
- instance/encounter/content contracts;
- `DUR-02` / `DUR-03` for durable suspended state and reward entitlement;
- QA/E2E disconnect/reconnect/instance-destruction scenarios;
- Game Intelligence disconnect-abuse telemetry;
- later enforcement-policy architecture.

## Guardrails

- ARCHITECTURE / ANALYSIS ONLY until the owner explicitly authorizes implementation.
- Do not mix client and server ownership. Server-side authority decides liveness, fencing and protection state; the client cannot grant itself protection.
- Preserve one-authoritative-character and transport-generation fencing invariants.
- Do not convert disconnect into invulnerability, combat reset, free loot, duplicate reward, duplicate actor or safe-logout exploit.
- Work through remaining decisions one concrete owner question at a time.

## Current checkpoint

```yaml
status: owner_accepted_checkpoint_current
main_contains_pr_75: true
main_contains_pr_76: true
canonical_disconnect_baseline_merge: 4434f3f16ba5d313b31cb6959ca68fb934b0572a
canonical_reentry_action_policy_merge: b457fb70954f46df2f5f11f6b7a2b2e056b79b03
implementation_authorized: false
blocker: none
next_decision_candidates:
  - classify support/defensive/non-combat actions during the 4-second re-entry window
  - define exact server-authoritative liveness evidence for the 2-second activation boundary
  - define post-15-second held/suspended recovery/admission semantics
  - define PvP-specific disconnect policy
```

## Resume instruction for the next agent

Read the four canonical baseline documents listed in the task metadata and verify current `main` before making any new decision.

Do **not** restart the already accepted timing, re-entry, instance-recovery or telemetry discussions.

Continue only with an unresolved architecture decision and keep the work analysis-only unless the owner explicitly requests implementation.

## Lifecycle disposition (v3.10 audit)

Archived by the v3.10 Doc/Agent IA lifecycle audit because this architecture checkpoint declares no live owning GitHub Issue or PR, and exact task-ID issue search returned no live owner. The canonical architecture files named in the packet plus live GitHub/current main supersede this historical projection.
