# OTV2-20260930-chest-c2-entry-room

```yaml
task_id: OTV2-20260930-chest-c2-entry-room
title: C2 entry-room reward chest wired end to end (USE_INTENT to the fenced reward-claim MINT)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/chest-c2-entry-room
pr: 1388
allocation: "#162 escalation 5914818272, control-plane answers 5914960502 (Q1a, Q2a, Q3a)"
base_sha: 7be0677
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "Oteryn: chest lane hard worker (claude-code-session-01LYewEE92doyhA6e3hYrp97)"
created_at: 2026-09-30T16:00:00Z
updated_at: 2026-09-30T16:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/interaction/chest_use.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/resume.rs
  - apps/game-server/src/gameplay_transport/qualification.rs  # owners composition only
  - apps/game-server/src/node/serve.rs  # boot composition only
  - apps/game-server/src/lib.rs  # dead_code allows only
  - apps/game-server/src/achievement_catalogue.rs  # test-only len()
  - apps/game-server/tests/support/chest_use_postgres_cases.rs
  - apps/game-server/tests/support/item_transfer_postgres_cases.rs  # harness mint_definition
  - docs/agents/tasks/archive/OTV2-20260930-chest-c2-entry-room.md
public_contracts:
  - USE-WIRE-V1 (consumed; field 1 now also names the entry chest; no wire change)
  - GAME-INTERACTION-01 chest USE slice (consumed)
  - DUR-03 reward-claim MINT (consumed; fence semantics unchanged)
depends_on: [D39 (#1297), CHEST-CONTENT-1, ACHIEVEMENT runtime]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Production answers `NoMainBackpack` (wire `REJECTED`) for every Character until STARTER-BACKPACK
(separate task, Sol queue, STARTER-BACKPACK-0) provisions a main backpack. Everything else of the
first reward chest is wired end to end.

- **Routing (USE-WIRE-V1 field 1, no wire change).** `ComposedFreshAdmission::use_object` sends a
  `USE_INTENT` that names the injected chest placement (`oteryn:placement/entry-chest`) to
  `use_chest`. Every other placement keeps the unchanged door path. The chest `USE` checks the
  chest's World, then reach (the door's rule, read under the Channel-owner lock). It releases
  that lock and calls `interaction_chest_use::use_chest`, which calls `settle_chest_use`.
  Mapping, reusing `UseDisposition`:
  - mint -> `COMMITTED`, with no overlay delta;
  - `AlreadyClaimed` -> `NOTHING_TO_USE`;
  - any other refusal, a missing fence, or an outcome still unproven after 3 same-request
    replays -> `REJECTED`.
- **Bind-time injection (Q2a).** At Channel activation, `with_entry_chest` clones the activated
  entry-room Content and adds:
  - the chest, reward and backpack Item definitions, with known stack classes (D82);
  - the `once` RewardClaim `oteryn:reward-claim/entry-chest` (10 `oteryn:item/entry-reward-coin`);
  - the chest placement, on the non-walkable `entry-north` cell.

  This works like `bind_native_entry_door`. The compiled native Content and its digests are
  unchanged, and the clone is never hashed or projected. `serve.rs` also checks the clone's
  claim achievements against the catalogue.
- **Fence plumbing (Q3a, in-crate).** `AdmittedSession.item_fence: Option<CurrentCharacterItemFence>`:
  - Fresh admission sets it from the reconciled committed session, read by
    `initialize_first_entry` once current authority is proven. It is set only for a positioned
    actor.
  - A committed resume refreshes it from a fresh `current_session_at` read (ACTIVE, successor
    generation, this transport). The lost connection's fence is never carried over.
  - The fence semantics are unchanged: DUR-03 rechecks every field against current rows.
- **Assumption (reversible, listed for the control plane).** The chest `USE` binds the entry
  room's accepted content, ruleset and sim revisions (`accepted::REVISIONS[0]`, `[2]`, `[6]`).
  STARTER-BACKPACK must provision `oteryn:item/entry-backpack` at `oteryn:rev/entry-chest-r1`, or
  amend the injected definition.

## High-risk authority qualification

```yaml
applicable: true  # a production fenced durable write (reward-claim MINT) now has a caller
authority_invariants:
  - identity/binding: the fence names the admitted session's Character, GameSession and scope
  - current liveness/authority: connection generation, lease generation, scope generation current
consumer_boundaries: [freeze_reward_claim_mint, commit_reward_claim_mint (unchanged, recheck all)]
mutation_operators:
  applicable:
    - missing fence -> REJECTED, nothing written (PG)
    - stale connection generation -> AuthorityRejected, nothing written (PG)
    - missing main backpack -> NoMainBackpack, nothing written (PG)
    - replay of a taken once claim by a new command -> NOTHING_TO_USE, nothing written (PG)
    - resume -> fence rebuilt from a current read, never carried (unit: item_fence_of)
  considered_not_applicable:
    - time/expiry: the fence carries no time; DUR-03 owns trusted time
independent_current_fact_sources: [reconciled committed session at admission, current_session_at after resume]
record_derived_matching_helper: none added
```

## Validation

- `cargo fmt --check`, `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass.
- `cargo test --locked -p oteryn-game-server --lib`: 1162 passed.
- PostgreSQL 17.6 (docker `postgres:17.6`): `chest_use_postgres` 763 passed,
  `reward_claim_mint_postgres` 691, `character_authority_postgres chest` 12.
- `validate_governance.py`, `validate_repository_policy.py`, `git diff --check`: pass.

## Closeout

- merge commit/result: squash merge of #1388 (pending)
- frozen head: the FREEZE_SHA comment on #162
- review: required independent exact-head review (fencing), triggered by the control plane
- ownership release: at merge
- follow-up: STARTER-BACKPACK (Sol queue, STARTER-BACKPACK-0) gates the first playable reward
