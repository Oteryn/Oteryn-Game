# PVP-0 PvP gap closure

- Decision: `PVP0-PVP-GAP-CLOSURE-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (combat,
  persistence, security and protocol) and protected integration. Owner questions Q1-Q4 (§10) are
  answered: control plane D971 accepts 1a 2a 3a 4a.
- Answers: coordination #1622, "mechanic missing on `main`: PvP rules". Inventories what PvP code
  exists on `main`, checks the accepted PvP contract against the reference evidence in this
  repository, closes five gaps the accepted contract leaves open, and fixes the slice plan.
- Governing contract: `OTERYN_GAME_PARTY_PVP0_PARTIES_AND_PVP_DECISION_2026-09-30.md`
  (`PARTYPVP0-PARTIES-AND-PVP-V1`, PARTY-PVP-0), **ACCEPTED** by
  `OTERYN_GAME_ACCEPT_SOCIAL_MAP0_ACCEPTANCE_DECISION_2026-10-04.md`. PVP-0 does not re-decide
  PARTY-PVP-0. Every rule, table, limit row (`PARTYPVP0-RL-*`) and wire element it names stays as
  written unless §6 amends it, and each §6 amendment is pending on acceptance of PVP-0.
- Builds on: GAME-VISION-01 PvP secondary-pillar baseline (owner-accepted); ATTACK-0; DEATH-0 and
  the first player death decision; WORLD-INTERACTION-0 §8.2 (fields); GAME-CHANNEL-01 §8 and §25;
  the multichannel scope matrix PvP rows; owner rule 5905825574 (Global parity).
- Runtime, migration, wire and production authority: NONE. Each slice in §8 needs its own
  allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

`combat/attack/target.rs` still says "first slice, no PvP". What does `main` already hold, what
is left between it and the accepted PvP contract, what does the reference evidence say about the
exact numbers, and in which slices is PvP delivered?

## 2. Current state on `main` (2026-10-10, `348b2b76`)

| Area | On `main` | PvP status |
|---|---|---|
| Attack target (ATTACK-1) | `combat/attack/target.rs`: `TargetFacts.attackable_kind` true only for a creature target of a player; PZ refusal on either side; 60 s in-fight deadline (`AttackConstants.in_fight_ms`); 4 s re-entry PvE protection | Character targets refused. Correct until the §8 go-live gate. |
| Death (DEATH-1, DEATH-2) | `durability/character_death.rs`: one fenced death receipt, D58 XP loss, blessing consumption, pending respawn; no item loss yet (DEATH-3), no Amulet of Loss selection | No PvP fields (`pvp_death`, `skull_at_death`, `twist_of_fate_used`, `adventurer_applied`, `unfair_fight_milli`). |
| Parties (PARTY-1) | migration `0046_world_parties.sql`, `durability/world_party.rs` | PvP rule 5 (party immunity) has its durable source. |
| Field World policy | migration `0048_spell_field_world_policy.sql`, `durability/spell_field_policy.rs`: control-published, immutable per `world_policy_revision`; `world_type` 1/2/3 = `NoPvp`/`Pvp`/`PvpEnforced`; `protection_level`; `in_fight_ms` | A second World PvP type source beside PARTY-PVP-0 §6.1 (gap G2). |
| Tile zone facts | `content/project/native_spell_tiles.rs`: `protection_zone` flag; source step facts `no_pvp_zone`, `pvp_zone` Read only by player-made fields: `gameplay_transport/ordinary_field_items.rs` selects the safe variant on a `no_pvp_zone` tile (unknown facts refuse); `gameplay_transport/field_step_ingress.rs` skips a player-made field hit on a `no_pvp_zone` tile and, on a `NoPvp` World, lets it hit a character when the field owner's and the target's current tiles are both `pvp_zone`. No attack or spell legality rule reads either (gaps G3, G4). |
| Ruleset tree | `rulesets/pvp/{skulls,wars,arena}/index.json` | `READY_UNPOPULATED`. No `pvp_type` row yet. |
| Client (CLIENT-COMBAT-INPUT-1) | attack target, fight modes (`secure` carried), spell input | No expert mode, skull, frame or party shield rendering. |
| PvP state, ledger, points, marks, blocks, wire | nothing | PVP-1, PVP-RT-1, PVP-DEATH-1, PVP-BLOCK-1, PVP-WIRE-1 unallocated; no open PR. |

## 3. Reference evidence

Evidence classes as in `docs/agents/programs/OTERYN_REFERENCE_INVESTIGATION_SOURCE_REGISTRY_20260910.md`.

**CIPSOFT_OFFICIAL** (`docs/reference/tibia-manual/combat.md` §5.3.7, §5.3.11, §5.3.12;
`characters.md` §5.1.11; `world.md`; capture 2026-09-28). Already summarised in PARTY-PVP-0 §2.
Points that matter for this delta:

- White skull "persists as long as the linked logout block is active"; killing a character sets a
  15-minute logout and PZ-entry block, extended again by violence in its last minute.
- Red and black thresholds are "rule of thumb" ranges (red 3-5 / 5-9 / 10-19, black 6+ / 10+ /
  20+ per 24 h / 7 d / 30 d); the manual's open question records that exact values are not given.
- Newhaven and Rookgaard: no PvP below level 8, and no PvP on Rookgaard at all, on every World
  type (`world.md`). Temples and depots are protection zones.

**OTS_HYPOTHESIS_ONLY** (`zimbadev/crystalserver` issue 810, "Open PvP system — retail 2014
rules", stored in `docs/reference/spells/r21-local-candidate/upstream-issue-research.json`,
entry 2, issue index 5; verified in game by its author on protocol 15.25):

| Fact | Value | Agrees with PARTY-PVP-0 |
|---|---|---|
| `dayKillsToRedSkull` / `weekKillsToRedSkull` / `monthKillsToRedSkull` | 3 / 5 / 10 | yes, `PARTYPVP0-RL-14` |
| black skull thresholds | 2 × red: 6 / 10 / 20 | yes, `PARTYPVP0-RL-15` |
| `redSkullDuration` / `blackSkullDuration` | 30 / 45 days (upstream had 1 / 3, fixed) | yes, `PARTYPVP0-RL-16` |
| `orangeSkullDuration` | 7 days, or until avenged | yes, `PARTYPVP0-RL-17` |
| `whiteSkullTime` | 15 minutes after a kill; about 1 minute after mere aggression | **no**: PARTY-PVP-0 §8.2 ties white to the 60 s PZ block only (gap G1) |
| "day" frag window | 24 h (upstream had 4 h, fixed) | yes, rolling 24 h |
| frag load window on login | 30 days (upstream 24 h lost bars and orange marks, fixed) | yes: durable point rows, 45-day retention (`PARTYPVP0-RL-23`) |
| frag share | 5 or fewer participants: 1.0 each; 6+: `5 / N` each | yes, `PARTYPVP0-RL-18` |
| PvP situation | mutual, 60 s (`pzLocked`), refreshed by each aggressive act, runtime only | yes, §8.2 aggression relations |
| a player-made field harms bystanders only in a PvP situation | per-viewer safe item variants (`ITEM_*_NOPVP`, `*_SAFE`) | **partly**: WORLD-INTERACTION-0 §8.2 runs a player field's hit through legality rules 1-5 and 7 with no PvP-situation gate (the manual's incidental skulls), so an eligible unmarked bystander on an `OPEN` World is hurt; the PvP-situation gate is not adopted. The safe variant follows that legality; viewer rendering unassigned (gap G5) |
| expert modes Dove / White Hand / Yellow Hand / Red Fist | wire byte 0-3 appended to fight modes; restricts who may be attacked and blocked | wire and blocking yes (§7.5, §11); attack restriction **no**: the manual (`interface.md` §3.5.2) makes each mode restrict attacks too, and PARTY-PVP-0 §7.2 has no such rule (gap G6) |
| a character not first in its tile stack cannot start PvP or cast area effects | `Player::isFirstInStack` | not in the manual; not adopted (§5) |
| PZ duration ("pzLocked") and in-fight | 60 s | yes, `PARTYPVP0-RL-12`, `ATTACK0-RL-03` |

The Canary `protectionLevel` convention (a character at or below the value is protected; default
7) gives PvP from level 8, matching `PARTYPVP0-RL-09`. The field World policy row stores
`protection_level` in that convention (§6.2).

## 4. Gaps

| Id | Gap | Closed by |
|---|---|---|
| G1 | An unjustified killer keeps a white skull for only 60 s after the kill; the manual and the OTS evidence keep it for the 15-minute kill block. | §6.1 amendment (Q1) |
| G2 | Two World PvP type sources: `rulesets/pvp/` `pvp_type` (PARTY-PVP-0 §6.1, not yet populated) and the published field policy `world_type` (migration 0048), plus two minimum-level and two in-fight values. They can disagree. | §6.2 amendment (Q2) |
| G3 | Map tiles carry `no_pvp_zone`, but no PvP legality rule reads it. A non-PZ no-PvP tile (regeneration and logout allowed, PvP refused) has no rule. | §6.3 amendment (Q3) |
| G4 | Map tiles carry `pvp_zone` (arena tiles), and arenas are deferred. No rule says what such a tile does until then, and `field_step_ingress.rs` already holds an arena-like exception: on a `NoPvp` World a player-made field hits a character when both stand on `pvp_zone` tiles. | §6.4 amendment (Q4) |
| G5 | Viewer-relative field and wall item variants (a bystander sees the harmless or walkable variant) have no slice, and nothing refreshes them when the legality between viewer and owner changes. | §6.5, PVP-BLOCK-1, PVP-WIRE-1 |
| G6 | Expert mode is carried on the wire and decides blocking, but PARTY-PVP-0 §7.2 never uses it to restrict attacks; a client could show Dove while the server lets it attack anyone secure mode allows. | §6.6 amendment |

Rookgaard ("no PvP on every World type") is G3 applied to map data: the map marks it `no_pvp_zone`.

## 5. Not adopted

- **First-in-stack rule** (OTS only, not in the manual): not adopted. Revisit only with
  CIPSOFT_OFFICIAL evidence.
- **Exact thresholds beyond the lower bounds, the assist share (`PARTYPVP0-RL-19`), the
  unfair-fight formula, PvP damage factor placement**: stay `PARITY_PENDING` as in PARTY-PVP-0.
  The OTS values in §3 corroborate the lower bounds; they are not a new source.
- **Retro World types, guild wars, arenas, Death Redemption**: stay deferred as PARTY-PVP-0
  §17 says.

## 6. Amendments to PARTY-PVP-0 (pending on acceptance of PVP-0)

### 6.1 White skull after an unjustified kill (G1, Q1 a, D971)

PARTY-PVP-0 §8.2 "White skull" gains: on an `OPEN` World, an unjustified kill (§9) sets each
unjustified contributor's `white_skull_until` to at least its `kill_block_until` in the victim's
death transaction. Aggression without a kill keeps the 60 s rule. A red or black skull still
shows instead of white. No new column: `white_skull_until` exists (§6.2) and is restored at
admission (§8.1). After that transaction commits, the channel runtime holding an online
contributor sets its live white-skull deadline to the committed `white_skull_until` before it
processes that contributor's next input, and VIS-2 publishes the skull; visibility, kill
justification (§9) and targeting (secure mode, rule 7, §6.6) read the extended deadline from then
on. A contributor offline at commit gets it at admission.

### 6.2 One World PvP type (G2, Q2 a, D971)

- `rulesets/pvp/` `pvp_type` stays the only authority for PvP legality, skulls and death.
- The published field World policy is a projection of it, checked, not a second authority:
  `NoPvp` = `OPTIONAL`, `Pvp` = `OPEN`, `PvpEnforced` = `HARDCORE`;
  `protection_level` = `PARTYPVP0-RL-09` − 1 (7); `in_fight_ms` = `ATTACK0-RL-03` (60,000).
- Admission readiness for a World refuses (`WORLD_POLICY_MISMATCH`, a readiness error, never a
  default) when the current field policy revision disagrees with the ruleset revision on any of
  the three. No missing row grants PvP (migration 0048 rule kept).
- PVP-1 writes the check; no migration of migration 0048's table.

### 6.3 No-PvP tiles (G3, Q3 a, D971)

PARTY-PVP-0 §7.2 gains rule **3b, no-PvP tile**: an offensive effect from a character (or its
summon) to a character is refused when either stands on a `no_pvp_zone` tile, on every World type.
A tile with unknown zone facts refuses (fail closed, as `ordinary_field_items.rs` already does).
A no-PvP tile does not stop regeneration or logout and sets no PZ semantics; PvE is unaffected.
Single target: `PVP_REFUSED {NO_PVP_TILE}`; an area effect skips the actor. Fields keep the
§8.2 safe variant on such a tile.

### 6.4 PvP-zone tiles until ARENA-0 (G4, Q4 a, D971)

A `pvp_zone` tile grants nothing until an arena decision: World rules apply on it unchanged.
PVP-RT-1 moves the player-made field hit in `field_step_ingress.rs` onto the legality stage
(§7.2, with the WORLD-INTERACTION-0 §8.2 field amendment) and removes its `NoPvp` both-tiles-`pvp_zone`
exception, so an `OPTIONAL` World has no PvP field damage outside a war. Arena death without loss,
arena-only PvP on Optional Worlds and the arena exit rules belong to ARENA-0
(`rulesets/pvp/arena/`).

### 6.5 Viewer-relative field and wall variants (G5)

PVP-BLOCK-1 also owns the viewer-relative item variant of a player-made field or wall: a viewer
for whom the field legality (§7.2 rules 1-5 and 7 with the WORLD-INTERACTION-0 §8.2 field rules,
plus rule 3b) refuses the owner's field hit receives the harmless or walkable variant; every
other viewer receives the real one. The server decides every step and every field application
independently of what the viewer was sent. The variant rides the existing item entry; no new
domain and no PVP-WIRE-1 element. When an input of that predicate changes for a viewer and an
owner (a skull or mark, an aggression relation (§8.2) starting, expiring or ending by logout,
party or guild membership, a war, the level threshold, the viewer's or owner's tile zone),
PVP-BLOCK-1 republishes every affected entry in that viewer's view in the next projection update,
as an item update on the same entry. A channel
restart clears both the relations (runtime only, §8.2) and player fields and walls
(WORLD-INTERACTION-0 volatile field overlay), so nothing is republished for it. A missed
republish never changes a server decision.

### 6.6 Expert mode restricts attacks (G6)

PARTY-PVP-0 §7.2 gains rule **6b, expert mode**, after secure mode, on `OPEN` and `OPTIONAL`
Worlds (`HARDCORE` is locked to Red Fist, which restricts nothing beyond rule 5). The actor's
current `expert_mode` (ATTACK-0 fight modes; default Dove) limits whom it may start or continue
to harm:

- **Dove:** only a character with an aggression relation (§8.2) toward the actor.
- **White Hand:** as Dove, or a character with an aggression relation toward a member of the
  actor's party or guild, confirmed as rule 5 confirms membership.
- **Yellow Hand:** as White Hand, or any character showing the actor a skull (white, yellow toward
  the actor, red, black, orange toward the actor).
- **Red Fist:** any character rules 1-7 allow. A black-skulled actor cannot select it
  (`REJECTED`); a black skull acquired while in Red Fist sets Dove.

Single target: `PVP_REFUSED {EXPERT_MODE}`; an area effect skips the actor. As with rule 6, rule 6b
does not filter a player-made field's damage to a character who enters or stands in it. The mode
is server state from the last accepted fight modes intent; a client-shown mode is never authority.
PVP-RT-1 enforces it; PVP-BLOCK-1 uses the same predicate for blocking.

## 7. Model summary (from PARTY-PVP-0, unchanged unless §6 says)

- **Authority:** one legality stage (GAME-ABILITY-01) for melee, spells, runes, fields and
  condition ticks; channel-local execution; a summon acts for its owner; no client fact is
  authority (target ids are D85 actor ids; never a CharacterId, PartyId or skull from a client).
- **Rules, in order:** World type (+ `war_between`), level ≥ 8, PZ, **no-PvP tile (§6.3)**, 10 s
  post-login immunity, party/guild (durable confirmation), secure mode, **expert mode (§6.6)**,
  black skull.
- **Durable state (Character + World, no `CharacterRevision` advance):**
  `game_character_pvp_state`, `game_character_pvp_ledger`, `game_character_unjustified_points`,
  `game_character_revenge_marks`. PvP deadlines are written ahead under the acting Character's
  session-generation fence (rule 2) before the action takes effect; a failed write refuses the
  action. Kill consequences commit in the victim's death transaction keyed by
  `PlayerDeathOccurrence`.
- **Frag decay:** none as a job; rolling 24 h / 7 d / 30 d windows over point rows; skulls end at
  `skull_until`, reset by new points; point rows deleted after 45 days.
- **Death interplay:** PvP death test (5% in 5 minutes or a PvP final blow), red/black full loss,
  Twist of Fate, Adventurer's Blessing (Open, level ≤ 20), black skull respawn 40 HP / 0 mana,
  unfair-fight 0 until sourced, no XP for a PvP kill.
- **Channel:** combat lock (logout, PZ, kill blocks) blocks a voluntary switch; a switch never
  clears a PvP consequence.
- **Wire:** capabilities `PARTY_V1`, `PVP_V1`; `PVP_INTENT {join_aggression}`; fight modes gain
  `expert_mode` (Dove 0, White Hand 1, Yellow Hand 2, Red Fist 3); domain `PVP`; VIS-2 skull
  (none, white, yellow, red, black, orange), shield, frame (none, yellow, orange, brown).
  Reservations stay on #162 at allocation.

## 8. Slice plan

Each slice is one PR in its own owned paths, test-first, and tests `OPTIONAL` and `OPEN` (and
the `HARDCORE` branch it touches). PvP goes live on a World only when PVP-1, PVP-RT-1,
PVP-DEATH-1, PVP-WIRE-1 and PVP-CLIENT-1 have merged (without the client's expert mode control a
character stays in the default Dove mode and cannot start aggression, §6.6); until then `attackable_kind` stays creature-only. The
first World is `OPTIONAL`, so it shows no PvP until GUILD-WAR-0.

| Slice | Worker / review | Builds | Depends on (state on `main`) |
|---|---|---|---|
| PVP-1 | hard; persistence, security | four PvP tables and the migration; skull evaluation and commit in the death transaction; World cleanup job; `rulesets/pvp/skulls` rows and `pvp_type` (`OPTIONAL` for the first World); the §6.2 field policy check | DEATH-1 (landed), PARTY-1 (landed) |
| PVP-RT-1 | hard (combat); combat, security | legality rules 1-7 with 3b (§6.3) and 6b (§6.6); player-made field hits moved onto the legality stage and the `NoPvp` `pvp_zone` field exception removed (§6.4); aggression relations; white (§6.1) and yellow skulls; logout, PZ and kill blocks with durable write-ahead and restore; damage factor; PvP damage ledger and snapshot; kill classification; Join Aggression; friendly fire; Adventurer forfeiture write | PVP-1; ATTACK-1 (landed); COND-1 |
| PVP-DEATH-1 | hard (persistence); persistence | PvP death test; receipt fields; red/black loss; Twist of Fate; Adventurer's Blessing; black skull respawn | PVP-1; DEATH-3 (not landed) |
| PVP-WIRE-1 | impl; protocol | `PVP_V1`; `PVP_INTENT`; `expert_mode`; domain `PVP`; VIS-2 skull and frame fields; `PVP_REFUSED` reasons incl. `NO_PVP_TILE`, `EXPERT_MODE` | PVP-RT-1; VIS-2; ATTACK-WIRE-1 |
| PVP-BLOCK-1 | impl; movement | walk-through and expert-mode blocking; viewer-relative field and wall variants and their republish when the field legality changes (§6.5) | PVP-RT-1; SPEED-1 |
| PVP-CLIENT-1 | impl; client | expert mode control, skull, frame and shield rendering, PvP refusal text | PVP-WIRE-1 |

PVP-1 and PVP-RT-1 split cleanly at the tables (PVP-1 writes them only from the death
transaction; PVP-RT-1 adds the runtime write-ahead path). Allocation order: PVP-1, then PVP-RT-1
and PVP-DEATH-1 (when DEATH-3 lands) in parallel, then PVP-WIRE-1, PVP-BLOCK-1, PVP-CLIENT-1.

## 9. Conformance cases

Each is a focused test in the named slice; "O" = `OPEN`, "P" = `OPTIONAL`, "H" = `HARDCORE`.

| Case | Slice | Given → expected |
|---|---|---|
| PVP-CC-01 | RT-1 | O, both level 8, open field → hit lands at 50% (black-skulled target 100%) |
| PVP-CC-02 | RT-1 | O, attacker level 7 (or target level 7) → `PVP_REFUSED`; area skips |
| PVP-CC-03 | RT-1 | either on a PZ tile → refused; PZ-blocked character cannot step onto a PZ tile |
| PVP-CC-04 | RT-1 | either on a `no_pvp_zone` tile, every World type → `PVP_REFUSED {NO_PVP_TILE}`; PvE on the same tile works; unknown zone facts → refused |
| PVP-CC-05 | RT-1 | `pvp_zone` tile → behaves as an ordinary tile (§6.4); P, no war, field owner and target both on `pvp_zone` tiles, target steps into the owner's field → no damage |
| PVP-CC-06 | RT-1 | P, no war → every character target refused; H → allowed with no skull |
| PVP-CC-07 | RT-1 | admitted 9 s ago starts aggression → refused; answers an aggressor → allowed |
| PVP-CC-08 | RT-1 | same party, no shared enemy → refused; both aggressive to one enemy → area hits ally, no skull, no block, no points |
| PVP-CC-09 | RT-1 | secure mode on, unmarked target → refused; marked target → allowed |
| PVP-CC-10 | RT-1 | black skull attacker, unmarked target → refused |
| PVP-CC-11 | RT-1 | O, hit on unmarked → attacker white for 60 s, PZ block 60 s; refreshed by the next hit; a marked character that attacks a viewer first shows yellow to that viewer only |
| PVP-CC-12 | RT-1, 1 | O, unjustified kill → killer kill block 15 min and white skull 15 min (§6.1): an online killer still shows white, is still a justified target and still passes secure mode for others 2 min after its last hit; also across logout and node restart |
| PVP-CC-13 | 1 | 3,000 milli in 24 h → red, `skull_until` +30 d; 6,000 → black +45 d; a new point resets the end; a skull never drops by evaluation |
| PVP-CC-14 | 1 | 7 unjustified damage contributors → each 714 milli (`1,000 × 5 / 7`, rounded down); assist-only → 500 |
| PVP-CC-15 | 1 | victim with white/red/black, or yellow toward the killer, or orange toward the killer → justified, no points |
| PVP-CC-16 | 1 | unjustified kill → orange mark 7 d; the victim kills the killer → oldest open mark avenged |
| PVP-CC-17 | 1 | point rows older than 30 d leave every window; deleted after 45 d; skull unaffected until `skull_until` |
| PVP-CC-18 | RT-1 | node crash mid-fight → blocks and white skull restored no shorter than before, at most 10 s longer; no ledger contributor lost |
| PVP-CC-19 | RT-1 | a channel switch under any block → refused; after a switch PvP state is unchanged |
| PVP-CC-20 | RT-1 | stale session generation on a PvP deadline write → write refused, action refused, no hit |
| PVP-CC-21 | DEATH-1 | 4% PvP damage in 5 min, final blow by a monster → PvE death; 5% → PvP death; PvP final blow alone → PvP death |
| PVP-CC-22 | DEATH-1 | red/black at death → all equipment and backpack lost, regular blessings consumed, Amulet of Loss ignored, Twist of Fate neither used nor consumed; level ≤ 8 → no loss |
| PVP-CC-23 | DEATH-1 | PvP death with Twist of Fate and regular blessings → regular kept, Twist consumed |
| PVP-CC-24 | DEATH-1 | O, level 20, not forfeited → no XP, item or blessing loss; first aggression → forfeited durably before the hit |
| PVP-CC-25 | DEATH-1 | black skull respawn → 40 HP, 0 mana; a PvP kill grants no XP |
| PVP-CC-26 | DEATH-1, 1 | replayed death command (same `PlayerDeathOccurrence`) → one receipt, points and marks written once |
| PVP-CC-27 | 1 | field policy `world_type` or `protection_level` or `in_fight_ms` disagrees with the ruleset → World not ready (`WORLD_POLICY_MISMATCH`) |
| PVP-CC-28 | BLOCK-1 | P, no war: bystander receives the safe field variant and stepping onto it causes no damage; O, eligible unmarked bystander: real variant, the step hurts and the owner gains a white skull; a spoofed step into a real wall is refused |
| PVP-CC-29 | WIRE-1 | client-supplied skull, CharacterId or PartyId never changes a server decision; unknown `expert_mode` value → `REJECTED` |
| PVP-CC-30 | RT-1 | O, secure off: Dove, unmarked non-aggressor → `PVP_REFUSED {EXPERT_MODE}`, aggressor → allowed; White Hand, character aggressive to a party member → allowed; Yellow Hand, white-skulled stranger → allowed, skulled party member → refused (rule 5); Red Fist → any; area effect skips a filtered actor; field damage not filtered |
| PVP-CC-31 | RT-1, WIRE-1 | black-skulled actor selects Red Fist → `REJECTED`; actor in Red Fist gains a black skull → mode Dove; H → mode fixed at Red Fist |
| PVP-CC-32 | BLOCK-1 | O, viewer in the owner's party sees the safe variant of the owner's field; the viewer leaves the party → that entry republished as the real variant in the next update; the viewer moves onto a `no_pvp_zone` tile → safe variant again |

## 10. Owner questions (answered: D971, 1a 2a 3a 4a)

Each recommended option is adopted; §6.1-§6.4 are written to it.

1. **White skull after an unjustified kill (G1).** a) It lasts the 15-minute kill block, as the
   manual and the OTS evidence say (recommended); b) keep PARTY-PVP-0's 60 s.
2. **One World PvP type (G2).** a) `rulesets/pvp/` `pvp_type` is the authority; the field policy
   must match it or the World is not ready (recommended); b) retire the field policy's
   `world_type`, `protection_level` and `in_fight_ms` columns by migration and read the ruleset.
3. **No-PvP tiles (G3).** a) Enforce `no_pvp_zone` in PVP-RT-1 on every World type, failing closed
   on unknown tiles (recommended); b) defer with arenas, leaving Rookgaard-type areas to PZ flags.
4. **PvP-zone tiles before ARENA-0 (G4).** a) Grant nothing; ordinary World rules, and PVP-RT-1
   removes the existing `NoPvp` both-tiles-`pvp_zone` field exception (recommended); b) keep that
   field-only exception until ARENA-0; c) refuse to load a map with `pvp_zone` tiles until ARENA-0.

## 11. Decision test

- **Must decide now:** YES. PvP is missing on `main`; the accepted contract leaves G1-G6 open,
  and G2 is already live code that the first PvP slice would otherwise contradict.
- **Minimum sufficient:** no new table, column, capability or slice beyond PVP-CLIENT-1; five
  rule amendments (§6.1-§6.4, §6.6), one ownership assignment and one refresh rule (§6.5).
- **Superseding evidence:** CIPSOFT_OFFICIAL values for the thresholds, white-skull duration or
  assist share; an accepted arena decision.

## 12. Before-freeze checklist

1. PARTY-PVP-0 text unchanged; every amendment here is "pending on acceptance of PVP-0".
2. Every value has a cited source and class; OTS values are corroboration only.
3. Slice dependencies match `main` at `348b2b76`.
4. No runtime, migration, wire or content change.
