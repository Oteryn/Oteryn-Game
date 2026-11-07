# Disconnect Re-entry PvE Protection Owner Decision

- Status: Owner-accepted architecture decision
- Date: 2026-08-08
- Decision owner: Oteryn project owner
- Coordination ID: `OTV2-NATIVE-FOUNDATION`
- Applies to: valid reconnect/re-entry after unexpected loss of playable control such as connectivity loss, client crash, power loss or equivalent failure
- Related baselines:
  - `LAG_DISCONNECT_PROTECTION_OWNER_BASELINE.md`
  - `LAG_DISCONNECT_REENTRY_ACTION_POLICY_OWNER_BASELINE.md`
  - `FND-ID-01_ACCOUNT_SINGLE_ONLINE_CHARACTER_OWNER_BASELINE.md`
  - `DISCONNECT_FORENSIC_EVIDENCE_OWNER_BASELINE.md`
  - `DISCONNECT_CLIENT_OS_FORENSICS_OWNER_DIRECTION.md`
- Does not authorize: runtime, protocol, persistence, client, Platform or production implementation

## Purpose

Freeze the owner decision for the player-visible PvE behavior immediately after a valid return to playable control and remove the remaining ambiguity between the reconnect anti-reset invariant and the accepted defensive re-entry window.

## Eligibility boundary

The protection window is for recovery from an **unexpected loss of playable control**. It is not a benefit attached to an ordinary voluntary logout/login cycle.

Binding direction:

- an accepted graceful logout does not create the four-second PvE re-entry protection window;
- a normal login after an accepted graceful logout is an ordinary admission/login flow, not protected re-entry;
- a client crash, abrupt process loss, connectivity loss, network-path loss, system crash, power interruption or equivalent unexpected loss may enter the reconnect/re-entry path according to the later authoritative `FND-04` state machine;
- protection eligibility must remain server-authoritative and must not depend synchronously on client/OS evidence proving the cause of the loss;
- suspicious abrupt exits or network-interface transitions may still receive the mechanically required protection when the authoritative reconnect rules say so, while the incident is preserved for later forensic and longitudinal abuse analysis.

This prevents voluntary logout/login from becoming a trivial way to manufacture the protection window while avoiding false denial of protection to a genuine outage whose cause cannot be proved in real time.

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
- the protected player may not heal another player;
- another player may heal the protected character under the healer's normal legality, resource, range, cooldown and exhaustion rules;
- the player may not initiate offensive combat against PvE monsters;
- prohibited outgoing input is not buffered for execution after the protection expires.

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

The protected character may heal itself and may receive healing from another player. The protected character may not target another player with a healing action while the four-second protection remains active.

A healing attempt by the protected character whose authoritative target is another player must be rejected or otherwise prevented and must not be buffered for execution after protection expires.

Incoming healing from another player is not blocked merely because the recipient has re-entry protection. The healing player's action remains subject to the ordinary authoritative rules for legality, resources, range, cooldown, exhaustion and any later combat/support restrictions.

This decision concerns healing direction only. It does not implicitly allow or prohibit non-healing buffs, cleanses, shields, resource transfers or other support effects; those remain for the owning combat/action contract.

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

The player may survive by spending the resources and consumables already legitimately available to the same authoritative character or by receiving legal healing from another player.

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

Abuse prevention therefore has three layers:

1. **eligibility boundary** — ordinary graceful logout/login does not create protected re-entry;
2. **immediate mechanical restriction** — no offensive PvE action and no healing of another player can be executed by the protected character during protection;
3. **longitudinal evidence** — repeated suspicious disconnect/re-entry patterns remain observable through the accepted Game Intelligence / disconnect-forensics architecture and may feed a separately governed human-reviewed enforcement policy.

The accepted client/OS forensic direction may add corroborating evidence for abrupt process loss, network-interface state changes, path loss, client/system crash, power interruption or similar classes, but server-generated evidence remains authoritative.

Game Intelligence remains observational/investigative and cannot autonomously ban players or mutate gameplay state.

## Required downstream consumers

This decision is mandatory input to:

- `FND-03` runtime timers, liveness transitions and command scheduling;
- `FND-04` reconnect/re-entry session state machine and graceful-logout versus unexpected-loss eligibility;
- combat/action classification;
- PvE monster AI targeting/attack eligibility;
- player-to-player healing eligibility during re-entry protection;
- QA/E2E disconnect and reconnect scenarios;
- disconnect-forensics and later security analytics;
- client presentation if a visible re-entry protection indicator is introduced later.

## Required future tests

Future implementation evidence must prove at minimum that:

1. a valid unexpected-loss re-entry starts exactly one four-second PvE defensive protection interval;
2. an accepted graceful logout followed by ordinary login does not create that protection interval;
3. PvE monsters cannot begin new offensive attacks against the protected character during that interval;
4. self-healing remains legal subject to normal cost/cooldown rules;
5. health and mana/resource potion use remains legal subject to normal item/cooldown rules;
6. movement remains legal under the previously accepted movement rule;
7. the protected character cannot heal another player while protection is active;
8. another player can legally heal the protected character subject to ordinary healer-side rules;
9. prohibited outgoing healing input is not buffered for execution after protection expires;
10. no offensive action against a PvE monster can execute while protection is active;
11. offensive input attempted during protection is not buffered and does not burst-execute at expiry;
12. already committed pre-protection effects are not rolled back;
13. protection expiry restores normal combat eligibility without resetting authoritative character state;
14. client/OS diagnostic evidence is not required synchronously for protection eligibility;
15. session-generation, one-character-per-account, item/economy and instance-recovery invariants remain intact.

## Deliberately unresolved

This decision does not yet decide:

- PvP re-entry behavior;
- exact client UI/countdown presentation;
- exact protocol error/result for temporarily prohibited offensive or outgoing-healing actions;
- non-healing support actions such as buffs, cleanses, shields or resource transfers;
- whether non-combat interactions such as loot, containers, switches or NPC interaction are permitted;
- whether the player may voluntarily cancel protection early;
- exact evidence thresholds or sanction thresholds for deliberate disconnect abuse;
- exact Windows APIs/providers/event IDs, Guardian process topology or Guardian heartbeat cadence.

Those subjects require their owning later contracts and must not be inferred from this decision.

## Canonical concise rule

```text
unexpected loss of playable control
-> valid same-actor reconnect/re-entry
-> 4 seconds PvE defensive protection
-> movement allowed
-> self-healing allowed
-> health/mana/resource potions allowed
-> protected player cannot heal other players
-> protected player can receive legal healing from other players
-> normal costs/cooldowns still apply
-> no offensive action against PvE monsters
-> prohibited outgoing actions are never buffered
-> no automatic heal/reset/teleport/state rollback
-> client/OS evidence is corroborating, not synchronous authority
-> after 4 seconds normal PvE combat resumes

accepted graceful logout/login
-> ordinary login/admission
-> no 4-second re-entry protection
```
