# OTV2-20260929-d39-chest-use-wiring

```yaml
task_id: OTV2-20260929-d39-chest-use-wiring
title: D39 chest USE wiring to the CHEST-1 reward-claim MINT
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/zealous-edison-3ttg1s
issue: 162
pr: 1297
allocation: "#162 comment 5900222015 (architect 5888977195; request 5891571285)"
base_sha: 50d75c65
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: CHEST-1 worker (claude-code-session-01AVd6BKKTRbeW1Pub9bg9Jk)"
created_at: 2026-09-29
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/interaction/chest_use.rs
  - apps/game-server/tests/chest_use_postgres.rs
  - apps/game-server/tests/support/chest_use_postgres_cases.rs
  - docs/agents/tasks/archive/OTV2-20260929-d39-chest-use-wiring.md
  - apps/game-server/src/lib.rs  # shared: one #[path] module declaration only
  # owned-path extension granted by review 5906461018 (items 1 and 2):
  - apps/game-server/src/durability/reward_claim_mint.rs
  - apps/game-server/tests/support/reward_claim_mint_postgres_cases.rs
public_contracts:
  - GAME-INTERACTION-01 (chest USE slice)
  - DUR-03 (consumed; the reward-claim MINT intent now binds its source placement)
depends_on: [CHEST-1 (#1190), D39 contract text (#1210)]
blocks: [client USE command dispatch, cooldown claims, container rewards]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

`interaction_chest_use::settle_chest_use` wires a server-side player `USE` on a placed plain
`once` chest to the CHEST-1 MINT:

- **Target (§5.4).** The server resolves the chest from the current Content. The placement must
  exist and be an Item placement.
- **Child identity (§4.1, §4.3, §5.1, §5.3-§5.7).** The fields are:
  - root: the `USE` `CommandRef` occurrence;
  - definition: the resolved chest placement definition (`Item:key@revision`) plus the claimed
    RewardClaim (key and revision);
  - target: the placement key;
  - edge: `USE`;
  - ordinal: none;
  - revisions: the content, ruleset and sim revisions in force at the `USE`.

  The occurrence key is the placed chest plus the claim (D40).
- **Facts.** The reward item's and the equipped backpack's `ItemDefinitionFacts` are read from
  Content (B3-2 `resolve_item_definition_facts`). With no backpack equipped, the `USE` is refused
  with `NoMainBackpack` before freeze.
- **DUR-03 (§19.1, §17).** The claim goes through `freeze_reward_claim_mint` and then
  `commit_reward_claim_mint`.
  - The same `CommandRef` replays its first outcome, and a changed intent conflicts. The placed
    chest is part of the MINT intent (`source_placement`, intent binding version 2), so the same
    `CommandRef` on another chest conflicts.
  - §17.2: while another `CommandRef`'s MINT of the same (character, claim) is pending, a new
    `CommandRef` is refused with `ClaimPending`. Pending means: reserved, no receipt, RL-08
    budget not spent and its GameSession still live (states 1-2). Only such a reservation can
    still commit.
  - An ambiguous commit is reconciled by calling again with the same request.
- **Replay precondition.** A replay recomputes the intent from the current Content and the
  equipped backpack. It returns the first outcome only while the chest placement, the reward
  item and the backpack definitions are unchanged and the same backpack is equipped. Otherwise it
  is refused or conflicts before any DUR-03 write, and the committed outcome stays durable. The
  FND-02 command-result replay (§17.1) is the path that must answer such a duplicate.

## Architecture and source of truth

- `PROVEN`: the D39 amendment decision §4.1 and §4.3; the reward chest decisions D39-D42; DUR-03
  §39.3; the CHEST-1 API (#1190).
- `DERIVED`: the module is top-level (`crate::interaction_chest_use`), because
  `tests/interaction_workflow.rs` recompiles `interaction/mod.rs` without Content or durability.
  This is the same reason as `combat_pickup`, and it needs one `lib.rs` declaration.
- `UNKNOWN` (declared Content gap): runtime Content has no RewardClaim definition, so there is no
  claim → reward-item and quantity binding.
  - The caller names the claim, the reward item and the quantity, as B3-2's caller names its
    item. The facts still come from Content.
  - A Content child that declares claims is needed before a production caller.

## Acceptance criteria

- [x] A `USE` mints once into a new backpack entry and returns its child occurrence.
- [x] Replay returns `AlreadyCommitted` with the same child; a changed intent returns
  `ConflictingCause`; a new command returns `AlreadyClaimed`.
- [x] Another claim on the same chest is another occurrence and takes the next entry.
- [x] The same `CommandRef` and claim on another chest returns `ConflictingCause`.
- [x] A pending claim refuses a new `CommandRef` (`ClaimPending`) on the same and on another
  chest. A spent RL-08 budget or a non-live GameSession does not block.
- [x] An ambiguous claim (frozen, not committed) reconciles through `settle_chest_use`:
  `Committed` with the frozen transaction and item identities, then `AlreadyCommitted`, with one
  claim row.
- [x] Another edge on the same root, definition and target is another occurrence.
- [x] A replay with the chest gone from Content is refused with `ChestNotPlaced` and writes
  nothing; restored Content replays the first outcome.
- [x] Refused before any write, with zero claims, zero reservations and zero entries:
  - no backpack;
  - placement not found;
  - not an Item placement;
  - reward absent from Content;
  - reward under a non-Item family;
  - empty revision.

  After these refusals, the refused command still commits.

## Excluded scope

- Keys, cooldowns, containers, weight and D37/D38.
- The client `USE` command and its dispatch (control-wire lane). There is no production caller
  yet.

## Review 5906461018 (FIX on 551bf924): dispositions

1. MATERIAL_BLOCKER, chest not in the intent: accepted and repaired.
   `RewardClaimMintRequest.source_placement` is validated and hashed; the binding version is 2.
   PG case: same `CommandRef` and claim on another chest gives `ConflictingCause`.
2. MATERIAL_BLOCKER, a new `CommandRef` admitted while one is pending: accepted and repaired as
   the control plane chose. `admit` refuses with `ClaimPending`. It narrows "unreceipted" to
   reservations that can still commit, because a spent RL-08 budget or a terminal GameSession
   can never commit and would otherwise block the `once` claim forever. The CHEST-1 race test
   now expects one reservation and one `ClaimPending` at freeze. The fence-operator test uses
   one claim per operator command, because a stale commit leaves its reservation pending.
3. EVIDENCE_GAP, ambiguous result: PG case added (frozen identities, `AlreadyCommitted`, one
   claim row).
4. EVIDENCE_GAP, other edge: unit test added.
5. HARDENING, replay from live state: the precondition is declared in the module doc and tested
   (chest removed from Content, then restored). Reconciling from the stored candidate would
   need the facts stored with the reservation, which is a schema change.
6. §5.3 definition component: now the resolved placement definition plus the claim; unit test.
7. Preconditions for the dispatch and Content children, recorded under Closeout.

## Validation

- `cargo fmt --all --check` and `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass.
- `--lib`: includes 5 `chest_use` unit tests and the `ClaimPending` plan cases. `interaction_workflow`: 15/15.
- PostgreSQL 17.11 (local; CI pins 17.6): `chest_use_postgres` cases 3/3 and
  `reward_claim_mint_postgres` cases pass; full `oteryn-game-server` suite on the repair head.

## Closeout

- merge commit/result: squash merge of #1297 (pending)
- review: independent exact-head review by the control plane after freeze (pending)
- ownership release: at merge
- preconditions for the dispatch and Content children (review 5906461018 item 7), not this PR:
  - `content_revision` is not compared with the Content actually used;
  - the placement's world is not checked against the fence scope;
  - any Item placement counts as a chest, and the caller names the reward and quantity;
  - there is no reach or visibility check.
- follow-ups (routed on #162):
  - a Content child for RewardClaim definitions;
  - the client `USE` command dispatch;
  - cooldown claims and container rewards.
