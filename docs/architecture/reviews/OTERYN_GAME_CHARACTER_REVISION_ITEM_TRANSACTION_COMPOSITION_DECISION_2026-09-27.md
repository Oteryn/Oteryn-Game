# Character revision and item transaction composition decision

- Decision: `CHARACTER-REVISION-ITEM-TRANSACTION-COMPOSITION-V1`
- Status: **ACCEPTED** (protected integration on `main@74bb3fd`, PR #1033; §7)
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.1)
- Source conflict: DUR-03 §39.3 `CONFLICT` and `OTERYN_REWARD_CHEST_PLAYABLE_SLICE_DECISIONS_V1.md` §5
- Related: Issues #162, #707, #513; owner decisions D40-D42 (#1029)
- Admission baseline: `main@bab42d5c9900b05d9a7b4ff941df1fb60d2ea760`
- Runtime, migration and production authority: **NONE**
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## 1. Question

Migration `0009_character_progression.sql` lets the global `CharacterRevision` advance only
together with one immutable XP receipt and a strictly larger `total_experience`. DUR-03 §39.3 and
the reward-chest decisions (D41, D42) need a non-XP write that concerns a Character:

- a pickup TRANSFER into `CharacterInventory`;
- a reward-chest MINT into `CharacterInventory` plus its `RewardClaim` record, in one transaction.

How does such a transaction relate to the global `CharacterRevision`?

## 2. Facts

**PROVEN**

- DUR-02 owner baseline rule 2: every committed Character *semantic* transaction advances one
  global `CharacterRevision` exactly once. The same baseline lists item/currency persistence under
  "Does not authorize".
- DUR-02 schema decision packet §4.1: `character_root` "does not own items/currency, quest
  aggregates, Platform entitlements, lease/session generations".
- DUR-03 §5.1: every live durable ItemInstance has exactly one `ItemLocationRef`, and
  `CharacterInventory { character_id, position }` is one DUR-03 location family.
  GAME-ITEM-01 marks an item's current location as DUR-03-owned.
- DUR-03 §7.2: the durable COMMIT for an item transaction validates the TransactionId/cause, the
  expected item state, the current CharacterLease/session authority and the runtime-scope fence.
  It does not list `CharacterRevision`.
- DUR-03 §39.3: "MINT does not touch Character state". The same section says it neither invents
  an XP receipt for inventory work nor joins XP and item transfer into one transaction.
- DUR-03 §31: a reserved CommandRef may survive an eligible reconnect of the same GameSession
  while `connection_generation` advances. DB commit does not require the old transport
  generation. It requires originally valid ingress, the current logical
  GameSession/lease/runtime authority under FND continuation rules, and current participant
  commit fences.
- `0009` guards: `game_character_roots` accepts only a `+1` revision successor.
  `game_character_progression_state` accepts only a `+1` successor with strictly greater
  `total_experience`. The commit-time consistency guard then requires
  receipts = `CharacterRevision − 1` and an XP receipt at the current revision. So under `0009`
  every revision after 1 must be an XP award.
- The XP writer (`commit_character_experience` in
  `apps/game-server/src/durability/character_progression.rs`) runs these steps in one
  transaction, in this order:
  1. assert the recovery fence and lock the admission relations;
  2. take a per-occurrence advisory lock;
  3. replay: an existing receipt with the same binding returns its retained result without
     reacquiring session authority, and a changed binding conflicts;
  4. for a new occurrence, check the `game_durability_reconnect_sessions` row `FOR SHARE`:
     GameSession, Character, World, Channel scope, `current_generation` = the fence's
     `connection_generation`, lease generation, scope ownership generation and
     `session_state IN (1,2)`;
  5. check the `game_runtime_scope_assignments` row `FOR SHARE` (active, ownership generation,
     holder node and registration revision) and prove the current node incarnation;
  6. check the admission character, account and runtime guards;
  7. lock `game_character_roots ... FOR UPDATE`, and only then check `expected_character_revision`.

  `reconcile_character_experience` also asserts the recovery fence before its receipt lookup.
  `lock_admission_relations` takes an EXCLUSIVE lock on the admission relations, including the
  character, account and runtime guards and `game_durability_reconnect_sessions`. So a concurrent
  authority change serializes with the writer.

  The fence is supplied by the current runtime owner at commit time. It is not captured at
  ingress: the binding test shows a reconnected fence yields the same command binding.
- No migration on protected main creates an inventory or reward-claim table.
- D40: a reward MINT is idempotent per (claim, character). D42: `RewardClaim` is its own
  per-character record, is not a quest-progress track, does not reuse the XP tables, and commits
  in the same DUR-03 transaction as the minted items, fenced by the character session generation.

**DERIVED**

- An item-location change into, out of or inside `CharacterInventory` is a DUR-03 value
  transaction, not a Character semantic transaction under DUR-02 rule 2, because DUR-02 excludes
  items from Character root ownership.
- `RewardClaim` is the durable non-reuse record for the D40 MINT source cause (claim, character).
  It is DUR-03 cause state keyed by a Character, not Character root semantic state.
- So the conflict comes from reading "concerns a Character" as "advances `CharacterRevision`".
  Under the accepted ownership split it does not, and `0009` needs no change.

**UNKNOWN** (still owned by their existing gates)

- inventory position legality, capacity and weight policy, and their numbers;
- the `RewardClaim` physical schema and the identity of a cooldown claim cycle;
- the source-cause receipt grammar and retention.

**CONFLICT**

- None remain once the rule in §3 is accepted.

## 3. Decision

1. **No revision advance for item transactions.** A DUR-03 transaction whose only
   Character-related effects are item locations in `CharacterInventory` and DUR-03 cause records
   keyed by a Character (for example `RewardClaim`) does not advance `CharacterRevision`. It does
   not write `game_character_roots`, `game_character_progression_state` or
   `game_character_xp_receipts`. `0009` stays unchanged.
2. **Fence.** Such a transaction uses the XP writer's complete fence (§2, steps 1–6), with
   the DUR-03 cause taking the place of the XP occurrence:
   - the recovery fence and admission-relation locks;
   - a lock on the DUR-03 cause identity (for a reward, (claim, character)), then replay: the
     same cause and binding return the first outcome, and a changed binding conflicts;
   - for a new effect, the reconnect-session row (GameSession, Character, World, runtime scope,
     `current_generation`, lease generation, scope ownership generation,
     `session_state IN (1,2)`);
   - the runtime-scope assignment and current node incarnation;
   - the admission character, account and runtime guards;
   - binding checks: the cause's CharacterId equals the fenced Character, and the fenced runtime
     scope owns the source placement or Ground (DUR-03 §32). A fence taken from another session
     or scope, or a cause keyed to another Character, commits nothing.

   It never uses `expected_character_revision` as its authority fence.
3. **Session-generation semantics and the DUR-03 §31 continuation.** The committing runtime
   supplies the fence from its *current* authority at commit time, as the XP writer does:
   - After an eligible same-GameSession reconnect, the current owner commits a still-pending
     reserved CommandRef with the current `connection_generation`. The same GameSession, lease
     and scope generations must still match.
   - Original ingress validity is proven by the current owner's FND-02 `CommandIngress` still
     holding that CommandRef as pending (#663). A CommandRef that the current owner does not hold
     as pending is not continued.
   - A stale predecessor that presents an older `connection_generation`, a replaced GameSession,
     a moved lease or scope, or a session outside states 1–2 fails the reconnect-session row
     check and commits nothing.
   - A lost response or retry after commit resolves through the cause replay step. Replay still
     asserts the current recovery fence, as `commit_character_experience` and
     `reconcile_character_experience` both do before the receipt lookup. It waives only the live
     session, lease and runtime checks. A stale recovery fence returns no outcome.
4. **Per-Character serialization.** A transaction whose admission depends on the Character's
   inventory occupancy, capacity or claims must hold `SELECT ... FROM game_character_roots ...
   FOR UPDATE` for that Character before it reads or writes item, occupancy or claim rows. The
   lock order is the XP writer's: recovery fence and admission relations → cause lock → session,
   assignment and guard checks → `character_root` → domain rows. XP awards and item transactions
   for one Character therefore serialize without deadlock. A row lock is not an UPDATE, so the
   `0009` revision guard does not fire.
5. **Atomicity stays in DUR-03.** D41 and D42 are met inside one PostgreSQL transaction: capacity
   check, MINT of every reward item, the `RewardClaim` insert and mandatory audit all commit or
   none do. A refused capacity check writes nothing.
6. **XP-backed Character writes are unchanged; other Character semantic writes are not
   covered.**
   - An XP award still advances `CharacterRevision` exactly once, with its `0009` receipt.
   - Under `0009` no non-XP Character semantic mutation can advance `CharacterRevision`,
     whether a stat, level-without-XP, profile or other change. It would fail the deferred
     consistency guard or need a fabricated XP receipt. Admitting any such mutation needs a
     later migration and receipt redesign under its own architecture decision.
   - An XP award and DUR-03 item effects may share one transaction under `0009`. The XP part
     advances the revision with its receipt, and §3.1 covers the item part. This decision does
     not require or allocate such a combined transaction.

### 3.1 Amendment (2026-09-28): B3 and reward chest locations

The B3 decision (`OTERYN_GAME_B3_INVENTORY_DESTINATION_CAPACITY_AND_STACKS_DECISION_2026-09-28.md`,
D80) places items in the character's `CharacterEquipment` container slot and in direct entries of
its equipped main backpack (DUR-03 §5.2 `Container`), and the reward chest `CHEST-1` child mints
into those entries (reward chest decisions §5.1, D92). Rule 1 applies to those locations exactly
as to `CharacterInventory`: a DUR-03 transaction whose only Character-related effects are item
locations in the character's container slot or in direct entries of its equipped main backpack,
and DUR-03 cause records keyed by a Character, does not advance `CharacterRevision` and writes no
Character root, progression or receipt row. Rules 2-6 (the complete fence, the cause lock and
replay, the `character_root` row lock, atomicity in DUR-03) apply unchanged. Nested bags and other
equipment slots are not covered until their own decisions.

## 4. Rejected options

- **Put every item transaction into the `CharacterRevision` chain** with a closed union of typed
  per-revision receipts. This rewrites the accepted and tested `0009` guards, pulls DUR-03 value
  receipts into Character progression consistency and adds a new receipt abstraction. It also
  makes every XP award fail with `CharacterRevisionMismatch` after any loot pickup that ran in
  between.
- **A separate inventory revision domain.** This adds an identity and revision scalar that no
  accepted requirement asks for. DUR-03 already has TransactionId, item non-reuse and
  per-transaction receipts.
- **Invent an XP receipt for inventory work.** It fabricates XP evidence and breaks the
  progression receipt chain. DUR-03 §39.3 explicitly declines it.
- **Attach an XP award to every item transaction just to satisfy the guard.** Most item
  transactions carry no XP, so it would change D42's meaning. DUR-03 §39.3 declines it. A genuine
  XP award that happens to share a transaction with item effects is a different case (§3.6).

**Amendment (DEPOT-0, 2026-09-30), pending on acceptance of DEPOT-0.** Once DEPOT-0 is accepted,
rule 1 also covers `CharacterDepot` locations of the acting
character (`OTERYN_GAME_DEPOT0_CHARACTER_DEPOT_DECISION_2026-09-30.md` §7.2): an item-only
transaction between its backpack and its depot does not advance `CharacterRevision`. No runtime
scope owns the depot, so rule 2's DUR-03 §32 binding does not apply.
Rule 4's lock order: `character_root`, then the items in ItemInstanceId order, then the
container-slot row.

## 5. Decision test

- **Must decide now?** YES. It blocks step 2 of the reward-chest order of work (DUR-03 and
  Character text for D40-D42) and the Character-destination part of DUR-03 TRANSFER admission
  (§39.3).
- **Concrete work blocked:** the `RewardClaim`/MINT migration and durability code (reward chest
  §7 step 3), and the Combat pickup TRANSFER after DUR-03 B2 (#1031).
- **Harder later if left open:** each implementation would pick its own composition, or stall.
  Code written against a revision-bumping model would need migration.
- **Superseding evidence:** an accepted requirement for one Character-wide total order over item
  moves (for example a Character export or snapshot keyed by `CharacterRevision` that must cover
  inventory), or measured contention on the `character_root` lock that needs another anchor.
- **Deliberately not decided:**
  - inventory position, capacity or weight policy and numbers;
  - `CharacterEquipment` and stat-affecting equip semantics;
  - the `RewardClaim` physical schema and cooldown identity;
  - TRANSFER admission itself;
  - resource limits;
  - protocol, client and production.

## 6. Handback

```yaml
result: RESOLVED
source_escalation: "DUR-03 §39.3 CONFLICT / reward-chest decisions §5 (#707, #513)"
durable_decision_ref: docs/architecture/reviews/OTERYN_GAME_CHARACTER_REVISION_ITEM_TRANSACTION_COMPOSITION_DECISION_2026-09-27.md
resource_values_changed: false
production_authority_changed: false
cross_repository_authority_changed: false
implementation_lane: DUR-03 with the Character owner, after a fresh #162 allocation
implementation_may_resume: false   # until this decision is protected-integrated
required_fresh_allocation: true
required_independent_review: "exact-head independent review (persistence/value and session-fence semantics)"
required_revalidation:
  - DUR-03 and Character contract text for D40-D42 cites this decision (reward chest §7 step 2)
  - "the first RewardClaim/MINT migration proves: no CharacterRevision change; each one-changed stale fence rejected (connection_generation, GameSession, lease generation, scope ownership generation, session_state, runtime assignment, node incarnation); a still-pending reserved CommandRef commits after an eligible same-GameSession reconnect with the current connection_generation; a cause keyed to another Character or a fence scope that does not own the source is rejected; a retry after commit replays the first outcome under the current recovery fence; a stale recovery fence on replay returns no outcome; a concurrent authority change serializes with the item commit through the admission-relation locks; refused capacity writes nothing; idempotent repeat per (claim, character); a concurrent XP award and item transaction serialize on character_root without deadlock"
remaining_unknowns:
  - inventory position/capacity/weight policy and numbers
  - RewardClaim physical schema and cooldown identity
next_action: "#162 validates this exact head, routes the required independent review and integrates it through the governed Merge Queue."
```

## 7. Protected integration

- PR #1033, frozen head `88351368710f9c03f5835b945013874caa99d9fa`.
- Independent review: Codex exact-head review of that head found no issues. All earlier findings
  (P1s on `432dc37`, `ec82b6c` and `f7e4970`; the P2 on `432dc37`) were accepted and repaired.
- Merge Queue: merge_group `game-gate` SUCCESS (run 36387297582); squash-merged as
  `74bb3fd38698ba0f76cb36f449d2fa023f12f4f4` on 2026-09-28.
- Protected-main readback: this file and the task record matched the frozen head blobs
  (`53234d3a`, `c8106ee3`).
- DUR-02 rule 2 (the Character contract), DUR-03 §39.3 and reward-chest decisions §5 cite this
  decision (§6 `required_revalidation`, first item).

```yaml
result: ACCEPTED
implementation_may_resume: true   # under a fresh #162 allocation; the §6 revalidation cases still bind it
superseded_handback_fields:
  implementation_may_resume: "false until protected-integrated -> true"
remaining_unknowns:
  - inventory position/capacity/weight policy and numbers
  - RewardClaim physical schema and cooldown identity
  - TRANSFER admission
```
