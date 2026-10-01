# WHEEL-GEM-0 Wheel of Destiny gems: the Gem Atelier

- Decision: `WHEEL-GEM0-GEM-ATELIER-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (Character
  state, persistence, economy, protocol and combat) and protected integration. It extends WHEEL-0
  and integrates after it. Owner questions G1 and G2 (§13) are answered (G1 a, G2 b; owner,
  2026-09-30, #162) and binding.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the owner direction of 2026-09-30 (build now, full Tibia Global parity) and WHEEL-0's
  deferral of gems, the Gem Atelier, vessels, fragments and mod grades to WHEEL-GEM-0
- Builds on: WHEEL-0 (`game_character_wheel_state`, slots, receipts, `commit_character_wheel` on
  CHAR-REV-SEQ-1 with expected `wheel_revision`, W-R, W-FX-1, capability `WHEEL_V1`, eligibility
  §6 with owner answer W1 a); the Wheel state candidate §5; QUEST-STATE-0 §5.2 (CHAR-REV-SEQ-1);
  DUR-02 baseline rules 2 and 10; DUR-03 §7, §11.3, §11.5, §14-§17 and §39; the composition
  decision rules 1-4; the gold fee decision (D174-D178, §4.3 and §4.4 with the NPC-0 amendment,
  D208); BANK-FEE-0 (coins, then the bank); NPC-0 (trade offers); D3 (loot); SIM-DETERMINISM-01
  (named RNG purposes); GAME-ABILITY-01 hooks; ITEM-USE-0; owner rule 5905825574 (Global parity)
- Amends, pending on acceptance of WHEEL-GEM-0, in this PR: the Wheel state candidate §5 (pointer).
  WHEEL-0 §4 and §7, DUR-03 §15 and §39.3, and the gold fee decision §4.4 are amended by the
  children named in §11, after WHEEL-0 integrates.
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| GEM-R | impl, content review | the gem ruleset: families, mod catalogues, values per grade, resonance slots, fees, yields, grade costs; validator; revision migration rule (§4, §5.3) | W-R |
| GEM-CONTENT-1 | content lane | gem and fragment item facts (stack of 100), the vocation family of each gem item, loot rows for bosses, the jewelry NPC offers check (§3) | none |
| GEM-1 | hard, persistence and economy review | the Atelier tables, a new receipt kind, `commit_character_atelier` on CHAR-REV-SEQ-1; the initial gems; reveal, dismantle, switch domain, lock, grade up; the causes and their DUR-03 and gold fee amendments (§5-§8) | W-1; GEM-R; GOLD-FEE-2 |
| GEM-VESSEL-1 | hard, persistence review | the vessel table in `commit_character_wheel`, the Wheel receipt extension (§9) | W-1; GEM-1 |
| GEM-WIRE-1 | impl, protocol review | capability `WHEEL_GEM_V1`, `ATELIER_QUERY`, `ATELIER_INTENT`, the vessel field of `WHEEL_INTENT` (§10) | W-2; GEM-1; GEM-VESSEL-1 |
| GEM-FX-1 | hard, combat review | mod effects through the W-FX-1 contribution and the ability hooks; resonance; revelation mastery; Grade IV points (§9.3) | W-FX-1; GEM-VESSEL-1 |
| GEM-CRUSH-1 | impl, persistence review | crushing an unrevealed gem with a crusher (§8) | GEM-1; ITEM-USE-0's item-target arm |
| W-3 | client owner | the Atelier and Fragment Workshop tabs of the Wheel window | GEM-WIRE-1 |

Later, each with its own decision: gem drops from fiendish creatures (the Forge fiendish registry,
native behaviours 1c), the amber crusher (a Store item), presets, and vocation change.

## 1. Question

How does a player get gems, reveal them into modded gems, place them in the four vessels, turn
them into fragments and grade mods up, with every gold, item and Character write classified and
replay-safe?

## 2. Facts

**PROVEN**

- WHEEL-0 (open PR): W-1's state, slot and receipt tables; `commit_character_wheel` on
  CHAR-REV-SEQ-1 with expected `wheel_revision`; eligibility checked at each use; extra points "0
  until scrolls, the monk quest and Grade IV mods exist" (§6.3); W-FX-1's one Wheel contribution.
- Gold fee decision: D177 puts a Character change, its BURN lines and the change MINT in one
  transaction with one receipt; D178 makes the BURN cause a closed type, one variant per fee
  source, "admitted only by an amendment of DUR-03 §39.3 and this decision, with its own owner
  decision". BANK-FEE-0: coins first, then the bank; `F` is bounded by `T` plus `BANK0-RL-01`; the
  main backpack holds at most 20 entries.
- DUR-03 §15: a burn sink must be a closed admitted cause. §14: every mint names a typed source
  and occurrence; the same occurrence cannot mint twice. §11.3: planned output identities are
  fixed before the first commit attempt. §37: DUR-03 does not decide loot. SIM-DETERMINISM-01:
  each random decision has a named RNG purpose.
- The composition decision rule 1: an item-only transaction advances no `CharacterRevision`.
  CHAR-REV-SEQ-1 already carries a Character writer with an in-transaction fee burn (charm).
- Content already holds: the 15 gem items (lesser, regular, greater × guardian, marksman, sage,
  mystic, spiritualist; `i44602`-`i44613`, `i49371`-`i49373`), gem rows in one creature loot table
  (`content/loot/loot-00000-00499.json`), NPC offers that buy gems (2,500, 5,000 and 10,000 gold)
  and sell lesser and greater fragments (`i46625`, `i46626`) and the crusher (`i46627`, 500 gold)
  (`content/services/trade/trade-00000-00321.json`), all under D208's admitted NPC sources.

**CIPSOFT_OFFICIAL** (the Tibia manual, `docs/reference/tibia-manual/characters.md:91-107`)

- Gems drop from fiendish creatures and select bosses; revealing costs gold. Lesser, regular and
  greater gems reveal 1, 2 and 3 mods, assigned at random; larger gems cost more.
- Only the matching vocation reveals a gem (Guardian knight, Marksman paladin, Spiritualist monk,
  Sage sorcerer, Mystic druid). Unrevealed gems are tradeable, revealed gems are not.
- A revealed gem has a domain; "Switch Domain" moves it one domain clockwise for a fee.
- Gems are placed in and removed from vessels. A placed gem has as many active mods as "Vessel
  Resonance" perks completed in its domain (3 per domain).
- A gem can be locked (no domain switch, no dismantle). Dismantling a revealed gem gives
  fragments; refused if locked, in a vessel, or the last gem of its domain.
- The Fragment Workshop grades mods from I to IV with gold and fragments; basic mods take lesser
  fragments, supreme mods greater. Grade is shared per mod type across the character's gems,
  capped by the lowest grade of a preceding mod. Each mod at Grade IV gives +1 promotion point.
- Yields: lesser 1-2 unrevealed, 1-3 revealed; regular 2-4 and 2-5 (lesser fragments); greater
  1-2 and 1-3 (greater fragments). Fragments are tradeable and sold by jewelry NPCs.
- Cooldown mods add a chance of "Momentum", not a direct cooldown reduction.
- Reveal gold costs are given only qualitatively (`characters.md:224`).

**OTS_HYPOTHESIS_ONLY** (Canary `04b83b5`, `src/creatures/players/components/wheel/`)

- Revealed gems live in a Character KV store, not as items (`player_wheel.cpp:1052-1080`). Reveal
  burns one gem of the vocation and quality, charges gold, then draws domain, basic mod 1
  from a slot-1 list, basic mod 2 compatible with mod 1, and a supreme mod from the vocation list
  with `uniform_random` (`:1132-1183`, `:33-87`, `:4127`).
- Reveal costs 125,000, 1,000,000, 6,000,000; switch costs 125,000, 250,000, 500,000
  (`config.lua.dist:263-269`).
- Grade costs (gold, fragments): basic 2 M/5, 5 M/15, 30 M/30; supreme 5 M/5, 12 M/15, 75 M/30;
  a mod reaching the top grade adds one point (`player_wheel.cpp:1448-1531`).
- Destroy yields 1-5, 2-10, 1-5 fragments and skips the vessel and last-gem checks (`:1212-1285`).
  Switch skips the vessel check (`:1287-1306`). Lock toggles (`:1308-1315`).
- Vessels are set in the same save packet as the slots, anywhere (`:1707-1716`, `:2064-2081`).
- Mods activate by resonance count: basic 1 at 1, basic 2 at 2, supreme at 3 (`:2689-2722`); a
  resonance is a full resonance slot (`io_wheel.cpp:382-386`, three slots per domain).
- Each character gets 8 free revealed gems, a lesser and a regular per domain, on first opening
  (`addInitialGems`, `:2004-2031`).
- A crusher (10 charges) turns one unrevealed gem into 1-4, 2-8 or 1-4 fragments
  (`data/scripts/actions/tools/crushers.lua:3-6`).

## 3. Gems as items (GEM-CONTENT-1)

- An **unrevealed gem** is an ordinary item: stackable (100), tradeable, droppable, sold to NPCs.
  Its definition carries its family (one of the five) and its quality. No new item mechanics.
- **Drops** are loot content under D3 and DUR-03 §37, an existing value source. GEM-CONTENT-1
  adds boss rows from the reference sources. Fiendish creatures wait for the Forge registry.
- **Fragments** (lesser `i46625`, greater `i46626`) are ordinary stackable items. The existing
  NPC offers are checked against the reference sources; offer prices are content.

## 4. Ruleset (GEM-R)

`rulesets/progression/wheel-of-destiny/gems.json`, versioned with the Wheel ruleset (ADR-0019):
gem item keys per family and quality; family to vocation; the basic mod catalogue with the slot-1
and slot-2 lists and the compatibility rule; the supreme catalogue per vocation; each mod's value
per grade and vocation; the resonance slots (three per domain) as a slot flag; the clockwise domain
order; fees (§6); fragment yields; grade costs; the preceding-mod chain. Source order: official
statements win, then Canary. Unsourced values are marked `PARITY_PENDING` in the data. A new
revision carries the migration obligation of §5.3.

## 5. Revealed gem state (GEM-1)

### 5.1 Where it lives

A revealed gem is **Character build state**, not an item: it cannot be traded, moved or dropped,
and Global keeps it in the Atelier. It is a typed Character relation (DUR-02 rule 10), beside the
Wheel.

- `game_character_gems`: `gem_id` (UUIDv7, the planned identity of the revealing occurrence),
  `character_id`, family, quality (0-2), domain (0-3), `basic_mod_1`, `basic_mod_2` (regular and
  greater), `supreme_mod` (greater), `locked`, `ruleset_revision`, the revealing occurrence.
- `game_character_gem_grades`: (`character_id`, kind basic or supreme, mod) → grade 1-3 and
  `ruleset_revision`; no row is Grade I. Grades belong to the character, not to a gem (manual; Canary `m_basicGrades`).
- `game_character_atelier_state`: `character_id`, `atelier_revision` (+1 per change), `gem_count`.
- `game_character_atelier_receipts`: one per change: occurrence, SHA-256 request binding, action,
  the CharacterRevision it advanced, before and after `atelier_revision`, the gem or grade row
  before and after, the RNG outputs, the TransactionId of the item lines, the fee binding and
  the Wheel ruleset revision the outcome was committed under.
- **Cap** `WHEELGEM0-RL-02`: 250 revealed gems per character (`PARITY_PENDING`). A reveal over it
  is `ATELIER_FULL`, nothing written.
- **Guards** (deferred): `gem_count` equals the rows; quality matches the non-null mods; the
  latest receipt's after state equals the stored row. A new CharacterRevision receipt kind joins
  the consistency guard (the #162 guard serialization rule).
- No favourite flag: neither the manual nor Canary has one (`PARITY_PENDING`).

### 5.2 Writer

`commit_character_atelier` runs on the CHAR-REV-SEQ-1 sequencer: the full gameplay fence,
`character_root FOR UPDATE`, the Atelier rows, then the main backpack and its entries, then the
bank balance (BANK-FEE-0 lock order). The request carries the expected `atelier_revision`: stale is
`STALE_REVISION`; a CharacterRevision move by another writer retries once (WHEEL-0 §4). Each change
advances `CharacterRevision` once, with its receipt, and commits its item lines in the same
transaction (D177). A refusal writes nothing.

**Replay first (`WHEELGEM0-RP`).** The writer resolves the receipt by occurrence before any other
check. A found receipt whose request binding matches returns the stored outcome, using the ruleset
revision bound in that receipt, and draws, validates and writes nothing; a mismatched binding is
the existing conflict. The request binding covers the client request only, never the server's
current ruleset revision (SIM-DETERMINISM-01 §12: a newer ruleset does not change a retry-safe
occurrence). The current-ruleset checks (`RULESET_MIGRATION_PENDING`, §5.3; the `WHEELGEM0-RNG`
seed revision, §6) apply only to an occurrence with no receipt.

**Runtime projection (`WHEELGEM0-RT`).** Combat reads the runtime actor's Wheel projection, never
the database (WHEEL-0 §4 Load). Admission loads the vessel gems, their rows and the grade rows into
it with the Wheel state. After an Atelier commit that changes a gem in a vessel or a grade row (a
grade up can change an active mod's effective grade and, at Grade IV, the extra Wheel points), the
writer refreshes the actor's projection from the committed rows, re-deriving active mods, effective
grades and available points, before it reports `OK`. If the refresh cannot complete, the actor is
reloaded from durable state before it processes another command; the committed change is never
rolled back and never reported before the projection matches. An offline character has no actor;
admission loads the committed rows. A replay returns the first outcome and refreshes nothing new.

**Eligibility.** Every Atelier action needs Wheel eligibility (WHEEL-0 §6, with W1 a): Global opens
the Atelier inside the Wheel window. Gems and grades survive a lapse; their effects are 0 (§9.3).

### 5.3 Ruleset revision compatibility (`WHEELGEM0-RV`)

Gem and grade rows follow the Wheel state candidate §3.2.1 (DUR-02 rules 10 and 19): stored data is
never reinterpreted under another ruleset. Every write stamps the world's current Wheel ruleset
revision on the gem or grade row it writes.

- **Current revision.** A row whose `ruleset_revision` is current loads and derives normally.
- **Non-current revision: fail closed.** The row is kept exactly as stored and never read against
  the current catalogues, values, compatibility rule or preceding-mod chain. Its mods, and the mod
  type of a grade row, contribute 0 to every effect of §9.3, count as Grade I for the preceding-mod
  cap and give no Grade IV point. Every Atelier action on such a gem or mod type (dismantle, switch
  domain, lock, grade up) and every vessel placement of such a gem is refused with
  `RULESET_MIGRATION_PENDING`, writing nothing; this applies to a new occurrence only, since a
  replay returns its receipt first (`WHEELGEM0-RP`, §5.2). A gem already in a vessel stays there, dormant.
- **Migration obligation (GEM-R).** A revision that changes mod identities, values, the
  compatibility rule, the grade chain, fees or yields ships exactly one of: (1) an explicit
  source-to-destination migration of gem and grade rows, validated against the destination
  catalogues and run as a DUR-02 rule 19 staged migration; (2) a declared-compatible mapping, a
  validated statement that both revisions read the stored rows identically, so the revision is
  re-stamped without changing any mod or grade. The release names which one applies; a revision
  without one cannot become current for a world holding rows under an older revision.
- **Architect ruling R3.** WHEEL-0's third option, reset with refund, is not admitted here: a
  reset destroys revealed gems and grades the player paid for, and a refund of gold or fragments
  would be a value source outside G1 and G2. Fail closed until a migration exists.
- The first revision has no migration; it lands with the first ruleset change.

## 6. Reveal (GEM-1)

- **Command** `reveal {quality}`: the first unreserved stack of the character's family gem of that
  quality in the main backpack direct entries, in B3 order (`PARITY_PENDING`: Global may search
  wider). Refusals: `NOT_ELIGIBLE`, `NO_GEM`, `ATELIER_FULL`, `NOT_ENOUGH_GOLD`, `STALE_REVISION`.
- **Shape.** One gem unit is a **BURN** (§17), keeping the stack (§11.1) or retiring it at zero
  (§11.5), under the new closed cause `GemAtelierCause::Reveal {gem_id, occurrence}`. The gem row
  is a Character write, not a MINT: it is not an asset and leaves the Atelier only through
  dismantling (§7), which names its own mint. The fee (D177, BANK-FEE-0) is in the same
  transaction: one TransactionId, one Atelier receipt, one CharacterRevision advance. Touched items:
  the gem stack and the coin inputs (all backpack entries, at most 20) plus 2 change outputs = 22.
- **Draw.** The RNG purpose `gem_reveal` (SIM-DETERMINISM-01 §10-§12) derives its seed from the
  protected server-controlled gameplay RNG root through the `gem_reveal` purpose substream, plus
  the occurrence, CharacterId and ruleset revision (`WHEELGEM0-RNG`). The root and substream are
  access-controlled, never sent to a client or ordinary telemetry (§28), so the command's public
  identity alone cannot predict the result before the fee is paid; the occurrence keeps a retry of
  the same command on the same draw. It draws in order: the domain (0-3), basic mod 1 from the slot-1
  list, basic mod 2 from the slot-2 list compatible with mod 1 (regular and greater), the supreme
  mod from the vocation's list (greater). The outputs are written in the receipt; a replay returns
  them and never draws again. A replay returns the outcome under the revision bound in its
  receipt (`WHEELGEM0-RP`, §5.2); only a new occurrence draws under the current revision.
- **Fees (D178, owner answer G1 a).** `FeeBurnCause` gains `GemReveal {gem_id, occurrence}`,
  `GemSwitchDomain {gem_id, occurrence}` and `GemGradeUp {kind, mod, to_grade, occurrence}`, all
  three admitted as in Tibia. Values are ruleset data; the first revision takes Canary's (§2) as
  `PARITY_PENDING`. Large fees fall to the bank part (BANK-FEE-0); short is `NOT_ENOUGH_GOLD`.
- **Initial gems (owner answer G2 b, `WHEELGEM0-INIT`).** As in Global (Canary `addInitialGems`,
  §2; the official detail `PARITY_PENDING`), each character receives 8 revealed gems of its
  vocation's family: one lesser and one regular gem per domain. One server-originated Atelier write
  keyed by (CharacterId, `GEM_INIT`), on CHAR-REV-SEQ-1 under the admitted session's fence (the
  STARTER-BACKPACK-0 variant, no CommandRef), is written at the first admission at which the
  character is Wheel-eligible (WHEEL-0 §6) with no `GEM_INIT` receipt or, for a session that
  becomes eligible while online, before its first `ATELIER_QUERY` or Atelier command. A read never
  writes. It inserts the 8 gem rows (8 planned gem ids) with no item line and no fee; the domain is
  fixed per gem, not drawn, and the mods are drawn under `gem_initial`, seeded as `WHEELGEM0-RNG`,
  in domain order, lesser before regular: basic mod 1 from the slot-1 list and, for the regular
  gem, basic mod 2 compatible with it. One Atelier receipt, one CharacterRevision advance. A replay
  returns the stored gems; it is written once per character, never again after dismantling or a
  lapse of eligibility. Until it exists, every Atelier command is refused `NOT_ELIGIBLE` and writes
  nothing.

## 7. Dismantle, switch domain, lock (GEM-1)

- **Dismantle** `{gem_id}`: refused if `LOCKED`, `IN_VESSEL`, `LAST_OF_DOMAIN` (the character's
  only revealed gem of that domain) or `RULESET_MIGRATION_PENDING` (§5.3). The gem row is deleted (the receipt keeps it), and fragments
  are a **MINT** (§14) under `GemAtelierCause::Dismantle {gem_id, occurrence}` into a compatible
  stack with room or a fresh entry planned in the transaction (§11.3); else `NO_ROOM`, nothing
  written. The count is drawn under `gem_dismantle`, seeded as `WHEELGEM0-RNG` (§6), from the
  manual's revealed ranges (lesser 1-3,
  regular 2-5 lesser fragments; greater 1-3 greater fragments). A value source admitted by
  owner answer G2 b.
- **Switch domain** `{gem_id}`: refused if `LOCKED`, `IN_VESSEL` (architect ruling R2) or
  `RULESET_MIGRATION_PENDING`; fee
  `GemSwitchDomain`; the domain moves one step clockwise.
- **Lock** `{gem_id, locked}`: a Character change with no item line and no fee; refused with
  `RULESET_MIGRATION_PENDING` (§5.3).

## 8. Grades and fragments (GEM-1, GEM-CRUSH-1)

- **Grade up** `{kind, mod, expected_grade}` raises one mod type one grade, I to IV (stored 0 to
  3). Any mod of the vocation's catalogue can be graded, as in Canary (`PARITY_PENDING`). Refusals:
  `MAX_GRADE`, `STALE_GRADE`, `NOT_ENOUGH_FRAGMENTS`, `NOT_ENOUGH_GOLD`, `RULESET_MIGRATION_PENDING`
  (§5.3). Grades never decrease.
- **Shape.** Lesser (basic) or greater (supreme) fragments are a **BURN** from backpack stacks in
  B3 order under `GemAtelierCause::GradeUp {kind, mod, to_grade, occurrence}`; the fee is
  `GemGradeUp`. Fragments and coins share the 20 backpack entries: 22 touched items.
- **Crushing (GEM-CRUSH-1).** A crusher used on an unrevealed gem is an item-only transaction
  (composition rule 1): one gem unit BURN, one crusher charge (STATE_MUTATION, retiring at 0), one
  fragment MINT under `GemAtelierCause::Crush {occurrence}`, drawn under `gem_crush`, seeded as
  `WHEELGEM0-RNG` (§6), from the manual's unrevealed ranges. It waits for ITEM-USE-0's item target. A value source admitted by owner answer G2 b. The
  amber crusher is a Store item and is not decided here.

## 9. Vessels and effects

### 9.1 Placement (GEM-VESSEL-1, amends WHEEL-0 §4)

- `game_character_wheel_vessels`: (`character_id`, domain 0-3) → `gem_id`, unique per gem.
- Placement is part of the Wheel configuration and is written by `commit_character_wheel` with
  the expected `wheel_revision`, in the same full replacement as the slots (Canary's save packet).
  The Wheel receipt gains before and after vessel vectors (4 gem ids each).
- Checks: the gem is the character's, its domain equals the vessel's (`WRONG_DOMAIN`), its row is
  under the current ruleset revision (`RULESET_MIGRATION_PENDING`, §5.3), one gem per vessel. The Atelier rows are read under the same `character_root` lock.
- **Where:** anywhere while eligible, in and out, as in Canary (architect ruling R1). Point
  removal keeps WHEEL-0's temple rule; a vessel change alone needs no temple.
- A vessel with no active resonance may hold a gem; its mods are dormant.

### 9.2 Resonance

A domain's resonance count is its number of full resonance slots (0-3), derived from the
allocation. A placed gem's active mods: basic 1 at 1, basic 2 at 2 (regular and greater), supreme
at 3 (greater).

### 9.3 Effects (GEM-FX-1)

- Each active mod adds its value at its effective grade (the stored grade, capped by the preceding
  mods' grades) through W-FX-1's one Wheel contribution: resistances, max health, mana, capacity,
  mitigation, dodge, critical damage, leech.
- Spell mods (damage, critical extra damage, healing, the Momentum chance of cooldown mods) enter
  the spell as augment inputs through the GAME-ABILITY-01 hooks, never through scattered checks.
- Revelation mastery mods add points to the domain sum that sets the revelation stage (WHEEL-0's
  superseding evidence).
- Each mod type at Grade IV adds 1 extra Wheel point (WHEEL-0 §6.3), derived from the grade rows.
- All are 0 while the character is not eligible, and for rows under a non-current ruleset revision
  (§5.3); the state stays stored.

## 10. Wire (GEM-WIRE-1)

- **Capability `WHEEL_GEM_V1`**, which requires `WHEEL_V1`; its number, two command types and one
  domain are reserved on #162 at allocation. `WHEEL_V1` is not revised, so W-2 can ship first.
- **`ATELIER_QUERY`:** revealed gems (id, family, quality, domain, mods, locked, in vessel), grades,
  `atelier_revision`, fragment counts in the main backpack.
- **`ATELIER_INTENT`:** `reveal`, `dismantle`, `switch_domain`, `set_lock`, `grade_up`, each with
  `expected_atelier_revision`. Results: `OK` and the refusals of §6-§8, plus the common results.
- **`WHEEL_INTENT set_allocation`** gains `vessels[4]` (a gem id or none), sent and read only
  under `WHEEL_GEM_V1`; without it the vessels are unchanged. New results `WRONG_DOMAIN` and
  `RULESET_MIGRATION_PENDING`.
- **Domain:** active mods and effective grades, so the client shows them.

## 11. Durable writes and causes

| Write | Class | CharacterRevision | Cause |
|---|---|---|---|
| Reveal: gem unit | BURN | +1 with the gem row | `GemAtelierCause::Reveal` |
| Reveal, switch, grade up: coins and bank | BURN, change MINT, `FEE_DEBIT` | same transaction | `FeeBurnCause::Gem*` (G1) |
| Dismantle: fragments | MINT | +1 with the row delete | `GemAtelierCause::Dismantle` (G2) |
| Grade up: fragments | BURN | +1 with the grade row | `GemAtelierCause::GradeUp` |
| Switch domain, lock | none (Character) | +1 | Atelier receipt |
| Initial gems | none (Character) | +1 | Atelier receipt `GEM_INIT` (G2) |
| Vessel change | none (Character) | +1 (Wheel receipt) | Wheel receipt |
| Crush: gem, crusher charge, fragments | BURN, STATE_MUTATION, MINT | none (rule 1) | `GemAtelierCause::Crush` (G2) |
| Drops, NPC fragment sale | existing | none | D3 loot; `NpcTrade` |

`GemAtelierCause` is closed. GEM-1 writes the DUR-03 §15 and §39.3 amendment and the gold
fee §4.4 variants admitted by G1 and G2; GEM-CRUSH-1 adds `Crush`. GEM-VESSEL-1 writes the WHEEL-0 §4 and §7
pointers once WHEEL-0 is on `main`. This PR edits none of them.

## 12. Rows (registered by the children before implementation)

| Row | Value |
|---|---|
| `WHEELGEM0-RL-01` gem qualities, mods | 3; 1, 2, 3 mods |
| `WHEELGEM0-RL-02` revealed gems per character | 250 (`PARITY_PENDING`) |
| `WHEELGEM0-RL-03` vessels | 4, one per domain |
| `WHEELGEM0-RL-04` resonance slots per domain | 3 |
| `WHEELGEM0-RL-05` grades | I to IV, stored 0 to 3 |
| `WHEELGEM0-RL-06` reveal and switch touched items | 22 (20 backpack entries + 2 change) |
| `WHEELGEM0-RL-07` dismantle and crush touched items | 1 and 3 |
| `WHEELGEM0-RL-08` grade up touched items | 22 |
| `WHEELGEM0-RL-09` Atelier receipt | 2 gem rows or 2 grade rows (8 gem rows for `GEM_INIT`), RNG outputs, fixed fields |
| `WHEELGEM0-RL-10` Atelier changes in flight per character | 1 (CHAR-REV-SEQ-1) |
| `WHEELGEM0-RL-11` fragments per dismantle | 1-5 (manual ranges) |
| `WHEELGEM0-RL-12` initial gems per character | 8, once (1 lesser and 1 regular per domain) |

## 13. Owner questions (answered 2026-09-30; fee and value questions under D178 and D208)

**G1. Gold fees of the Gem Atelier (D178).** Three new fee sources: revealing a gem (125,000,
1,000,000, 6,000,000 by size), switching its domain (125,000, 250,000, 500,000) and grading a mod
up (2 to 75 million a grade). These are the reference server's values; the manual has none.
a) Admit all three as in Tibia, with these values until official ones are sourced (recommended:
without the reveal fee no gem can be used, and these are the gold sinks the Wheel is balanced on);
b) admit reveal and grade up only, no domain switch; c) no gold fees: gems reveal for free.
Owner answer (2026-09-30, #162): a — as in Tibia: the reveal, domain switch and grade upgrade fees
are all admitted.

**G2. New value sources of fragments and gems (D208).** Dismantling a revealed gem and crushing an
unrevealed gem create fragments; the reference server also gives every character 8 free revealed
gems. a) Admit dismantling and crushing as in Tibia, no free gems until an official source is found
(recommended: the manual names dismantling and crushing, not free gems); b) admit all three;
c) no new source: fragments only from NPCs and trade.
Owner answer (2026-09-30, #162): b — as in Tibia: dismantling, crushing and the 8 free initial
gems are all admitted (§6 `WHEELGEM0-INIT`).

## 14. Rejected options

- **Revealed gems as items (TRANSFORM, PRESERVE_INSTANCE).** A revealed gem cannot be traded or
  moved; a custody family for items that never move carries DUR-03 cost with no value.
- **BURN plus a MINT of a revealed-gem item.** The same objection, and a MINT would count an
  untradeable thing as value.
- **CONVERSION for reveal.** The output is not an asset with a conserved quantity.
- **Grades on each gem.** The manual shares them per mod type across the character's gems.
- **Extending `WHEEL_V1` in place.** It would hold W-2 behind gems.
- **Canary's yields and missing dismantle checks.** The manual gives the ranges and refusals.
- **Mods drawn at display or at load.** Only a draw bound to the occurrence is replay-safe.
- **A reveal seed from public values only.** The player could compute the result before paying.
- **Reset with refund on a ruleset change (R3).** It destroys paid state and mints unadmitted value.

## 15. Owner-rule applications (Global parity, 5905825574)

**Kept:** three sizes with 1-3 mods; vocation-locked reveal; tradeable unrevealed and bound revealed
gems; domain switch; lock; four vessels with resonance; dismantle refusals and yields; grades I-IV
shared per mod type with lesser and greater fragments; +1 point per Grade IV mod; Momentum; the 8
initial gems.

**Architect rulings:**
- **R1.** Vessel changes are allowed anywhere while eligible (Canary; the manual names no place).
  a) Anywhere (ruled); b) at a temple only. Ruled a): the only evidence, and no value moves.
- **R2.** A gem in a vessel cannot switch domain. Canary allows it and leaves a gem in the wrong
  vessel; refusing keeps the vessel check true without a silent unplace.

**Declared differences (`PARITY_PENDING`):** gems and fragments are found in main backpack direct
entries only; the 250 gem cap; any catalogue mod can be graded; no favourite flag; the initial
gems' timing and mod draw (Canary).

## 16. Decision test

- **Must decide now:** YES. The owner asked for the Wheel at full parity now, and WHEEL-0 sent gems
  here.
- **Minimum sufficient:** four small Character tables, one writer on the existing sequencer, one
  closed item cause, three fee variants, one vessel table in the existing Wheel writer, one
  capability; items, loot, NPC trade, fees and the effect path are reused.
- **Superseding evidence:** official reveal, switch and grade values; a Global gem cap; the
  official starter gem detail; where vessels may change.
- **Deliberately not decided:** fiendish drops, the amber crusher, presets, vocation change.

## 17. Before-freeze checklist

1. **Contract amendments:** the Wheel state candidate §5 pointer here; WHEEL-0 §4 and §7, DUR-03
   §15 and §39.3 and gold fee §4.4 by the children, "pending on acceptance of WHEEL-GEM-0".
2. **Serialization:** every Atelier and vessel change on CHAR-REV-SEQ-1; fence and
   `character_root` first; expected `atelier_revision` or `wheel_revision`.
3. **Restart:** receipts keyed by occurrence; planned gem ids and output slots; RNG outputs in the
   receipt, seeded from the protected root (`WHEELGEM0-RNG`); replays return the first outcome;
   the runtime projection refreshed before `OK` (`WHEELGEM0-RT`); non-current ruleset rows fail
   closed (`WHEELGEM0-RV`).
4. **Typed references:** CharacterId, gem id, mod keys, ruleset revision, occurrence, TransactionId.
5. **Wire:** §10, capability `WHEEL_GEM_V1`, numbers reserved on #162 at allocation.
6. **Owner:** G1 a) and G2 b) answered and recorded on #162 (2026-09-30).
