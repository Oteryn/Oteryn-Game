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
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/interaction/chest_use.rs
  - apps/game-server/tests/chest_use_postgres.rs
  - apps/game-server/tests/support/chest_use_postgres_cases.rs
  - docs/agents/tasks/archive/OTV2-20260929-d39-chest-use-wiring.md
  - apps/game-server/src/lib.rs  # shared: one #[path] module declaration only
public_contracts:
  - GAME-INTERACTION-01 (chest USE slice)
  - DUR-03 (consumed, unchanged)
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
  - definition: the claimed RewardClaim (key and revision);
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
  - The same `CommandRef` replays its first outcome, and a changed intent conflicts.
  - An ambiguous commit is reconciled by calling again with the same request.

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

## Validation

- `cargo fmt --all --check` and `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass.
- `--lib`: 1024 passed, including 3 `chest_use` unit tests. `interaction_workflow`: 15/15.
- PostgreSQL 17.11 (local; CI pins 17.6): `chest_use_postgres` cases 2/2.

## Closeout

- merge commit/result: squash merge of #1297 (pending)
- review: independent exact-head review by the control plane after freeze (pending)
- ownership release: at merge
- follow-ups (routed on #162):
  - a Content child for RewardClaim definitions;
  - the client `USE` command dispatch;
  - cooldown claims and container rewards.
