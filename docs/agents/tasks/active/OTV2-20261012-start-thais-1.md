# OTV2-20261012-start-thais-1

```yaml
task_id: OTV2-20261012-start-thais-1
title: "START-THAIS-1: new-player start moves to Thais (Tibia 32369, 32238, 7)"
mode: IMPLEMENT
status: waiting
repository: Oteryn/Oteryn-Game
base_branch: main
implementation_branch: claude/start-thais-20261012-r2
issue: 1622
pr: null
depends_on:
  - "PR 2017 (terrain_floor fix; the pin-check passes only on a tree that has it)"
next_action: "host follow-up after merge and redeploy: set node.toml and ROOT_BASE/ops/preprod-topology.toml [world_bundle] start_x = 32369, start_y = 32238, start_floor = -7"
owned_paths:
  - content/world/pins/oteryn.json
  - deploy/synology-game/preprod-topology.toml
  - tools/world-bundle-compiler/tests/pin_check.rs
  - docs/agents/tasks/active/OTV2-20261012-start-thais-1.md
```

## Outcome

Governing issue: 1622. The owner decided the new-player start is Thais, Tibia (32369, 32238, 7) = bundle (32369, 32238, floor -7). The owner chose y = 32238, 3 tiles north of `hometown.temple` (32369, 32241). The tile is plain ground i409 and is enterable with the PR 2017 `terrain_floor` fix.

## Result

- `content/world/pins/oteryn.json` `entry_start` and `deploy/synology-game/preprod-topology.toml` `[world_bundle] start_x/start_y/start_floor` are (32369, 32238, -7) and equal (pin_check.rs).
- The `pin()` fixture in `tools/world-bundle-compiler/tests/pin_check.rs` follows the new start.
- Bundle bytes and digest (`5c32d7ac...4965e7`) are unchanged. The pin `inputs_digest` is re-pinned to 0548b5b1...e3e1c, because editing the `pin()` fixture in `pin_check.rs` changes the hash of the tracked inputs.
- Not changed: the bundle, the node code, any host file.

## Host follow-up (no host was changed)

After PR 2017 and this change merge and the node is redeployed: node.toml and `ROOT_BASE/ops/preprod-topology.toml` `[world_bundle] start_x = 32369`, `start_y = 32238`, `start_floor = -7`; digest, path and `[readiness] map_revision` unchanged.

## Validation

- cargo run --release -p oteryn-world-bundle-compiler -- pin-check: pass (digest unchanged)
- cargo test --release -p oteryn-world-bundle-compiler -- --include-ignored: pass (incl. the release-only pin_check test)
- bash deploy/synology-game/test-preprod-topology.sh: pass
- cargo fmt --all --check: pass
- python tools/agents/validate_governance.py: pass
- python -m unittest discover -s tools/agents/tests: pass (60 tests OK)
