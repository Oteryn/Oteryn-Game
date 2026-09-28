# OTV2-20260928-chest1-reward-claim-mint

```yaml
task_id: OTV2-20260928-chest1-reward-claim-mint
title: CHEST-1 DUR-03 reward-claim MINT into a main-backpack entry with a once RewardClaim
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zealous-edison-3ttg1s
issue: 162
pr: null
allocation: "#162 comment 5879744231 (owner consent in session; D100 routing 5879348805)"
base_sha: 34ebcef9
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: CHEST-1 implementation worker (claude-code-session-01AVd6BKKTRbeW1Pub9bg9Jk)"
created_at: 2026-09-28
updated_at: 2026-09-28
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/migrations/0012_reward_claim_backpack_mint.sql
  - apps/game-server/src/durability/reward_claim_mint.rs
  - apps/game-server/src/durability/reward_claim_mint_audit.rs
  - apps/game-server/tests/reward_claim_mint_postgres.rs
  - apps/game-server/tests/support/reward_claim_mint_postgres_cases.rs
  - docs/agents/tasks/active/OTV2-20260928-chest1-reward-claim-mint.md
  - apps/game-server/src/durability/mod.rs                  # shared: module wiring + linkage test
  - apps/game-server/src/durability/item_transfer.rs        # shared: fence helper extraction and pub(crate) visibility, no behaviour change
  - apps/game-server/src/durability/item_mint_audit.rs      # shared: oneof tag 4 only
  - apps/game-server/tests/character_authority_postgres.rs  # shared: #[path] include only
  - docs/contracts/GAME_EVENT_FOUNDATION_REGISTRY.json      # shared: type 2 note only
  - docs/contracts/game-events/v1/native_one_item_transaction.proto
public_contracts:
  - DUR-03
  - ANL-01
  - GAME-ITEM-01
depends_on: [B3-1]
blocks: [chest USE wiring (D39), cooldown claims, container rewards]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

A player USE of a reward chest mints exactly one fresh live item directly into a new direct entry of
the character's equipped main backpack and records the character's `once` RewardClaim, in one DUR-03
transaction with its receipt and audit event
(`OTERYN_REWARD_CHEST_PLAYABLE_SLICE_DECISIONS_V1.md` D40-D42 and §5.1 D92; DUR-03 §39.3 reward chest
amendment):

- D92 room: `current_entry_count < definition_capacity` (at most 20); weight is a declared gap (B3-3).
- The new entry takes the highest ordinal plus one; nothing is renumbered.
- Never into an existing stack (DUR-03 §39.1), never a container reward (RL-05 = 0), never onto Ground.
- D42: one RewardClaim per (character, claim family, production key), `next_allowed_at` NULL; a later
  claim revision does not re-open it. The claim commits with the item, entry, receipt and audit event.
- A refusal writes nothing; the player can make room and use the chest again.
- Nothing advances `CharacterRevision` (composition decision §3.1).

API in `durability/reward_claim_mint.rs`: `freeze_reward_claim_mint`, `commit_reward_claim_mint` and
`reconcile_reward_claim_mint`, taking the B3-1 `CurrentCharacterItemFence` and a
`RewardClaimMintRequest { command, claim, item facts, quantity, backpack facts, revisions }`.
Refusals: `AlreadyClaimed`, `RewardIsContainer`, `UnknownStackClass`, `UnsupportedStackMaximum`,
`QuantityAboveStackMaximum`, `NoMainBackpack`, `DefinitionMismatch`, `UnsupportedContainerCapacity`,
`MainBackpackFull`.

Migration 0012 adds `game_reward_claims`, `game_reward_claim_mint_reservations` and
`game_reward_claim_mint_receipts` with deferred guards, and narrows three existing guards so that the
new shape is admitted and nothing else is:

- `game_item_mint_consistency_guard` (0010): an item also exists as the fresh item of a reward-claim
  receipt of the same physical transaction, in its new entry with its audit event. The Ground branch
  is unchanged.
- `game_item_placement_proven` (0011): a backpack entry also exists as the new entry of such a receipt,
  at its exact parent and ordinal. The slot branch and the entry ceiling are unchanged.
- `game_item_ground_insertion_guard` (0011): Ground additionally refuses a reward-claim item and any
  item holding a container slot or entry (a forged MINT receipt cannot give a claimed item a second,
  Ground location).

## Architecture and source of truth

- `PROVEN`: chest decisions D40-D42, §5.1 (D92, CHEST-1 shape, RewardClaim schema); DUR-03 §39.3 reward
  chest amendment; B3 decision §4 (D80-D82 placement, capacity, stack maximum); composition decision
  §3.1 (no revision advance, `character_root` lock, same fence).
- `DERIVED`: the logical transaction is keyed by the player USE CommandRef, like B3-1 TRANSFER. D40
  idempotency per (claim, character) is the RewardClaim primary key: the same CommandRef returns its
  first outcome; another command on a claimed `once` claim is refused with nothing written.
- `DERIVED`: the fence is the B3-1 TRANSFER fence, extracted unchanged into
  `item_transfer::character_item_fence_is_current` and shared.
- `DERIVED`: audit operation tag 4 `OneItemRewardClaimMintV1` (after state, `OneItemInventoryV1`
  destination with `container_entry` only, `OneItemRewardClaimCauseV1` with the CommandRef and the
  claimed RewardClaim definition, explicit prior absence). The envelope carries the fenced Channel.
- `DERIVED` (coverage): one top-level reward item per claim, with no item attributes. In the
  transcribed chest data (`tools/content-schema/quest-authoring/samples/chests/claims.json`) this
  covers the 232 plain `once` claims; every non-container claim has exactly one top-level item. The
  22 key rewards (key binding) and 12 written-text rewards need item attributes, the 64 container
  rewards need the nested-bags decision, and the 6 cooldown claims need the cooldown child.
- `UNKNOWN` (content): the backpack `i00002752` is container-slot equippable with capacity 20 (D114),
  but content does not yet declare it; the tests use the B3-1 fixture backpack.

## High-risk authority/recovery qualification

```yaml
applicable: YES   # fenced durable write consuming current session/lease/scope/incarnation authority
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - identity/binding: one claim MINT per CommandRef; same CommandRef + other intent = CONFLICT; CommandRef session = fenced session; reservation character/scope = fenced; one RewardClaim per (character, claim)
  - current liveness: reconnect-session row, runtime assignment + node incarnation, admission guards, recovery fence
  - temporal/provenance: TransactionId/EventId/ItemInstanceId/occurred_at reserved per CommandRef; receipt, claim and audit bytes terminal
  - bounded work: RL-08 charged durably per CommandRef
consumer_boundaries: [freeze_reward_claim_mint (reservation write), commit_reward_claim_mint (item/entry/claim write), reconcile_reward_claim_mint]
mutation_operators:
  applicable: [stale connection_generation, replaced GameSession, foreign CommandRef, moved lease, stale scope generation,
               another Channel scope, cause keyed to another Character, instance scope, other node holder,
               ended incarnation, session_state 3, ineligible character guard, runtime guard not ready,
               stale recovery fence (freeze and reconcile), reconnect continuation, same command other intent,
               second command on a claimed claim, later claim revision, concurrent same command on two roots,
               concurrent two commands on one claim, concurrent pickup for the last entry, concurrent XP award,
               room lost between freeze and commit, RL-08 4th unit, direct SQL forgery]
  considered_not_applicable:
    - "expected_character_revision: not an item-transaction fence (composition decision §3.2)"
    - "pending-CommandRef ingress proof and the USE child occurrence: runtime-owned (D39 wiring child)"
one_invariant_per_negative_case: yes
record_derived_matching_helper: not used
evidence: apps/game-server/tests/support/reward_claim_mint_postgres_cases.rs (every operator at freeze and at commit)
finding_dispositions: {p0_p1_accepted_and_repaired: [], p0_p1_rejected_with_exact_evidence: [], p2_fixed_accepted_or_deferred: []}
```

Self-review finding fixed before freeze (RED then GREEN): a claimed item is live and never
transferred, so 0011's Ground insertion guard would have accepted a forged MINT receipt plus a Ground
row for it (the item in the backpack and on Ground). 0012 closes this in
`game_item_ground_insertion_guard`.

## Acceptance criteria

- [x] One item mints into a new entry (highest ordinal plus one), never into a compatible stack; no
  Ground location; the claim row is recorded; no revision advance and no XP receipt.
- [x] Replay and reconcile return the first result; the same command with another intent conflicts;
  RL-08 rejects the fourth unit.
- [x] A second command, or a later claim revision, on a claimed `once` claim is refused with nothing
  written; another claim takes the next entry.
- [x] Every refusal and invalid input writes nothing. The 20th entry (max) is admitted; max+1 is
  refused at freeze and, when room is lost after freeze, at commit.
- [x] Every fence operator is rejected at freeze and at commit; the reconnect continuation commits.
- [x] Forged SQL (claim without MINT, second claim, cooldown value, item or entry without receipt,
  missing reservation, quantity mismatch, second Ground location) is rejected at COMMIT; claims,
  receipts and reservations are immutable; the positive control commits; least-privilege grants.
- [x] Concurrency: same command on two roots commits once; two commands on one claim mint once; a
  pickup and a claim racing for the last entry leave exactly one winner; an XP award and a claim
  serialize on `character_root`.

## Excluded scope

The chest USE wiring (D39) and content loading, cooldown claims, container rewards, rewards with
item attributes (key binding, written text), weight (B3-3), achievements, protocol, client and
production.

## Validation

- `cargo fmt --all --check` and `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`: pass.
- `--lib reward_claim` 6, `--lib item_transfer` 11, `--lib item_mint` 12, example
  `dur03_native_one_item_audit` 23: pass.
- PostgreSQL 17.11 (local): `reward_claim_mint_postgres` cases 5/5, `item_transfer_postgres` cases
  15/15, `item_mint_postgres` cases 12/12. `character_progression_postgres` 506/506 and
  `character_authority_postgres` 547/547 ran with the 17.6 pin relaxed locally and restored before
  commit; CI runs 17.6.
- Governance and repository-policy validators: see the handback.

## Open follow-ups

- D39: the chest `USE` wiring binds the GAME-INTERACTION child occurrence and resolves the claim and
  item facts from the current Content.
- Rewards with item attributes (keys, written text), cooldown claims, container rewards and weight,
  each as its own child.

## Context checkpoint

```yaml
last_progress: implementation and local validation complete on claude/zealous-edison-3ttg1s
next_action: publish PR, freeze, post the review packet to #162
```
