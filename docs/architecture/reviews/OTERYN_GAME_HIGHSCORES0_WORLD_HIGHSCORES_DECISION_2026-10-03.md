# HIGHSCORES-0 World Highscores

- Decision: `HIGHSCORES0-WORLD-HIGHSCORES-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence
  and protocol) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers:
  - control-plane allocation D297 (#1622), under owner extension D296;
  - the Highscores system of owner decision 1a's Q1a list (CYCLOPEDIA-0 header);
  - ADR-0004 §6 ("candidate public projections include ... highscores").
- Builds on:
  - the multichannel scope matrix ("Ranking: Ranking projection, World, Eventual, aggregates all
    channels, shared projection");
  - ADR-0004 §6 and §7 (Game-owned public projections; no Portal writes);
  - migration `0009` (`game_character_progression_state`: level, total experience);
  - migration `0030` and the A13 character build state decision (vocation, magic level, the seven
    skills);
  - CHARM-0 and migration `0020` (Charm Points, derived);
  - BOSS-RAID-0 §10.2 (Boss Points, derived);
  - `OTERYN_ACHIEVEMENT_OWNER_CONTRACT_V1` §4 (points per account; "rankings rank accounts");
  - the account characters projection (`0024`);
  - FND-02 (command path) and the achievements panel query (command 10) as the paged-result
    idiom;
  - owner rule 5905825574.
- Amends: none. The Platform and Atlas export of the same rows is a later export profile.
- Runtime, migration and production authority: NONE. Each child needs its own #1622 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| HS-1 | impl, persistence review | the snapshot tables, the per-World snapshot job, the categories of §3 that have a source, ranks and filters (§4, §5) | CHAR-BUILD-1b (build writer); CHARM-1 (Charm Points); account achievements (`0021`) |
| HS-WIRE-1 | impl, protocol review | capability `HIGHSCORES_V1` and the paged query (§6) | HS-1 |

Later, each with its own decision or amendment:
- Boss Points, when the Bosstiary state of BOSS-RAID-0 §10 exists (HS-1 adds the category then);
- Loyalty (Platform owns loyalty points);
- Drome Score, Goshnar's Taint and Phosphorus Record (their systems);
- the Platform and Atlas export of the snapshot rows (an export profile);
- other-World and world-type filters in the client.

## 1. Question

How does a World rank its characters in Tibia's highscore categories, how fresh are the lists, and
who may read them?

## 2. Facts

**PROVEN**

- Scope matrix: ranking is a World-scoped, eventually consistent shared projection that aggregates
  every channel of the World. ADR-0004 §6 names highscores a candidate Game-owned public
  projection.
- Level and total experience are durable in `game_character_progression_state` (`0009`). Vocation,
  magic level and the seven skills (fist, club, sword, axe, distance, shielding, fishing) are
  durable in `game_character_build_state` (`0030`), with skill level and tries.
- Charm Points (CHARM-0, `0020`) and Boss Points (BOSS-RAID-0 §10.2) are derived and never stored.
  The Bosstiary state they derive from is not built yet.
- Achievement points belong to the account (D48, scope matrix). The owner contract §4: "Rankings rank
  accounts. How a ranking ... reads them is a Platform or Atlas export question."
- Loyalty points are Platform's (manual `accounts.md`, marked `[platform]`).

**CIPSOFT_OFFICIAL** (Tibia manual, capture 2026-09-28)

- `interface.md` §3.6.26: the Highscores are a top-character leaderboard "filterable by vocation,
  game world, world type, or specific skill".
- `achievements.md` line 26: the top 300 characters by achievement points per game world.
- `accounts.md` lines 81 and 179: Loyalty Highscores list the top 300 accounts per world, shown
  under a chosen character (default: the highest-level non-hidden one).
- `combat.md` line 262: a separate highscore list tracks lifetime Drome levels.

**TIBIAWIKI_STRUCTURED** (Highscores, revid 1190424)

- 16 categories: Achievement Points, Axe Fighting, Boss Points, Charm Points, Club Fighting,
  Distance Fighting, Drome Score, Experience, Fishing, Fist Fighting, Goshnar's Taint, Loyalty
  Bonus, Magic Level, Phosphorus Record, Shielding, Sword Fighting.
- The client lists refresh about every 30 minutes; pages hold 50 rows.

**OTHER_STRUCTURED_TIBIA_DATA** (TibiaData API v4 `/highscores`)

- At most 1,000 rows per category, in pages of 50. Each row shows rank, name, vocation, world,
  level and value. Equal values share a rank number (observed: three characters at rank 944 with
  210 achievement points).

**UNKNOWN**

- Whether deleted, banned, hidden or auctioned characters are listed (no official source).

## 3. Categories (HS-1)

| Id | Category | Value (read from committed state) | Unit | Now |
|---|---|---|---|---|
| 1 | Experience | `total_experience` (level shown) | character | yes |
| 2 | Magic Level | (`magic_level`, `mana_spent`) | character | yes |
| 3-9 | Fist, Club, Sword, Axe, Distance, Shielding, Fishing | (skill level, tries) | character | yes |
| 10 | Achievement Points | the account's points (owner contract §4) | account | yes |
| 11 | Charm Points | the Charm Points earned, as CHARM-0 derives them | character | yes |
| 12 | Boss Points | BOSS-RAID-0 §10.2 | character | when Bosstiary state exists |
| 13 | Loyalty Bonus | Platform | account | later (Platform) |
| 14-16 | Drome Score, Goshnar's Taint, Phosphorus Record | their systems | character | later |

- A category whose source does not exist is not offered; the query refuses it (`NOT_SUPPORTED`).
- A skill category orders by (level, tries), so progress inside a level breaks ties as Tibia's
  hidden progress does; the row shows the level only.
- **Account categories** (10, later 13) list each account once per World, under the account's
  highest-level live character in that World (ties: the lowest CharacterId), as Tibia's Loyalty
  list does. A chooser is later (§9).

## 4. Snapshots (HS-1)

- **Job.** A per-World snapshot job runs every `HIGHSCORES0-RL-01` (30 minutes, `PARITY_PENDING`)
  and after each planned World reset. It reads only committed durable state in one repeatable-read
  transaction, never live runtime values, so every channel of the World is counted alike.
- **Tables.**
  - `game_world_highscore_snapshots`: (`world_id`, `category`, `snapshot_id`), `computed_at`, the
    source cut, and the row count.
  - `game_world_highscore_rows`: (`snapshot_id`, `position`), `rank`, `character_id`, `value`,
    `level`, `vocation`, and the name read at the cut.
- **Retention.** The current and the previous snapshot per (World, category); older ones are
  deleted by the job. The rows are a derived projection: rebuildable from durable state, never an
  authority, never read by a gameplay rule, and not a DUR-03 value.
- **Grants.** The job's role writes and deletes the snapshot tables only; `oteryn_game_runtime`
  SELECT.
- A failed job keeps the previous snapshot; the reader shows its `computed_at`.

## 5. Ranking rules (HS-1)

- **Who is listed.** Live Character roots of the World (`lifecycle = 1`) with a vocation other than
  `none` for categories 1-9 and 11. A character in a deletion or sale workflow, or under an
  account sanction that hides it, is left out when that workflow's decision exists
  (`PARITY_PENDING`); none exists today.
- **Size.** At most `HIGHSCORES0-RL-02` (1,000) rows per (World, category).
- **Rank.** Standard competition ranking: equal values share a rank, and the next rank skips
  (1, 2, 2, 4). Positions inside a tie order by name (Unicode code point), then CharacterId, so
  pages are stable.
- **Vocation filter.** The base vocation (knight, paladin, sorcerer, druid, monk); a promoted
  vocation counts as its base. The filtered list keeps the global rank of each row (`PARITY_PENDING`).

## 6. Wire (HS-WIRE-1)

- **Capability `HIGHSCORES_V1`**, number reserved by the control plane before HS-WIRE-1, with one
  command type (number from the control plane).
- **Query.** `HighscoresQueryV1 {category, vocation (0 = all), page}` for the session's own World.
  The result carries `{category, computed_at, page, has_more, rows[<= 50]}`; each row is `{rank,
  name, vocation, level, value}`. A page beyond the last returns no rows.
- **Own row.** The result also carries the asking character's own row when it is listed, or none.
- Results: `NOT_SUPPORTED` (a category not offered) and the FND-02 ones. The query reads the
  snapshot only; it never computes ranks.
- Without the capability, the client shows no Highscores.

## 7. Rows (registered by HS-1 and HS-WIRE-1)

| Row | Value |
|---|---|
| `HIGHSCORES0-RL-01` snapshot interval | 30 minutes (`PARITY_PENDING`, TibiaWiki) |
| `HIGHSCORES0-RL-02` rows per (World, category) | 1,000 (TibiaData) |
| `HIGHSCORES0-RL-03` rows per page | 50 |
| Snapshots kept per (World, category) | 2 |

## 8. Rejected options

- **Live queries over durable state on every request.** Unbounded scans on the World's hot tables;
  Tibia itself shows refreshed lists.
- **Per-channel lists.** Ranking is World-scoped (scope matrix).
- **Ranking from runtime values.** A crash or a channel could disagree with the durable state.
- **Ranking achievement points per character.** The points belong to the account (D48 and the
  owner contract §4).
- **Top 300 for every category.** Only achievements and loyalty have an official 300; other
  categories show 1,000 (R2).

## 9. Architect rulings (owner rule 5905825574)

- **R1, source: a) periodic durable snapshots**; b) live queries; c) an event-fed projection.
  Recommendation and ruling: a).
- **R2, size: a) 1,000 rows in every category** (TibiaData, the observed lists); b) 300 for
  achievements as the manual says. Recommendation and ruling: a), `PARITY_PENDING` for
  achievements.
- **R3, ties: a) shared ranks with a skip, positions by name**; b) dense ranks. Recommendation
  and ruling: a).
- **R4, account categories: a) shown under the highest-level live character**, a chooser later;
  b) a chooser now. Recommendation and ruling: a).
- **R5, World filter: a) own World in the client**, others through the later export; b) any World
  now. Recommendation and ruling: a).

## 10. Owner questions

None. Every choice above is a reversible architect ruling under owner rule 5905825574.

## 11. Decision test

- **Must decide now:** YES. Control-plane allocation D297 (owner D296).
- **Blocked without it:** the Highscores window and the public ranking projection.
- **Harder later:** the category ids and the rank rule become client-visible and export-visible.
- **Supersede if:** official evidence on listed sizes, exclusions or the refresh interval; a
  Platform or Atlas export profile that needs other fields.
- **Deliberately not decided:** Boss Points until Bosstiary state; Loyalty; Drome, Goshnar's
  Taint and Phosphorus Record; the export profile; deletion, sale and sanction exclusions; other
  Worlds in the client; the account-category character chooser.

## 12. Before-freeze checklist

1. **Contracts:** none amended. ADR-0004 §6 is applied, not changed.
2. **Serialization:** one repeatable-read read transaction per snapshot; no write to any source.
3. **Restart:** snapshots are durable; a failed job keeps the previous one.
4. **Typed references:** WorldId, CharacterId, AccountId, category id.
5. **Wire:** §6, capability `HIGHSCORES_V1`.
6. **Split work:** 1,000 rows per list, 50 per page; one World per job run.
