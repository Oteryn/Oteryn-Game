# STARTER-BACKPACK-0 Starter backpack

- Decision: `STARTER-BACKPACK0-STARTER-GRANT-V1`
- Status: **CANDIDATE**. Acceptance needs exact-head validation, independent review (persistence and
  security) and protected integration. No owner question: the owner accepted starter inventory as
  template content bound at creation (GAME-CHAR-01 Stage B §6), and the backpack is its Global-parity
  application (owner rule 5905825574).
- Role: Sol Supervising Architect (`OTV2_SOL_SUPERVISING_ARCHITECT` 1.3)
- Answers: the control plane's routing of the C2 escalation (#162 5914960502, Q1 a, answering
  5914818272): who provisions a Character's main backpack, which the reward chest needs.
- Builds on: GAME-CHAR-01 Stage B §6 (owner-accepted: creation binds a starter-template revision;
  starter inventory and equipment are template semantics, not constructor defaults; exact starter
  state is a Reference parity-fixture gate); B3 (D80-D83: the `CharacterEquipment` container slot
  holds the main backpack; migrations `0011`-`0013`); the reward chest decisions §5.1 (D40, D92) and
  their DUR-03 amendment; DUR-03 §14, §37, §39; the composition decision (rules 1-6, §3.1); the
  Character bootstrap writer (`character_authority.rs` `bootstrap_character`)
- Runtime, migration and production authority: NONE. Each child needs its own #162 allocation.
- `MERGE_AUTHORITY: WORK_COORDINATOR_ONLY`

## Implementation brief

| Child | Worker | Builds | Depends on |
|---|---|---|---|
| STARTER-1 | hard, persistence and security review | the grant table, guards and audit operation of §5-§6 in a new migration, and the writer called after fresh admission | this decision; the C2 fence plumbing (`CurrentCharacterItemFence` in `AdmittedSession`); STARTER-CONTENT-1 |
| STARTER-CONTENT-1 | content lane | the backpack `oteryn:item.tibia.i2854` made admissible (materializable, non-stackable, container-slot equippable, capacity 20) and the `StarterKit` record of §4 in the starter template of the current creation revision | this decision |

C2 (`claude/chest-c2-entry-room`) lands before STARTER-1 as ruled (Q3 a): production answers
`NoMainBackpack` until STARTER-1 is live.

## 1. Question

Who gives a Character its main backpack, and how, so that reward chests and later pickups have a
legal destination (D80)?

## 2. Facts

**PROVEN**

- `bootstrap_character` creates the Character in one transaction from an authenticated Platform
  intent: the root with its receipt and intent binding, the name reservation (`0022`) and the
  audit/outbox row (`game_character_audit_outbox`). It writes no item. It is authority-critical.
- The root carries an immutable `starter_template_revision` bound at creation (`0005:59`, guarded by
  `0009` and `0022`), as GAME-CHAR-01 Stage B §6.2 requires.
- `0011`: `game_item_container_slots` holds at most one item per Character, its main backpack. Its
  deferred guard `game_item_placement_proven` admits a slot row only with a same-transaction
  TRANSFER receipt of shape 1 for that item (`0011:307-319`, `0012:256-268`); slot rows are
  immutable (`0011:491`). `game_item_mint_consistency_guard` (latest `0013:171`) admits an item
  insert only as Ground loot, a reward-claim entry or a corpse entry. So no writer can mint into
  the slot today.
- `read_character_backpack` (`item_transfer.rs:886`) finds no slot row for any production Character,
  so the reward chest and pickup refuse with `NoMainBackpack`. Only test harnesses seed a slot item.
- MINT causes: creature-death loot (`ItemMintCause::from_creature_death`) and the reward-claim MINT
  (`0012`, `reward_claim_mint.rs`, not yet called in production).
- `character_item_fence_is_current` (`item_transfer.rs:1005`) requires a CommandRef and a reserved
  World and Channel.
- The item cannot leave the slot today: slot rows are immutable, drop is closed, PLAYER-TRADE-0
  refuses the slot item (`NOT_TRADEABLE`), and death loses no items yet (`0016`,
  `cardinality(lost_item_ids) = 0`).
- The backpack record `oteryn:item.tibia.i2854` exists (capacity 20, weight 1,800) but is
  `materializable: false`, with stack class and equipment `UNKNOWN`.

**DERIVED**

- A13 CHAR-BUILD-1 (candidate) writes no build state at creation.

**OTS_HYPOTHESIS_ONLY**

- A new Tibia character starts with a backpack in its container slot. The full starting kit
  (Dawnport and vocation items) is not verified here; STARTER-CONTENT-1 verifies it against Canary
  and the Tibia wiki before any further template item (Stage B §6.6).

## 3. Ruling: a Game writer after fresh admission, not character creation

- **Not in `bootstrap_character`.** That transaction is the Platform-intent authority path; an item
  MINT there widens an authority-critical writer and its intent binding, and still leaves existing
  Characters without a backpack. The creation transaction already binds what matters: the
  Character's `starter_template_revision`.
- **Not in CHAR-BUILD-1.** Build state is not written at creation.
- **STARTER-1:** a separate, item-only DUR-03 writer that realizes the Character's **own** starter
  template (the one named by its root's `starter_template_revision`) once per template item,
  forever. Attempt 1 runs right after the fresh-admission commit and before the session becomes
  input-eligible. Every Character, including those created before STARTER-1, gets its grant at its
  next fresh admission; no backfill migration.

## 4. The template

- `StarterKit` content records (data only, STARTER-CONTENT-1) belong to a starter template revision
  and name a key, an item definition key and revision, a quantity and a destination. The template
  of the current creation revision holds one record, `oteryn:starter.main_backpack`: one
  `oteryn:item.tibia.i2854` into the container slot.
- The writer resolves the records from the root's `starter_template_revision`, never from the
  World's current content revision, and records that revision in the grant row. A record that fails
  admission (definition not admissible, not container-slot equippable, unknown revision) makes the
  grant fail closed; admission is unaffected.
- A later record (for example the verified Dawnport kit) is a new key; each key is granted once,
  and no key is ever re-granted.

## 5. The grant (STARTER-1)

### 5.1 Table and receipt

`game_character_starter_grants` is both the cause record and the receipt:

- primary key (`character_id`, `template_key`); `world_id`; `starter_template_revision`; the item
  definition key and revision;
- `outcome`: `MINTED` or `SKIPPED_SLOT_OCCUPIED`; `item_instance_id` (set only for `MINTED`);
- `transaction_id`, `event_id`, `envelope_sha256`, `created_xact_id` (stamped by
  `game_item_stamp_created_xact_id`);
- the fence references: GameSessionId, session generation, lease generation, scope ownership
  generation;
- `created_at`. Insert only: immutable and no-truncate triggers; the runtime role gets INSERT and
  SELECT only.

### 5.2 Guards (the STARTER-1 migration)

- `game_item_placement_proven` gains a slot branch that admits a slot row whose item was minted in
  the same transaction by a `MINTED` grant row naming it.
- `game_item_mint_consistency_guard` gains a starter branch for that item.
- A deferred consistency trigger on the grant row checks: for `MINTED`, the item is live with
  quantity 1, its `minted_transaction_id` = the slot's `placed_transaction_id` = the grant's
  `transaction_id`, the audit row comes from the same physical transaction and is pending, and the
  grant's World = the item's World = the root's World; for `SKIPPED_SLOT_OCCUPIED`, the slot row
  exists and `item_instance_id` is NULL.
- The audit uses a new `OneItemTransactionV1` operation `starter_grant` (the next free oneof tag,
  registered by STARTER-1) with a server-originated cause and no CommandRef (DUR-03 §39.2), and a
  MINT destination variant for the slot in `OneItemInventoryV1`.

### 5.3 Fence and shape

- **No CommandRef.** The grant is server-originated. It uses the admitted session's
  `CurrentCharacterItemFence` at commit through a variant without a CommandRef: the grant's
  Character = the fence's Character, the grant's World = the fence's World = the root's World, and
  the session row is current (state 1 or 2, generations equal). No synthetic CommandId is ever
  created. The fence references are recorded in the grant row.
- **Order in one transaction:** the recovery fence and the session-row check; then the
  `character_root` row `FOR UPDATE`; then a re-read of the grant row (a row found is a replay: its
  outcome is returned); then the slot row; then:
  - slot occupied: insert the grant row as `SKIPPED_SLOT_OCCUPIED`, mint nothing;
  - slot empty: mint one item with a fresh ItemInstanceId (DUR-03 §11.3) into the slot, insert the
    slot row, the grant row as `MINTED` and the audit row.
- A primary-key conflict on the grant row (SQLSTATE 23505) is treated as a replay and re-read, never
  as a failure.
- **No revision advance.** The grant is item-only: no `CharacterRevision` advance and no root,
  progression or receipt row (composition rule 1, amended in this PR).

### 5.4 Retry and races

- A transient failure leaves no row; admission is unaffected (item destinations already refuse
  without a backpack). One retry runs within the session after `STARTER0-RL-03` (5 s), alongside
  gameplay, serialized by the `character_root` lock and the slot primary key; later attempts wait
  for the next fresh admission.
- If a player's own shape-1 TRANSFER into the empty slot commits first, the grant records a
  permanent `SKIPPED_SLOT_OCCUPIED`.

### 5.5 Value

- The grant is once per Character and template; a lost backpack is never replaced, as in Tibia.
- Today the item cannot leave the slot (§2), so creating Characters to farm backpacks yields
  nothing. DEATH-3 (item loss) and every ITEM-MOVE child that lets the slot item leave must account
  for this item source in their own value review.

## 6. DUR-03 and composition amendments (in this PR, pending)

- **DUR-03** (after the reward chest amendment): admits one bounded MINT shape and for it supersedes
  the §39.1-§39.3 statements that every MINT descends from a committed creature-death output, that a
  MINT first establishes Ground custody, and that §39.1's exceptions reach only `Container` entries.
  Cause: `StarterGrant {character_id, template_key}`; first and only location: the empty container
  slot; audit evidence as in §5.2. Mint into a stack stays excluded.
- **Composition** (after the PLAYER-TRADE-0 amendment): rule 1 also covers the container-slot
  location written by a starter grant and its grant row. Rules 2 and 3 apply in a server-originated
  variant: the cause lock is the grant key, and the fence is the admitted session's
  `CurrentCharacterItemFence` without a CommandRef (§5.3); rules 4-6 are unchanged.

## 7. Rows (registered by STARTER-1)

| Row | Value |
|---|---|
| `STARTER0-RL-01` records per starter template | 1 in the current template |
| `STARTER0-RL-02` items minted per grant | 1 (one touched item; within `DUR03-RL-08` work units and the `DUR03-RL-07` envelope bounds) |
| `STARTER0-RL-03` in-session retry | one retry, 5 s after the first attempt |

## 8. Rejected options

- **Mint at character creation.** It changes the authority-critical bootstrap and misses existing
  Characters.
- **A seeding migration.** It writes items without a cause per Character and misses Characters
  created afterwards.
- **Reading the template from the World's current content.** A Character's starter state is bound
  at creation (Stage B §6.2).
- **Re-grant when the slot is empty.** It would be a repeatable free item source.
- **A synthetic CommandRef for the grant.** It would invent client authority for a server action.

## 9. Decision test

- **Must decide now:** YES. It gates the first playable reward (#162 5914960502).
- **Minimum sufficient:** one table, guard branches, one audit operation, one writer, one content
  record.
- **Superseding evidence:** the verified Tibia starting kit (STARTER-CONTENT-1) or a Platform
  character-creation flow that carries items.
- **Deliberately not decided:** the rest of the starting kit, vocation items, Dawnport.

## 10. Before-freeze checklist

1. **Contract amendments:** DUR-03 and the composition decision, pending on acceptance (§6).
2. **Serialization:** one grant row per (Character, template key), under the `character_root` lock.
3. **Restart:** the grant row makes every retry replay; no partial state exists.
4. **Typed references:** CharacterId, template key, starter template revision, item definition key
   and revision, ItemInstanceId, fence references.
5. **Wire:** none; the backpack appears through the existing inventory views.
