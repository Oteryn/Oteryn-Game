# Lag / Disconnect Re-entry Action Policy Owner Baseline

- Status: Owner-accepted pre-contract baseline
- Date: 2026-08-07
- Decision owner: Oteryn project owner
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Parent baseline: `docs/architecture/LAG_DISCONNECT_PROTECTION_OWNER_BASELINE.md`
- Applies to: the accepted 4-second PvE re-entry protection window after valid recovery from Internet loss, client crash, power loss or equivalent connectivity failure

## Purpose

Refine the already accepted four-second PvE re-entry protection window so a returning player can actively survive and escape instead of being forced to remain passive, while preventing the protection window from becoming a free offensive burst.

This document is architecture only. It does not authorize runtime implementation and does not yet freeze final protocol fields, input opcodes, combat-action taxonomy or UI presentation.

## Owner-accepted re-entry principle

The four-second re-entry window is a **defensive recovery window**, not a passive observation-only window.

During the full protection window, the returning player may:

- move and change direction;
- attempt to escape the dangerous area;
- use ordinary self-healing actions;
- use health potions;
- use mana/resource potions required for recovery;
- otherwise regain survivability through normal non-offensive recovery actions that later gameplay classification explicitly places in the allowed defensive set.

PvE monsters remain unable to begin new offensive actions against the protected character for the configured four-second window, subject to the existing parent-baseline rules for effects already committed before protection became active.

## Offensive-action suspension

While re-entry protection remains active, the returning player must not be able to initiate new offensive combat against creatures or other gameplay targets.

At minimum, the offensive suspension covers:

- basic/auto attack initiation;
- explicit attack-target selection that would start offensive combat;
- offensive abilities/spells/runes/actions once the later combat taxonomy classifies them as offensive.

Offensive input received during the protection window must **not** be buffered for delayed burst execution when protection expires.

Conceptually:

```text
valid reconnect / re-entry
        |
        v
4 s PvE re-entry protection
        |
        +--> movement / escape          ALLOWED
        +--> self-healing               ALLOWED
        +--> health/mana potions        ALLOWED
        +--> new offensive actions      SUSPENDED
        |
        v
protection expires
        |
        v
normal combat authority resumes
```

The later runtime contract may reject, ignore or return a stable temporary-action-not-allowed result for offensive commands during the protected interval, but it must not queue them for automatic execution after the interval.

## Why healing remains allowed

A genuine reconnect may return the player to a dangerous position with reduced HP, depleted mana/resources, ongoing conditions and no opportunity to react during the connectivity loss.

The accepted product goal is therefore to give the player approximately two gameplay turns not merely to observe the screen, but to **recover and escape**.

The protection window itself still does not automatically:

- heal the character;
- refill resources;
- clear conditions or cooldowns;
- remove combat/PZ/logout obligations;
- reset aggro, encounter state or position.

Any healing or potion use is an ordinary player-authorized gameplay action using the character's existing resources, cooldowns and item consumption rules.

## Anti-abuse boundary

Allowing movement and recovery actions makes the re-entry window intentionally strong protection for genuine outages. The architecture therefore relies on the already accepted longitudinal disconnect telemetry and Game Intelligence evidence path to detect repeated deliberate exploitation.

The server must record sufficient evidence to distinguish patterns such as repeated low-HP disconnect -> protected reconnect -> heal/escape cycles from ordinary isolated outages.

A single use is not proof of abuse. Repeated evidence-backed patterns may feed the separately defined warning/sanction policy. Game Intelligence remains observational/investigative and does not autonomously ban or mutate gameplay.

The action policy itself still prevents one immediate abuse class: a player cannot use the protected four seconds to attack freely while monsters are forbidden from responding.

## State and durability invariants

All parent-baseline invariants remain binding:

- the returning character is the same authoritative character state;
- re-entry does not respawn, heal, refill or reset the character automatically;
- stale transport generations remain fenced;
- old delayed offensive commands do not regain authority;
- combat/PZ/logout and encounter obligations are not cleared by reconnect;
- account-global single-authoritative-character rules remain in force;
- destroyed instances still recover through their deterministic `recovery_exit_anchor` rather than being resurrected.

## Explicitly unresolved

This baseline does not yet decide:

- the exact classification of every defensive spell, support spell, rune, item or class-specific ability during re-entry;
- whether healing/support actions targeting another player are allowed or only self-recovery is allowed;
- whether non-combat interaction such as loot, container use, push, switch/lever use or NPC interaction is allowed during the four seconds;
- whether the client receives a dedicated visual indicator/countdown;
- the exact stable server/protocol error for suspended offensive commands;
- whether a player may voluntarily cancel re-entry protection early to regain offensive authority;
- PvP-specific re-entry behavior.

These require later gameplay/protocol decisions and must not be inferred by implementation.

## Required consumers

This baseline is mandatory input to later:

- gameplay combat/action classification;
- `FND-02` if the protocol needs a stable temporary-action-suspended result or explicit protection-state field;
- `FND-03` runtime command scheduling and stale-input policy;
- `FND-04` reconnect/re-entry state machine;
- QA/E2E recovery scenarios;
- Game Intelligence disconnect-abuse telemetry and later enforcement-policy design.

Tests must eventually prove at minimum that:

1. valid re-entry starts the accepted four-second PvE protection window;
2. the player can move during that window;
3. ordinary self-healing and health/mana potion use remain available subject to normal gameplay cooldown/resource/item rules;
4. PvE monsters cannot begin new attacks against the protected player during that window;
5. new offensive attacks/actions are not executable while the window is active;
6. offensive input attempted during the window is not queued and does not burst-execute when the window expires;
7. protection expiry restores normal offensive authority without resetting character/combat/encounter state;
8. all reconnect/session fencing and anti-duplication invariants remain intact.

## Programme effect

Accepted now:

```text
4 s re-entry protection = defensive recovery window
movement / escape = allowed
self-healing = allowed
health/mana potion use = allowed
new offensive combat = suspended
suspended offensive input = never queued for post-protection burst
normal resource costs/cooldowns/item consumption still apply
anti-abuse patterns remain observable through Game Intelligence telemetry
```

No runtime, protocol, persistence, database, client or Platform implementation is authorized by this baseline.
