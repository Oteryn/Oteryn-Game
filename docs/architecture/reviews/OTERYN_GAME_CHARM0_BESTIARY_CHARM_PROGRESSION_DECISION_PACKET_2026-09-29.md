# Oteryn Game — CHARM-0 Bestiary and Charm progression decision packet

- Date: 2026-09-29
- Issue: `#162`
- Task: `OTV2-20260929-charm0-decision-packet`
- Mode: `CONTRACT / PREPARATION`
- Runtime, protocol, DDL, content and production authority: **NONE**. This packet proposes; only owner decisions make it binding.

## 1. Purpose

Charms need two kinds of data:

- a static catalogue: the candidate package `tools/content-schema/charm-authoring/` (#1293);
- per-creature Bestiary facts: already on Creature definitions as `authoring.profile.bestiary`.

To make Charms playable, the following must also exist, and none of it does today:

- Bestiary kill progress;
- Charm Points and Minor Charm Echoes;
- Charm unlocks and creature assignments;
- Charm effects in combat.

`DUR-02` §4.6 names Charms as a typed Character-owned profile extension. Its physical contract must be accepted
before implementation. This packet lays out that contract and the decisions it needs, so the work can be split into
an ordinary content slice and a governed Character-state slice.

## 2. Current state

- `PROVEN`: the static Charm catalogue candidate has 25 charms (14 major, 11 minor) and validates (#1293).
  - TibiaWiki and Canary agree on category, stage cost and stage value for all 25.
  - 5 element and percent conflicts are open (#1293 `samples/charm-source-comparison.json`).
- `PROVEN`: 686 Creature definitions carry `bestiary {difficulty, occurrence, kill_thresholds, charm_points}` from
  Canary `47dfd51f`.
  - The client staticdata (`imports/cipsoft-staticdata/creatures/`, #1288) lists 833 Bestiary races.
  - Its difficulty and occurrence tiers agree with TibiaWiki for all 833.
  - 12 Oteryn records differ from that source and 10 carry a Bestiary block for a non-Bestiary creature.
  - The creature lane owns the correction (handed over in session, 2026-09-29).
- `PROVEN`: `combat/death_reward.rs` composes one committed creature death into independent reward descendants: the
  corpse and loot MINT, and one XP award through `durability::character_progression`.
  - This slice has a single reward principal (`COMBAT01-REWARD-PRINCIPALS`).
  - The XP award is fenced by `CurrentCharacterGameplayFence`.
- `PROVEN`: no Bestiary, Charm, Cyclopedia or Charm-proc code, table or protocol message exists.
- `UNKNOWN`: whether the Canary charm id (0–24) is the client protocol id.

## 3. Proposed split

| Slice | Content | Governance |
|---|---|---|
| CHARM-1 content | Populate `content/charms/` from the candidate catalogue; mint `oteryn:charm.<name>` keys (owner decision, 2026-09-29: keys are minted at population) | Ordinary content lane |
| CHARM-2 Bestiary progress | Per-character, per-race kill counter written as a death-reward descendant | DUR-02 §4.6 extension, session-generation fence, independent review |
| CHARM-3 Charm state | Charm stage unlocks, creature assignments, derived Charm Points and Echoes | DUR-02 §4.6 extension, independent review |
| CHARM-4 effects | Charm procs in combat for assigned creatures | Combat contract amendment, determinism |
| CHARM-5 client | Cyclopedia Bestiary and Charm views, unlock and assign commands | `protocol-oteryn` change, protocol review |

Each slice is playable-first on its own: CHARM-2 alone makes Bestiary progress visible once CHARM-5 has a view.

## 4. Character-state proposal (DUR-02 §4.6 declarations)

### 4.1 Bestiary progress (CHARM-2)

- **Semantic owner:** Character Authority. The rules live in `rulesets/progression/bestiary/`.
- **Relation:** `(CharacterId, bestiary race key) -> kill_count`.
  - The race key is the Creature definition key of the race's Bestiary entry.
  - Unlocked stages are derived from `kill_count` and the definition's `kill_thresholds`. They are never stored.
- **Write path:** one more independent descendant of a committed creature death, next to the XP award. It has the
  same `CurrentCharacterGameplayFence`, memoized per (death, character) occurrence, idempotent on replay, and never
  blocks or rolls back XP or loot.
- **Credit rule (Tibia):** the character damaged the creature within the last 5 minutes before its death. The
  current slice has a single reward principal. Multi-principal credit waits for the multi-principal reward contract.
- **Bound:** counters saturate at the final threshold. Nothing is gained by counting further, and it keeps the value
  bounded.
- **CharacterRevision, migration and retention:** follow XP. A definition's thresholds change only by a new
  definition revision. The stored count is kept, and stages are re-derived.

### 4.2 Charm state (CHARM-3)

- **Stored:**
  - `(CharacterId, charm key) -> unlocked_stage (1..3)`;
  - `(CharacterId, charm key) -> assigned bestiary race key`, at most one race per charm.
  - How many charms one race may hold after the 14.10 rework (one per race, or one major plus one minor) is
    `UNKNOWN` from the captured sources and must be verified before CHARM-3.
- **Derived, never stored:**
  - Charm Points earned is the sum of `charm_points` over completed entries.
  - Echoes earned is 50/100/200 per major stage unlocked, plus 100 for a promoted character (TibiaWiki, *Minor Charms*).
  - Spent is the sum of the unlocked stage costs.
  - Available is earned minus spent.
  - Storing only unlocks and assignments removes a whole class of balance drift and double-spend bugs.
- **Assignment rules (Tibia):**
  - A major charm needs a completed entry (final stage); a minor charm needs stage 2.
  - Removing an assignment costs gold (level × 100). That is a cross-domain Character and Item write, which `DUR-02`
    §7 forbids until `GAME-ITEM-01`/`DUR-03` prove the boundary.
- **Commands:** unlock the next stage, assign, unassign. Each is one fenced Character transaction validated against
  the derived balance.

### 4.3 Effects (CHARM-4)

- A charm applies only against the creature race it is assigned to.
- Trigger rolls use the simulation-determinism RNG, so replay reproduces them.
- Damage procs follow the catalogue's `effect` shape (e.g. 5% of the creature's maximum health, capped at 2× level).
- The candidate catalogue has three effect groups:
  - the attack-damage effects (7 elemental, plus Overpower and Overflux) and Carnage;
  - control effects;
  - economy effects: Gut, Scavenge and Bless, which touch loot, skinning and death penalty.
- Each group depends on a different runtime system.

## 5. Owner decisions requested

1. **Scope order.**
   - a) CHARM-1 now, then CHARM-2 → CHARM-3 → CHARM-5 → CHARM-4 as one playable line.
   - b) CHARM-1 only for now.
   - c) All slices in parallel.
   - Recommendation: **a**. Each step is visible in play, and the governed slices are reviewed one at a time.
2. **Economy storage.**
   - a) Store only unlocks and assignments; derive Points and Echoes (§4.2).
   - b) Also store balances.
   - Recommendation: **a**.
3. **Unassign fee.**
   - a) The first slice has no unassign command; assignments are replaced only by a free re-assign after reset.
   - b) Free unassign in the first slice, with the gold fee added after the `DUR-03` cross-domain boundary.
   - c) Wait for `DUR-03` and ship the fee from the start.
   - Recommendation: **b**. It is playable now, and the deviation is listed and reversible.
4. **First effect group (CHARM-4).**
   - a) The 7 elemental attack procs plus Overpower and Overflux only.
   - b) All offensive charms.
   - c) All 25.
   - Recommendation: **a**. They share one shape and one combat hook.
5. **Source conflicts in the catalogue.**
   - a) Follow TibiaWiki (Carnage, Overpower and Overflux are physical; Bless reduces loss by 6/9/12% with no extra
     10%; Parry reflects as neutral displayed as physical).
   - b) Follow Canary.
   - Recommendation: **a**. The client staticdata confirms the wiki everywhere it can be checked.
6. **Credit rule.**
   - a) Implement the 5-minute damage window for the single reward principal now.
   - b) Credit the killer only until multi-principal rewards exist.
   - Recommendation: **a**. It is the Tibia rule and fits the current single-principal slice.

## 6. Excluded

- Charm reset, Charm Upgrade potions, the Store "all charms" expansion, slot limits by premium status, and Bosstiary.
  These are later decisions.

**Amendment (pending on acceptance of BOSS-RAID-0; `OTERYN_GAME_BOSS_RAID0_BOSSES_RAIDS_AND_BOSSTIARY_DECISION_2026-09-30.md` §10, §11).** The Bosstiary, boss slots
and the Boosted Boss are decided there on this packet's Bestiary pattern.
- Any wire message numbers, DDL or code. Those follow in the accepted slice's own contract and PR.

## 7. Owner answers (2026-09-29, in session)

The owner answered in session. Decision-register numbers are assigned by the coordinator batch on `#162`.

| Question | Answer | Effect |
|---|---|---|
| 1. Scope order | **c**: all slices in parallel | CHARM-1..5 start together. Each slice has its own writer and disjoint owned paths. Dependent slices code against the interfaces in §4 and merge in dependency order (1, 2, 3, 4, 5). |
| 2. Economy storage | **a** | Only unlocks and assignments are stored; Charm Points and Echoes are derived. |
| 3. Unassign fee | **c** | No unassign command until `GAME-ITEM-01`/`DUR-03` prove the Character and Item boundary. Unassign then ships with the gold fee from the start. |
| 4. First effect group | **c**: all 25 | CHARM-4 covers every effect type in the catalogue. Effects whose runtime system does not exist yet (loot, skinning, death penalty, leech, critical hits, fleeing, mana drain) fail closed until that system exists. |
| 5. Source conflicts | **a**: follow TibiaWiki | The owner verifies Bless, Carnage, Overpower, Overflux and Parry in the live game. A result that disagrees with the wiki reopens the item. |
| 6. Credit rule | **a** | Credit is given when the character damaged the creature within the last 5 minutes before its death. |

Also verified by the owner in the live game before CHARM-3 freezes: how many charms one creature may hold at once
(§4.2 `UNKNOWN`). CHARM-3 (#1307) was merged with one major plus one minor charm per race (TibiaWiki `Updates/14.10`,
Canary `iobestiary.cpp`).

## 8. Owner answers (2026-09-30, in session)

Questions 8 to 11 come from the CHARM-5 wire proposal (#1301). Questions 12 to 14 come from CHARM-4 (#1303), and
question 15 from CHARM-3 (#1307). Question 16 was raised in session. Decision-register numbers are assigned by the
coordinator batch on `#162`.

| Question | Answer | Effect |
|---|---|---|
| 8. Names on the client | **a** | The client resolves race and charm names from a client content export of the same content generation. The wire carries no text. |
| 9. State-domain owner | **a** | The Bestiary and Charm state domains are owned by the Character Authority. |
| 10. Races with no kills | **a** | The view carries only counted races. The client lists the other races of a class from the content export, shown as unknown. |
| 11. Wire identifiers | **a** | Indices follow SPELL-D1: 1-based and derived per content generation in key order, never stored. Durable state keeps the Creature and charm keys, so a display-name change moves nothing, and a key is not renamed. |
| 12. Proc damage commit | **b** | A proc is committed as its own owner damage commit. It comes after the attack's damage and only when that attack reduced the creature's health. It triggers no charm or leech, and it counts for loot and experience credit. Canary `game.cpp:8762` / `iobestiary.cpp:221` and Crystal `game.cpp:8338` do the same. |
| 13. Carnage element | **a** | Physical with resistances, as on TibiaWiki and tibiapal.com. The owner verifies it in the live game. |
| 14. Credit-window damage time | **a** | The caller supplies the time of the character's last damage. Durable carrier support comes later. |
| 15. Charm slot limits | **a** | Free 2, premium 6, Charm Expansion unlimited (Canary). |
| 16. Charms and area attacks | **a**, Tibia 15.25 | An auto-attack triggers charms only on its main target, including area ammunition such as Diamond Arrows. Spells and runes trigger on every creature they hit. Low Blow still covers the whole area. Sources: TibiaWiki `Updates/15.25.3a4a52` and `Cyclopedia`, and the CipSoft forum post 39596528. Canary lacks the rule, and Crystal also applies it to spells. |
