# OTV2-20261006-game-lcfa-enable-1

```yaml
task_id: OTV2-20261006-game-lcfa-enable-1
title: GAME-LCFA-ENABLE-1 enable the ListCharactersForAccount projection publisher
mode: IMPLEMENT
status: validating
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/game-lcfa-enable-1-20261006
base_sha: ee71e79eccd1d498d6c39ea25ac01ee74ccd118c
pr: 1898
owner: control plane
owned_paths:
  - apps/game-server/src/node/config.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/src/native_admission_source/account_characters.rs
  - apps/game-server/src/native_admission_source/account_characters_tests.rs
  - apps/game-server/src/bin/oteryn-game-ops.rs
  - tools/qualification/login_local/
  - deploy/synology-game/node.toml.template
  - deploy/synology-game/ops.toml.template
  - deploy/synology-game/README.md
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
- Preproduction (CP, from Codex P1 4200331270 on #1893): this packet owns the
  `[platform.account_characters]` section of `deploy/synology-game/node.toml.template`, the ops
  `[projection]` section, F initialization and the initial resync (first start step 6a), which
  must precede disabling mode 33a. The #1893 topology doc only references them.

## Notes

- The raise predicts `greatest(current + 1, transaction Unix ms)`, exactly as migration 0028
  redefines `game_character_account_projection_resync` (0024 used seconds); this matches contract §5.
- D607: 0024 grants EXECUTE on the resync function to no role, so the raise uses an operator-only
  credential from the optional ops `[projection]` section (absent by default; the command fails
  closed without it). The stack uses the local admin login for it.
- Codex P1 4200340107: a busy or re-establishing durability holder at startup is no answer, not a
  privilege refusal; the publisher retries the privilege check with bounded backoff and stops only
  on a definite refusal.
- Codex P1 4204083252 (D742): a fence rename not followed by a successful directory sync leaves F
  unusable (every read refused, so `admit` refuses even at the same epoch) until a directory sync
  succeeds; every fence handle also syncs the directory before its first read.
- Codex P1 4204399506: a process holds one production durability root, so `projection resync` is
  dispatched before the operator root is built and connects only the projection login.
- Follow-up GAME-LCFA-RESYNC-DEADLINE-1 (Codex P2 4200478837): `ops projection resync` runs inside
  the ordinary 2 s root pass (`DB_PASS_DEADLINE`); a bounded maintenance deadline needs a
  durability-root change outside this packet's paths.
- Merge blocker for the e2e stack, not for this PR: the RUNBOOK-1 Platform pin stays at `3896bcdf`
  until Platform#1465 (PLATFORM-LCFA-1) merges; the stack's feed environment names match its head.

## Validation

`cargo fmt --check`; `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`;
`cargo test --locked -p oteryn-game-server --lib account_characters` (21), `--lib node::config`
(7), `--bin oteryn-game-ops`; `bash -n` and `shellcheck -x` on `run.sh`; `git diff --check`; all pass.

- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass (59 tests)
- `python tools/repository/validate_repository_policy.py`: pass
