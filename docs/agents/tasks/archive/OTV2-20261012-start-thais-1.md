# OTV2-20261012-start-thais-1

```yaml
task_id: OTV2-20261012-start-thais-1
title: "START-THAIS-1: new-player start moves to Thais (Tibia 32369, 32238, 7)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
implementation_branch: claude/start-thais-20261012-r2
issue: 1622
pr: 2042
depends_on:
  - "PR 2017 (terrain_floor fix; the pin-check passes only on a tree that has it)"
next_action: "host operator: install the committed manifest as ROOT_BASE/ops/preprod-topology.toml, stage the bundle, add the [world_bundle] table and map_revision to node.toml (README World Bundle), redeploy"
owned_paths:
  - content/world/pins/oteryn.json
  - deploy/synology-game/preprod-topology.toml
  - deploy/synology-game/test-preprod-topology.sh
  - deploy/synology-game/README.md
  - tools/world-bundle-compiler/tests/pin_check.rs
  - docs/agents/tasks/archive/OTV2-20261012-start-thais-1.md
```

## Outcome

Governing issue: 1622. The owner decided the new-player start is Thais, Tibia (32369, 32238, 7) = bundle (32369, 32238, floor -7). The owner chose y = 32238, 3 tiles north of `hometown.temple` (32369, 32241). The tile is plain ground i409 and is enterable with the PR 2017 `terrain_floor` fix.

## Result

- `content/world/pins/oteryn.json` `entry_start` and `deploy/synology-game/preprod-topology.toml` `[world_bundle] start_x/start_y/start_floor` are (32369, 32238, -7) and equal (pin_check.rs).
- The `pin()` fixture in `tools/world-bundle-compiler/tests/pin_check.rs` follows the new start.
- Bundle bytes and digest (`5c32d7ac...4965e7`) are unchanged. The pin `inputs_digest` is re-pinned to 0548b5b1...e3e1c, because editing the `pin()` fixture in `pin_check.rs` changes the hash of the tracked inputs.
- Not changed: the bundle, the node code, any host file.

## Host follow-up PR (owner decisions 2026-10-13: Thais start, World Bundle on)

- The start (32369, 32238, -7) is the temple start recorded above (3 tiles north of `hometown.temple` 32369, 32241). The repository manifest already carried it; this follow-up enables the World Bundle in `deploy/synology-game/preprod-topology.toml` (`enabled = true`, `[readiness] map_revision` = the bare pin digest `c49538eb...8524`). `node.toml` is a host file (only `node.toml.template` is in the repository), so the host operator sets it.
- `test-preprod-topology.sh` now fails if the shipped manifest is not enabled or its `map_revision` or `digest` differs from `content/world/pins/oteryn.json`; its plain fixtures disable the bundle again. `pin_check.rs` is untouched (editing it would change the pin `inputs_digest`).
- Host operator, outside the repository: install the manifest at `ROOT_BASE/ops/preprod-topology.toml` (keeping the host's issued ids); stage the bundle (run the deploy with `stage_world_bundle` ticked, default off); append `[world_bundle]` (no `enabled`, `<BASE>` replaced; start 32369/32238/-7) to `node.toml` with `map_revision = "<digest>"`; redeploy. No new secret or environment variable: `OTERYN_NATIVE_GAMEPLAY_MANIFEST` for `supervisor.sh start` is unchanged.

## Host follow-up (original note; no host was changed)

After PR 2017 and this change merge and the node is redeployed: node.toml and `ROOT_BASE/ops/preprod-topology.toml` `[world_bundle] start_x = 32369`, `start_y = 32238`, `start_floor = -7`; digest, path and `[readiness] map_revision` unchanged.

## Validation

- cargo run --release -p oteryn-world-bundle-compiler -- pin-check: pass (digest unchanged)
- cargo test --release -p oteryn-world-bundle-compiler -- --include-ignored: pass (incl. the release-only pin_check test)
- bash deploy/synology-game/test-preprod-topology.sh: pass
- cargo fmt --all --check: pass
- python tools/agents/validate_governance.py: pass
- python -m unittest discover -s tools/agents/tests: pass (60 tests OK)
- bash deploy/synology-game/test-preprod-topology.sh: pass (this follow-up, with the new pin checks)
- bash deploy/synology-game/test-supervisor-gap.sh: pass
- python tools/agents/validate_governance.py: pass (re-run on this candidate)
- python -m unittest discover -s tools/agents/tests: pass (60 tests OK, re-run on this candidate)
