# OTV2-20261004-map-bundle-2

```yaml
task_id: OTV2-20261004-map-bundle-2
title: "MAP-BUNDLE-2: World Bundle format v2 with terrain semantics"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/map-bundle-2-20261004
issue: 1622
pr: 1751
head_sha: "exact frozen head in the FREEZE_SHA message to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA message to the control plane"
owner: claude-code-session_01FqspUyiMBJXa3VZqUkq6ra
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
owned_paths:
  - tools/world-bundle-compiler/**
  - docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md
  - docs/agents/tasks/archive/OTV2-20261004-map-bundle-2.md
public_contracts:
  - docs/contracts/OTERYN_WORLD_BUNDLE_FORMAT_V1.md
depends_on: []
blocks: [MAP-LOAD-1, SPAWN-CONTENT-1]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

Bundle format v2 (decision MAP-LOAD-PACKET-1 §1.4, owner D496 1a). Each palette entry carries a
required `terrain`: `null` for a WorldObject route or plain Item, else `{kind, walkable,
ground_speed}`. The compiler resolves it fail-closed; the reader and writer reject malformed,
out-of-range or v1 input. A Terrain-family entry with `terrain: null` is refused by both the
reader and the writer (allocation note, #1744 P2 4177139457), with a test.

## High-risk authority/recovery qualification

`NOT_APPLICABLE`: build-time format and compiler change; no session, fence, lease or persisted recovery state.

## Implementation / findings

- `KeyResolver` gains a required `terrain` method; existing test resolvers return their prior
  routes (`None`, or ground for `terrain:grass`). No other existing test changed besides the
  `PaletteEntry` field and the v2 digest domain string.
- Real map (`parity .`): placed entries by kind: ground 2016, border 3453, wall 1819, roof 198,
  field 102; 7719 WorldObject-routed and 4673 plain-Item entries (`null`); 50 Terrain-routed
  records with UNKNOWN kind. The real compile therefore stops until the content lane (through
  the control plane) classifies them, before MAP-CUTOVER-1.
- Security review of the format change (ADR-0021 §4.8) is requested on the PR.

## Validation

- `cargo test --locked -p oteryn-world-bundle-compiler`: pass
- `cargo check --locked --workspace --all-targets`: pass
- `cargo fmt --all -- --check`: pass
- `cargo clippy -p oteryn-world-bundle-compiler --all-targets -- -D warnings`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
