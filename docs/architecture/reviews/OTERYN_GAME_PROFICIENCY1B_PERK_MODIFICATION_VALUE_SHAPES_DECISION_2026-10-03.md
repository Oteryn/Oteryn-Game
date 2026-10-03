# PROFICIENCY-1B Perk modification value shapes

- Decision: `PROFICIENCY1B-PERK-MODIFICATION-VALUE-SHAPES-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence,
  economy, determinism and protocol) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3), second architect lane
- Answers:
  - control-plane allocation D351 (#1622, owner 1b);
  - PROFICIENCY-1 §3, §4 and §6 (the value-shape amendment with fourteen entry conditions);
  - PROF-WIRE-0 §3 and §5 (PROFICIENCY-1's rows re-measure RL-01; its commands join capability 2).
- Builds on:
  - PROFICIENCY-0 §4.1-§4.3 (the track row, receipts, lines, guards, the writer and the
    STANCE-1 occurrence rule);
  - PROFICIENCY-1 §2-§6 (the state shape, the reserved names, the operation list, the gate);
  - PROF-WIRE-0 (capability 2, domain 6, the revision rule, RL-01 to RL-05);
  - PROF-EFFECT-0 (perk effects);
  - IMBUE-FORGE-0 §9-§10 (forge dust, its ledger and lock, the `REVISION_CHANGED` terminal record);
  - DUR-03 §11.5, §15, §18, §24, §28 and §39.3; ITEM-USE-0 §3 and §4 (finding a stack by
    definition, the one-unit burn);
  - SIM-DETERMINISM-01 (named RNG purposes, revision binding);
  - migration 0032 (`game_character_proficiency*`, the shared progression guard);
  - the gold fee decision (D178); owner rule 5905825574.
- Amends, each pending on acceptance of PROFICIENCY-1B and written by PROF-SHAPE-1 or
  PROF-SHAPE-WIRE-1 in its own docs commit:
  - PROFICIENCY-0 §4.2 (the `perk_modification` cause) and §4.5 (pointer);
  - PROFICIENCY-1 status line (its entry conditions are answered here);
  - PROF-WIRE-0 §3 (RL-01 and RL-02 re-measured) and §4 (`MODIFIED_LEVEL`);
  - DUR-03 §15 and §39.3 (`ProficiencyCause` shapes); IMBUE-FORGE-0 §9 (the dust sink admitted).
- Runtime, migration and production authority: NONE. Each child needs its own #1622 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| PROF-SHAPE-CONTENT-1 | content lane | the shaping content family (§3): pools, odds, per-rank values and costs, each cell with its evidence class; the Lunar Ascension Orb definition (`i53695`) | PROF-CONTENT-1 |
| PROF-SHAPE-1 | hard (persistence), persistence, economy and determinism review | the modification table and lines (§4), the six operations (§5), the receipts and terminal records (§6), the composed shapes (§7), the retention check (§8), the integrity extensions (§9), `MODIFIED_LEVEL` (§10) | PROF-1; PROF-2; FORGE-1 (the dust balance and ledger); PROF-SHAPE-CONTENT-1 |
| PROF-SHAPE-WIRE-1 | hard (protocol), protocol review | the six commands, their results and payloads, the extended snapshot and delta, re-measured RL-01 and RL-02 (§11) | PROF-WIRE-1; PROF-SHAPE-1 |

Every operation ships answering `NOT_ADMITTED` until its value cells are evidenced (§3.3). Building
the commands, the table and the receipts does not wait for evidence; admitting an operation does.

## 1. Question

How do the six perk modification operations of PROFICIENCY-1 run, what do they write, what do
they return, and when may each be admitted?

## 2. Facts

**PROVEN**

- Tibia manual (`CIPSOFT_OFFICIAL`, `combat.md` §5.3.4; PROFICIENCY-1 §2): modification happens
  only in a protection zone, rerolls the current perk to a new random one and costs dust; at most
  two modified perks per weapon, the first at proficiency level 3, the second at Mastery; a rank
  1-10 rises one step for dust or straight to 10 with a Lunar Ascension Orb; a reshape offers 3 new
  perks at the same rank and may be declined; clearing restores the unmodified perk.
- English TibiaWiki (`Weapon_Proficiency`, read 2026-10-01, PROFICIENCY-1 §2): modifying the first
  perk costs 250 dust, the second 1,000 dust; the Lunar Ascension Orb is item `i53695`.
- IMBUE-FORGE-0 §9: dust is a capped per-Character balance with an immutable ledger (`SPEND`).
- Migration 0032: receipts and lines carry `cause IN ('training', 'perk_selection', 'migration')`;
  the per-track check links each line to the previous one; the track row equals its latest line.

**UNKNOWN** (hard parity gates; TibiaWiki was unreachable from this lane's container, HTTP 402)

- the modification pool and its odds; the value of each perk per rank;
- the dust cost of each rank step, of a reshape offer and of a clear;
- whether the orb is consumed per use; any gold cost (none is known).

Canary's shaping opcodes are no-ops and Crystal's are empty; neither is evidence.

## 3. Shaping content (PROF-SHAPE-CONTENT-1)

### 3.1 The family

`ProficiencyShaping` is a content definition family, one definition per Proficiency definition,
keyed `oteryn:proficiency-shaping.tibia.p<ProficiencyId>`, with its own **shaping revision**. It
holds:
- **pool:** an ordered list of entries; each entry is a perk option of a closed PROFICIENCY-0 perk
  kind (its full non-value identity, §4.2 of PROFICIENCY-0) with a draw weight;
- **rank values:** for each entry, its numeric values at ranks 1 to 10;
- **costs:** dust for MODIFY per slot, for each rank step 1→2 … 9→10, for a reshape offer and for a
  clear; and the number of orbs ORB_RANK burns (1 if consumed, 0 if not);
- for each cell, its **evidence class** and source reference.

### 3.2 Entry identity

A modified perk is `(shaping revision, entry index)`. The entry index is stable within a revision.

### 3.3 Admission by evidence (entry condition 9)

- A cell is **evidenced** when its class is, in this order, `CIPSOFT_OFFICIAL`, owner-verified
  TibiaPal or tibiatools.io, or English TibiaWiki. OTS sources are never evidence here.
- An operation is **admitted** for a shaping definition only when every cell it reads is evidenced:

  | Operation | Cells it reads |
  |---|---|
  | `MODIFY` | the slot's dust cost, the pool and its weights, rank 1 values |
  | `RANK_UP` | the step's dust cost, the next rank's values |
  | `ORB_RANK` | the orb count, rank 10 values |
  | `RESHAPE_OFFER` | the offer's dust cost, the pool and its weights, and every pool entry's values at the row's current rank (any entry can be offered and chosen, so none may activate an unevidenced value) |
  | `RESHAPE_CHOOSE` | nothing: it is admitted whenever the row has a pending offer, whatever the active revision's cells (the offer was admitted and paid under the row's own revision) |
  | `CLEAR` | the clear's dust cost |

- A command for an operation not admitted answers `NOT_ADMITTED` (PROF-WIRE-0 §5) and writes
  nothing. Admission is read from the active shaping revision when the occurrence is reserved,
  except `RESHAPE_CHOOSE`, which follows the pending offer, so a paid offer is never stranded.
- **No gold.** No cost cell may be gold. A gold cost is a new fee source and needs an owner answer
  (D178) and an amendment of this decision.
- If evidence never appears for a cell, the architect puts a declared difference to the owner in
  one batch (PROFICIENCY-1 §4). None is put now: the cells are unread, not contradicted.

## 4. State (PROF-SHAPE-1, entry condition 13)

### 4.1 The modification table

`game_character_proficiency_modifications`, PROFICIENCY-1 §3's shape, created in the same migration
as the operations that write it:

| Column | Meaning |
|---|---|
| `character_id`, `item_key` | the track; FK to `game_character_proficiency` |
| `slot` | 1 or 2 (`PROF1-RL-01`) |
| `level` | the proficiency level whose perk is modified, or NULL when cleared |
| `shaping_key`, `shaping_revision` | the shaping definition and revision of the modified perk, NULL when cleared |
| `entry_index` | the modified perk's entry, NULL when cleared |
| `rank` | 1..10 (`PROF1-RL-02`), NULL when cleared |
| `pending_offer` | NULL, or exactly 3 distinct entry indexes of the row's shaping revision |
| `committed_character_revision`, `last_proficiency_occurrence_id` | the receipt that last changed the row |

- **Keys.** Primary key `(character_id, item_key, slot)`; UNIQUE `(character_id, item_key, level)`
  (NULLs distinct): at most 2 rows per track, and two slots never modify the same level.
- **Rows are never deleted.** `CLEAR` sets the row to the cleared state (all of `level`,
  `shaping_*`, `entry_index`, `rank`, `pending_offer` NULL). A CHECK makes those columns all NULL or
  all non-NULL (`pending_offer` may be NULL either way, but must be NULL when cleared). DELETE and
  TRUNCATE are rejected; grants as 0032 (runtime SELECT, INSERT, UPDATE; control SELECT).
- **Unlock and activity** (PROFICIENCY-1 §3): slot 1 is unlocked while the derived level is at
  least 3, slot 2 while the track has Mastery; the modified level must have a selection. An
  unlocked, non-cleared row whose level still has a selection is **active**: its perk, at its rank
  and evaluated against its own stored shaping revision, replaces that level's selected perk for
  PROF-EFFECT-0. Any other row is stored and inactive.
- **Index** `(shaping_key, shaping_revision)` for the retention check (§8).

### 4.2 Modification lines

`game_character_proficiency_modification_lines`, immutable, one line per changed row:
- the receipt key `(proficiency_occurrence_id, character_id, committed_character_revision, cause)`
  with a composite FK to the receipt, as 0032's lines;
- `item_key`, `slot`, `operation` (`MODIFY`, `RANK_UP`, `ORB_RANK`, `RESHAPE_OFFER`,
  `RESHAPE_CHOOSE`, `RESHAPE_DECLINE`, `CLEAR`, `MIGRATION_CLEAR`);
- every row column before and after (absent before the row's first line: the cleared state);
- `dust_spent` and `orbs_spent` (0 or more).

### 4.3 Receipts

- The receipt and line CHECKs of 0032 gain the cause **`perk_modification`** (the reserved name).
- A `perk_modification` receipt has **exactly one track line** and **exactly one modification
  line**. Its track line's CHECK: progress, selections, definition key and definition revision are
  unchanged (the line only advances the track's `committed_character_revision`, so the per-track
  wire revision moves, PROF-WIRE-0 §2).
- A `migration` receipt (PROFICIENCY-0 §4.1, PROFICIENCY-1 R2) carries one `MIGRATION_CLEAR`
  modification line for each modified row whose level the migration changes; the row becomes
  cleared with no refund.
- No other cause carries a modification line.

## 5. Operations (PROF-SHAPE-1)

Every operation is a command of one track and one slot, from an actor standing in a protection
zone, under the PROFICIENCY-0 §4.3 writer. Checks run in this order and each refusal writes nothing
(except §6.3's terminal record):
1. capability 2 negotiated (else `CAPABILITY_MISMATCH`, PROF-WIRE-0 §4);
2. replay lookup by occurrence (§6.1): a known occurrence returns its original result and is never
   charged against the cap;
3. the rate cap `PROF1B-RL-03`, charged when a new occurrence is reserved;
4. revision binding (§6.2);
5. the operation admitted (§3.3), else `NOT_ADMITTED`;
6. the track exists, else `UNKNOWN_TRACK`; `expected_revision` equals the track's committed
   revision, else `STALE_REVISION`;
7. protection zone, else `NOT_IN_PROTECTION_ZONE`;
8. the operation's own checks below;
9. the cost: dust balance at least the cost, else `INSUFFICIENT_DUST`; an orb stack, else `NO_ORB`.

| Operation | Payload | Own checks (result) | Effect |
|---|---|---|---|
| `MODIFY` | item, slot, level, expected revision | slot unlocked (`NOT_UNLOCKED`); the row is absent or cleared (`SLOT_OCCUPIED`); the level has a selection (`NO_SELECTION`); no other slot modifies it (`LEVEL_TAKEN`); at least one pool entry differs from the level's selected perk identity (`POOL_TOO_SMALL`) | draw one entry (§5.1) other than the selected perk; rank 1; burn the slot's dust |
| `RANK_UP` | item, slot, expected revision | modified (`NOT_MODIFIED`); rank < 10 (`RANK_MAX`); no pending offer (`OFFER_PENDING`); row shaping revision active (`SHAPING_OUTDATED`) | rank + 1; burn the step's dust |
| `ORB_RANK` | item, slot, expected revision | as `RANK_UP` | rank = 10; burn the orb count (0 or 1) |
| `RESHAPE_OFFER` | item, slot, expected revision | modified (`NOT_MODIFIED`); no pending offer (`OFFER_PENDING`); row shaping revision active (`SHAPING_OUTDATED`); at least 3 pool entries other than the current one (`POOL_TOO_SMALL`) | draw 3 distinct entries (§5.1), none the current one; store them as `pending_offer`; burn the offer's dust |
| `RESHAPE_CHOOSE` | item, slot, choice 0..2 or decline, expected revision | a pending offer (`NO_OFFER`) | choose: `entry_index` = the chosen entry, rank kept, offer cleared (`RESHAPE_CHOOSE`); decline: offer cleared, perk kept (`RESHAPE_DECLINE`); no cost |
| `CLEAR` | item, slot, expected revision | modified (`NOT_MODIFIED`) | row cleared, pending offer dropped without refund; burn the clear's dust |

- **Rank operations are refused while an offer is pending** (entry condition 4). A pending offer
  is durable and already paid; only `RESHAPE_CHOOSE` or `CLEAR` ends it.
- **Older shaping revisions** (entry condition 5): an operation that draws or ranks (`MODIFY`
  aside, which always uses the active revision) on a row of an older shaping revision is refused
  `SHAPING_OUTDATED`; `RESHAPE_CHOOSE` of an offer drawn under the row's own revision and `CLEAR`
  stay allowed. No implicit migration exists.
- **The orb** is found as ITEM-USE-0 field 5 finds a stack: the first matching unreserved stack of
  `i53695`, equipment slots first, then the main backpack's direct entries in B3 order, then nested
  containers breadth-first (BAGS-0 amendment of ITEM-USE-0 §3), read limit 509.

### 5.1 Draws (entry condition 6)

- Draws use the SIM-DETERMINISM-01 purpose **`proficiency_shaping`**, keyed by the occurrence and a
  server seed the client cannot derive, under the bound `SimulationDeterminismProfileRevision`.
- `MODIFY` draws one entry by weight from the pool without the excluded entry; `RESHAPE_OFFER` draws
  3 entries by weight without replacement and without the current entry, in draw order.
- The outcome is stored in the line, so a replay never redraws.

## 6. Occurrence, binding and replay (PROF-SHAPE-1)

### 6.1 Occurrence (entry condition 1)

- The occurrence derives from the CommandId exactly as PROFICIENCY-0 §4.3 (the STANCE-1 rule).
- The writer looks up a receipt, then a terminal record (§6.3), by occurrence **before any write**.
  Found with the same binding: the original result is returned. Found with a different binding:
  `CONFLICT`, nothing written.
- The binding holds the operation, item key, slot, level (MODIFY), choice (RESHAPE_CHOOSE),
  expected revision and the bound revision set.

### 6.2 Revision binding (entry condition 2)

- At reservation the occurrence binds the track's **definition revision**, the active **shaping
  revision** and the **`SimulationDeterminismProfileRevision`**.
- If any differs when the commit is attempted (including a retry after a known non-committed
  abort), the occurrence is refused `REVISION_CHANGED` and persisted as a terminal record (§6.3).
  It is never re-evaluated under a newer set, even if the set later matches again.
- The rate cap is charged at reservation, so a terminal write is never refused by it.

### 6.3 Terminal records

- `game_character_proficiency_modification_terminals`, keyed `(proficiency_occurrence_id,
  character_id)`, holds the binding, the bound revision set and the refusal (`REVISION_CHANGED`).
- It is the receipt of a refused occurrence (IMBUE-FORGE-0 §10): zero lines, no dust, no item, no
  row change and **no `CharacterRevision`**, so it never enters the 0009 chain.
- Every later replay of that occurrence returns the same refusal. If its write is ambiguous, the
  occurrence stays refused until reconciliation reads the record (`PROF1B-RL-05`).

### 6.4 Losing writer

A stale commit (another receipt advanced the root or the track) wrote nothing. It is re-validated
against the committed rows and, still valid, retried once under the **same** command-derived
occurrence and the same draw, so a later duplicate of the command finds its receipt by that
occurrence (§6.1) and never pays twice. If the track's committed revision no longer equals the
command's `expected_revision`, or the re-validation fails, the command answers `STALE_REVISION`
and nothing is persisted; a duplicate re-evaluates against the same rows and gets the same answer.
This deliberately differs from PROFICIENCY-0 §4.3's new-occurrence retry for `perk_selection`,
because a modification spends value.

## 7. Receipt CHECKs and composed shapes (PROF-SHAPE-1)

### 7.1 Line CHECKs (entry condition 3)

Per operation, only the fields it may change change:

| Operation | May change | Must hold |
|---|---|---|
| `MODIFY` | from cleared or absent to `level`, `shaping_*`, `entry_index`, `rank` = 1 | `pending_offer` NULL after; `dust_spent` > 0 when its cost is |
| `RANK_UP` | `rank` + 1 | everything else equal; offer NULL before and after |
| `ORB_RANK` | `rank` to 10 | everything else equal; offer NULL before and after; `orbs_spent` equals the content count |
| `RESHAPE_OFFER` | `pending_offer` from NULL to 3 distinct indexes, none equal to `entry_index` | everything else equal |
| `RESHAPE_CHOOSE` | `entry_index` to one of the offer's indexes; offer to NULL | rank, level, shaping equal; no cost |
| `RESHAPE_DECLINE` | offer to NULL | everything else equal; no cost |
| `CLEAR`, `MIGRATION_CLEAR` | to the cleared state | `MIGRATION_CLEAR` spends nothing |

Exactly one slot changes per `perk_modification` receipt. The per-track check (0032) extends to the
modification row: each line's before values equal the previous modification line's after values of
that (track, slot), or the cleared state; the row equals its latest line.

### 7.2 Composed DUR-03 shapes (entry condition 7)

- **Dust burn:** the receipt (CharacterRevision + 1), its two lines, and one dust ledger `SPEND`
  entry with its balance update, under `ProficiencyCause {track, slot, operation, occurrence}`. The
  dust is burned (DUR-03 §15 sink), no item touched.
- **Orb burn:** the receipt, its lines and, only when the evidenced orb count is 1, one BURN of one
  unit of the found orb stack (it keeps its identity, or retires at zero, DUR-03 §11.1 and §11.5),
  under the same cause. With a count of 0 the orb must still be held (`NO_ORB` otherwise, a
  read-only check under the same lock order) and no item line is written; the shape is then the
  receipt with its lines only.
- **Lock order:** the item writer's and session-generation fences, `character_root`, the track row,
  the modification rows, the dust balance row, then the orb stack.
- **Audit:** one audit event per receipt with the track, slot, operation, rows before and after,
  the draw outcome, the dust ledger entry and the orb line.
- **Rows** (DUR-03 §28), registered by PROF-SHAPE-1 with max and max+1 tests:

  | Row | Value |
  |---|---|
  | `DUR03-RL-01-PROF` touched items | 1 (orb shape), 0 (dust shape) |
  | `DUR03-RL-02-PROF` location lines | 1 (an orb stack retired at zero) |
  | `DUR03-RL-03-PROF` value lines | 1 (dust) |
  | `DUR03-RL-06-PROF` participants / work units | 1 / 4 |
  | `DUR03-RL-07` envelope and payload | unchanged; measured on the orb shape |

## 8. Retention check (entry condition 10)

- **Invariant** (PROFICIENCY-1 §3): a shaping revision is retained while any row references it, and
  a row is evaluated against its own stored revision.
- **Serialized enforcement.** Content activation of a shaping definition takes an exclusive
  transaction-scoped advisory lock on `proficiency_shaping:<shaping_key>`; every modification write
  takes it shared until commit. Under the exclusive lock, activation probes the §4.1 index for every
  revision it would drop and is refused if one is referenced. A write referencing a revision checks,
  under the shared lock, that it is in the retained set. So no write can reference a revision being
  retired, and no activation can drop a referenced one.
- Tests: an activation dropping a referenced revision is refused; a concurrent write and
  activation never leave a row on a dropped revision.

## 9. Integrity (entry condition 11)

- The shared progression guard (0032's `game_character_progression_consistency_guard`) counts
  `perk_modification` receipts in its chain. The migration replaces the guard with the **union** of
  every body merged before it (D325 precedent), with a PostgreSQL ordering test.
- The per-track check covers modification lines and rows (§7.1).
- `verify_character_integrity` and `reconcile_character_proficiency` extend to the modification
  rows, lines and terminal records, so each row equals the after value of its latest line, and a
  terminal record never coexists with a receipt of the same occurrence.

## 10. Selections at modified levels (entry condition 14)

`PROFICIENCY_SELECT_PERK` refuses a change or clear at a level with an active modification with
`MODIFIED_LEVEL`, a new result code in PROF-WIRE-0 §4. Tested. Clearing the modification first
re-opens the level.

## 11. Wire (PROF-SHAPE-WIRE-1, entry conditions 8 and 12)

### 11.1 Commands

Six command types under capability 2, numbers leased by the control plane (D353):

| Command type | Payload | Measured bound |
|---|---|---|
| 15 `PROFICIENCY_MODIFY` | item id, slot, level, expected revision | 19 B |
| 16 `PROFICIENCY_RANK_UP` | item id, slot, expected revision | 17 B |
| 17 `PROFICIENCY_ORB_RANK` | item id, slot, expected revision | 17 B |
| 18 `PROFICIENCY_RESHAPE_OFFER` | item id, slot, expected revision | 17 B |
| 19 `PROFICIENCY_RESHAPE_CHOOSE` | item id, slot, choice (0..2, or 3 = decline), expected revision | 19 B |
| 20 `PROFICIENCY_CLEAR` | item id, slot, expected revision | 17 B |

Derivation, worst-case protobuf: item id tag + varint ≤ 4 B; slot, level and choice tag + 1 B each;
expected revision tag + varint ≤ 11 B. `PROF1B-RL-01` registers 24 B for each, as PROF-WIRE-0
RL-03.

### 11.2 Results

One closed result enum, shared by the six commands, at most 16 B (PROF-WIRE-0 RL-04): `ACCEPTED`,
`NOT_ADMITTED`, `NOT_UNLOCKED`, `NOT_IN_PROTECTION_ZONE`, `UNKNOWN_TRACK`, `STALE_REVISION`,
`REVISION_CHANGED`, `RATE_LIMITED`, `CONFLICT`, `NO_SELECTION`, `SLOT_OCCUPIED`, `LEVEL_TAKEN`,
`NOT_MODIFIED`, `RANK_MAX`, `OFFER_PENDING`, `NO_OFFER`, `POOL_TOO_SMALL`, `SHAPING_OUTDATED`,
`INSUFFICIENT_DUST`, `NO_ORB`. An accepted `RESHAPE_OFFER` is visible through the domain delta (the
pending offer), not the result. These are command results, not protocol errors.

### 11.3 Domain

- Each track entry of `ACTOR_PROFICIENCY` gains up to 2 modification entries: slot, level, shaping
  revision index, entry index, rank and the pending offer (3 indexes). A cleared row is omitted.
- **Re-measured bounds.** One modification entry: tag and length 2 B, slot 2 B, level 2 B,
  shaping revision index tag + varint ≤ 5 B, entry index ≤ 3 B, rank 2 B, offer packed (tag,
  length, 3 × ≤ 2 B) ≤ 8 B: 24 B. A track entry grows from 36 B to at most 84 B. 666 tracks:
  55,944 B plus a 32 B header. So:
  - `PROFWIRE0-RL-01` snapshot: **65,536 B** (was 32,768), sent through `SnapshotChunk`, 15% margin;
  - `PROFWIRE0-RL-02` delta: **1,408 B** (16 × 84 B plus header; was 640).
  PROF-SHAPE-WIRE-1 measures the encoded worst case and fails the build above them.

## 12. Rows

| Row | Value |
|---|---|
| `PROF1-RL-01` modification slots per track | 2 (registered here) |
| `PROF1-RL-02` rank | 1..10 (registered here) |
| `PROF1B-RL-01` each modification command payload | 24 B |
| `PROF1B-RL-02` pending offer entries | 3 |
| `PROF1B-RL-03` modification commands per character | 2 per second, charged at reservation |
| `PROF1B-RL-04` pool entries per shaping definition | 64 (content validation refuses more) |
| `PROF1B-RL-05` ambiguous-commit bound before a reconciliation hold | 2,000 ms |

Each with max and max+1 tests.

## 13. Entry conditions and tests

| PROFICIENCY-1 §6 condition | Answered by | Tests (PROF-SHAPE-1 unless named) |
|---|---|---|
| 1. occurrence from the CommandId; replay before any write | §6.1 | a retry returns the first receipt; a different binding is `CONFLICT`; a stale commit retried under the same occurrence, then a duplicate of the command, spends dust once and returns that receipt; a duplicate arriving under an exhausted rate cap still gets its original result |
| 2. revision binding, terminal `REVISION_CHANGED`, never refused by a rate cap | §6.2, §6.3 | a shaping activation between reservation and commit gives a terminal record; its replay after a rollback still refuses; a capped character still gets the terminal record |
| 3. receipt CHECKs | §4.3, §7.1 | each operation changing a forbidden field is refused by the CHECK; two slots in one receipt are refused |
| 4. offers durable, paid when drawn, ≥ 3 others, exactly 3; rank refused while pending | §5 | an offer survives a restart; a pool of 3 entries (2 others) refuses before any burn; `RANK_UP` with an offer is `OFFER_PENDING` |
| 5. older shaping revisions | §5 | `RANK_UP` on an old-revision row is `SHAPING_OUTDATED`; `CLEAR` works |
| 6. draws under `proficiency_shaping` | §5.1 | the same occurrence and seed draw the same entries; a replay never redraws |
| 7. composed shapes, lock order, audit | §7.2 | each DUR-03 row at max and max+1; the dust balance and the receipt commit together or not at all; with an orb count of 0 the orb is required but not burned |
| 8. measured payloads and snapshot | §11 | PROF-SHAPE-WIRE-1: each payload at its bound; the 666-track worst case under RL-01 |
| 9. values by the evidence order | §3.3 | an operation with an unevidenced cell answers `NOT_ADMITTED`; an OTS-classed cell is refused by content validation; `RESHAPE_OFFER` is `NOT_ADMITTED` while any pool entry lacks a value at the row's rank; a paid offer stays choosable after a newer revision is activated |
| 10. retention serialized with activation | §8 | the §8 tests |
| 11. integrity extended | §9 | `verify_character_integrity` fails a row that differs from its latest line; the guard union passes the ordering test |
| 12. commands, numbers, results, payloads | §11 | PROF-SHAPE-WIRE-1: each command and result round-trips |
| 13. the table with keys, unlock rule, retention invariant and migration clear | §4 | an incompatible migration clears a modified level with no refund; a slot above the unlocked level is inactive and kept |
| 14. `MODIFIED_LEVEL` | §10 | the §10 test |

## 14. Rejected options

- **One generic command with an operation field.** PROFICIENCY-1 §6 item 12 and PROF-WIRE-0 §5 ask
  for a command per operation; separate payloads stay smaller and their results narrower.
- **Terminal records in the receipt chain.** A receipt advances the CharacterRevision; a refused
  occurrence changes nothing, so its record stays outside the chain.
- **Deleting a cleared row.** Rows are never deleted, as track rows; the cleared state keeps the
  per-track line chain unbroken.
- **Implicit migration of an older shaping revision.** Refusing ranks and draws on it, while
  `CLEAR` stays open, needs no compatibility rule.
- **Admitting with OTS values.** PROFICIENCY-1 R1.

## 15. Architect rulings (owner rule 5905825574)

- **R1. Gate.** a) Build all six now and admit each by evidenced cells (recommended); b) build each
  only when evidenced. **Ruled a)**: the shapes, receipts and wire are evidence-independent.
- **R2. Snapshot bound.** a) Raise RL-01 to 65,536 B (recommended); b) cap modified tracks. **Ruled
  a)**: no gameplay cap exists in Tibia.
- **R3. Offer while one is pending.** a) Refuse `OFFER_PENDING` (recommended); b) replace it. **Ruled
  a)**: a paid offer is never discarded by a second payment.

## 16. Owner questions

None. No gold cost is proposed. Missing values keep their operations closed; if evidence never
appears, they come back in one batch.

## 17. Decision test

- **Must decide now:** YES (D351). PROFICIENCY-1 blocks every modification on this decision.
- **Blocked:** PROF-SHAPE-CONTENT-1's schema, PROF-SHAPE-1, PROF-SHAPE-WIRE-1.
- **Minimum sufficient:** one table, one line table, one terminal table, six commands, one result
  enum, two composed shapes.
- **Harder later:** six command numbers become permanent registry identities; the
  `perk_modification` CHECKs and the raised RL-01 bind clients and later receipts.
- **Superseding evidence:** official or owner-verified pools, odds, values and costs.
- **Deliberately not decided:** catalysts (PROFICIENCY-1 §4), Mastery display, any gold cost.

## 18. Before-freeze checklist

1. **Contract amendments:** listed in the header; each written by its child.
2. **Serialization:** the PROFICIENCY-0 writer under `character_root`; §7.2 lock order; §8 advisory
   lock with activation.
3. **Restart:** pending offers are durable; terminal records stay terminal.
4. **Typed references:** track key, shaping key and revision, entry index, ItemInstanceId of the
   orb stack.
5. **Wire:** six commands under capability 2, one result enum, the extended domain.
6. **Leases:** migration `0055` (D353), command types 15-20 (D353).

## 19. Implementation packets

### PROF-SHAPE-1

```yaml
task_id: OTV2-YYYYMMDD-prof-shape-1
decision: PROFICIENCY1B-PERK-MODIFICATION-VALUE-SHAPES-V1 §4-§10, §12-§13
worker: oteryn-hard-worker (persistence, durable value)
review: independent persistence, economy and determinism review
depends_on: [PROF-1, PROF-2, FORGE-1, PROF-SHAPE-CONTENT-1, this decision accepted]
migration_lease: 0055
owned_paths:
  - apps/game-server/migrations/0055_character_proficiency_modification.sql
  - apps/game-server/src/durability/character_proficiency_modification.rs
  - apps/game-server/src/durability/character_proficiency.rs      # the perk_modification cause
  - apps/game-server/src/durability/character_authority.rs        # verify_character_integrity arm
  - apps/game-server/src/durability/content_activation.rs         # §8 retention lock
  - apps/game-server/src/durability/mod.rs                        # one mod line
  - apps/game-server/src/domain/weapon_proficiency.rs             # operations, active modified perk
  - apps/game-server/tests/character_proficiency_modification_postgres.rs
  - apps/game-server/tests/durability_postgres.rs                 # one mod line
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json                  # §7.2 and §12 rows
  - docs/architecture/DUR-03_ITEM_TRANSACTION_AND_ANTI_DUPLICATION_CONTRACT.md  # §15, §39.3
  - docs/architecture/reviews/OTERYN_GAME_PROFICIENCY0_WEAPON_PROFICIENCY_DECISION_2026-09-29.md  # §4.2, §4.5
  - docs/architecture/reviews/OTERYN_GAME_PROFICIENCY1_PERK_MODIFICATION_AND_CATALYSTS_DECISION_2026-10-01.md  # status
  - docs/architecture/reviews/OTERYN_GAME_IMBUE_FORGE0_IMBUEMENTS_AND_EXALTATION_FORGE_DECISION_2026-09-30.md  # §9
  - docs/agents/tasks/active/OTV2-YYYYMMDD-prof-shape-1.md
validation:
  - cargo fmt --all --check
  - cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings
  - cargo test --locked -p oteryn-game-server --quiet
  - OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test character_proficiency_modification_postgres --quiet (PostgreSQL 17.6)
  - python tools/agents/validate_governance.py; git diff --check
acceptance: every §13 row except 8 and 12
```

### PROF-SHAPE-WIRE-1

```yaml
task_id: OTV2-YYYYMMDD-prof-shape-wire-1
decision: PROFICIENCY1B §10-§11
worker: oteryn-hard-worker (protocol-oteryn wire format)
review: protocol review
depends_on: [PROF-WIRE-1, PROF-SHAPE-1]
command_lease: 15-20 under capability 2
owned_paths:
  - docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json   # six command rows, MODIFIED_LEVEL, RL-01/RL-02
  - crates/protocol-oteryn/                           # messages (files named at allocation)
  - the game-server proficiency command handler and domain encoder (named at allocation)
  - docs/architecture/reviews/OTERYN_GAME_PROF_WIRE0_PROFICIENCY_WIRE_ACCEPTANCE_DECISION_2026-10-01.md  # §3, §4
  - docs/agents/tasks/active/OTV2-YYYYMMDD-prof-shape-wire-1.md
validation: cargo fmt/clippy/test for protocol-oteryn and oteryn-game-server; registry validator; git diff --check
acceptance: §13 rows 8 and 12
```

PROF-SHAPE-CONTENT-1 is a content-lane packet: the shaping schema and definitions under
`content/proficiencies/` and its validator, with each cell's evidence class; it admits nothing by
itself.
