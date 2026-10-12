# LOOT-LIVE-1

```yaml
task_id: LOOT-LIVE-1
title: "LOOT-LIVE-1: a live kill's committed corpse binds to its projection, so its loot can be opened and taken"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/loot-live-1-20261012
issue: 1622
pr: 2008
base_sha: e128fd6cc
head_sha: null
final_head_sha: null
owner: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
control_plane: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
created_at: 2026-10-12
updated_at: 2026-10-12
migration_lease: none
owned_paths:
  - apps/game-server/src/gameplay_transport/kill_reward.rs
  - apps/game-server/src/gameplay_transport/kill_reward_tests.rs
  - docs/agents/tasks/active/LOOT-LIVE-1.md
  - docs/agents/tasks/archive/LOOT-LIVE-1.md
public_contracts: []
external_repositories: []
```

## Outcome

On the live node a monster's kill already settled its corpse Ground MINT and its authored loot
entries into that corpse durably (ARCH-KILL-REWARD-LOGOUT-1), but nothing bound the committed corpse
Item to the Channel's corpse projection. The corpse stayed invisible in domain 1, so USE could not
open it and command 9 could not take from it. Now the durable settle, once it returns a committed
corpse, binds that Item to the killed actor's projection (`ChannelRuntimeV1::bind_corpse_item`).
The existing visibility refresh then shows the corpse, USE opens it and command 9 takes its
entries through the existing durable TRANSFER.

## Architecture and source of truth

- PROVEN: MAP-ITEM-REF-1 Part B (`docs/architecture/reviews/OTERYN_GAME_ARCH_MAP_TRACK_PACKETS_2026-10-05.md` section 1.6; `docs/agents/tasks/archive/OTV2-20261005-map-item-ref-1.md`) names the live bind after `commit_corpse_mint` as the remaining KILL-REWARD part B step (D607).
- PROVEN: ARCH-KILL-REWARD-LOGOUT-1 (`docs/architecture/reviews/OTERYN_GAME_ARCH_KILL_REWARD_LOGOUT_PACKETS_2026-10-04.md`) owns the capture, the queue and the drain; the settle refuses a principal of another character, lease generation or session (`PrincipalMismatch`), so every durable write stays session-generation fenced.
- DERIVED: the bind is runtime memory only. Durable corpse and loot rows are written once per cause by the existing MINTs (`AlreadyCommitted` on replay), and a replayed settle carries the same corpse Item, which binds idempotently. Another Item never rebinds a bound corpse (`CorpseProjectionConflict`). A projection of another world, Channel or scope generation is refused (`CommittedLethalUnavailable`, D52).
- DERIVED: the lane permit and the revision slot are released before the runtime lock is taken. The runtime lock comes first in the lock order, and no channel guard is held across the settle.
- DERIVED: D133 corpse exclusivity on taking is enforced on the DB clock by the existing corpse TRANSFER (`CorpseExclusiveWindow`). Opening is not gated.
- No new wire format, item semantics, durable write, migration or contract.

## Acceptance criteria

- [x] A committed corpse binds to the killed actor's projection, and that binding becomes the domain-1 corpse with the row's corpse definition at the death position (`a_committed_corpse_binds_to_the_killed_actors_projection`).
- [x] A replayed settle binds the same corpse once (`a_replayed_settle_binds_the_same_corpse_once`).
- [x] Another Item never rebinds a bound corpse (`another_item_never_rebinds_a_bound_corpse`).
- [x] A settle with no committed corpse binds nothing (`no_committed_corpse_binds_nothing`).
- [x] A corpse of another Channel's scope generation is refused (`a_corpse_of_another_channel_generation_is_refused`).
- [x] A live actor with no projected death is refused (`a_live_actor_without_a_projected_death_is_refused`).
- [x] A refused bind leaves the durable result and its Final/Retry classification unchanged, and is logged as `kill_reward_corpse_unbound`.

## Excluded scope

- Client UI.
- Corpse survival across a restart or a generation change (D52, existing behaviour).
- `combat/death_reward.rs` and `foundation/runtime_actor_carrier.rs`, which are hotspots (#2004, MON-RESPAWN-1).
- The stale `dead_code` allowances on `bind_corpse_item` and `top_damage_character` stay in place for a follow-up in the carrier's owning lane.

## Validation

- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --quiet --lib -- kill_reward`: pass (29 passed, 0 failed)
- `cargo test --locked -p oteryn-game-server --quiet --lib -- item_ref_admission item_move corpse death_reward`: pass (72 passed, 0 failed)
- `cargo test --locked -p oteryn-game-server --test combat_death_reward_postgres --quiet` (PostgreSQL 17.6): pass (1076 passed, 0 failed)
- `cargo test --locked -p oteryn-game-server --test corpse_transfer_postgres --quiet` (PostgreSQL 17.6): pass (1039 passed, 0 failed)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass

### Exact-head CI

- PR #2008 CI on 65c062d4c: green (control plane). The frozen head's checks are recorded on the PR.

## Self-review

- method: the implementing agent read the whole diff against the owned paths and the lock order.
- material findings: none open.
- residual risk: a final loot `Mint` error partway through a batch leaves the committed corpse unbound, because that error carries no corpse id. Fixing it means changing `combat/death_reward.rs`, which is a hotspot.

## Independent review

- required: high risk (the authority-bearing runtime projection that follows a fenced durable write). The control plane requests it on the frozen head under the bound review policy.

## PR and closeout

- PR: 2008
- changed-file review: within owned paths
- unresolved review threads: none
- protected merge: control plane, through Merge Queue
- merge commit/result: squash merge of #2008
- source branch closeout: `claude/loot-live-1-20261012` is deleted after the merge
- ownership release: on merge

## Context checkpoint

```yaml
last_progress: corpse bind shipped with tests; CI green; record archived
status: completed
branch: claude/loot-live-1-20261012
pr: 2008
blocker: null
next_action: none (terminal; lifecycle closes on the squash merge of #2008 and the source branch deletion)
```
