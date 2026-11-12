# OTV2-20261112-pr3115-wu26-qualification

```yaml
task_id: OTV2-20261112-pr3115-wu26-qualification
title: Qualify the WU26 server package through batched review and fixture repairs
mode: REPAIR
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: codex/pr3115-merge-20261112
pr: 3115
base_sha: f05944d8852342d30c933cc3a0f4338676f92571
head_sha: null
owner: Codex integration lead
owned_paths:
  - crates/gameplay/quests/src/quest/task_board_full.rs
  - crates/gameplay/quests/src/quest/task_board_full_tests.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/atelier/fixture.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/atelier/paid.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/atelier/reshape.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/atelier/wire_paid.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/atelier/wire_paid/reshape.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/prey.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/temple_scroll.rs
  - docs/agents/tasks/archive/OTV2-20261112-pr3115-wu26-qualification.md
```

## Scope and dispositions

The owner assigned #3115 before client #3098, with independent Codex review and
repairs in coherent P0/P1/P2 batches until no findings remain. This task-specific
instruction overrides the previous review-count and P2-deferral limits for these
PRs. Normal forward merges brought accepted main into the server package; no
workflow or protection changes were authored by this repair.

- Fixed the reviewed P2: a capped bounty claim no longer refuses a historical
  maximum BIGINT token balance when its token credit is zero. The boundary
  regression failed with ArithmeticOverflow before the fix and passes after it.
- Preserved the current capped-credit and Task Board token guard around PREY's
  deliberate reapplication of migration 0547. Reapplication had overwritten that
  guard, so the eleventh daily claim failed at the ten-token cap.
- Made the Temple test's Inbox container a real stamped MAIL parcel. The native
  MAIL writer retains its complete source tree and delivers it with guards on.
  The invalid ordinary Inbox backpack had caused a deferred custody failure that
  rolled back every scroll retirement; all ten offline assertions remain.
- Made legacy Atelier fragment fixtures and references canonical. Its stash
  preparation loads and publishes a real native registry and invokes the actual
  stash writer with matching NativeItemWeights; it does not weaken the digest or
  custody checks or replace the legacy Atelier owner.
- Added an actual selected command-6214 test through serve_admitted, including a
  fresh non-Premium refusal, a paid redraw, modifier preservation, one fragment
  and fee, and retained-result replay without a second payment. The existing
  direct-writer carried/stash and negative scenarios remain.

## Validation

- `cargo fmt --all --check`: pass.
- `cargo metadata --locked --no-deps --format-version 1`: pass.
- `cargo test --locked -p oteryn-gameplay-quests --lib`: pass, 424 tests; the
  focused token-boundary test has separate red and green evidence.
- `cargo clippy --locked -p oteryn-gameplay-quests --all-targets -- -D warnings`:
  pass. Dispatch/game-server builds and PostgreSQL Rust suites are deferred to
  GitHub runners under the owner's explicit local-test limit.
- `python tools/agents/validate_governance.py`: pass.
- `python -m unittest discover -s tools/agents/tests`: pass, 129 tests.
- `git diff --check`: pass; duplicate migration-number check: no duplicates.
- Isolated PostgreSQL 18.6 probes retained the real guards: capped-credit
  reapplication reproduced the failure and current-guard restoration committed
  eleven grants at balance ten. All-migration MAIL/custody probe reproduced the
  invalid Inbox failure, then real MAIL posting and all ten retirements committed.
  Every probe database was removed. This is SQL evidence, not a claim that the
  complete Rust scenarios or Node qualification have run locally.

## Review and integration

The first current-task whole-server independent Codex review found the bounty
P2 and recorded the command-6214 evidence gap and failing CI cases. All are
addressed together here. Independent final-delta review and exact-head GitHub
native, PostgreSQL and Node qualification follow the final authoring freeze.
No clean-review or green-CI claim is made in advance. The local Codex CLI failed
authentication before model execution; reviews use the native independent Codex
reviewer, which authored none of these changes.

Authoring is complete; merge and post-merge qualification remain pending. The
freeze comment binds the exact head. Merge result: squash merge of #3115, only
after qualification. #3098 takes accepted server main afterwards; no force push,
red-Node waiver or protection bypass is authorized by this record.
