# Character revision and item transaction composition decision

- Decision: `CHARACTER-REVISION-ITEM-TRANSACTION-COMPOSITION-V1`
- Status: **CANDIDATE; acceptance requires exact-head validation, independent review and protected integration**
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
- DUR-03 §39.3: "MINT does not touch Character state". The same section forbids inventing an XP
  receipt for inventory work and forbids joining XP and item transfer into one transaction to
  paper over the conflict.
- `0009` guards: `game_character_roots` and `game_character_progression_state` accept only a
  `+1` revision successor. The commit-time consistency guard then requires
  receipts = `CharacterRevision − 1` and an XP receipt at the current revision.
- The XP writer (`apps/game-server/src/durability/character_progression.rs`) checks the admission
  character, account and runtime guards (lease generation, holder GameSession, scope ownership
  generation). It then locks `game_character_roots ... FOR UPDATE`, and only then checks its
  `expected_character_revision`.
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
2. **Fence.** Such a transaction is fenced by the same current-authority facts as the XP writer:
   - the admission character guard (eligible, lease generation, holder GameSession);
   - the account guard (presence and holder);
   - the runtime-scope guard (ready, ownership generation).

   It also carries the DUR-03 TransactionId/cause idempotency. It never uses
   `expected_character_revision` as its authority fence.
3. **Per-Character serialization.** A transaction whose admission depends on the Character's
   inventory occupancy, capacity or claims must hold `SELECT ... FROM game_character_roots ...
   FOR UPDATE` for that Character before it reads or writes item, occupancy or claim rows. The
   lock order is: authority guards → `character_root` → domain rows. This is the XP writer's
   order, so XP awards and item transactions for one Character serialize without deadlock. A row
   lock is not an UPDATE, so the `0009` revision guard does not fire.
4. **Atomicity stays in DUR-03.** D41 and D42 are met inside one PostgreSQL transaction: capacity
   check, MINT of every reward item, the `RewardClaim` insert and mandatory audit all commit or
   none do. A refused capacity check writes nothing.
5. **Character semantic writes are unchanged.** A transaction that changes Character root or
   progression state (an XP award, a future stat or level change) still advances
   `CharacterRevision` exactly once, with its `0009` receipt. If a later accepted transaction must
   change both Character semantic state and item locations, it still advances `CharacterRevision`
   once, for the Character part only. §3.1 covers the item part. This decision does not require
   or allocate such a combined transaction.

## 4. Rejected options

- **Put every item transaction into the `CharacterRevision` chain** with a closed union of typed
  per-revision receipts. This rewrites the accepted and tested `0009` guards, pulls DUR-03 value
  receipts into Character progression consistency and adds a new receipt abstraction. It also
  makes every XP award fail with `CharacterRevisionMismatch` after any loot pickup that ran in
  between.
- **A separate inventory revision domain.** This adds an identity and revision scalar that no
  accepted requirement asks for. DUR-03 already has TransactionId, item non-reuse and
  per-transaction receipts.
- **Invent an XP receipt for inventory work.** DUR-03 §39.3 forbids it.
- **Join the XP award and the item transfer in one transaction to satisfy the guard.** DUR-03
  §39.3 forbids it, and it would change D42's meaning.

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
  - "the first RewardClaim/MINT migration proves: no CharacterRevision change; stale lease, session or scope generation rejected; refused capacity writes nothing; idempotent repeat per (claim, character); a concurrent XP award and item transaction serialize on character_root without deadlock"
remaining_unknowns:
  - inventory position/capacity/weight policy and numbers
  - RewardClaim physical schema and cooldown identity
next_action: "#162 validates this exact head, routes the required independent review and integrates it through the governed Merge Queue."
```
