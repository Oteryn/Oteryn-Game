# OTV2-20261015-public-projections-producer

```yaml
task_id: OTV2-20261015-public-projections-producer
title: "GAME-PUBLIC-PROJ-PRODUCER-1: Game producer of highscores, character_profile and world_online public projections (disabled by default)"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: feat/public-projections-producer
issue: 1622
pr: 2224
base_sha: b02196916
head_sha: null
final_head_sha: null
final_head_frozen_at: null
owner: wave builder GAME-PUBLIC-PROJ-PRODUCER-1 (Claude Code)
created_at: 2026-10-15
updated_at: 2026-10-17
execution_policy: continuous_progress
migration_lease: "0091"
owned_paths:
  - apps/game-server/migrations/0091_public_projections.sql
  - apps/game-server/src/native_admission_source/public_projections.rs
  - apps/game-server/src/native_admission_source/public_projections_tests.rs
  - apps/game-server/src/native_admission_source/descriptor.rs
  - apps/game-server/src/native_admission_source/http1_mtls.rs
  - apps/game-server/src/native_admission_source/mod.rs
  - apps/game-server/src/durability/public_projections.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/src/node/config.rs
  - apps/game-server/src/node/serve.rs
  - apps/game-server/src/bin/oteryn-game-ops.rs
  - apps/game-server/tests/native_admission_source_transport.rs
  - apps/game-server/tests/character_authority_postgres.rs
  - apps/game-server/tests/support/public_projections_postgres_cases.rs
  - docs/contracts/OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md
  - docs/contracts/RESOURCE_LIMITS_REGISTRY.json
  - docs/contracts/fixtures/public-projections/character_profile_v1.json
  - docs/contracts/fixtures/public-projections/character_profile_guild_hidden_v1.json
  - docs/agents/tasks/active/OTV2-20261015-public-projections-producer.md
  - docs/agents/tasks/archive/OTV2-20261015-public-projections-producer.md
public_contracts: [OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md, RESOURCE_LIMITS_REGISTRY.json]
depends_on: []
blocks: [RANK-1 HS-1 (attaches the highscores revision trigger)]
cross_repository_coordination_id: Oteryn/Oteryn-Platform#1645
external_repositories: [Oteryn/Oteryn-Platform]
```

## Outcome

The Game producer of `OTERYN_GAME_PUBLIC_PROJECTIONS_V1.md` (revision 2, accepted for
`highscores`, `character_profile`, `world_online` and the watermark; owner Q1 to Q4, 2026-10-15):
a transactional outbox advanced by definer triggers, an mTLS publisher with its own client
identity and epoch fence `F_pub`, per-family watermarks, the 10 s `world_online` scan and the
operator command `oteryn-game-ops public-projection resync`. The publisher runs only with
`[platform.public_projections] enabled = true`.

## Architecture and source of truth

- PROVEN: contract §3 (identity, paths, 3 s exchange), §4.0 to §4.2 and §4.6 (exact wire, shared
  Platform fixtures), §5 (ordering, outbox, epoch, resync), §5.1 (watermark, scan deadline), §6
  (backoff 1 to 60 s, definite hold 300 s with alarm), §7.3 name handoff, §7.4 logging, §8 bounds.
- DERIVED: vocation ids follow Game's `ReferenceBaseVocation` (control-plane ruling, matches the
  Platform consumer); `guild_visible` defaults to `true` (public, Tibia reference) because no
  Game owner records the setting yet; only a Game-side setting may lower it. Authority: CP ruling
  https://github.com/Oteryn/Oteryn-Game/pull/2224#issuecomment-6206973613 (Platform owns guild privacy; its preference
  is an upper bound over Game's `guild_visible`).
- DERIVED (architect review of #2224): the publisher shares the single durability root, so it
  paces its passes (`PUBPROJ-STORE-PASS-GAP`), yields while gameplay waits and batches profiles
  (`PUBPROJ-PROFILE-BATCH`); the name handoff hold is bounded (`PUBPROJ-NAME-HANDOFF-HOLD`);
  "online" is a live session with a transport and no control loss (§4.6).
- DERIVED: highscores reads the RANK-1 draft (#2168) table interface; HS-1 attaches
  `game_public_highscores_snapshot_trigger()` and grants SELECT (contract §11). Until then the
  highscores family sends nothing and stays stale.

## High-risk authority/recovery qualification

```yaml
applicable: true
model: AuthorityInvariant_x_ConsumerBoundary_x_MutationOperator
authority_invariants:
  - a revision advances only in the transaction of a committed projected change
  - the public epoch only rises, above the persisted fence F_pub, and only by the operator resync
  - nothing below F_pub is ever sent; a store epoch above F_pub persists F_pub first
  - complete_through never passes an undelivered change or a scan that missed its deadline
consumer_boundaries: [Platform public read model (mTLS), HS-1 snapshot job, operator resync]
mutation_operators:
  applicable: [missing facts, stale epoch, unknown vocation or state, over-bound snapshot, replay, concurrency, late scan]
  considered_not_applicable: [session generation fencing (the publisher writes no Character state)]
one_invariant_per_negative_case: true
independent_current_fact_sources: [Character root, progression, build state, committed sessions]
```

## Acceptance criteria

- [x] Shared `character_profile` wire fixture files (`docs/contracts/fixtures/public-projections/`),
      encoded byte for byte.
- [x] Exact Platform wire fixtures, empty lists, worst-case bounds and refusals
      (`public_projections_tests.rs`).
- [x] Publisher: fence, definite hold and alarm, backoff, highscores filter resume, scan revisions,
      deadline and online watermark (`public_projections_tests.rs`).
- [x] Disabled unless configured; identity, authority and fence kept separate (`config.rs` tests).
- [x] Operator resync command, connecting only its own login (`oteryn-game-ops` tests).
- [x] PostgreSQL 17.6: outboxes atomic with the Character write, guards, resync, least privilege
      and the HS-1 trigger interface; store reads over a real bootstrap
      (`tests/support/public_projections_postgres_cases.rs`).
- [x] Limits registered as `PUBPROJ-*`.
- [x] Architect review of #2224 (B1, B2, S1 to S5, N1 to N4): default-public `guild_visible`,
      migration 0091, pacing, yield and batching, bounded name handoff, unclassifiable entries
      published as `[]`, the online session predicate, filter and category ids pinned to the
      Platform consumer, one-statement online read, ASCII names, invoker resync, no resend of an
      unchanged `world_online` watermark.

- [x] Codex P2 and control-plane hardening of #2224: per-World `world_online` backoff 1 to 60 s
      with retries between scans (§6); the `guild_visible` default cites the CP ruling (§4.2,
      §12); projection names also refuse `/`, `.` and `..` (scoped to the projection encoders and
      store reads; the account-characters wire keeps its rule); the online read uses the exact
      `spell_character_lookup` predicate, eligible admission guard included (§4.6).

## Excluded scope

Deployment, PKI, enabling the publisher in any stack, the Platform consumer, HS-1 (RANK-1 #2168),
and the `character_deaths`, `guild` and `house` families.

## Validation

Codex P2 and hardening fix head (2026-10-16):

- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --lib --bins`: 3212 passed, 0 failed (29 ignored);
  `oteryn-game-ops` 7 passed
- PostgreSQL 17.6: `character_authority_postgres` 1526 passed (online predicate with eligible,
  foreign-World and foreign-holder admission guards)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: 124 passed
- `git diff --check`: pass

Architect-review fix head (2026-10-16):

- `cargo fmt --all --check`: pass
- `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`: pass
- `cargo test --locked -p oteryn-game-server --lib --bins`: 3207 passed, 0 failed (29 ignored);
  `oteryn-game-ops` 7 passed
- PostgreSQL 17.6: `character_authority_postgres` 1521 passed (public projections cases,
  bootstrap reads, name handoff and cycle, online session states, unclassifiable online
  entries, invoker resync)
- `python tools/agents/validate_governance.py`: pass
- `python -m unittest discover -s tools/agents/tests`: 124 passed
- `git diff --check`: pass

## Self-review

- No name, id or body in a log line; every failure leaves the outbox row queued.
- Definite refusals never drop or rewrite a row; the watermark stalls instead.

## Independent review

- required: YES (protocol, privacy, persistence); architect review run by the control plane.

## PR and closeout

- Draft PR on `feat/public-projections-producer`; the slot commit archives this record.

## Context checkpoint

```yaml
last_progress: Codex P2 (online backoff) and control-plane hardening of #2224 fixed in one push
status: completed
branch: feat/public-projections-producer
pr: 2224
owner_action_required: null
blocker: null
next_action: exact-head CI, Codex and architect review, then the slot commit at queue turn
```

## Archive closeout

- canonical PR #2224 merged on 2026-10-16 (`git log --grep "(#2224)"`); the record stayed in
  `tasks/active/` and failed the protected-main live-state check, so #2163 archives it
  as a ported `main` governance fix
- merge commit/result: squash merge of #2224
- ownership release: released with the merge of #2224
