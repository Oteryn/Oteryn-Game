# OTV2-20260929-n3-client-mouse-input

```yaml
task_id: OTV2-20260929-n3-client-mouse-input
title: N3 - client mouse input, click to tile and click to target (ADR-0020)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
allocation: "#162 allocation of ADR-0020 child N3 (repository worker)"
base_branch: main
branch: claude/n3-client-mouse-input
base_sha: b90f85c9aba8f74f344bdb0969ee04f9c6eebe31
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: "repository worker (Claude Code session_01PwTJFS62J35S88Srpqnrgx)"
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - apps/client/**
  - crates/renderer/src/**
  - docs/agents/tasks/active/OTV2-20260929-n3-client-mouse-input.md
public_contracts: []
depends_on: [ADR-0020, N2]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

ADR-0020 section 5 for lane N3, client-only: pure mouse-to-tile mapping, a click-to-tile walk
state machine over the existing `step` directions, and click-to-target with a renderer highlight.
No command, wire message, server or session change.

## Design

- `apps/client/src/input.rs`: `click_tile` (pixel to tile through the N2 `TileView`, the single
  projection), `step_toward`, `ClickWalk` (one goal, one outstanding step, stops on refusal or
  arrival, no pathfinding), `pick_target` (entity before object before nothing). `StepDir` mirrors
  `StepDirection`; N4 maps it 1:1 when it wires the session.
- `apps/client/src/scene.rs`: fixture-backed `visible` targetables, `select_tile`, `clear_target`;
  the highlight is the existing `Target` placeholder cell pushed on the sprite batch, so
  `crates/renderer` needs no change.
- `apps/client/src/windows_shell.rs`: left click selects a target or sets the walk goal. It is
  Windows-only and was not compiled on the Linux authoring host.

## Acceptance criteria

- [x] Screen-to-tile is a pure function with unit tests.
- [x] Walk steps one tile per accepted step, stops on refusal or arrival.
- [x] Click on a visible entity or object targets it and draws a highlight (fixture tests).
- [x] Tests, clippy `-D warnings` and fmt for `oteryn-client` and `oteryn-renderer`.
- [ ] Required checks green on the exact frozen head.
- [ ] Independent review routed by the control plane.

## Deviations and gaps

- `use` is not sent: no client session exists yet (N4) and targeting selects only. Sending `use`
  for a targeted tile object needs the overlay placement and revision from the session (N4).
- The walk is not driven by a session yet; `ClickWalk` is driven by N4 with `step` results.
- Entities are fixtures until VIS-2.

## Archive closeout

- completed; squash merged as #1239 (b004a8f); archived by the leftover-record housekeeping PR
