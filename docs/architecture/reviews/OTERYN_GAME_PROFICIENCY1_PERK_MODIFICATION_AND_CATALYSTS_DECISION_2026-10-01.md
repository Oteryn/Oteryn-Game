# PROFICIENCY-1 Perk modification, the Lunar Ascension Orb and catalysts

- Decision: `PROFICIENCY1-PERK-MODIFICATION-AND-CATALYSTS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence,
  economy, determinism) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Scope: **narrowed at Codex round 5** (control-plane scope cut). This decision fixes the state shape,
  the reserved causes, the operation list and the fail-closed gate. Every operation's semantics,
  receipts, offers and wire move to the value-shape amendment **PROFICIENCY-1B** (§6), which also
  admits the value shapes.
- Answers: PROFICIENCY-0 §4.5 ("Perk modification (reroll, rank, reshape), the Lunar Ascension Orb and
  catalysts spend value ... They get their own decision (PROFICIENCY-1)"); the Weapon Proficiency
  packet #162 5936420312 (architect item 2: typed mutation causes, transaction, refusal and replay);
  owner answer 5b (D281) opening Weapon Proficiency runtime work.
- Builds on: PROFICIENCY-0 §4.1-§4.5 (definitions, the track row, the receipt chain, the writer);
  PROF-WIRE-0 (capability 2; later commands join it); IMBUE-FORGE-0 §9-§10 (forge dust is a
  Character balance, `game_character_forge_dust`, with a ledger; `ForgeCause`); DUR-03 §15 and §18
  (burn sinks, non-item assets); the composition decision rules 1-4; ITEM-USE-0 (item use);
  SIM-DETERMINISM-01 (named RNG purposes); the gold fee decision (D178: a new gold fee source needs
  the owner); the FORMULA rule; owner rule 5905825574.
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| PROF-SHAPE-1 | hard (persistence), persistence review | the modification table and its constraints (§3), the reserved causes, the operation commands answering `NOT_ADMITTED` (§4), the shaping-revision retention check (§3) | PROF-1 |
| PROF-SHAPE-CONTENT-1 | content lane | evidence for pools, odds, rank values, costs and catalyst effects (§2), each with its evidence class | PROF-CONTENT-1 |

The operations themselves, their receipts and their wire are built after PROFICIENCY-1B.

## 1. Question

What state, causes and gate does Weapon Proficiency perk modification need now, so that PROF-1 and
PROF-2 can land without it and the value operations can be added later without a migration of their
shape?

## 2. Facts

**PROVEN** (Tibia manual `combat.md` §5.3.4, CipSoft official)

- Modification is done only in a protection zone, rerolls the current perk to a new random one, and
  costs dust.
- At most 2 perks per weapon can be modified: the first slot unlocks at proficiency level 3, the
  second at Mastery.
- A modified perk has a rank 1-10. A rank rises one step for dust, or straight to 10 with a Lunar
  Ascension Orb.
- Reshaping a modified perk offers 3 new perks at the same rank; the player may decline and keep the
  current one.
- Clearing a modified perk restores the unmodified perk and allows a different modification later.
- Catalysts (Proficiency Catalyst, Greater Proficiency Catalyst) accelerate progress gain.
- IMBUE-FORGE-0 §9: forge dust is a per-Character capped balance with a ledger.

**PROVEN** (English TibiaWiki, `Weapon_Proficiency`, read 2026-10-01): replacing the first perk
needs proficiency level 3 and costs 250 Dust; the second needs Mastery and costs 1,000 Dust. Item
facts: Lunar Ascension Orb `i53695`; a Test Proficiency Catalyst is listed, which the manual does not
name (its Global status is UNKNOWN).

**UNKNOWN** (hard parity gates, PROFICIENCY-0 §4.5): rank-up, reshape and other costs; whether any
operation costs gold; the modification pool, the reroll and reshape odds, the value of a perk per
rank; what a catalyst does and how it is consumed. Canary's shaping opcodes are no-ops and Crystal's
are empty; neither is evidence (packet 5936420312).

## 3. State (PROF-SHAPE-1)

Table `game_character_proficiency_modifications`:

| Column | Meaning |
|---|---|
| `character_id`, `item_key` | the track (PROFICIENCY-0 §4.2) |
| `slot` | 1 or 2 |
| `level` | the proficiency level whose perk is modified |
| `modified_perk` | the shaping revision and entry index of the modified perk |
| `rank` | 1..10 |
| `pending_offer` | NULL, or up to 3 entries of the same shaping revision |

- **Keys.** Primary key `(character_id, item_key, slot)`, and **UNIQUE `(character_id, item_key,
  level)`**: at most 2 rows per track, and two slots never modify the same level.
- **Unlock.** Slot 1 is unlocked while the derived level is at least 3, slot 2 while the track has
  Mastery. A modified level must have a selection (PROFICIENCY-0 `selections`); while active, the
  modified perk replaces that level's selected perk. A row whose level or slot is no longer unlocked
  stays stored and inactive.
- **Modified levels keep their selection.** A `perk_selection` that changes or clears the selection
  of a level with a row is refused `MODIFIED_LEVEL`. Amended: PROFICIENCY-0 §4.4's result codes.
- **Shaping revisions.** `modified_perk` and `pending_offer` name entries of the shaping revision they
  were drawn from. That revision is **retained while any row references it**: content validation
  refuses a build that drops or changes a referenced shaping revision, so a stored row always
  resolves to the same perk, and a row is evaluated against its own stored revision.
- **Definition revisions.** An incompatible definition revision whose migration changes a modified
  level clears that row in its `migration` receipt (PROFICIENCY-0 §4.1), with no refund
  (`PARITY_PENDING`, R2); the migration line carries both slots' rows before and after.
- **Reserved causes.** The receipt cause `perk_modification` (PROFICIENCY-0's chain) and the DUR-03
  burn cause `ProficiencyCause {track, slot, operation, occurrence}` are reserved names. Their lines,
  CHECKs and shapes are PROFICIENCY-1B's.
- While every operation is `NOT_ADMITTED` (§4), no row is ever written; the table, keys and
  retention check exist so that PROFICIENCY-1B adds behaviour, not a schema change.

## 4. Operations and the gate

The closed operation list is `MODIFY`, `RANK_UP`, `ORB_RANK`, `RESHAPE_OFFER`, `RESHAPE_CHOOSE` and
`CLEAR`. No other operation exists. **Every one is `NOT_ADMITTED`** in this decision: the server
answers it with that typed result (PROF-WIRE-0 §5) and writes nothing. PROFICIENCY-1B admits each
with its semantics, its composed DUR-03 §39.3 shape and its value evidence (official, owner-verified
TibiaPal or tibiatools.io, then English TibiaWiki; OTS sources are not evidence here). A gold cost
is a new fee source (D178) and needs an owner answer first. If evidence never appears for some
value, the architect puts a declared difference to the owner in one batch.

**Catalysts.** No catalyst is admitted; using one is refused `NOT_ADMITTED`, and catalysts stay
ordinary tradeable items. Their effect kind decides their transaction (a progress grant is a
composed burn-plus-receipt shape; a timed multiplier is a condition or item state), so they get
their own decision with that evidence. The Test Proficiency Catalyst stays not admitted until
evidence shows it exists on Global servers.

## 5. Rows

| Row | Value |
|---|---|
| `PROF1-RL-01` modification slots per track | 2 |
| `PROF1-RL-02` rank | 1..10 |

Each with max and max+1 tests, plus: a second row for an already modified level violates the
UNIQUE key; every operation and catalyst use answers `NOT_ADMITTED` and writes nothing; a content
build that drops a referenced shaping revision is refused.

## 6. Entry conditions for PROFICIENCY-1B

PROFICIENCY-1B, the value-shape amendment, is accepted only when it states and tests each of these
(the findings of review rounds 1-5 on this PR):

1. **Occurrence.** The occurrence derives from the CommandId exactly as PROFICIENCY-0 §4.3 does; a
   retry returns the first receipt by key before any write.
2. **Revision binding.** The reserved occurrence binds the definition revision, the shaping revision
   and the `SimulationDeterminismProfileRevision`; a change refuses `REVISION_CHANGED`, persisted as
   a receipt-only terminal record (as IMBUE-FORGE-0 §10), never refused by a rate cap (a cap is
   charged at reservation).
3. **Receipt CHECKs.** One `perk_modification` line per receipt (zero only for the terminal record);
   progress, selections and definition unchanged; exactly one slot changes; per operation, only the
   fields it may change change (rank operations change only `rank`).
4. **Offers.** A reshape offer is durable and paid when drawn; it needs at least 3 other pool entries
   and offers exactly 3, refusing before any burn otherwise; **rank operations are refused while an
   offer is pending**.
5. **Shaping revisions.** Operations that draw or rank under a newer revision on a row of an older
   one refuse or migrate explicitly; `CLEAR` stays allowed.
6. **Draws** use the SIM-DETERMINISM-01 purpose `proficiency_shaping`, bound to the occurrence.
7. **Composed shapes** for the dust burn and the one-unit orb burn with their DUR-03 §28 rows, lock
   order (root, track, rows, dust, item) and audit.
8. **Wire.** Command payloads with a **measured** byte bound, and the re-measured `ACTOR_PROFICIENCY`
   snapshot bound (PROF-WIRE-0 RL-01) including the rows.
9. **Values** for every admitted operation, by the evidence order of §4.

## 7. Rejected options

- **Copying Canary or Crystal.** Their shaping is a no-op or empty; using it would invent Global
  behaviour.
- **Deciding operation semantics now.** Five review rounds showed the receipt, offer and wire detail
  belongs with the value shapes that use it; nothing can run before those exist.
- **No table until 1B.** The keys and retention rule are cheap now and keep 1B from changing the
  schema.
- **Dust as an item.** IMBUE-FORGE-0 keeps it a capped Character balance.

## 8. Architect rulings (owner rule 5905825574)

- **R1. Unknown costs and odds.** a) Keep every operation closed until evidence (recommended); b)
  admit with OTS values. **Ruled a).**
- **R2. Incompatible definition revision.** a) Clear the affected modification without refund
  (`PARITY_PENDING`); b) keep it unchanged. **Ruled a)**, because a modification of a perk that no
  longer exists has no meaning.

## 9. Owner questions

None now. A gold cost, or a value that evidence never settles, comes back to the owner in one batch.

## 10. Decision test

- **Must decide now:** YES. PROF-1 and PROF-2 are allocated (D281); the track, its migration line and
  the selection rule must know the modification rows exist.
- **Blocked:** PROF-1's migration line and `MODIFIED_LEVEL`; PROFICIENCY-1B.
- **Minimum sufficient:** one table with its keys, two reserved causes, a closed operation list that
  answers `NOT_ADMITTED`, a retention rule and the entry conditions for 1B.
- **Harder later:** the table and its keys join the Character state, so changing them needs a
  migration; the retained shaping revisions grow the content build while referenced; the six
  operation names are fixed.
- **Superseding evidence:** official or owner-verified costs, pools, odds and catalyst effects.
- **Deliberately not decided:** all operation semantics, receipts, offers and wire (PROFICIENCY-1B);
  values; gold costs; catalysts.

## 11. Before-freeze checklist

1. **Contract amendments:** PROFICIENCY-0 §4.4 and §4.5 (the rows, `MODIFIED_LEVEL`, migration
   lines, the gate); DUR-03 §15 (the reserved `ProficiencyCause`); IMBUE-FORGE-0 §9 (a future dust
   sink). Applied in this PR.
2. **Serialization:** no write in this decision; PROFICIENCY-1B defines the transactions.
3. **Restart:** nothing new.
4. **Typed references:** track (CharacterId, item key), shaping revision and entry.
5. **Wire:** none; operations answer `NOT_ADMITTED` under capability 2.
6. **Split work:** PROFICIENCY-1B.
