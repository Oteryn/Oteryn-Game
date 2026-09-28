# OTV2-20260928-cw1-teleporter-rearm

```yaml
task_id: OTV2-20260928-cw1-teleporter-rearm
title: D90 teleporter re-arm - lower the post-revert re-arm forward and select the forward by state
mode: IMPLEMENT
status: implementing
repository: Oteryn/Oteryn-Game
issue: 162
base_branch: main
branch: claude/cw1-teleporter-rearm
pr: null
base_sha: 4e65a5e450f7f6f301abbd45fa2457d56a628ba5
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: Oteryn: content world runtime
created_at: 2026-09-28T20:00:00Z
updated_at: 2026-09-28T20:00:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/content/encounter_map_item.rs
  - apps/game-server/src/world_object_revert.rs (tests and the forward-selection helper)
  - docs/agents/tasks/active/OTV2-20260928-cw1-teleporter-rearm.md
  - docs/agents/tasks/archive/OTV2-20260928-cw1-timed-revert-runtime.md (closeout of #1144)
public_contracts: []
depends_on:
  - "task B, PR #1144, merged as ad7a94ca (§7 timed revert runtime)"
  - "owner decision D90 on #162 comment 5875958040"
  - "allocation comment on issue #162: ALLOCATION: OTV2-20260928-cw1-teleporter-rearm"
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Implements owner decision D90 (§9 of
`docs/architecture/OTERYN_INTERACTION_RELOCATION_AND_WORLD_OBJECT_OWNERS_PROPOSAL_V1.md`) as the
smallest slice, and closes out task B:

- `content/encounter_map_item.rs`: for each `revert_destination`-bearing action,
  `lower_map_item_transforms` also lowers a re-arm forward `<action id>/rearm` from the post-revert
  state C back to the forward's target B. It is bound like A→B: the same definition, TRANSFORM
  family, owner capability and guards. It enters B, so it exposes B's attributes (the reward
  destination). Its `revert_after_ms` entry is keyed by the re-arm `TransitionKey` and the same
  action, with the same duration. `LoweredEncounterMapItems` gains `rearm_transitions`, and
  `apply_to_source` adds them to the source before linking.
- `bind`'s widened unique-inverse search accepts the re-arm forward without any runtime change. Its
  only candidate is the existing dedicated revert B→C, whose target is the forward's own source.
  `world_runtime.rs` is unchanged.
- `world_object_revert.rs`: `select_timed_forward` picks the timed forward of one action from the
  runtime's current state. That is A→B from the natural state, C→B from the post-revert variant,
  and `None` while the teleporter is open. More than one candidate fails closed
  (`AmbiguousForward`). It reads only, and it mints no ordinal.

## Tests

- `world_object_revert::tests::duke_teleporter_re_arms_on_every_kill_and_a_kill_while_open_is_a_no_op`
  uses `ManualClock` and `ScopeRevertDriver` on the lowered duke sample:
  - kill 1 commits A→B with the reward destination. After 1,200,000 ms, the revert B→C lands on
    the warzone exit.
  - kill 2 selects C→B and reopens with the reward destination. It schedules a second, distinct
    record with the same inverse. After 1,200,000 ms, the teleporter reverts to C again.
  - The test ends with two distinct `TERMINAL` records.
  - A kill while open, after either kill, selects nothing. No ordinal is minted, no record is
    created, and nothing mutates. The first record stays `TERMINAL` with its outcome.
- `encounter_map_item` tests assert the lowered re-arm transition and its duration entry. They also
  assert that `bind` validates the dedicated revert as the unique inverse of both forwards.
- Existing duke fixtures now also bind the re-arm transition. `bind` requires every
  `revert_after_ms` transition to be bound at the placement. The assertions are unchanged.

## Excluded scope and open points

- D91 (a typed `PLAYER_USE`/`EVENT(owner)` origin on bound transitions) is not implemented here.
  The existing rule still applies: a timed transition is neither USE-selectable nor
  session-invocable, which also covers the re-arm forward.
- Live scope-owner wiring, the encounter trigger (creature death) and any teleport consumer remain
  out of scope.
- A `revert_after_ms` without `revert_destination` reverts to the literal natural state, where the
  authored forward already matches again. No re-arm edge is lowered for it.

## Validation

- `cargo +1.94.0 fmt --all --check`: pass.
- `cargo +1.94.0 clippy --locked --workspace --all-targets -- -D warnings`: pass.
- `cargo +1.94.0 test --locked -p oteryn-game-server`: the full package passes.
- `cargo +1.94.0 run --locked -p oteryn-architecture-check -- workspace .`: pass.
- `python3 tools/agents/validate_governance.py` and
  `python3 tools/repository/validate_repository_policy.py`: pass.
- `git diff origin/main -- content/`: empty.

## Context checkpoint

last_progress: D90 re-arm implemented and validated locally; PR opened for CI and review
jira: pending (no mapped Story resolved in this worker session)
