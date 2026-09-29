# STANCE-0 Character stance persistence decision

- Decision: `STANCE0-CHARACTER-STANCE-PERSISTENCE-V1` (architect item A11)
- Status: **CANDIDATE**. Acceptance requires exact-head validation, independent review and protected
  integration.
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Answers: owner decision D140 (#162 comment 5888104688) for spell native behaviour Part C `stance`
  (section C.4)
- Builds on: migration `0009_character_progression.sql`; the DEATH-0 receipt decision
  (2026-09-28); the Character/item composition decision (2026-09-27, §3.6)
- Admission baseline: `main@d4d5e2c4`
- Runtime, migration and production authority: **NONE**. The migration belongs to the STANCE-0
  implementation allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

Part C.4 says the `standard` stance slot survives logout and is not reset by death. Nothing stores
it. Migration `0009` admits a `CharacterRevision` successor only with strictly larger experience and
one XP receipt. Where does the slot live, and how does a change commit without weakening the chain?

## 2. Owner decisions

- **D140** (#162 comment 5888104688, 2026-09-29): the spell stance slot persists as in Global; it
  survives logout (N8833) and death (N8933). No session-only interim.
- **S27** (2026-09-29, spell authoring schema) accepted the `stance` native_behavior key and its
  parameters (`persist_across_sessions` true, `keep_on_death` true) as specified in Part C.

## 3. Facts

**PROVEN** (main `d4d5e2c4`)

- Part C.4:
  - One `standard` slot holds at most one stance and may be empty.
  - A successful cast of the active stance switches it off; any other stance replaces it.
  - The stance persists across logout and is not reset by death (N8833, N8933, `CIPSOFT_OFFICIAL`
    as cited by Part C). A vocation change drops a stance that no longer fits.
  - The sorcerer elemental and crippling slots are out of scope.
  - Stance cooldown groups (2 s and 10 s) are content values, not resource limits.
- `0009`:
  - The state guard admits an update only with `revision + 1`, `total_experience` strictly larger,
    `level` not lower and every revision field unchanged.
  - `game_character_xp_receipts` holds one immutable receipt per committed revision, keyed by a
    UUIDv7 occurrence, with `command_binding` (1..1,024 B), `policy_digest` (32 B) and the revision
    fields.
  - The deferred consistency guard requires `revision − 1` receipts forming one chain
    (`before` = predecessor `after`) and a receipt that matches the state.
  - Root, state and receipts reject truncate. `character_revision` is NUMERIC(20,0).
- `commit_character_experience` (`durability/character_progression.rs`): recovery fence,
  admission-relation locks, `character_root` row lock, replay by occurrence, `ConflictingOccurrence`
  on a changed binding; `reconcile_character_experience` resolves an ambiguous outcome.
- Composition decision §3.6: no non-XP Character semantic mutation can advance `CharacterRevision`
  under `0009`; it needs a later migration and receipt design.
- DEATH-0 (CANDIDATE, not merged) adds a second receipt kind and a state-guard branch "XP not
  larger, level not higher", which also admits equal experience.
- Spell cast wire contract: "no stance owner means virtue none" (interim, #162 5884682203).
- Migrations end at `0014_corpse_container_transfer.sql`. No migration stores a stance, a vocation
  or a Character position.

**UNKNOWN**

- Whether the spell-cast `CommandRef` is a UUIDv7.
- Where vocation is stored and whether a vocation-change lane exists; none is on main.
- The rule that decides whether a stance fits a vocation (Part C Q20).
- The cost of one database transaction on the stance cast path.

## 4. Decision

### 4.1 Storage: one row, plus a receipt kind

- New table `game_character_stance`: `character_id` (primary key, references the Character root),
  `stance_key` (NULL or at most 128 B, same character class as the revision text keys),
  `committed_character_revision`, `last_stance_occurrence_id`.
  - No row means an empty slot. The `0009` initializer is not changed.
  - A row exists only after the first stance receipt. It is never deleted; truncate is rejected.
  - Only slot `standard` is stored. Other slots need a later decision.
- The row is the projection of the latest stance receipt. It is not a column of
  `game_character_progression_state`, so the XP and death guard branches stay untouched.

### 4.2 Third receipt kind

- New immutable table `game_character_stance_receipts`, keyed by `stance_occurrence_id` (the toggle
  `CommandRef`; STANCE-0 confirms it is a UUIDv7, otherwise the owner issues one per occurrence):
  - `character_id`, `original_character_revision`, `committed_character_revision` = original + 1;
  - `level` and `experience` before and after, each equal, so the cross-kind chain reads uniformly;
  - `stance_before` and `stance_after` (each NULL or at most 128 B); they must differ, so a no-op is
    not a receipt;
  - `command_binding` (1..1,024 B), `policy_digest` (32 B), the revision fields of an XP receipt and
    `committed_at`.
- The binding is a digest of the complete intent: character, expected revision, occurrence, target
  stance (or off), observed `stance_before`, and the content and policy revisions.
- Update, delete and truncate are rejected like XP receipts.

### 4.3 Guard changes (the STANCE-0 migration)

- **State guard:** a further branch admits `revision + 1` with equal `total_experience`, equal
  `level` and every revision field unchanged. Because the DEATH-0 branch also admits equal
  experience, the state guard cannot tell the kinds apart; the deferred guard decides.
- **Consistency guard**, for revision > 1:
  - receipts of all kinds total `revision − 1`, and each revision has exactly one receipt of one
    kind;
  - the receipt for the current revision matches the state;
  - level and experience chain across kinds (`before` = predecessor `after`);
  - the stance chain holds across every receipt that carries a stance transition (a stance receipt,
    or a later combined vocation-change receipt, §4.6): `stance_before` equals the previous such
    receipt's `stance_after`, and is NULL for the first;
  - the `game_character_stance` row equals the latest stance receipt's `stance_after` and revision,
    and is absent when no stance receipt exists. An XP or death commit that changes the row fails.
- **Triggers on the stance row:** `game_character_stance` has a deferred constraint trigger that
  runs the consistency guard on every insert, update and delete, like the `0009` root, state and
  receipt triggers. A delete is rejected outright. So a write that touches only the row, without a
  revision and a receipt, fails at commit.
- **Ordering with DEATH-0:** both migrations replace the same guard functions. The one that merges
  second carries every kind and its tests cover the mixed chain. The migration number is the next
  free number at allocation (0015 at this baseline).

### 4.4 Writer (STANCE-1)

- `commit_character_stance` mirrors `commit_character_experience`: the same recovery fence,
  admission-relation locks, reconnect-session row, runtime-scope assignment, session-generation
  fence and `character_root` row lock; the revision comes from `fence.expected_character_revision`;
  it needs an initialized progression row. A stale fence, including an XP award that moved the
  revision first, writes nothing.
- A retry with the same occurrence and binding returns the first receipt; the same occurrence with
  a different binding conflicts. The binding is compared before replay.
- `reconcile_character_stance` resolves an ambiguous outcome by occurrence.
- The runtime changes the live slot only after the receipt commits; a refused or stale commit leaves
  it unchanged. The ordering with mana and cooldown belongs to the cast wire owner. STANCE-1
  measures the latency of one transaction per toggle (UNKNOWN today).

### 4.5 Death, logout, login

- **Death:** `keep_on_death` is true. The death transaction does not touch the stance and writes no
  stance receipt; the respawned actor reads its slot from the row. The guard in §4.3 rejects a death
  commit that changes the row.
- **Logout:** no write; the last committed toggle is already durable.
- **Login:** the slot is read from `game_character_stance` after the admission fence; no row means
  an empty slot.
- **Stale key:** a stored key that no longer resolves in the active content is treated as inactive
  and not written. The next toggle commits with `stance_before` = the stored key. Content renames
  are UNKNOWN.

### 4.6 Vocation change pruning

- A vocation change is one Character semantic transaction and advances `CharacterRevision` exactly
  once (composition decision). Its own receipt kind, defined by the vocation lane, carries the
  stance transition (`stance_before`, `stance_after`) when the stored stance no longer fits; there is
  no separate stance receipt in that transaction. The stance row follows that receipt, and the
  stance chain in §4.3 counts it.
- Dependency: no vocation store or vocation-change lane exists; that lane defines the combined
  receipt (composition §3.6).
  The fit rule is UNKNOWN (Part C Q20). STANCE-0 and STANCE-1 do not implement pruning.

### 4.7 Rate and size bounds

- Toggles are limited by the Part C.4 cooldowns, enforced by the runtime.
- Receipts grow by one per successful toggle and cannot be pruned (untruncatable chain).
  `CharacterRevision` cannot be exhausted (NUMERIC(20,0)).
- Receipt growth per active character is UNKNOWN; STANCE-1 measures it and reports before Part C
  ships. No resource row is added unless the measurement shows a need.

## 5. Delivery (each child needs its own #162 allocation)

| Child | Scope | Depends on |
|---|---|---|
| STANCE-0 | Migration (`game_character_stance`, `game_character_stance_receipts`), guard changes, immutability and truncate guards, tests. Durability lane, independent review. No writer. | this decision |
| STANCE-1 | `commit_character_stance`, `reconcile_character_stance`, slot load at admission and respawn, cast integration, latency and receipt-growth measurement. | STANCE-0; progression readiness |
| Prune (later) | Clear on vocation change. | a vocation-change lane |

The Part C `stance` key unblocks after STANCE-1. Part C Q19-Q22 stay open.

## 6. Rejected options

- **A stance column on `game_character_progression_state`.** Every guard branch would have to
  freeze it.
- **Write-back at logout.** A crash before the write loses D140 persistence.
- **A runtime-only slot, or clearing on death.** Both violate D140.
- **A stance change without a revision successor, or as a fabricated XP receipt.** Both break the
  chain the guard proves.
- **One generalized receipt table replacing XP receipts.** It rewrites committed evidence, as
  DEATH-0 also rejected.
- **A JSON blob of all slots.** Speculative before the other slots are decided.

## 7. Decision test

- **Must decide now:** YES. STANCE-1 cannot commit under `0009`, and D140 forbids a runtime-only
  slot.
- **Blocked:** Part C `stance` persistence until STANCE-1; pruning until a vocation lane exists.
- **Harder later:** DEATH-0 and STANCE-0 rewrite the same deferred guard; deciding the kind now lets
  both share one guard shape before production rows of mixed kinds exist.
- **Minimum sufficient:** two new tables and the guard branches; no XP or death table changes.
- **Superseding evidence:** a general Character semantic-write receipt design; a vocation-change
  lane; an elemental or crippling slot lane; measured receipt growth that needs a limit.
- **Deliberately not decided:** vocation storage and the fit rule; other slots; Q19-Q22; cast
  latency acceptance; content-rename handling.

## 8. Handback

```yaml
result: RESOLVED
source_escalation: "#162 owner decision D140 (5888104688), Part C.4 stance persistence (A11)"
owner_decisions: [D140, S27]
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_STANCE0_CHARACTER_STANCE_PERSISTENCE_DECISION_2026-09-29.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
implementation_may_resume: true   # STANCE-0 may be allocated
required_fresh_allocation: true
required_independent_review: "exact-head independent review (receipt chain, guards, fencing)"
implementation_lanes: [STANCE-0, STANCE-1]
required_revalidation:
  - "STANCE-0: the stance row has a deferred consistency trigger on insert/update/delete and rejects delete; a row-only write fails at commit; a stance receipt advances the revision with equal XP and level; an XP receipt still needs a strict increase; mixed chains XP -> stance -> death -> XP pass; a gap, duplicate revision, cross-kind mismatch, stance_before mismatch or a stance row that differs from the latest receipt fails the deferred guard; stance receipts are immutable and untruncatable; the row is never deleted"
  - "STANCE-1: same occurrence and binding replays the first receipt; changed intent conflicts; a stale fence writes nothing; stance and XP commits serialize on character_root; logout writes nothing; a death commit leaves the row; admission loads the slot"
remaining_unknowns:
  - CommandRef being UUIDv7
  - vocation storage and vocation-change lane; fit rule (Part C Q20)
  - receipt growth per character; stance-cast latency
  - stored keys that no longer resolve
  - DEATH-0 versus STANCE-0 merge order and migration number
  - the combined vocation-change receipt (vocation lane)
next_action: "#162 validates this exact head, routes the independent review, integrates it, then allocates STANCE-0."
```
