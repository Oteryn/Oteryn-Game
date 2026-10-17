# OTV2-20261015-npc-content-2

```yaml
task_id: OTV2-20261015-npc-content-2
title: NPC-CONTENT-2 movement facts and admitted voices on NPC definitions
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/npc-content-2
issue: 1622
pr: 2129
base_sha: 601db9a7
head_sha: null
owner: claude-code-worker (CP session_01XYC239givrMonrYoBwwECV)
control_plane: session_01XYC239givrMonrYoBwwECV
created_at: 2026-10-15
updated_at: 2026-10-15
decision: NPC-BEHAVIOUR0 §5, §6; OTERYN_NPC_AUTHORING_SCHEMA_V1 D9
plan_row: OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md NPC-WORLD-1, NPC-CONTENT-2
builds_on: NPC-CONTENT-1 (#1858)
migration_lease: none
owned_paths:
  - tools/content-migration/npc_presence_stage.py
  - tools/content-migration/test_npc_presence_stage.py
  - tools/content-schema/npc-authoring/convert.py   # movement.base_speed only
  - tools/content-schema/npc-authoring/npc.schema.json   # movement.base_speed only
  - docs/agents/evidence/OTV2-20261015-npc-content-2-presence.json
  - apps/game-server/examples/npc_materializer/presence.rs
  - apps/game-server/examples/materialize_content_world_project_v2.rs   # the presence step only
  - apps/game-server/tests/content_npc_catalogue.rs
  - apps/game-server/tests/content_world_project_repository.rs   # package digests only
  - content/world/{project,manifest,content.lock}.json
  - content/world/definitions/declarations.json
  - content/world/pins/oteryn{,.identity}.json
  - content/**/index.json, content/{manifest,content.lock,spells.manifest}.json   # regenerated digests only
  - content/npcs/definitions/npcs-*.json
  - content/creatures/definitions/spell-native-profiles.json   # regenerated digest only
  - deploy/synology-game/npc-data.pin.json
  - deploy/synology-game/preprod-topology.toml   # world pin digest and revision only
  - .github/workflows/content-tree-migration.yml   # runs the new unit test only
  - tools/content-schema/quest-authoring/samples/binding_packets/{index.json,npc/quest-npc-dialogue-links.json,recipes/supplementary-index.json}   # regenerated NPC input digests only
  - tools/content-schema/quest-authoring/samples/server-completion/{talk-npc,explore-area}-candidates.json   # regenerated input digests only
  - docs/agents/tasks/{active,archive}/OTV2-20261015-npc-content-2.md
public_contracts: []
external_repositories: []
```

## Outcome

- **Staged facts (D9):** `npc_presence_stage.py` reads the `convert.py --include-text` outputs of
  the pinned Canary (`47dfd51f`), Crystal (`ff7ede59`) and Crystal summer-update (`00ce02a5`)
  checkouts. Each NPC takes the facts of the source files its definition binds to and is admitted
  only where every bound source agrees (or one source has the NPC). The packet
  `OTV2-20261015-npc-content-2-presence.json` is pinned by SHA-256.
- **On the NPC definitions:** 1122 sourced NPCs carry `oteryn:source.npc.base_speed` (55, the
  engine default; no source sets `speed`) and `oteryn:source.npc.floor_change` (false; no source
  sets `floorchange`). 282 NPCs carry `voice_interval_ms`, `voice_chance_percent` and
  `voice_lines` (626 static lines, a JSON array of `{text, yell}`; the longest is 200 characters,
  none has a placeholder). 7 NPCs whose sources differ are held and say nothing (§6); 160 NPCs
  have no bound source file and get nothing.
- **Walk interval and radius** already live on each NPC's Behavior profile
  (`movement.can_walk`, `movement.wander {interval_ms, radius_tiles}`) and agree with the
  sources; they are not restaged.
- The materializer chains the step on the ranged package (RANGED-CONTENT-1, tree digest
  `71e96ccf`, itself chained on the consumables package) and mints `npc-content-2-presence-20261015-r1` (DUR-04). The successor tree, the NPC data pin,
  the World pin and the test digests are regenerated from it.

## Excluded

- Reading the facts at runtime: NPC-ACTOR-1 (walking, §5) and NPC-VOICE-1 (voices, §6).
- No protocol, persistence or registry change.

## Validation

- `cargo run -p oteryn-game-server --example materialize_content_world_project_v2` from the pinned
  predecessor equals the tracked package
- `python tools/content-migration/npc_presence_stage.py ... --check`: pass
- `python tools/content-migration/test_npc_presence_stage.py`: pass
- `python tools/content-migration/world_project_v2_to_tree.py` and
  `validate_world_project_v2_to_tree.py`, `test_world_project_v2_to_tree.py`: pass
- `cargo run --release -p oteryn-world-bundle-compiler -- pin-check`: pass
- `deploy/synology-game`: `test_stage_npc_data`, `test-preprod-topology.sh`: pass
- `tools/content-schema/npc-authoring` unit tests: pass
- `cargo test -p oteryn-game-server --test content_npc_catalogue --test content_npc_service_model
  --test content_world_project_repository --test content_world_project_v2
  --test content_world_cw2_b1_import`: pass
- `cargo fmt --all -- --check`, `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass
- `python tools/agents/validate_governance.py`, `git diff --check`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `tools/content-schema/quest-authoring/run_checks.py`: pass, except `quest_donor_source_authoring.py --check`,
  whose local drift is only the gzip header of Python 3.13 (CI runs 3.12; decompressed content identical)

## Review

- CI on `004d240`: `quest-authoring` failed on stale NPC input digests in the Quest binding
  packets and the talk-NPC and explore-area candidates; regenerated with their generators.

- Merge of main (RANGED-CONTENT-1 landed on the same consumables predecessor): the presence step
  now applies on the ranged package; the packet's `from_project_revision` is the ranged revision
  (`FROM_REVISION` in the stage tool), the tree digest is `16913e9c`, and the tree, World pin
  (`03f64820`), NPC data pin, preprod topology, test digests and Quest NPC input digests were
  regenerated with their tools. The red `World bundle`, `validate` and `game-gate` checks were one
  cause: a stale World pin `inputs_digest` against the tree.

## PR and closeout

- changed-file review: within owned paths
- merge commit/result: squash merge of #2129
