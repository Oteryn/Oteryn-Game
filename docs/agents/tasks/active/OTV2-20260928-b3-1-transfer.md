# OTV2-20260928-b3-1-transfer

```yaml
task_id: OTV2-20260928-b3-1-transfer
title: B3-1 DUR-03 TRANSFER from Ground into the container slot and main backpack entries
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/b3-1-transfer
issue: 162
pr: 1152
allocation: "#162 comments 5875188437 and 5875903216"
base_sha: 7d1134f
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: B3-1 implementation worker under the #162 control plane (claude-code-session-01U1WRHgL9X8RbuiG1pwczrF)"
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/migrations/0011_item_transfer_backpack.sql
  - apps/game-server/src/durability/item_transfer.rs
  - apps/game-server/src/durability/item_transfer_audit.rs
  - apps/game-server/tests/item_transfer_postgres.rs
  - apps/game-server/tests/support/item_transfer_postgres_cases.rs
  - docs/agents/tasks/active/OTV2-20260928-b3-1-transfer.md
  - apps/game-server/src/durability/mod.rs                  # shared: module wiring + linkage test
  - apps/game-server/src/durability/item_mint.rs            # shared: uuid_text visibility only
  - apps/game-server/src/durability/item_mint_audit.rs      # shared: oneof tag 3, shared envelope gate, amended RL constants
  - apps/game-server/examples/dur03_native_one_item_audit.rs # shared: registry-equality test for the amended rows
  - apps/game-server/tests/character_authority_postgres.rs  # shared: #[path] include only
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json            # shared: registry single-writer lease (B3 §4.5 rows)
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json      # shared: type 2 note only
  - docs/contracts/game-events/v1/native_one_item_transaction.proto
public_contracts:
  - DUR-03
  - ANL-01
  - GAME-ITEM-01
depends_on: []
blocks: [B3-2]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

A player TRANSFER moves one live Ground item into the character's CharacterEquipment `container`
slot (an empty container into an empty slot) or into a direct entry of the item in that slot (the main
backpack), with the D83 full-merge and top-up shapes. The rules come from
`B3-INVENTORY-DESTINATION-CAPACITY-STACKS-V1` §4:

- D80: newest first, one level only.
- D81: `count < capacity`, at most 20.
- D82: stack maximum 100 or the proven value; an unknown class is refused.
- D83: the receiver is the compatible stack with room that has the highest ordinal.

A refusal writes nothing. Nothing advances `CharacterRevision`.

API in `durability/item_transfer.rs`: `freeze_item_transfer`, `commit_item_transfer`,
`reconcile_item_transfer` and `read_character_backpack`. Inputs are a `CurrentCharacterItemFence` and
an `ItemTransferRequest { command: CommandRef, source item, ContainerSlot | MainBackpack, item/backpack
ItemDefinitionFacts, revisions }`. Migration 0011 adds:

- a lifecycle 2 RETIRED item state;
- `last_transaction_id` on items;
- `game_item_container_slots` and `game_item_container_entries`;
- `game_item_transfer_reservations` and `game_item_transfer_receipts`.

Deferred guards prove every item update, Ground removal and placement through that transaction's
receipt, reservation and audit event.

## Architecture and source of truth

- `PROVEN`: B3 decision §4 (D80-D83, §4.5 rows); DUR-03 §§5.2, 10-13, 20-25, 29-32, 39 and the §39.3 B3
  pointer; `CHARACTER-REVISION-ITEM-TRANSACTION-COMPOSITION-V1` §3 (the XP writer's fence, no revision
  advance, the `character_root` row lock); the MINT pattern (0010, `item_mint*.rs`).
- `DERIVED`: the DUR-03 cause is the FND-02 CommandRef (DUR-03 §§21, 39.1 "actual CommandRef"). The
  reservation fixes TransactionId, EventId, timestamp and the RL-08 budget. The plan and the exact
  event bytes are materialized inside the commit transaction before COMMIT. After a proven non-commit the
  same TransactionId re-materializes for the same intent (§23.2). After commit, the receipt and the
  audit bytes are terminal.
- `DERIVED`: the fenced scope must be the Ground's World and Channel (§32). The Ground's historical MINT
  generation is provenance; current ownership is proven by the assignment and the session row.
- `DERIVED`: definition facts come from the caller, as MINT takes its `TypedDefinitionRef`, and are
  checked against the stored definitions. Binding them to the current Content belongs to B3-2.
- `DERIVED`: audit schema. `OneItemTransferV1` gains `receiver` (6), `OneItemInventoryV1` gains
  `container_entry` (6) and `equipment_container_slot` (7), `OneItemContainerEntryV1`,
  `OneItemReceiverV1` and `OneItemTransferCauseV1` are added, and lifecycle 2 means RETIRED. The
  direct-root `typed_position` stays reserved.
- `UNKNOWN` (content): no content definition declares a known equipment pattern. The backpack
  `oteryn:item.registry.i00002752` has capacity 20 but its equipment is UNKNOWN. The tests use a
  test-only fixture backpack.

## High-risk authority/recovery qualification

```yaml
applicable: YES   # fenced durable write consuming current session/lease/scope/incarnation authority
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - identity/binding: one TRANSFER per CommandRef; same CommandRef + other intent = CONFLICT; CommandRef session = fenced session; reservation character/scope = fenced
  - current liveness: reconnect-session row, runtime assignment + node incarnation, admission guards, recovery fence
  - temporal/provenance: TransactionId/EventId/occurred_at reserved per CommandRef; receipt + audit bytes terminal
  - bounded work: RL-08 charged durably per CommandRef
consumer_boundaries: [freeze_item_transfer (reservation write), commit_item_transfer (item/location write), reconcile_item_transfer, read_character_backpack]
mutation_operators:
  applicable: [stale connection_generation, replaced GameSession, foreign CommandRef, moved lease, stale scope generation,
               scope not owning the Ground, cause keyed to another Character, instance scope, other node holder,
               ended incarnation, session_state 3, ineligible character guard, runtime guard not ready,
               stale recovery fence (freeze and replay), reconnect continuation, same command other intent,
               concurrent same command on two roots, concurrent XP award, RL-08 4th unit, direct SQL tampering]
  considered_not_applicable:
    - "expected_character_revision: not a TRANSFER fence (composition decision §3.2)"
    - "pending-CommandRef ingress proof: runtime-owned FND-02 CommandIngress (B3-2)"
one_invariant_per_negative_case: yes
record_derived_matching_helper: not used
evidence: apps/game-server/tests/support/item_transfer_postgres_cases.rs (every operator at freeze and at commit)
finding_dispositions: {p0_p1_accepted_and_repaired: [], p0_p1_rejected_with_exact_evidence: [], p2_fixed_accepted_or_deferred: []}
```

## Acceptance criteria

- [x] The empty container with a container-slot pattern enters the empty slot. A container without the
  pattern, or with an occupied slot, is refused.
- [x] Entries are placed newest first with no renumbering. The 21st entry is refused and the item stays
  on Ground. With no backpack equipped, the TRANSFER is refused.
- [x] Full merge retires the source, and top-up keeps the source id with the remainder. A top-up with no
  free entry is refused with nothing moved. Units are conserved, and stacks with different quantities
  are compatible. The receiver is the newest compatible stack.
- [x] A retry returns the first result, and reconcile does the same. Same command with another intent
  conflicts. No revision advance, and no XP receipt.
- [x] Rows (max and max+1): DUR03-RL-01 = 2, RL-06-PARTICIPANTS = 2, RL-06-EFFECT-WORK-UNITS = 6,
  RL-03 = 0, GAMEITEM01 stack 100, entries 20, depth 1, reachable 21. The RL-07 worst-case top-up is
  5,778 B of payload (limit 7,821) and 6,816 B of envelope.

## Excluded scope

Pickup wiring (B3-2), weight (B3-3), nested bags, other slots, drop, depot, starter kit, protocol,
client, content data.

## Validation

- `cargo fmt --all --check`: pass.
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: pass.
- `--test item_transfer_postgres`: 454 passed, including the 6 TRANSFER PostgreSQL cases, on local
  PostgreSQL 17.11.
- `item_mint_postgres`: 460 passed. `character_progression_postgres`: 452 passed.
  `character_authority_postgres`: 479 passed. These ran with the 17.6 assertion relaxed locally and
  restored before commit.
- Example `dur03_native_one_item_audit`: 23 passed.
- Governance, repository-policy and semantic-audit validators: see the handback.
- Exact-head CI and independent review: pending. The control plane publishes on
  `claude/b3-1-transfer`.

## Open follow-ups

- Content: a real backpack definition with a complete `container`-slot equip pattern (GAME-ITEM-01
  §6.2).
- B3-2: resolve `ItemDefinitionFacts` from the current Content, and prove that the CommandRef is still
  pending before calling commit.

## Context checkpoint

```yaml
last_progress: local implementation committed on wip/b3-1-transfer; awaiting control-plane publication
status: implementing
branch: claude/b3-1-transfer
pr: 1152
owner_action_required: null
blocker: null
next_action: "#162 control plane publishes, freezes and routes exact-head CI and independent review"
```
