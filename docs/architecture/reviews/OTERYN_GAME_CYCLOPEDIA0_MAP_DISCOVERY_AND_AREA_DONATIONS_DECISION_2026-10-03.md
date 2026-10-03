# CYCLOPEDIA-0 Cyclopedia map discovery and area donations

- Decision: `CYCLOPEDIA0-MAP-DISCOVERY-AND-AREA-DONATIONS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence,
  economy, protocol) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: owner decision 1a (#1622 comment 5968564302, 2026-10-03) to admit area donations as in
  Tibia, together with map discovery (the Measuring Tibia Quest). The Q1a systems are Cyclopedia,
  Supply Stash, Daily Reward, Highscores, Analysers and Familiars, and this decision covers only
  the Cyclopedia map part. Bestiary, Bosstiary, Charms, Character and Houses stay with their own
  decisions.
- Allocation: D286 (#1622 comment 5968568302), task `OTV2-20261003-cyclopedia0`, an owner
  exception to D252.
- Builds on:
  - Multichannel scope matrix (spawn runtime per channel, character progression per character).
  - CREATURE-AI-0 §6.4 (the respawn rate hook) and §6.5 (`WorldReset`).
  - BOSS-RAID-0 (raid schedule and announcements).
  - The gold fee decision (D174-D178) and BANK-FEE-0 §3.
  - DUR-03 (closed `FeeBurnCause`).
  - Account progress D47-D49 (outfits and achievements belong to the account, counters per
    character).
  - The Achievement owner contract, QUEST-STATE-0 (CHAR-REV-SEQ-1) and NPC-0.
  - The FORMULA source order (owner rule 5905825574).
- Reference only: crystalserver#812, an unmerged proposal, and Crystal's
  `player_cyclopedia.hpp` enum stubs. Canary has no implementation.
- Runtime, migration, content and production authority: **NONE**. Each child needs its own #1622
  allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| CYC-CONTENT-1 | content | areas, subareas, discoverable and donation-eligible flags, tile-to-subarea membership and POI pools from the client map data (§3) | none |
| CYC-DISCOVERY-1 | hard (persistence) | character discovery state, start and shuffle commands, POI runtime, completion and its grants (§4) | CYC-CONTENT-1; CHAR-REV-SEQ-1 |
| CYC-DONATE-1 | hard (economy, persistence) | world donation pools, the donation fee transaction with its DUR-03 `FeeBurnCause` amendment, selection at `WorldReset` (§5.2-§5.4) | CYC-CONTENT-1; BANK-FEE-0 |
| CYC-RESPAWN-1 | impl | the area factor in the CREATURE-AI-0 §6.4 hook (§5.5) | CYC-DONATE-1; SPAWN-1 |
| CYC-WIRE-1 | impl, protocol review | capability `CYCLOPEDIA_MAP_V1` (§6) | CYC-DISCOVERY-1; CYC-DONATE-1 |
| CYC-PARITY-1 | research | evidence for every `PARITY_PENDING` value (§8) | none |

The raid warning (§4.5) ships with RAID-1. The Charos reward (§4.5) ships with the NPC lane.

## 1. Question

How does a character discover the Cyclopedia map's areas, what does discovery unlock, and how do
gold donations and a random draw give an area an improved respawn rate between two server saves?

## 2. Facts

**PROVEN (official)**

The Tibia manual (`docs/reference/tibia-manual/interface.md:35-44`, capture 2026-09-28) states:

- Each subarea has points of interest (POIs), and a subarea is discovered at 7 found POIs.
- POI progress is per character and can be reset.
- An area's unlocks depend on the share of its subareas discovered:
  - at 30%, NPC locations are shown;
  - at 70%, passages and teleports are shown;
  - at 100%, the creature list (unrevealed Bestiary entries as silhouettes) and a 1-hour raid
    warning.
- A fully discovered area gives a permanent speed bonus there, and the bonus grows with the number
  of fully discovered areas. The values are not given (`interface.md:260`).
- NPC Charos gives the outfit at 10 areas, the first addon at 15 and the second addon at 20.
- One area per server save is randomly chosen for a faster respawn rate. A second area can be
  chosen by gold donations that reach a minimum by server save. A tie at the highest donation is
  broken at random. The winning area's gold resets to 0, and the other areas keep theirs.

The client 15.30 map data (`imports/cipsoft-staticdata/map/`) has:

- 465 area records, 28 of which list `subarea_ids`;
- 209 subarea overlay images;
- 1,270 markers.

Field meanings beyond the README's inferred names are UNKNOWN.

**PROVEN (English TibiaWiki, read 2026-10-03)**

From `Cyclopedia` (revision 1197515), section Map and Improved Respawn Rate:

- 7 POIs per subarea unlock it, and only one subarea can be explored at a time. Starting another
  subarea resets the open progress, and so does shuffling the POIs.
- There is a 33% chance per server save that one random area gets an improved respawn rate.
  Players can donate gold so that one area gets it after the next server save.
- Creatures spawn two times faster in an improved area, until the next server save.
- An area needs at least 10,000,000 gold donated. The highest total above that wins at server
  save, and if none reaches it no donation area is chosen.
- Only one donation area is chosen per day, and the gold of other areas is never lost.
- An area whose respawn is currently improved can receive neither donations nor the random pick.
- Until 2019-02-05 there was always one random area and the minimum was 1,000,000. Update 12.08
  changed both.

From `Measuring Tibia Quest/Spoiler` (revision 1198901):

- Tibia has 20 discoverable areas.
- More than 7 POIs are generated per start. They are placed differently for each player, chosen
  from a predefined pool, and never inside the current game window.
- A POI is invisible on another floor.
- A completed area gives its achievement and a step towards the Discoverer outfit. The titles are
  *Dedicated Entrepreneur* at 50% of the areas and *Globetrotter* at 100%.
- Raid warnings come 1 hour ahead, and the subarea is named 15 minutes before, unless the raid is
  underground.

Other pages:

- `Measuring Tibia Quest` (revision 935112): the quest needs a premium account.
- `Point of Interest Effect` (revision 848847): effect 193, and approaching the POI triggers
  `Point of Interest Found Effect` (revision 848848, effect 195).
- `Rapid Respawn Events` (revision 1199249): the event rate (5x) is additive with the improved
  area, giving 7x, and with the Boosted Creature. Only creatures with a monster home are affected,
  so bosses and special creatures are not.
- `Discoverer Outfits` (revision 1099576): outfits 1094 and 1095, premium only.

**Community observation**: TibiaMaps (`tibiamaps.io/map/poi`) collects most of the predefined POI
positions.

## 3. Content (CYC-CONTENT-1)

- **Areas and subareas** come from the client map data, keyed by the client `source_id` as an
  Oteryn `MapAreaId` crosswalk:
  - a subarea is an area id listed in a parent's `subarea_ids`;
  - the discoverable flag and the 20 discoverable areas are cross-checked against the wiki
    table, and a disagreement is CONFLICT and not admitted;
  - the donation-eligible areas are the discoverable top-level areas, `PARITY_PENDING` (CYC-PARITY-1
    confirms the set).
- **Tile membership.** The subarea overlay images are decoded into a tile-to-subarea map in the
  public map definition (World scope, immutable revision). A tile belongs to at most one subarea.
  The decode is checked against TibiaWiki examples (for example, the Duelling Arena belongs to its
  City subarea). An undecodable subarea is UNRESOLVED and not discoverable.
- **POI pools.** Each discoverable subarea has a pool of positions inside its tiles:
  - pinned TibiaMaps positions where the observation covers the subarea;
  - otherwise a pool generated deterministically from reachable tiles, which is a declared
    deviation listed in the content report;
  - a pool smaller than the generated count (§8) is refused.
- Bounds: `CYC0-RL-01` at most 64 areas, `CYC0-RL-02` at most 512 subareas, `CYC0-RL-03` at most
  256 pool positions per subarea.

## 4. Discovery (CYC-DISCOVERY-1)

### 4.1 State

Discovery is character progression: per character, strong durable, and the same on every channel.
All writes run on CHAR-REV-SEQ-1 with the expected character revision, under the session-generation
fence. There are two tables:

- `game_character_map_subareas` holds one row per discovered subarea. It is append only and never
  deleted while the character lives.
- `game_character_map_exploration` holds at most one row: the active subarea, the content
  revision of its pool, the generated positions, the found mask and the start revision.

### 4.2 Commands

- **Start discovering** a discoverable, undiscovered subarea:
  - the character needs a premium account, as in the Measuring Tibia Quest;
  - the open row, if any, is replaced, which resets its progress as in Tibia;
  - the positions are drawn under the RNG purpose `cyclopedia_poi`, from the pool minus the
    positions inside the character's current game window, and stored in the row.
- **Shuffle** draws new positions for the open subarea and resets its found mask.
- Each command is one idempotent Character write with a request id. Its draws are bound to the
  occurrence (CharacterId, request id, `cyclopedia_poi`, draw index) under SIM-DETERMINISM-01 §12,
  so a retried command reproduces the same positions.

### 4.2a Content revision change

No content revision reinterprets durable Cyclopedia state downwards. That state is:

- character discovery rows and exploration rows (§4.1);
- world donation pools (§5.2);
- world improved-respawn epoch rows, including the current epoch (§5.2);
- grants already made from them (area achievements, Charos outfits and addons).

A new World Bundle revision is classified under DUR-04 §12 by what it changes:

- **Tiles or pool inside a subarea that stays discoverable in the same area:**
  `READ_COMPATIBLE_NORMALIZE`. The exploration row pins the pool content revision. The next load of
  the character normalizes it in one Character write: found positions stay found and keep counting,
  and unfound positions that are no longer in the new pool are redrawn under the occurrence
  (CharacterId, normalize revision, `cyclopedia_poi`, draw index). No other state changes.
- **The set of areas or discoverable subareas, subarea area membership, the `MapAreaId`
  crosswalk, the donation-eligible set, or the title and reward thresholds**, in a world that holds
  any of the state above: `INCOMPATIBLE_REQUIRES_PRODUCT_DECISION`. Such a revision is not admitted
  until that decision exists. Its value-preserving default is fixed now:
  - no character's area percentage, 30%, 70% or 100% unlock, count of fully discovered areas,
    speed bonus eligibility or title may fall;
  - discovery rows are never rewritten or deleted, and an active exploration row on a subarea
    that stops being discoverable stays pinned to its revision until the decision says how it ends;
  - a donation pool is never lost or reduced: its area and total stay, even if the area stops
    being eligible, until the decision says where the gold goes;
  - the current epoch runs to its end with its selected areas and their tiles at the epoch's pinned
    revision;
  - grants already made stay.
- In a world that holds none of the state above, any revision is `COMPATIBLE_NO_MIGRATION`.

CYC-CONTENT-1's report lists each such change between two revisions. The classification is a
function of the two revisions and of whether the world holds any of the state above. The race
between that check and the reset is removed at its source:

- **Freeze row.** `game_world_cyclopedia_freeze` holds one row per World: none, or the staged
  structural revision with its staging id. A staging id is new and increasing for each staging, so
  staging the same revision twice gives two different ids. Every Cyclopedia write (§4.2, §4.3, the
  §4.2a normalization, §5.3, the Charos and achievement grants) reads this row with a shared lock
  inside its own transaction. If the row is set, the write is refused with the typed result
  `CyclopediaFrozen` and writes nothing.
- **Staging.** For a revision that is structural against the active one, staging is one
  transaction. It locks the freeze row exclusively, which waits for every Cyclopedia write in
  flight. It then checks that the world holds none of the state above, and sets the row to the
  revision and a new staging id. If the world holds state, the revision is
  `INCOMPATIBLE_REQUIRES_PRODUCT_DECISION`: nothing is set and the revision is not staged. After a
  successful staging, the state cannot grow. A revision that is not structural
  (`READ_COMPATIBLE_NORMALIZE`) is staged without a freeze.
- **Reset start.** A reset worker carries the staging id of its target. For a structurally staged
  target, ADR-0021 §4.7 step 1 is one transaction that holds the freeze row exclusively:
  1. If a RETIRING record already exists for the world, or the row does not name this worker's
     staging id, the worker writes nothing and exits. A stale worker therefore never touches
     another staging, even one of the same revision.
  2. Otherwise it checks again that the world holds no state.
  3. If the world still holds no state, it writes the RETIRING record with that staging id.
  4. If the world now holds state, it writes no RETIRING record. It unstages its own staging and
     sets the row to none, and the world keeps running on the active bundle.

  While admission is still open, a Cyclopedia write either commits before this transaction takes
  the lock, and is then seen by the check, or waits and is refused.
- **Lifting.** Before RETIRING is recorded, the freeze is lifted only by unstaging, or by the
  failed check in step 4 of reset start. Both clear the row only if it names their own staging
  id, under the exclusive lock, in the same transaction as the unstage. Once RETIRING is
  recorded, unstaging is refused. The freeze is then lifted only by ADR-0021 step 4, which sets the
  row to none in its activation transaction. A crash while RETIRING takes the ADR-0021 recovery
  path: boot refuses admission, resumes from step 2 and ends in that same step 4. A crash in any
  other transaction rolls it back, and a retry with the same staging id meets the checks above
  again. A frozen world keeps running until it is unstaged or reset. It never stops because of
  Cyclopedia.
- **Interleavings.**
  - Stale A, staged B: A's worker sees B's staging id and exits. B stays staged and frozen.
  - A, unstaged, B, unstaged, A again: the second A has a new staging id. The first worker exits.
  - Two workers for the same staging: the first writes RETIRING, and the second sees it and
    exits.
  - A crash between staging and reset start: the row stays set and the world keeps running frozen.
- **Invariant.** A transaction changes the freeze row only for the staging id it owns, and in the
  same transaction as that staging's own change: staging, unstage, reset start or activation. So
  from staging until step 4 commits, a world with a structurally staged revision always has the
  row set to that staging. No committed state has a structural staging and an unset row, or a row
  naming a different staging. Step 4 therefore needs no Cyclopedia check, and no Cyclopedia write
  commits between reset start and activation.

**Ordering against a reset.** Every Cyclopedia write (start, shuffle, POI found, normalize,
donate, Charos) is a write of an admitted session under its session-generation fence, and a
donation's pool increment commits in the same transaction as its fenced Character write (§5.3).
So none can commit after step 2 of a reset has ended the old generation: an in-flight write fails
its fence and changes nothing. Each write first checks the freeze row (above), then re-checks
its own preconditions against the active revision inside its transaction, not before it:

- start and shuffle: the subarea is discoverable and undiscovered, the account is premium, and the
  expected character revision matches;
- POI found: the exploration row is the one the event was computed from (expected revision and
  pool revision), and the found radius holds for the committed position;
- normalize: the pinned pool revision still differs from the active one at the expected character
  revision, so a retry after a commit does nothing;
- donate: §5.3;
- Charos: the count of fully discovered areas is derived inside the grant transaction;
- area completion: the area achievement is checked against the catalogue of the active revision
  inside the completing transaction (§4.3).

The §4.2a normalization runs at the character's first load after activation. A crash during any of
these transactions rolls it back, and the retry draws the same positions from the same occurrence.

### 4.3 Finding a POI

- POIs exist only for their character. The channel shows effect 193 on them to that character
  only, and only on the character's floor.
- A POI is found when the character approaches it within the found radius (§8). The found bit is
  committed as one Character write before effect 195 and the progress message are sent.
- Entering or leaving the active subarea sends the Tibia message. It needs no write.
- The 7th find is one Character transaction: the subarea row is inserted, the exploration row is
  deleted and the completion evidence is recorded. If the subarea completes its area, the same
  transaction also records the area achievement grant request, and the Achievement domain consumes
  it there (Achievement owner contract §3, D48). A missing catalogue key fails the whole
  transaction closed. There is no later grant step that a crash could lose.

### 4.4 Derived values

Each area's percentage, the 30%, 70% and 100% unlocks, the count of fully discovered areas and the
two title thresholds are derived from the subarea rows and content. None of them is stored.

### 4.5 Grants and consumers

- **Area achievement.** Granted inside the completing transaction of §4.3 through the Achievement
  owner contract §3 grant path, with the completion as `source_event`. It belongs to the account
  (D48), and the catalogue ids come from the client achievement data.
- **Titles** (*Dedicated Entrepreneur*, *Globetrotter*) are derived. They are shown when the title
  system exists.
- **Discoverer outfit.** Charos (NPC-0 dialog) checks the count of fully discovered areas: 10 for
  the outfit, 15 for the first addon, 20 for the second. It grants through the account outfit path
  (D47), and the dialog content ships with the NPC lane.
- **Raid warning.** For characters with the raid's area at 100%, RAID-1 sends the warning 1 hour
  ahead and the subarea 15 minutes ahead, but not for an underground raid. This uses the
  BOSS-RAID-0 announcement path.
- **Creature list.** The client gets the subarea creature list from content (spawn points by
  subarea), plus the character's Bestiary state for silhouettes.
- **Speed bonus.** This is a movement speed contribution in fully discovered areas that stacks
  with their count. It is inert, with factor 0, until CYC-PARITY-1 evidences the values. It is
  then added through the speed contribution owner.

## 5. Improved respawn rate and donations

### 5.1 Scope

Donation pools and the daily selection are **World** state: one Tibia world is one Oteryn
`WorldId`. Every channel of the world applies the selection to its own spawn runtime. Channels
never share creatures; only the selection record is shared.

### 5.2 State (CYC-DONATE-1)

- `game_world_area_donations` holds one row per (WorldId, area): the donated total (whole gold,
  non-negative) and a revision.
- `game_world_improved_respawn` holds one row per (WorldId, reset epoch): the random area or
  none, the donation area or none, the RNG records and the pool revisions consumed. It is written
  once by the reset job and is idempotent per epoch.

### 5.3 Donation

- The character donates `F` gold to an area that is donation-eligible in the active revision and
  is not improved in the current epoch. Both conditions are checked inside the donation
  transaction, after the pool row is locked. The epoch row is immutable during its epoch.
- `F` is between 1 and `CYC0-RL-04`, which is at most BANK0-RL-01. There is no premium or minimum
  rule until CYC-PARITY-1 shows one.
- **Payment** follows BANK-FEE-0 §3: coins in the main backpack first, then the (Account, World)
  bank balance. A junior character pays with coins only.
- **Burn cause.** The gold is burned under the new `FeeBurnCause::AreaDonation { world, area,
  occurrence }`. This decision authorizes the variant, and CYC-DONATE-1 amends DUR-03's closed
  cause paragraph with it.
- **Transaction.** The burn, any bank debit and the pool increment, at the expected pool revision,
  are one transaction under the character's session-generation fence. The lock order is the gold
  fee order, then the bank balance, then the pool row. If the fence fails, nothing commits.
- Nothing is refunded or minted back.

This decision adds `AreaDonation` as a new fee source and gold sink to the D178 list, next to
`CharmUnassign`, `NpcTrade`, `NpcTravel` and `NpcRepair`.

### 5.4 Selection at server save

Selection belongs to the world reset of ADR-0021 §4.7 (the server save is a `WorldReset`,
CREATURE-AI-0 §6.5). It runs **after** step 4 has committed the ACTIVATED record for epoch N+1 with
its target bundle digest, and **before** any channel of the world admits a player. Admission stays
closed in between. Selection is **one transaction** keyed (WorldId, epoch N+1). It first reads the reset record
inside the transaction and stops unless it is ACTIVATED for epoch N+1 with the active digest. The
epoch row has a unique key on (WorldId, epoch); if it already exists, the transaction ends without
a draw.

1. It locks every `game_world_area_donations` row of the world, in area id order. A donation
   (§5.3) takes the same row lock, so the selection reads one stable snapshot. No donation can run
   anyway while admission is closed.
2. Eligible = the donation-eligible areas of the **target bundle** minus the areas improved in
   epoch N.
3. **Donation area:** the eligible area with the highest total of at least 10,000,000
   (`CYC0-RL-05`, TibiaWiki). A tie is drawn under `improved_respawn_tie`.
4. **Random area:** with chance 33%, drawn under `improved_respawn_random`, one area uniformly from
   eligible minus the donation area.
5. It inserts the epoch row (both areas, the RNG records, every pool revision read, and the target
   bundle digest and content revision) and sets the winner's total to 0. The other totals stay.
   The order of steps 3 and 4 is `PARITY_PENDING`.

Every draw is bound to the occurrence (WorldId, epoch N+1, purpose, draw index) under
SIM-DETERMINISM-01 §12, so a retry reproduces the same outcome from the same snapshot.

**Recovery.**
- If activation fails or crashes while the reset record is RETIRING, there is no epoch N+1, so no
  selection runs. ADR-0021 recovery resumes the reset, and selection follows its activation.
- A crash during the selection transaction rolls it back. Boot then sees an ACTIVATED epoch with
  no epoch row and runs the selection again before admission opens. The pool rows are unchanged,
  so the same occurrences draw the same outcome.
- If the epoch row exists, the job reads it and never draws or resets again.
- A boot whose epoch row names a different bundle digest than the active one refuses admission.
  This is the same fail-closed rule as ADR-0021 §4.2.

### 5.5 Effect (CYC-RESPAWN-1)

- A creature in the selected areas whose spawn point lies on a tile of that area respawns with
  factor 2. The CREATURE-AI-0 §6.4 hook divides the content delay by the sum of the active
  factors (2 for the area; the event and the Boosted Creature are added when they exist; 5 + 2 = 7
  as TibiaWiki states). Other combinations are `PARITY_PENDING`.
- Bosses, special creatures and creatures without a spawn point (raids, summons) are not affected.
- The factor holds from one `WorldReset` to the next. Tile membership is read from the bundle
  that the epoch row names, which is the active bundle (§5.4).

## 6. Wire (CYC-WIRE-1)

Capability `CYCLOPEDIA_MAP_V1` is server-authoritative. Numbers are reserved on #1622 at
allocation.

| Message | Content |
|---|---|
| query: discovery | discovered subareas, the active subarea and its found count |
| query: areas | donation totals and the current improved areas of the character's world |
| intent: start discovering, shuffle | §4.2 |
| intent: donate | area and amount (§5.3) |
| events | a POI position only when it comes within the effect 193 range of its own character on the same floor; POI found; subarea entered or left; subarea and area completed |

The 30%, 70% and 100% map unlocks are client display that the server derives from the discovery
query. The server never sends POIs to another character, and it never sends a POI that is not yet
in range. The discovery query carries only the found count.

## 7. Rejected options

- **Donation pools per channel.** Tibia has one pool per world, and the scope matrix keeps economy
  state per world. A per-channel pool would split donations.
- **Area discovery by walking through without POIs** (the crystalserver#812 proposal). It is not
  what Tibia does.
- **Account-wide discovery.** Tibia keeps POI progress per character. D48 makes only the
  achievement account-wide.
- **No random area.** The owner chose "as in Tibia", and Tibia has the 33% random draw.

## 8. `PARITY_PENDING` (CYC-PARITY-1)

Each of these values is content or a constant. The runtime refuses to activate the feature until
the value is set. Nothing is guessed.

- the number of POIs generated per start (more than 7);
- the found radius;
- the speed bonus per number of fully discovered areas;
- the set of donation-eligible areas;
- any premium requirement or minimum for a donation;
- the order of the random and donation draws;
- how factors combine beyond event plus area;
- confirmation of the 33% chance in official text.

## 9. Decision test

1. **Must decide now?** YES. The owner admitted the system (1a), and its durable shapes (per
   character discovery, per world pools, a new burn cause) must be fixed before any child writes
   state.
2. **What is blocked?** CYC-CONTENT-1 through CYC-WIRE-1. Also blocked: the Cyclopedia map raid
   warning that BOSS-RAID-0 deferred, the Improved Respawn Rate factor that CREATURE-AI-0 §6.4
   defers, the Discoverer outfit and area achievements.
3. **What becomes harder later?** Moving discovery from per character to per account, or pools
   from per world to per channel, after rows exist would need a migration and a fairness ruling.
   Burned donations cannot be refunded, so the pool semantics are final once players donate. The
   client `source_id` crosswalk binds content to the 15.30 map data, so a later client revision
   needs a crosswalk revision.
4. **What would justify superseding it?** Official text that contradicts a TibiaWiki value (the
   33% chance, 10,000,000, 2x). Measured respawn load that a factor of 7 makes unsafe on a
   channel. Evidence that Tibia pools donations differently.
5. **What is not decided?** Every §8 value; Bestiary, Bosstiary, Charms, Character and Houses
   tabs; the title system; the free-account map; map marker sync; the other Q1a systems.
