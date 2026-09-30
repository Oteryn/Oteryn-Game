# OTV2-20260929-spell-client-ui-w3b

```yaml
task_id: OTV2-20260929-spell-client-ui-w3b
title: SPELL cast client UI - spell hotkeys, cast feedback, health and mana bars
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/spell-client-ui-w3b
issue: 162
lane_id: SPELL cast (native client UI)
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: d4cb72ee
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "impl worker"
control_plane: session_01MnSvpbKjAZEdzEaFrwiu7D
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - apps/client/**
  - workspace-boundaries.toml   # oteryn-client -> oteryn-session edge only
  - Cargo.lock
  - crates/session/src/lib.rs   # one re-export line (see Outcome); lead to confirm
  - docs/agents/tasks/archive/OTV2-20260929-spell-client-ui-w3b.md
public_contracts: []
depends_on: ["#1268 Session::cast_spell", "#1275 N5 closure check"]
blocks: []
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

Client half of SPELL wire contract section 9 step 3, in `apps/client/src/spell.rs`:

- `SpellHotkeys`: digit keys 1-4 routed through the N3 `InputRouter` (text context suppresses),
  yielding spell-book indices.
- `SpellFeedback` and `feedback_text`: one line per `SpellCastDisposition`; `Rejected` reads
  "Spells unavailable" (the closed server gate).
- `cast_selected`: calls `Session::cast_spell` (aimed `AttackTarget` when the scene has a target,
  else `None`) and records the disposition.
- `vitals_bars`: health and mana track and fill quads from `Session::actor_vitals`, drawn from
  solid placeholder atlas cells with the N2 `QuadInstance`; nothing while vitals are absent.
- No shell wiring (no connection in the client yet); pure state and one async helper.

Scope note: `oteryn-session` did not re-export the spell types, and the client may not import
`protocol-oteryn`. One re-export line was added in `crates/session/src/lib.rs`
(`pub use ... {ActorVitals, SpellCastDisposition, SpellTarget}`); no behavior change.

## Validation (local, isolated workspace)

- `cargo fmt --all --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`:
  pass. `cargo test -p oteryn-client -p oteryn-session`: pass (client 21, session 3 in this run).
- `oteryn-architecture-check workspace .`: PASS. N5 closure: `cargo tree -p oteryn-client` shows
  `oteryn-protocol-oteryn` only under `oteryn-session`; no forbidden fragment present.

## Closeout

- Review: none required (client-only, no wire or contract change); lead to confirm the session
  re-export.
- Merge commit/result: squash merge of the SPELL client UI PR (resolve with `git log --grep`).
