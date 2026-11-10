# Winter Update 2026 (test server): spec and gap list for Oteryn-Game

Status: evidence and planning only (lane wu26spec, 2026-11-10). Nothing here grants
authority, claims numbers or changes the mechanics ledger. Claim migration, capability and
command numbers with `.claude/skills/oteryn-dev/scripts/next-free-numbers.py` when a unit is
allocated.

## Sources

- Official test-server forum, Winter Update 2026 board: CipSoft staff posts only (Community
  Manager, Product Manager, Programmer, Game Content Designer) in threads 558-570, read
  2026-11-10. Player posts are used only where a staff reply answers them, and are marked
  as player reports otherwise. Thread 558 (Fixes and Changes) had no change entries yet.
- Test client 15.33.c68982 (`TibiaExternal`) against live 15.33.124f6c (`Tibia`):
  appearances, staticdata and client strings; names, looktypes and NPC sale data were read
  directly from the test `appearances.dat` and `staticdata.dat` (`client-delta.md`).
- TibiaWiki (MediaWiki API): `Winter Update 2026` (changelist from the teasers),
  `Temple Teleport Scroll`, `Task Board`. Pages for the new creatures, bosses, items and
  quests did not exist yet.
- Oteryn code at `b028bef4dd`, read in place; every claim in the feature files carries
  file:line evidence.

## Files

| File | Feature |
|---|---|
| `bounty-elite-platinum.md` | Elite and Platinum Bounty Tasks (thread 560) |
| `daily-reward-rework.md` | Daily Reward Rework, Cosmetic Tokens, XP boosts, resting areas (thread 565) |
| `gem-atelier-workshop.md` | Gem Atelier and Fragment Workshop (thread 559) |
| `qol.md` | Spell target options, Stash revisit, catalyst conversion, Create Mark (thread 567) |
| `banners-return.md` | Banner's Return quest, hunting grounds, boss, gear (threads 566, 568, 569, 570) |
| `i-smell-a-rat.md` | I Smell a Rat quest, hunting grounds, bosses, gear (threads 563, 562, 564, 561) |
| `client-delta.md` | Counts and id lists of the client diff |

## Headline

Much of the systems side already exists in Oteryn: Elite tasks with Silver/Gold/Platinum
grades and rarity build-up (T201), the one-click daily reward with Cosmetic Tokens, Bounty
Reroll Tokens and the proficiency streak bonus (DAILY30), and Cosmetic Token Store spending.
The gaps there are numbers and rules (caps, dust, XP boost stacking, resting areas, token
cap). The Gem Atelier rework is the largest systems change (persistence and wire). Both
content updates (two quests, six hunting grounds, six bosses) are blocked on map data and
creature stats; their gear and NPC trade data can land first.

## Units

Class: **A** = needs a wire (protocol-oteryn) change; **B** = Oteryn has a partial
implementation that needs rules or numbers changed; **C** = missing. Size: S (about a day,
one module), M (a few modules or one migration), L (several modules, migration and wire,
or content blocked on external data).

| Id | Feature | Unit | Class | Depends on | Size |
|---|---|---|---|---|---|
| WU26-01 | Bounty | Rarity caps 75/20/5%, Elite dust 5/15/?/70 | B | owner value for Expert dust | S |
| WU26-02 | Bounty | Reroll token storage cap 10 | B | - | S |
| WU26-03 | Daily reward | XP boost: daily + Store additive, daily replaces daily | B | - | M |
| WU26-04 | Daily reward | Resting-area bonuses independent of the streak | B | - | S |
| WU26-05 | Daily reward | Retire the item picker (6122 / 6123) | B | - | S |
| WU26-06 | Daily reward | Temple teleport scroll use and 7-day expiry | C | - | M |
| WU26-07 | Daily reward | Seven-day cycle and 7% streak cap | B | owner decision on DAILY30 | S |
| WU26-08 | Gem Atelier | Drop domain and resonance; Gem Vessel slots (+1 dmg/heal) | B | slot-order answer | L |
| WU26-09 | Gem Atelier | Atelier v2 wire and client (no domain, slots, Replace Vessel) | A | WU26-08 | M |
| WU26-10 | Gem Atelier | Mod Reshaping (temple, Premium, gold + fragments) | C | WU26-08, WU26-09 (tag) | M |
| WU26-11 | Gem Atelier | Workshop +5 HP / mana / capacity per grade | B | - | S |
| WU26-12 | Gem Atelier | 30% gem resistance cap per element | B | - | S |
| WU26-13 | Gem Atelier | Supreme mods for ten new spells | C | mod data | M |
| WU26-14 | QoL | Spell target options for wave / beam spells | A | - | M |
| WU26-15 | QoL | Catalyst conversion at Emael | B | - | S |
| WU26-16 | QoL | `Misc.: Create Mark` hotkey (client) | B | - | S |
| WU26-17 | QoL | Stash revisit and persisted sort (client) | B | - | S |
| WU26-18 | Banner's Return | Corrupted moonsilver gear (5 items) | C | WU26-30 | S |
| WU26-19 | Banner's Return | Omega creatures and soul cores | C | stats; WU26-30 | M |
| WU26-20 | Banner's Return | Red Gullet scorch level, blood cinders, Ash Sentry | C | numbers; WU26-23 for area | M |
| WU26-21 | Banner's Return | Boss drop pity tracker (generic) | C | pity numbers | M |
| WU26-22 | Banner's Return | Banner's Nemesis encounter (Archfoe, ally, difficulty) | C | map, stats, WU26-18, WU26-21 | L |
| WU26-23 | Banner's Return | Quest line 195, areas, achievements 618 / 623 / 624 | C | map; Order of the Stag | L |
| WU26-24 | I Smell a Rat | Rat gear (9 items) | C | WU26-30 | S |
| WU26-25 | I Smell a Rat | Frenzy mechanic and creature icon | A | - | M |
| WU26-26 | I Smell a Rat | Rattica and Greed creatures, soul cores | C | stats; WU26-25; WU26-30 | M |
| WU26-27 | I Smell a Rat | Lever-operator cooldown and separate reward cooldown | B | - | M |
| WU26-28 | I Smell a Rat | Quest line 192, Temple of Greed bosses, Greed Dimension | C | map, stats, WU26-26, WU26-27 | L |
| WU26-29 | Both | New valuables in Yasir and jewel-trader buy lists (client sale data) | B | WU26-30 | S |
| WU26-30 | Both | Client metadata capture at the release client (1,028 objects, 21 outfits, 3 effects, staticdata) | B | release client | M |

Units per feature: Bounty 2, Daily reward 5, Gem Atelier 6, QoL 4, Banner's Return 6,
I Smell a Rat 5, shared 2 (30 total).

### WU26-29 and WU26-30 (shared)

- **WU26-30** Owned files: the content capture tooling under `tools/content-schema/` and
  the reference capture under `docs/reference/content-prep/` (the last capture is
  `official-client:15.33.9d148f`, `docs/reference/content-prep/market-content-1/sources.json:24825`),
  regenerated item definition shards, pins. Capture from the **release** client, not the
  test client, so ids and sprite sheets match what players will run. The 43,515 object CHG
  entries are a sprite-id repack (`client-delta.md`) and need only the sprite catalogue
  re-import. Acceptance: new ids 54756-56102 present with names; `pin-check` green.
- **WU26-29** Owned files: Yasir (`content/npcs/definitions/npcs-01000-01281.json`) and the
  jewel traders' service offers. Data: every sell price listed in `banners-return.md` and
  `i-smell-a-rat.md` section 2 (from the client's NPC sale data). Acceptance:
  `yasir_buys_winter_2026_valuables_at_client_prices`, `pin-check` green.

## Suggested order

1. Number and rule fixes on existing systems, independent and small: WU26-01, WU26-02,
   WU26-04, WU26-05, WU26-11, WU26-12, WU26-15.
2. Medium systems: WU26-03, WU26-06, WU26-27, WU26-14, WU26-16, WU26-17.
3. Gem Atelier rework, in order: WU26-08 -> WU26-09 -> WU26-10 (one owner, one branch
   each; WU26-08 carries the migration).
4. Generic engine pieces for the content: WU26-25 (frenzy), WU26-21 (pity).
5. After the release client: WU26-30, then WU26-18, WU26-24, WU26-29.
6. Content once stats and map exist: WU26-19, WU26-26, WU26-20, WU26-22, WU26-23, WU26-28.

WU26-07 and WU26-13 wait for an owner decision and for data respectively.

## Not implementable yet

- **Maps.** Onyx Massif, Darkfire Gorge, Red Gullet, Hushwell Asylum, Rattica (Outskirts,
  Warrens, Underbelly), Temple of Greed and the Greed Dimension. Oteryn's world comes from
  the pinned CrystalServer map plus hand-authored tiles
  (`tools/content-schema/world-authoring/convert_world_base.py`); no item-level map of these
  areas exists in any available source. The test client changes only minimap colour tiles
  (no new tile files; 40 changed tiles), named `minimap-32-0996-1000-*` (floors 0, 1, 3-7),
  `minimap-32-1060-1016-*` (floors 0-7), `minimap-32-1012-1000-05..07`,
  `minimap-32-0996-1016-07`, `minimap-32-1028-0984-07`, `minimap-32-1044-1000-06`,
  `minimap-32-1060-0968-07`, plus the matching `minimap-64-*` tiles. These can guide a
  minimap draft (`minimap_draft.py`, generic ground) but not a playable map.
- **Creature and boss stats, loot and abilities** for all 12 creatures and 6 bosses
  (not in the client; no TibiaWiki pages yet).
- **Mission texts and NPC dialogue** for both quest lines (server-side only).
- **Numbers not published**: Expert Elite dust, rarity increments, scorch thresholds and
  cinder yields, pity curve, frenzy curves, supreme mod values for the ten spells, Gem Vessel
  slot order, the exact Global seven-day reward table.
- **Prerequisites missing in Oteryn**: the Order of the Stag quest is not completable
  (`content/quests/order-of-the-stag-death.json`, `runtime_enabled: false`). The spells
  named by the new gem mods and gear augments have bundle definitions; their runtime
  castability must be checked per spell before WU26-13, WU26-18 and WU26-24 (the
  source-import conflict list `content/abilities/source-imports/unavailable-spells.jsonl`
  names several of them).
- **Store purchases** with Cosmetic Tokens are disabled on the test server; Oteryn already
  has its own owner-approved token purchase contract
  (`docs/contracts/protocol-oteryn/v1/store-cosmetic-tokens-v1.md`).

## Owner questions (batched)

1. Keep the 30-day DAILY30 lane and the 8-10% streak chance, or follow the test server's
   seven-day cycle and 7% cap (WU26-07)?
2. Expert Elite dust value until CipSoft publishes it (WU26-01)?
3. Keep Crystal's free bounty reroll every 20 h, now that the client names the Daily
   Reward Wall as the only free token source (`bounty-elite-platinum.md` section 5)?
