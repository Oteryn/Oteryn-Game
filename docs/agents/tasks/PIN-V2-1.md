# PIN-V2-1 — remove input-only World pin churn

```yaml
task_id: PIN-V2-1
title: Introduce OTERYN_WORLD_PIN/v2 without inputs_digest
mode: REPAIR
status: validated
repository: Oteryn/Oteryn-Game
base_branch: main
pr: null
owner: task worker (owner-allocated worktree)
created_at: 2026-10-24
updated_at: 2026-10-24
execution_policy: continuous_progress
owned_paths:
  - tools/world-bundle-compiler/src/main.rs
  - tools/world-bundle-compiler/src/pin_refresh.rs
  - tools/world-bundle-compiler/tests/pin_check.rs
  - tools/content-migration/regenerate_content.py
  - content/world/pins/README.md
  - content/world/pins/oteryn.json
  - docs/architecture/reviews/OTERYN_GAME_ARCH_WORLD_CONTENT_SERVE_PACKETS_2026-10-04.md
  - docs/architecture/reviews/OTERYN_GAME_NPC_PLACE1_NPC_PLACEMENTS_DECISION_2026-10-06.md
  - docs/agents/tasks/PIN-V2-1.md
public_contracts: []
depends_on: []
blocks: []
```

## Summary

Write World pins as `OTERYN_WORLD_PIN/v2`, with the bundle digest and existing
revisions, production flag and entry cell, without `inputs_digest`. `pin-check`
reports the input-tree digest and retains the two-build byte equality,
equivalence, exact pinned bundle digest, identity and entry-cell checks.

`pin-check` rejects v1 with an explicit `pin-refresh` instruction. Refresh accepts
v1 solely to regenerate it as v2, keeping the reviewed entry cell. Identity and
ruleset schemas remain unchanged.

## Reason

STRUCTURE-TARGET-1 measured 50 commits to `content/world/pins/oteryn.json` in
30 days; 33 changed only `inputs_digest`, with no bundle digest change. All seven
simulated pairs of parallel content/compiler changes conflicted on that line.
The input digest adds no bundle protection: `pin-check` already rebuilds twice
and compares the rebuilt bundle digest exactly. Removing the persisted diagnostic
reduces measured merge conflicts without splitting files or weakening that gate.

## Scope

- Compiler writer, checker and CLI regression coverage for v2 and v1 migration.
- Regenerate the committed pin through the repository's `pin-refresh` command.
- Pin documentation and the regeneration script's obsolete explanatory comment.
- Supersession notes for the two dated architecture decisions that described v1.
- Search all input-digest readers, including Python, runtime crates and workflows.

## Excluded scope

No bundle/runtime protocol change, new merge drivers, dependency changes, workflow
or repository-policy changes, publication, or PostgreSQL changes. Historical task
evidence remains historical; dated architecture decisions link to the current format.
`tools/repository/` mentions
are comments about the unchanged diagnostic and input path list, not pin readers.
No workflow reads `inputs_digest`; the migration script delegates to `pin-refresh`.

## Validation

Commands used the absolute queued Cargo wrapper
`/home/mole/oteryn-cache/codex-bin/cargo`, with `--locked --offline` throughout.

- `~/.cargo/bin/cargo fmt -p oteryn-world-bundle-compiler` and
  `~/.cargo/bin/cargo fmt --all --check`: passed.
- `cargo clippy --locked --offline -p oteryn-world-bundle-compiler --all-targets --quiet -- -D warnings`: passed.
- `cargo test --locked --offline -p oteryn-world-bundle-compiler --quiet`:
  49 passed; the existing release-only full-world CLI case is ignored in debug.
- `cargo test --locked --offline --release -p oteryn-world-bundle-compiler --quiet`:
  50 passed, none ignored, including every full-world CLI drift case.
- The exact focused selectors covering this change are
  `--test pin_check -- pin_check_` (schema rejection, v1 refresh migration, accepted
  v2 after compiler-only input changes, bundle/revision/production/identity/entry
  drift) and `--test pin_check -- inputs_digest_` (diagnostic stability and scope).
  Both groups ran as part of the complete crate suites above.
- `cargo run --locked --offline --release -p oteryn-world-bundle-compiler --quiet -- pin-refresh .`:
  passed; only the pin schema and removed field changed. The bundle digest stays
  `0b037a2ad2ff44aef4c8bb9fdd590947ff4a3f5114717028de4b6233a9d551dd`.
- `cargo run --locked --offline --release -p oteryn-world-bundle-compiler --quiet -- pin-check . .pin-v2-validation`:
  passed; reports the input digest and exactly the unchanged pinned bundle digest.
  Temporary bundle output removed after validation.
- `python3 tools/agents/validate_governance.py`: passed.
- `python3 -m unittest discover -s tools/agents/tests`: 129 passed.
- `git diff --check`: passed. Whole diff reviewed against the owner checklist;
  workspace searches found and updated the sole private pin constructor and all
  live field readers. No new public functions, enum variants, struct fields or
  trait methods. No PostgreSQL code changed or PostgreSQL tests run.

## Unfinished

None locally. Local edits only; the current worktree index and history are
unchanged, with no publication.
