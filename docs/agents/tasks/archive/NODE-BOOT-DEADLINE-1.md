# NODE-BOOT-DEADLINE-1

```yaml
task_id: NODE-BOOT-DEADLINE-1
title: "NODE-BOOT-DEADLINE-1: configurable boot pass deadline and bounded retries for the boot durability steps"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/beautiful-franklin-1tp8kj
pr: 2016
base_sha: 591a89e2e
head_sha: null
final_head_sha: null
owner: claude-code-session-01XYC239givrMonrYoBwwECV
control_plane: claude-code-session-01XYC239givrMonrYoBwwECV
created_at: 2026-10-12
updated_at: 2026-10-17
owned_paths:
  - apps/game-server/src/durability/db.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/src/node/config.rs
  - deploy/synology-game/node.toml.template
  - docs/agents/tasks/active/NODE-BOOT-DEADLINE-1.md
public_contracts: []
external_repositories: []
```

## Outcome

On a loaded Synology host the node boot failed intermittently with E2004 (S2 custody at
`descriptor`) and E2005 (Character authority unavailable): the durability root's fixed 2 s pass
deadline covered whole boot transactions (PostgreSQL logged "terminating connection due to
transaction timeout"), the single root connection was not retried when busy or retired, and the
error class was discarded by `map_err`.

- `node.toml` gains an optional `[boot] pass_deadline_ms` (2000..=60000, default 2000 when the table
  is absent), validated in `node/config.rs`. The template sets 15000.
- `DurabilityRoot::boot_pass_deadline` is a scoped guard: while held, semantic passes issued on that
  root (and its clones) get the boot deadline, clamped to `DB_PASS_DEADLINE..=60 s`; dropping it
  restores 2 s. Only the sequential boot steps take it; gameplay passes keep 2 s.
- `with_retries` (bounded, five runs) now wraps the S2 custody read, claim, descriptor registration,
  the initialize re-read and the Character authority open. It retries only `RootUnavailable`,
  `RootPassDeadlineExceeded` and `CommitOutcomeUnknown`; every wrapped step is a read or an
  idempotent compare-and-set under a row lock, so a retry re-reads the stored state and confirms or
  refuses an earlier commit instead of re-applying it. Every other class fails closed at once.
- A failed boot step writes an always-on warn line `boot_step_failed step="..." error=<class>`
  before `boot_failed`; retries write `boot_step_retry`. Only the `Debug` class is written, never SQL
  text, parameters or credentials.

## Acceptance criteria

- [x] Config parsing and bounds: absent table, accepted 2000/15000/60000, refused 0/1999/60001/i64::MAX
  without echoing the value, refused malformed or unknown keys
  (`boot_pass_deadline_is_optional_bounded_and_defaults_to_the_pass_deadline`).
- [x] Root guard: default, shared by clones, restored on drop, clamped
  (`boot_pass_deadline_is_bounded_shared_by_clones_and_ends_with_its_guard`).
- [x] Retry classification and line format (`boot_steps_retry_only_root_contention_and_deadline_outcomes`,
  `character_authority_open_retries_only_a_retryable_root_cause`,
  `boot_step_lines_name_the_step_and_only_the_error_class`).

## Excluded scope

The durability functions themselves (`native_admission_source.rs`, `character_authority.rs`), the
pool size, gameplay pass deadlines, `MAX_CHARACTER_TRANSACTION`, the node registration loop and
`reconcile_retained` are unchanged. The live host `node.toml` needs the `[boot]` table added by the
operator; without it behavior is unchanged.

## Validation

- `cargo fmt`, `cargo clippy`, `cargo test -p oteryn-game-server`: not run locally (D1011, heavy Rust
  builds go to the PC); left to CI on the frozen head.
- `python3 tools/agents/validate_governance.py`: pass
- `python3 -m unittest discover -s tools/agents/tests`: pass (60 tests)
- `git diff --check`: pass.

### Exact-head CI

Recorded in the PR checks of the frozen head.

## Self-review

- method: implementing agent read the whole diff against owned paths.
- material findings: none open; the code was not compiled locally.

## Independent review

- required: per the bound review policy, requested by the control plane on the frozen head.

## PR and closeout

- changed-file review: within owned paths
- merge commit/result: squash merge of #2016 (merged 2026-10-12)

## Archive closeout

- canonical PR #2016 merged on 2026-10-12 (`git log --grep "(#2016)"`); the record stayed in
  `tasks/active/` and failed the protected-main live-state check, so #2163 archives it
  as a ported `main` governance fix
- merge commit/result: squash merge of #2016
- ownership release: released with the merge of #2016
