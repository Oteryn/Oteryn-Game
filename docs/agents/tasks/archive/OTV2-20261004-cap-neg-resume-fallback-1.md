# OTV2-20261004-cap-neg-resume-fallback-1

```yaml
task_id: OTV2-20261004-cap-neg-resume-fallback-1
title: "CAP-NEG-RESUME-FALLBACK-1: terminal release of a resume refused only for a capability mismatch"
mode: IMPLEMENTATION
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/cap-neg-resume-fallback-1-20261004
pr: 1708
base_sha: 2a5ce70
head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #1622 FREEZE_SHA entry"
final_head_frozen_at: null
owner: oteryn-hard-worker (CP #1622)
created_at: 2026-10-04
updated_at: 2026-10-04
execution_policy: continuous_progress
migration_lease: none
owned_paths:
  - apps/game-server/src/durability/fresh_admission.rs
  - apps/game-server/src/gameplay_transport/connection.rs
  - apps/game-server/src/gameplay_transport/resume.rs
  - apps/game-server/src/gameplay_transport/mod.rs
  - apps/game-server/tests/durability_postgres.rs
  - docs/agents/tasks/archive/OTV2-20261004-cap-neg-resume-fallback-1.md
public_contracts: []
depends_on: [CAP-NEG-1]
blocks: []
cross_repository_coordination_id: null
external_repositories: []
```

Packet: `docs/architecture/reviews/OTERYN_GAME_ARCH_BATCH_ITEM_EQUIP_PACKETS_2026-10-03.md` §2.0c
(ruling §1.13, D449). Closes the CAP-NEG-1 known limitation (#1705 Codex P1 4175460459).

## Outcome

- `FreshAdmissionStore::release_capability_mismatch(session, account, epoch)`: the FND-04B
  terminal release with no successor, through `release_current_claims` (the same fenced claim
  transition as grace expiry and `release_abandoned_session`). It releases only a RECONNECTABLE
  session at the exact verified loss epoch, before its original grace deadline. A terminal
  session returns `Terminal` (replay, no second commit). Anything else is `NotApplicable`.
- `gameplay_transport::mod`: `release_abandoned` and the new `release_capability_mismatch` share
  one `release_terminal` path: fence, monk save, store commit, then retire the exact actor only
  after the terminal fact. A refused or unavailable commit lifts the fence and keeps the session.
  The mismatch release also returns the premium seat and forgets the lost entry.
- `resume.rs`: the capability check runs after every other resume check (credential, account,
  character, World, RECONNECTABLE epoch within grace, runtime facts) and after the complete
  authorization of the candidate, attempt budget and current claims (#1708 Codex P1). Only a
  resume refused for the capability mismatch alone releases; every other refusal, an exhausted
  attempt budget included, releases nothing. The resume is still refused, so the client falls
  back to fresh admission, which now succeeds at once with a new `GameSessionId`.
- An unproven mismatch release is never a final refusal (#1708 Codex P2): `release_terminal`
  reconciles it from the durable row (TERMINAL retires the actor; a session still holding the
  lease lifts this exact fence), and the resume returns `Unavailable` with the lost entry kept, so
  a retry repeats the release. Grace expiry remains the backstop.
- The mismatch release fences with its lost epoch, as grace expiry does (#1708 Codex P1
  4175882774). Its retry and grace expiry join that fence, a compatible resume that wins the race
  lifts it in `restore_control`, and once control is restored no mismatch attempt can fence the
  slot again, so no fence outlives a resumed session even when the durable reads fail.
- No migration (lease 0067 unused), no wire, registry or contract change, no capability offered.
  CompleteReconnect `EarlyTerminalReplacement` stays refused by the PostgreSQL adapter.

## Tests (PostgreSQL)

- `owning_fresh_loss…` scenario 6: ACTIVE, another epoch and another account release nothing.
  An injected receipt failure rolls the release back, and a reloaded store sees RECONNECTABLE
  with claims held, so it can retry. The release then commits: TERMINAL under the same id with
  presence and holder cleared and the lease kept. A replay before and after a restart returns
  `Terminal` without another receipt. A fresh re-admission inside the original reconnect window
  commits under a new session id, with the lease advanced and the session-use ledger revision
  increased.
- Scenario 5: no mismatch release after the original grace deadline.
- `complete_reconnect_resumes…`: an exhausted attempt budget refuses the complete authorization
  (`AttemptCapacityExceeded`) that the mismatch release waits for, and the session stays
  RECONNECTABLE. An operation relabelled `EarlyTerminalReplacement` is never
  encoded. The same-session resume with a matching set is unchanged.
- Unit: `capability_mismatch_fence_is_lifted_by_a_winning_resume` (Channel runtime): the
  mismatch fence is joined by its retry and grace expiry, holds off other transitions, is lifted
  by the winning resume's `restore_control`, and cannot be set again afterwards.
- Unit: `capability_mismatch_refusal` is final (`Rejected`) only for a proven release; `Unknown`
  is `Unavailable`.

## Validation

- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --quiet`: pass
- `cargo test --locked -p oteryn-game-server --test durability_postgres --quiet` (local
  PostgreSQL 17.11): the changed and all other tests pass, except 26 that assert the
  canonical server version is exactly 17.6 (CI's pinned image) and fail on that check alone
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: pass
- `git diff --check`: pass
