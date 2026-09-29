# OTV2-20260928-appa-appearance-validation

```yaml
task_id: OTV2-20260928-appa-appearance-validation
title: APP-a - pure appearance-selection validation
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
issue: 162
pr: 1172
allocation_comment: "#162 5878933966 (request), owner-authorized in this window (2026-09-28)"
base_branch: main
branch: claude/appa-appearance-validation
base_sha: 51abd337c69b4040731a3c830fa538c32dfb2736  # stacked on PR 1171 (PREM-2a)
head_sha: null
owner: "Oteryn: impl domains" (Claude Code)
created_at: 2026-09-28T21:00:00Z
updated_at: 2026-09-29T06:40:00Z
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/domain/appearance.rs
  - apps/game-server/src/domain/mod.rs  # one line: pub mod appearance;
  - docs/agents/tasks/active/OTV2-20260928-appa-appearance-validation.md
public_contracts: []
depends_on: [OTV2-20260928-prem2a-promotion-soul-rules]
blocks: [APP-1, APP-3]
cross_repository_coordination_id: null
external_repositories: []
```

`execution_policy: continuous_progress` means productive authorized work has no wall-clock stop window.

## Outcome

`apps/game-server/src/domain/appearance.rs` holds the pure rules of the Character appearance owner
decision §4.2-§4.4. It has no caller yet.

- `validate_selection`: the outfit, every addon bit and the mount are each authorized by starter
  content for the sex or a compatible earned unlock; the Store path stays fail-closed until gap
  register §32; addon bits within the outfit's addon count; colours within the content palette;
  Premium not checked (D61). Content and `AccountUnlock` facts come through two caller traits.
- `displayed_look`: the stored selection if valid on this world, otherwise the content fallback for
  the sex; the stored selection is only borrowed (D49); no mount is displayed (§4.2).

## Excluded scope

APP-1 (persistence, fence, `AccountUnlock` storage, provenance), APP-2 (content definitions and
the fallback declaration), APP-3/APP-4 (command, protocol, client), mount activation.

## Validation

- `cargo fmt --all --check`
- `cargo clippy -p oteryn-game-server --all-targets -- -D warnings`
- `cargo test -p oteryn-game-server --lib domain::appearance`
- `python tools/agents/validate_governance.py`, `python tools/repository/validate_repository_policy.py`,
  `git diff --check`
