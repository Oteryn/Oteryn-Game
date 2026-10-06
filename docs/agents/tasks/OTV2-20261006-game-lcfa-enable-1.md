# OTV2-20261006-game-lcfa-enable-1

```yaml
task_id: OTV2-20261006-game-lcfa-enable-1
title: GAME-LCFA-ENABLE-1 enable the ListCharactersForAccount projection publisher
mode: IMPLEMENT
status: active
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/game-lcfa-enable-1-20261006
base_sha: ee71e79eccd1d498d6c39ea25ac01ee74ccd118c
owner: control plane
owned_paths:
  - apps/game-server/src/node/config.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/src/native_admission_source/account_characters.rs
  - apps/game-server/src/native_admission_source/account_characters_tests.rs
  - apps/game-server/src/bin/oteryn-game-ops.rs
  - tools/qualification/login_local/
  - docs/agents/tasks/OTV2-20261006-game-lcfa-enable-1.md
  - docs/agents/tasks/archive/OTV2-20261006-game-lcfa-enable-1.md
public_contracts:
  - docs/contracts/OTERYN_GAME_LIST_CHARACTERS_FOR_ACCOUNT_PROJECTION_V1.md
external_repositories: []
```

## Scope

Packet §3 of `docs/architecture/reviews/OTERYN_GAME_ARCH_LCFA_PROJECTION_CONTRACT_2026-10-06.md`.
No migration, wire change or Platform change.

## Outcome

- `[platform.account_characters]` (absent by default): a distinct projection mTLS identity, the
  reused Platform endpoint and roots, `source_authority`, and the epoch fence file F outside the
  Character fence directory. The node starts the publisher only when its database role can read
  Character ownership, projection, epoch and outbox (and delete from the outbox); otherwise it logs
  `state=refused reason=privileges`.
- Epoch fence (contract §5): the publisher refuses every publication with a missing, unreadable,
  malformed or group/world-writable F, or below F; a higher epoch persists F before the send.
  F is written by temp file, fsync, rename and directory fsync, keeping the existing owner.
- `oteryn-game-ops projection resync --raise-epoch true|false` uses the ops `[projection]`
  operator credential, locks the epoch row, refuses with a rollback unless the predicted epoch is
  above F, verifies the function's result, commits, persists F and prints only the epoch. The help
  text states the restore procedure.
- RUNBOOK-1 stack: projection certificate, Platform feed on and mode 33a off, F created with 0,
  resync raise before the node starts, and a wait for an accepted snapshot and watermark.

## Notes

- Migration 0024 computes the raised epoch in Unix seconds (`now_ms / 1000`), while the contract
  text says milliseconds; the command reproduces the function exactly.
- 0024 grants EXECUTE on the resync function to no role, so the stack uses the local admin login
  for the ops `[projection]` credential.
- The Platform feed environment names follow the existing `GAME_AUTH_NATIVE_*` pattern and need a
  Platform pin carrying PLATFORM-LCFA-1.

## Validation

`cargo fmt --check`; `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`;
`cargo test --locked -p oteryn-game-server --lib account_characters` (20), `--lib node::config`
(7), `--bin oteryn-game-ops`; `bash -n` and `shellcheck -x` on `run.sh`; repository policy and
governance validators; `git diff --check`.
