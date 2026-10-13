# OTERYN-20261012-npc-load-1

```yaml
task_id: NPC-LOAD-1
title: "NPC-LOAD-1: the World Bundle's NPC placements become NPC actors at Channel start"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/npc-load-1-20261012
pr: 2010
issue: 1622
base_sha: e128fd6cc
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
owner: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
control_plane: claude-code-session-01CwP6d84eCPvpgoEuyci8Tx
coordination: "#1622"
created_at: 2026-10-12
updated_at: 2026-10-12
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/node/serve/bundle_npcs.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/src/foundation/mod.rs
  - apps/game-server/src/foundation/npc_actor.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/src/gameplay_transport/world_spatial.rs
  - docs/agents/tasks/active/OTERYN-20261012-npc-load-1.md
  - docs/agents/tasks/archive/OTERYN-20261012-npc-load-1.md
public_contracts: []
external_repositories: []
```

## Outcome

A `[world_bundle]` World boots the bundle's format v4 NPC placements as NPC actors when its
Channel starts (NPC-ACTOR-1 of NPC-PLACE-1 §7). There is one actor per placement, in frame
order, placed before any creature. The actors are bound to the Channel's own scope, block
movement, are never targets, are never removed and never respawn. The join snapshot sends them
to clients as `EntityKind::Npc`.

## Architecture and source of truth

- PROVEN:
  - NPC-PLACE-1 §7: production refuses on a catalogue digest mismatch or an NPC that is not
    admitted or is held; non-production logs it and places no actor.
  - NPC-RT-1 `ChannelRuntimeV1::place_npcs` (carrier slot `NpcOccupied`, all or nothing,
    runs once).
  - The world-bundle v4 reader: `bundle::visit` returns `npcs` and
    `manifest.npcs.catalogue_sha256`.
- DERIVED: `node/serve/bundle_npcs.rs`:
  - `read` takes the frame from the verified bundle bytes.
  - `LoadedNpcCatalogue` holds the admitted keys of `service_model_for_project()` and
    `npc::catalogue_sha256` of the project root's parent.
  - `bind` applies the §7 decision.
  - `place` runs the placement and logs it.
  - `serve.rs` gets a minimal call site.
  - The production flag is the configured `[world_bundle].production` pin.

## Excluded scope

- No world-bundle or compiler change and no re-pin.
- Not done here:
  - the NPC talk boot wiring (`install_npc_talk` caller);
  - NPC services;
  - deploy staging of the catalogue (`--npc-data-project`).

Follow-ups:
- After #1981 merges, fold the extra `bundle::visit` pass into its spawn load path.
- Rebase onto #1981 and #2006, which both touch `serve.rs`.
- The deploy must pass `--npc-data-project` and stage the NPC, dialogue and service content.
  Until then, a non-production World logs `npc_actors state=skipped reason=no_catalogue`.

## Acceptance criteria

- [x] A synthetic bundle with 2 NPCs boots 2 NPC actors bound to the Channel scope
  (`a_bundle_with_two_npcs_boots_two_npc_actors_bound_to_the_channel_scope`).
- [x] An unknown NPC refuses a production World and leaves no actor; outside production it is
  skipped (`an_unknown_npc_refuses_a_production_world_and_is_skipped_outside_it`).
- [x] A missing or mismatched catalogue refuses production and places nothing outside it.
  A damaged bundle is refused.

## Implementation / findings

- The foundation edit is a visibility re-export.
- The `npc_actor.rs` and `world_spatial.rs` edits remove stale `dead_code` allows now that
  there is a caller.

## Validation

- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-game-server --lib --tests -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --lib -- bundle_npcs npc world_spatial node::serve`: pass (61 passed, 0 failed)
- `cargo test --locked -p oteryn-game-server --lib -- bundle_npcs gameplay_transport`: pass (467 passed, 0 failed, 6 ignored)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass (60 tests, OK)

## Self-review

- method: the implementing agent read the whole diff against the owned paths.
- material findings: none open.
- P2 accepted: one extra full bundle pass at boot, to be folded in after #1981.

## Independent review

- required: per the bound review policy, requested by the control plane on the frozen head.

## PR and closeout

- changed-file review: within owned paths
- merge commit/result: squash merge of #2010
