# PROFICIENCY-1 Perk modification, the Lunar Ascension Orb and catalysts

- Decision: `PROFICIENCY1-PERK-MODIFICATION-AND-CATALYSTS-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence,
  economy, determinism) and protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
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
| PROF-SHAPE-CONTENT-1 | content lane | the modification pool per definition and level, rank values, costs per operation, reshape and reroll draw tables, catalyst and orb item facts, each with its evidence class (§2, §6) | PROF-CONTENT-1 |
| PROF-SHAPE-1 | hard (persistence, economy), persistence, economy and determinism review | the modification rows, the receipt cause and its lines, the operations of §4, the dust and item lines, replay, the admission gate (§3-§5, §7) | PROF-1; FORGE-1 (the dust balance); ITEM-USE-1 |
| PROF-SHAPE-WIRE-1 | impl, protocol review | the commands of §8 under capability 2 | PROF-WIRE-1; PROF-SHAPE-1 |

## 1. Question

How are a weapon's perks modified, ranked, reshaped and cleared, what do the Lunar Ascension Orb and
the catalysts do, and how are they paid for, stored and replayed?

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
- Item facts (TibiaWiki): Lunar Ascension Orb `i53695`. TibiaWiki also lists a Test Proficiency
  Catalyst, which the manual does not name; whether it exists on Global servers is UNKNOWN.

**UNKNOWN** (hard parity gates, PROFICIENCY-0 §4.5)

- Dust costs of each operation and rank step; whether any operation also costs gold.
- The modification pool, the reroll and reshape odds, and the value of a perk per rank.
- What exactly a catalyst does (amount, duration, stacking) and how it is consumed.

Canary leaves the shaping opcodes as no-ops; Crystal's reset and apply are empty. Neither is
evidence for these values (packet 5936420312).

## 3. State (PROF-SHAPE-1)

Table `game_character_proficiency_modifications`, at most 2 rows per track:

| Column | Meaning |
|---|---|
| `character_id`, `item_key` | the track (PROFICIENCY-0 §4.2) |
| `slot` | 1 or 2 |
| `level` | the proficiency level whose perk is modified |
| `modified_perk` | a typed perk from the definition's modification pool: pool revision and entry index |
| `rank` | 1..10 |
| `pending_offer` | NULL, or the 3 reshape options drawn and paid for (pool entries), at the current rank |

- A slot is **unlocked** while the derived level is at least 3 (slot 1) or the track has Mastery
  (slot 2). `MODIFY` needs an unlocked slot. A modified level must have a selection (PROFICIENCY-0
  `selections`). While a modification is active, the modified perk replaces that level's selected
  perk.
- When a raised threshold makes the level inactive, or the slot locks again, the modification stays
  stored but inactive, and only `CLEAR` (and `RESHAPE_CHOOSE` of a pending offer) is allowed on it.
  It becomes active again when the level and the slot are unlocked again.
- A level with a modification keeps its selection: a `perk_selection` that changes or clears that
  level's selection is refused `MODIFIED_LEVEL` until the modification is cleared. Amended:
  PROFICIENCY-0 §4.4's result codes.
- **Receipts.** PROFICIENCY-0's receipt chain gains the cause `perk_modification`, with the
  operation (§4) and exactly one line (zero only for the `REVISION_CHANGED` terminal receipt, §4),
  which carries the track's modification row before and after
  next to the unchanged progress and selections. The line CHECKs for `perk_modification`:
  - progress, selections and the definition key and revision are unchanged;
  - exactly one slot's row changes: absent to present (`MODIFY`: rank 1, no offer), present to
    absent (`CLEAR`), or present to present with the same slot and level;
  - by operation: `RANK_UP` raises the rank by exactly 1 and `ORB_RANK` sets it to 10 from below 10, each with
    `modified_perk`, `pending_offer`, slot and level unchanged;
    `RESHAPE_OFFER` sets a NULL `pending_offer` and changes nothing else; `RESHAPE_CHOOSE` clears
    the offer and either keeps `modified_perk` or sets it to one of the offer's entries, with the
    rank unchanged.

  The per-track check of PROFICIENCY-0 §4.2 extends to the rows; the shared chain guard and `verify_character_integrity` count the new cause.
- **Revisions.** A compatible definition revision keeps the modifications. An incompatible one
  whose migration changes a modified level clears that modification in its `migration` receipt
  (PROFICIENCY-0 §4.1) with no refund (`PARITY_PENDING`). A `migration` line then carries the
  track's modification rows before and after for both slots (absent, unchanged or cleared), and the
  migration CHECKs and the reconcile of PROFICIENCY-0 §4.1 cover them: reconcile recomputes, from
  the two definition revisions, which slots must be cleared and compares them with the line.
- **Shaping revisions.** `modified_perk` and `pending_offer` name entries of the shaping revision
  they were drawn from. That revision is **retained while any row references it**: content
  validation refuses a build that drops or changes a referenced shaping revision, so a stored row
  always resolves to the same perk. A row is evaluated (its active perk, a `RESHAPE_CHOOSE` of its
  offer, a `CLEAR`) against its own stored revision, never the current one. An operation that would
  draw or rank under a newer revision (`RANK_UP`, `ORB_RANK`, `RESHAPE_OFFER`) on a row from an older
  revision is refused `SHAPING_REVISION_OLD` until the value-shape amendment (§4) defines that row's
  migration; `CLEAR` stays allowed, after which `MODIFY` starts under the current revision. While
  those operations are `NOT_ADMITTED`, no row exists, so this rule binds the amendment that admits
  them.

## 4. Operations (PROF-SHAPE-1)

Each operation is one Character transaction: one `perk_modification` receipt that advances one
CharacterRevision, plus its value lines, under the composition decision rules 1-4. The lock order
is the `character_root`, the track row and its modification rows, the forge dust row, then any item.

| Operation | Needs | Value lines | Result |
|---|---|---|---|
| `MODIFY {level, slot}` | protection zone; slot unlocked and empty; the level has a selection | dust BURN | a modification at rank 1 with a perk drawn from the pool |
| `RANK_UP {slot}` | rank < 10 | dust BURN | rank + 1 |
| `ORB_RANK {slot}` | rank < 10; a Lunar Ascension Orb as a direct entry of the main backpack (the first in B3 order, as NPC-0 SELL finds its item) | one-unit item BURN of the orb | rank 10 |
| `RESHAPE_OFFER {slot}` | protection zone; no pending offer; the pool holds at least 3 entries besides the current perk (4 distinct entries) | dust BURN | `pending_offer` = exactly 3 distinct perks drawn from the pool, excluding the current one |
| `RESHAPE_CHOOSE {slot, choice 0..2 or keep}` | protection zone; a pending offer | none | the chosen perk replaces the modified one at the same rank, or the current one stays; the offer is cleared |
| `CLEAR {slot}` | protection zone | none | the modification is removed; the level's selected perk applies again |

- **Draws.** Every draw uses the SIM-DETERMINISM-01 purpose `proficiency_shaping`, bound to the
  operation's occurrence, so a retry never draws again. The reshape offer is durable and paid when
  drawn, so asking again cannot fish for better options.
- **Revision binding.** When an operation's occurrence is reserved, it binds the full
  behaviour-affecting set: the definition revision, the shaping content revision that holds the
  pool, the odds, the costs and the rank values, and the `SimulationDeterminismProfileRevision`
  (SIM-DETERMINISM-01 §5 and §12) under which the draws run. The `REVISION_CHANGED` comparison
  covers all three. The receipt stores that set. If any part has
  changed when the transaction commits, the operation is refused `REVISION_CHANGED`, terminal for
  that occurrence; the client starts again with a new occurrence, as in the forge (IMBUE-FORGE-0
  §10).
- **`REVISION_CHANGED` is persisted (as IMBUE-FORGE-0 §10).** The refusal is the occurrence's
  terminal receipt: a `perk_modification` receipt with the outcome `REJECTED {REVISION_CHANGED}`, the
  bound set, **zero lines** and no value line, in a receipt-only Character transaction that advances
  one CharacterRevision like every receipt (DUR-02 rule 2). The line-count CHECK admits zero lines
  for this outcome only, and every other `perk_modification` receipt still has exactly one line.
  Every later replay of that occurrence, under any CommandId, returns the same rejection, even if the
  active set later equals the bound set again. If the terminal write's outcome is unknown, the
  occurrence stays refused until reconciliation reads the receipt; it is never evaluated meanwhile.
  `PROF1-RL-04` is charged when the occurrence is reserved, before evaluation, so the terminal
  receipt uses the slot its own reservation already took and is never refused by the cap.
- **Value: not admitted here.** The value-spending operations (`MODIFY`, `RANK_UP`, `ORB_RANK`,
  `RESHAPE_OFFER`) need composed DUR-03 §39.3 shapes: the proficiency receipt together with a forge
  dust burn or a one-unit orb burn, with resource, evidence and audit bounds. This decision does not
  admit those shapes. It reserves the cause name `ProficiencyCause {track, slot, operation,
  occurrence}` for them. A later amendment admits each shape together with the value evidence it
  needs (§5). Until then those four operations refuse `NOT_ADMITTED` and write nothing. `CLEAR` and
  `RESHAPE_CHOOSE` spend nothing; they act only on rows the other operations create, so they are
  idle until then.
- **Gold.** If evidence shows an operation also costs gold, that is a new fee source (D178): it needs
  an owner answer before the operation is admitted. This decision admits no gold cost.
- **Refusals** (nothing written): `NOT_IN_PROTECTION_ZONE`, `SLOT_LOCKED`, `SLOT_OCCUPIED`,
  `NO_SELECTION`, `NO_MODIFICATION`, `RANK_MAX`, `NO_PENDING_OFFER`, `OFFER_PENDING`,
  `INSUFFICIENT_DUST`, `SHAPING_REVISION_OLD` (§3), `NO_ORB`, `POOL_TOO_SMALL` (fewer than 3 other entries to offer, checked before any burn), `MODIFIED_LEVEL` (a
  selection change at a modified level, §3), `STALE_REVISION` (the client's expected track revision differs),
  `NOT_ADMITTED` (§5). `REVISION_CHANGED` is the one refusal that writes its terminal receipt
  (above).
- **Replay.** Each command carries an occurrence id; a retry with the same occurrence returns the
  first receipt by key, before any write (PROFICIENCY-0 §4.3). A different occurrence on a changed
  track is refused `STALE_REVISION`.

## 5. Admission gate (fail closed)

An operation is admitted only when two things hold. Its composed DUR-03 §39.3 shape is admitted by a
later amendment (§4 "Value"). And its cost row, and for `MODIFY` and `RESHAPE_OFFER` the pool, the
odds and the rank values, are `PARITY_CONFIRMED` in content (official, owner-verified TibiaPal or
tibiatools.io, then English TibiaWiki; OTS sources are not evidence here). Until then the server
refuses it with `NOT_ADMITTED` and writes nothing. If evidence never appears for some value, the
architect puts a declared difference to the owner in one batch; nothing is invented meanwhile.
`CLEAR` and `RESHAPE_CHOOSE` cost nothing and are admitted together with the operations that create
their state.

## 6. Catalysts (PROF-SHAPE-1)

- **No catalyst is admitted by this decision.** Using one is refused `NOT_ADMITTED`, and catalysts
  stay ordinary tradeable items. Their effect kind is not known. It might be a progress multiplier
  for a duration, or a progress grant to a track, and the kind decides the transaction:
  - an effect that writes durable proficiency state (a progress grant) needs one atomic transaction:
    the catalyst burn together with a proficiency receipt. That is a composed DUR-03 §39.3 shape, not
    an ITEM-USE-0 use;
  - a timed multiplier applied at PROFICIENCY-0 §4.3 accrual would be a condition or an item state.
  A later decision admits catalysts with the evidence for their effect and the matching transaction
  shape.
- The two catalysts the manual names (Proficiency Catalyst, Greater Proficiency Catalyst) are the
  candidates for that decision. The Test Proficiency Catalyst
  (TibiaWiki only) is gated separately: it stays not admitted until evidence shows it exists on
  Global servers, whatever the other two's status.

## 7. Rows

| Row | Value |
|---|---|
| `PROF1-RL-01` modification slots per track | 2 |
| `PROF1-RL-02` rank | 1..10 |
| `PROF1-RL-03` reshape options | 3 |
| `PROF1-RL-04` modification receipts per character per minute | 30, charged at reservation (anti-spam; a refusal other than `REVISION_CHANGED` writes nothing, and the `REVISION_CHANGED` terminal receipt is never refused by this cap) |

Each with max and max+1 tests.

## 8. Wire (PROF-SHAPE-WIRE-1)

The six operations are commands that join capability 2 (PROF-WIRE-0 §5), with type numbers reserved
on #162 at their allocation; each carries the item key, the slot, its operation fields, the expected
track revision and the occurrence, at most 32 bytes. `ACTOR_PROFICIENCY` snapshots and deltas carry
the modification rows (slot, level, pool entry, rank, pending offer); PROF-WIRE-0's RL-01 snapshot
bound is re-measured by PROF-SHAPE-WIRE-1 before the commands ship.

## 9. Rejected options

- **Copying Canary or Crystal.** Their shaping is a no-op or empty; using it would invent Global
  behaviour.
- **An undurable reshape offer.** A redraw on every request would let players fish for options.
- **Dust as an item.** IMBUE-FORGE-0 keeps it a capped Character balance.
- **Guessing costs.** The admission gate keeps the operations closed until evidence or an owner
  decision exists.

## 10. Architect rulings (owner rule 5905825574)

- **R1. Unknown costs and odds.** a) Build the operations and keep them closed until evidence
  (recommended); b) admit with OTS values. **Ruled a).**
- **R2. Reshape offer.** a) Durable, paid at offer (recommended); b) free redraws. **Ruled a).**
- **R3. Incompatible revision.** a) Clear the affected modification without refund
  (`PARITY_PENDING`); b) keep it unchanged. **Ruled a)**, because a modification of a perk that no
  longer exists has no meaning.

## 11. Owner questions

None now. A gold cost, or a value that evidence never settles, comes back to the owner in one batch.

## 12. Decision test

- **Must decide now:** YES. PROF-1 and PROF-2 are allocated (D281); without this the modification
  slots have no state, cause or transaction.
- **Minimum sufficient:** one table, one receipt cause, six operations with their refusals, replay
  and revision binding, a reserved cause name and a fail-closed gate; no value shape is admitted yet.
- **Harder later:** the modification table, the `perk_modification` cause and its line CHECKs join
  the Character chain, so a later change to them needs a migration of stored receipts and rows; the
  durable reshape offer means changing to free redraws later would orphan stored offers; the six
  command shapes join capability 2's compatibility surface once PROF-SHAPE-WIRE-1 ships.
- **Superseding evidence:** official or owner-verified costs, pools, odds and catalyst effects.
- **Deliberately not decided:** values (§2 UNKNOWN), gold costs, the Test Proficiency Catalyst's
  Global status.

## 13. Before-freeze checklist

1. **Contract amendments:** PROFICIENCY-0 §4.5 (modification rows, the cause, migration lines, the
   gate); DUR-03 §15 (the reserved `ProficiencyCause`, no shape admitted); IMBUE-FORGE-0 §9 (a future
   dust sink). Applied in this PR.
2. **Serialization:** one Character transaction per operation, PROFICIENCY-0's writer and chain;
   lock order root, track, dust, item.
3. **Restart:** all state is durable; a pending offer survives restart.
4. **Typed references:** track (CharacterId, item key), pool revision and entry, occurrences.
5. **Wire:** six commands under capability 2, later.
6. **Split work:** none.
