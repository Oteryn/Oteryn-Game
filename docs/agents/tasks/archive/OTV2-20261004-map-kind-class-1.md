# OTV2-20261004-map-kind-class-1

```yaml
task_id: OTV2-20261004-map-kind-class-1
title: MAP-KIND-CLASS-1 common Terrain kind (R2) and format amendment
mode: IMPLEMENT
status: ready_for_review
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/map-kind-class-1-20261004
issue: 1622
pr: 1786
base_sha: 69f171fc
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: worker for control plane session_013KJX6mv8LQveCKKXYgAX94 (#1622)
created_at: 2026-10-04T00:00:00Z
updated_at: 2026-10-04T00:00:00Z
execution_policy: continuous_progress
owned_paths:
  - docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md
  - docs/architecture/reviews/OTERYN_GAME_WO0_WORLD_OBJECT_AND_TERRAIN_AUTHORING_FORMAT_DECISION_2026-09-28.md
  - tools/content-schema/world-object-authoring/
  - tools/world-bundle-compiler/
  - crates/world-bundle/src/bundle.rs
  - apps/game-server/tests/map_load_base.rs
  - apps/game-server/src/map/mod.rs
  - content/world/terrain/
  - docs/agents/tasks/
public_contracts:
  - OTERYN_WORLD_BUNDLE format (closed Terrain kind set)
depends_on: []
blocks: [MAP-CUTOVER-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Terrain kind `common` for the 50 evidenced overlay tile records (MAP-KIND-CLASS-0 §3.1), so the real
compile no longer stops on UNKNOWN kinds. Unplaced UNKNOWN records (15, e.g. 29407, 31381) stay
UNKNOWN (Codex P2 4177502630): the converter rule is an explicit id list, not a primarytype default.

## Architecture and source of truth

`docs/architecture/reviews/OTERYN_GAME_MAP_KIND_CLASS0_TERRAIN_KIND_CLASSIFICATION_PACKET_2026-10-04.md` §3 R2, §4.
The format is v3 on main (SPAWN-CONTENT-1); `common` amends the closed kind set kept from §12 and
changes no version, since no bundle is published. PROVEN by the packet; format review required.

## High-risk authority/recovery qualification

NOT_APPLICABLE: no persistence, fencing, protocol, identity or authority change.

## Validation

- `python3 test_world_objects.py` (schema tests, 22121 checks): PASS
- `python3 test_official_corpses.py` and `python3 test_qualified_world.py`: OK
- `python3 build_catalogue.py --source <crystal> --donor-source <donor> --official-client --qualified-world --check`: PASS
- `cargo test --locked` in `tools/world-bundle-compiler`: ok
- `cargo test --locked -p oteryn-game-server --test map_load_base`: ok (10 passed)
- `oteryn-world-bundle-compiler parity .` (real map): pass; unknown_kind 0, refused 0, ground 2016, border 3453, wall 1819, roof 198, field 102, common 50, plain_item 4673, world_object 7719
- `cargo fmt --all -- --check`: pass
- `cargo clippy --locked --all-targets -- -D warnings` (game-server, world-bundle, compiler): pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
