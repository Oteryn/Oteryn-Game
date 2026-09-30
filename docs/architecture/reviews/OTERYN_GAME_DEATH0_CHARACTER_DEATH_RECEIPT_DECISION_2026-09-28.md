# DEATH-0 Character death receipt decision

- Decision: `DEATH0-CHARACTER-DEATH-RECEIPT-V1`
- Status: **CANDIDATE**. Acceptance requires exact-head validation, independent review and
  protected integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Answers: the storage prerequisite of the Reference first player death decision (§4.3, child
  DEATH-0), required before DEATH-1 can commit anything
- Builds on: migration `0009_character_progression.sql`; the Character/item composition decision
  (2026-09-27, §3.6), which leaves non-XP Character semantic writes to "a later migration and
  receipt redesign under its own architecture decision"
- Admission baseline: `main@7d1134f090ac249f964fede017efabba91e22b90`
- Runtime, migration and production authority: **NONE**. The migration belongs to the DEATH-0
  implementation allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`
- Amended by A13 (`OTERYN_GAME_A13_CHARACTER_BUILD_STATE_DECISION_2026-09-29.md` §4.6): §3.1
  death receipt build fields. The amendment takes effect only when A13 is accepted. The chain now
  has further kinds (stance, build) beyond the two named in §3.1.

## 1. Question

The death transaction lowers experience (and possibly level), consumes blessings and records a
respawn position, keyed by the death occurrence. Migration `0009` admits a `CharacterRevision`
successor only with a strictly larger `total_experience` and one XP-award receipt, and its commit
guard requires exactly `revision − 1` XP receipts forming one chain. How does a death commit
without weakening that chain?

## 2. Facts

**PROVEN** (main `7d1134f090ac249f964fede017efabba91e22b90`)

- `0009`:
  - `game_character_progression_state_guard` admits an update only with `revision + 1`,
    `total_experience` strictly larger, `level` not lower and every revision field unchanged.
  - `game_character_xp_receipts`: one immutable receipt per committed revision, keyed by a UUIDv7
    reward occurrence, with `experience_after > experience_before` and
    `level_after >= level_before`.
  - `game_character_progression_consistency_guard` (deferred, at commit): for revision > 1 the
    receipt count is `revision − 1`, the receipt for the current revision matches the state, and
    each receipt's `before` equals its predecessor's `after`.
  - Root, state and receipts reject truncate; receipts are immutable.
- `durability/character_progression.rs`: `commit_character_experience` writes under the recovery
  fence, the admission-relation locks and the `character_root` row lock;
  `reconcile_character_experience` resolves an ambiguous outcome by occurrence.
- The first player death decision (§4.3): one Character transaction keyed by the
  `PlayerDeathOccurrence` commits the XP loss, blessing consumption, the Amulet of Loss selection,
  the lost-item set and the respawn position. Item effects follow in DUR-03 (§4.4).
- No migration stores a Character position, home town or blessings today.
- Character progression readiness (#1143, in review, owner decision D88) adds an idempotent
  initializer that creates the revision-one progression row (level 1, 0 XP) through the same
  fence. A death can only follow an initialized row.

## 3. Decision

### 3.1 One receipt chain with two kinds

- A new immutable table `game_character_death_receipts`, keyed by the `PlayerDeathOccurrence`
  (UUIDv7), records per death:
  - `character_id`, `original_character_revision`, `committed_character_revision` = original + 1;
  - `level_before`, `level_after` (≤ before), `experience_before`, `experience_after`
    (≤ before, ≥ 0) and `experience_lost` = before − after;
  - the blessing set before and after (after is empty for regular blessings, D58-D68 §4.3.2);
  - the Amulet of Loss selection and the lost-item set (item ids, ≤ the DEATH-3 bound; empty
    until DEATH-3 is admitted);
  - the death cell: World, Channel, spatial position (≤ 128 B) and map revision, the durable
    destination that a resumed DEATH-3 item workflow needs after runtime state is lost
    (death decision §4.4);
  - the respawn position reference (≤ 128 B) and the death policy revision;
  - `command_binding` (1..1,024 B) and `policy_digest` (32 B), as in an XP receipt: the binding
    is a digest of the complete death intent (character, original revision, occurrence, the
    policy contents, blessings held, the promotion and Premium evaluation, the Amulet of Loss
    state, the equipment and backpack snapshot, the RNG stream binding and the death cell);
  - the same revision fields as an XP receipt, and `committed_at`;
  - the build fields (A13 amendment): `vocation`, and the before and after values of
    `magic_level` and `mana_spent`. They are all NULL, or all non-NULL when the death lowers
    magic-level progress. The rules, the build-row update and the binding are in A13 §4.6. The
    death still advances one revision with this one receipt; pending `mana_spent` is flushed in an
    earlier, separate transaction.
- The XP receipt table is unchanged. A `CharacterRevision` successor now has exactly one receipt
  of either kind.

### 3.2 Guard changes (the DEATH-0 migration)

- **State guard:** an update with `revision + 1` and unchanged revision fields is admitted when
  either `total_experience` is strictly larger and `level` not lower (XP), or `total_experience`
  is not larger and `level` not higher (death). The deferred guard decides which kind applies.
- **Consistency guard:** for revision > 1:
  - XP receipts plus death receipts = `revision − 1`, and each revision has exactly one receipt;
  - the receipt for the current revision matches the state (level, experience, revisions);
  - the chain holds across both kinds: each receipt's `before` equals its predecessor's `after`.
- XP receipts keep `experience_after > experience_before`; death receipts keep
  `experience_after <= experience_before`. A death cannot masquerade as an award, nor the reverse.
- Truncate and update or delete of death receipts are rejected like XP receipts.

### 3.3 Blessing state

- A new table `game_character_blessings` (`character_id`, `blessing_key` ≤ 128 B, provenance)
  holds held blessings. The death transaction deletes the regular blessings in the same
  transaction as its receipt, which records the set before and after.
- A blessing change outside a death (buying, DEATH-4) is a separate Character transaction with its
  own receipt kind, decided with DEATH-4. Until then, no path inserts blessings and every death
  records an empty set.

### 3.4 Respawn position and pending respawn

- The receipt records the respawn position.
- The death transaction also inserts one row in a new table `game_character_pending_respawns`
  (`character_id` primary key, `death_occurrence_id`, the respawn position). It is an obligation
  of the committed death outcome, outside the revision chain: it neither advances nor needs a
  `CharacterRevision`, like DUR-03 cause records keyed by a Character.
- **Consumption:** the runtime respawn (death decision §4.5), and after a restart the next
  admission or recovery of the character, reads the pending row, places the new actor at its
  position with full HP and mana, and deletes the row under the same session fences in the same
  transaction as the placement it records. A committed death therefore always respawns at its
  recorded temple, even if the generation ended before the runtime respawn.
- A second death cannot commit while a pending respawn exists (the character is not playable).
- **Amendment (NPC-0, 2026-09-30).** A death is never refused because of NPC travel. The death
  transaction deletes a pending arrival of the same character
  (`game_character_pending_arrivals`) in the same transaction; the respawn supersedes it. An
  occupied respawn tile uses the placement fallback of NPC-0 §6.1.
- Persisting a general Character position for ordinary logins remains a separate lane.

### 3.5 Writer

- `commit_character_death` mirrors `commit_character_experience`: the same recovery fence,
  admission-relation locks, reconnect-session row, runtime-scope assignment and `character_root`
  row lock, keyed by the death occurrence. A retry with the same occurrence and the same
  `command_binding` returns the first receipt; the same occurrence with a different binding
  conflicts (compared before replay, as `commit_character_experience` does); a stale fence writes
  nothing.
- `reconcile_character_death` resolves an ambiguous outcome by occurrence before respawn.
- An XP award and a death serialize on the `character_root` lock; the chain rejects any
  interleaving that breaks `before = predecessor.after`.

## 4. Delivery

| Child | Scope | Depends on |
|---|---|---|
| DEATH-0 | The migration (§3.1-§3.4: death receipts, blessings, pending respawns), guard changes and tests; no writer | this decision |
| DEATH-1 | `commit_character_death` and `reconcile_character_death`, the calculator change, pending-respawn consumption at respawn, admission and recovery | DEATH-0; progression readiness |

## 5. Rejected options

- **Negative XP receipts.** They would break `experience_awarded > 0` and let a death look like
  an award.
- **A single generalized receipt table replacing XP receipts.** It rewrites committed P03
  evidence for no gain; a second kind in the same chain is smaller.
- **Death without a revision successor.** Experience would change outside the chain the guard
  proves.

## 6. Decision test

- **Must decide now:** YES. DEATH-1 cannot commit under `0009`.
- **Minimum sufficient:** one new receipt table, one blessing table, two guard changes; XP
  receipts untouched.
- **Superseding evidence:** a Character position lane; DEATH-4 blessing purchases; a general
  Character semantic-write receipt design.
- **Deliberately not decided:** Character position persistence, home towns, blessing purchase
  receipts, item effects (DEATH-3).

## 7. Handback

```yaml
result: RESOLVED
source_escalation: "first player death decision §4.3 storage prerequisite (DEATH-0)"
owner_decisions: []
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_DEATH0_CHARACTER_DEATH_RECEIPT_DECISION_2026-09-28.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: true   # DEATH-0 may be allocated
required_fresh_allocation: true
required_independent_review: "exact-head independent review (receipt chain, guards, fencing)"
implementation_lanes: [DEATH-0, DEATH-1]
required_revalidation:
  - "DEATH-0: a death receipt lowers experience and advances the revision; an XP receipt still requires a strict increase; mixed chains XP → death → XP pass; a gap, a duplicate revision or a before/after mismatch across kinds fails the deferred guard; death receipts are immutable and untruncatable"
  - "DEATH-1: a retry with the same occurrence and binding returns the first receipt; the same occurrence with a changed policy or input conflicts even when the stored outputs would match; a stale fence writes nothing; an XP award and a death serialize without deadlock"
  - "DEATH ML loss child (A13 §4.6): a death with NULL build fields keeps the version 1 binding and leaves the build row; a death with non-NULL fields strictly lowers (magic_level, mana_spent), updates the build row in the same revision and joins the build chain"
  - "DEATH-1: a restart after the death commits and before respawn places the character at the recorded temple on the next admission and deletes the pending row once; a death receipt carries the death cell a resumed DEATH-3 workflow reads"
remaining_unknowns:
  - Character position persistence
  - blessing purchase receipts (DEATH-4)
next_action: "#162 validates this exact head, routes the independent review, integrates it, then allocates DEATH-0."
```
