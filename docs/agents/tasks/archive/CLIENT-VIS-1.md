# CLIENT-VIS-1

```yaml
task_id: CLIENT-VIS-1
title: "CLIENT-VIS-1 milestone 1: the Windows client draws real map, item and outfit sprites"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/client-vis-1
pr: 1922
base_sha: eece95e5
head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
final_head_sha: "exact frozen head in the FREEZE_SHA report to the control plane"
merge_commit: "squash merge of #1922"
owner: claude-code-session-01KTDaD4o6oGNavhiXtL6XH7
control_plane: claude-code-session-0114oBVR3osF1auvFMu6ksMH
created_at: 2026-10-07
updated_at: 2026-10-07
owned_paths:
  - apps/client/src/**
  - apps/client/Cargo.toml
  - crates/renderer/**
  - crates/client-assets/**
  - crates/placeholder-assets/**
  - Cargo.toml (workspace member removal)
  - Cargo.lock
  - workspace-boundaries.toml (placeholder-assets removal, oteryn-client edges)
  - docs/agents/tasks/archive/CLIENT-VIS-1.md
public_contracts: []
external_repositories: []
```

## Outcome

The client no longer draws the placeholder board. Offline and in a logged-in session it draws
the floor-7 map around the Thais temple (32369, 32241), within 64 tiles, from the B3 placement
regions and the pinned 15.30 appearances and sprite sheets, with the player in outfit 128.
Arrow keys move the view (offline locally, in a session through the existing step path).
`crates/placeholder-assets` and `PlaceholderScene` are removed.

## Runtime assets

No installer exists today; packaging is unchanged. The client reads assets from
`OTERYN_ASSET_DIR` (a root laid out like the repository: `content/assets/files`,
`imports/official/client-assets/15.30/manifest.json`, `content/world/placements`). The installer provides no map assets, so when the variable is
unset the client draws an empty map. Reads are size-capped, each region is checked against its sha256 in the
placement index, and if loading fails the client prints the reason and draws an empty map.

## Acceptance criteria

- [x] Real appearance ids (map items and outfit 128 in each direction) resolve to cells inside
  the atlas: `client-assets/tests/real_assets.rs`, `apps/client/src/world.rs` and `scene.rs`
  tests.
- [x] Offline and session scenes draw the same real sprites (anchored scene test).
- [x] Placeholder removed; workspace-boundaries PASS.

## Excluded scope

Installer packaging, `tools/qualification/login_local`, `apps/game-server`, Platform, native
login code. Other floors, creatures other than the own player, animation and light are later
milestones.

## Validation

- `cargo fmt --all --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo run -p oteryn-architecture-check -- workspace .`: pass
- `cargo test -p oteryn-client -p oteryn-client-assets -p oteryn-renderer --release`: pass
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: OK
- Windows (`cfg(windows)` shell): the repository Windows CI job on the PR head.

## Review

Codex review on the frozen head; P0/P1 fixed, P2 answered as follow-ups.
