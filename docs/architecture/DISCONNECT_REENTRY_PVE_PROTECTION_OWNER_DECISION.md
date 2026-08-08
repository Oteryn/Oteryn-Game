# Disconnect Re-entry PvE Protection Owner Decision

- Status: Owner-accepted architecture decision
- Date: 2026-08-08
- Decision owner: Oteryn project owner
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: valid reconnect/re-entry after connectivity loss, client crash, power loss or equivalent loss of playable control
- Related baselines:
  - `LAG_DISCONNECT_PROTECTION_OWNER_BASELINE.md`
  - `LAG_DISCONNECT_REENTRY_ACTION_POLICY_OWNER_BASELINE.md`
  - `FND-ID-01_ACCOUNT_SINGLE_ONLINE_CHARACTER_OWNER_BASELINE.md`
- Does not authorize: runtime, protocol, persistence, client, Platform or production implementation

## Purpose

Freeze the owner decision for the player-visible PvE behavior immediately after a valid return to playable control and remove the remaining ambiguity between the reconnect anti-reset invariant and the previously accepted defensive re-entry window.

## Accepted rule

After a valid re-entry to the same authoritative character state, the character receives exactly:

```text
reentry_pve_protection = 4 seconds
```

The four-second interval is a defensive recovery window. It is not a respawn, combat reset, new character state or general invulnerability grant.

During the full four-second window:

- PvE monsters may not begin new offensive attacks against the protected character;
- monsters that were already targeting the character may not issue new offensive attacks against that character while protection remains active;
- the player may move and attempt to leave the dangerous area under the previously accepted movement rule;
- the player may use ordinary self-healing actions;
- the player may consume health potions;
- the player may consume mana/resource potions required for recovery;
- all allowed healing and potion actions still consume their normal resources/items and obey the normal cooldown, exhaustion and legality rules;
- the player may not initiate offensive combat against PvE monsters;
- offensive input is not buffered for execution after the protection expires.

At the end of the four-second interval, normal PvE attack eligibility resumes according to the ordinary authoritative combat rules.

## Offensive-action prohibition

While re-entry protection is active, the server must reject or otherwise prevent any new player action whose authoritative gameplay effect is offensive against a PvE monster.

This includes at minimum:

- basic or auto attack initiation;
- selecting or changing to an attack target when that selection would begin offensive combat;
- offensive spells;
- offensive runes;
- offensive abilities or class actions;
- damaging area-of-effect actions against monsters;
- offensive item-triggered actions against monsters;
- any later action category whose accepted combat contract classifies the resulting authoritative effect as offensive against a PvE creature.

The later combat/action taxonomy may refine exact categories, but it may not permit an offensive action merely by representing it through another input type.

An attempted offensive command during the protected interval must never be queued for automatic execution when protection ends.

## Healing and potion behavior

Healing and potion use remain normal authoritative gameplay actions rather than free restoration granted by reconnect.

The protection window itself does not:

- restore HP;
- restore mana or another resource;
- generate potion charges/items;
- clear debuffs or damage-over-time;
- reset cooldowns or exhaustion;
- clear combat/PZ/logout state;
- reset threat/aggro history;
- move or teleport the character automatically;
- restore encounter state;
- rewind committed damage or other authoritative effects.

The player may survive by spending the resources and consumables already legitimately available to the same authoritative character.

## Previously committed effects

Protection is prospective, not a rollback boundary.

An action or effect authoritatively committed before the protection boundary may resolve according to the owning combat contract. Examples include already committed projectiles, damage-over-time, environmental hazards or an already committed area effect.

No committed gameplay history is rewritten solely because re-entry protection became active.

## Explicit supersession and conflict resolution

`FND-ID-01_ACCOUNT_SINGLE_ONLINE_CHARACTER_OWNER_BASELINE.md` contains older generic wording that same-character reconnect must not "protect" the actor or create an "invulnerability window".

This decision **supersedes only that generic no-protection/no-invulnerability wording for the exact owner-approved four-second PvE defensive re-entry interval defined here**.

All underlying anti-reset and anti-duplication invariants from that baseline remain binding:

- the reconnecting player controls the same authoritative character actor;
- no second character copy or second gameplay authority is created;
- HP/resources/position/conditions/cooldowns/combat obligations are not reset automatically;
- stale transport generations remain fenced;
- reconnect does not clear combat/PZ/logout locks;
- reconnect does not resurrect destroyed instances or duplicate rewards/items;
- one AccountId still has at most one authoritative online CharacterId.

This decision does not supersede the rule that a healthy combat-locked incumbent cannot be forcibly disconnected by a second client login.

## Anti-abuse posture

The owner accepts the four-second defensive window despite the fact that an individual connectivity loss cannot always be proven to be accidental from one event alone.

Abuse prevention therefore has two layers:

1. immediate mechanical restriction: no offensive PvE action can be executed during protection;
2. longitudinal evidence: repeated suspicious disconnect/re-entry patterns remain observable through the accepted Game Intelligence / disconnect-forensics architecture and may feed a separately governed human-reviewed enforcement policy.

Game Intelligence remains observational/investigative and cannot autonomously ban players or mutate gameplay state.

## Required downstream consumers

This decision is mandatory input to:

- `FND-03` runtime timers, liveness transitions and command scheduling;
- `FND-04` reconnect/re-entry session state machine;
- combat/action classification;
- PvE monster AI targeting/attack eligibility;
- QA/E2E disconnect and reconnect scenarios;
- disconnect-forensics and later security analytics;
- client presentation if a visible re-entry protection indicator is introduced later.

## Required future tests

Future implementation evidence must prove at minimum that:

1. valid re-entry starts exactly one four-second PvE defensive protection interval;
2. PvE monsters cannot begin new offensive attacks against the protected character during that interval;
3. self-healing remains legal subject to normal cost/cooldown rules;
4. health and mana/resource potion use remains legal subject to normal item/cooldown rules;
5. movement remains legal under the previously accepted movement rule;
6. no offensive action against a PvE monster can execute while protection is active;
7. offensive input attempted during protection is not buffered and does not burst-execute at expiry;
8. already committed pre-protection effects are not rolled back;
9. protection expiry restores normal combat eligibility without resetting authoritative character state;
10. session-generation, one-character-per-account, item/economy and instance-recovery invariants remain intact.

## Deliberately unresolved

This decision does not yet decide:

- PvP re-entry behavior;
- exact client UI/countdown presentation;
- exact protocol error/result for temporarily prohibited offensive actions;
- whether support/healing actions targeting other players are permitted during the four seconds;
- whether non-combat interactions such as loot, containers, switches or NPC interaction are permitted;
- whether the player may voluntarily cancel protection early;
- sanction thresholds for deliberate disconnect abuse.

Those subjects require their owning later contracts and must not be inferred from this decision.

## Canonical concise rule

```text
valid re-entry
-> 4 seconds PvE defensive protection
-> movement allowed
-> self-healing allowed
-> health/mana/resource potions allowed
-> normal costs/cooldowns still apply
-> no offensive action against PvE monsters
-> offensive input is never buffered
-> no automatic heal/reset/teleport/state rollback
-> after 4 seconds normal PvE combat resumes
```
