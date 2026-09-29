# OTV2-20260929-stance0-character-stance-persistence

```yaml
task_id: OTV2-20260929-stance0-character-stance-persistence
title: STANCE-0 - Character stance slot and stance receipts (migration 0017, no writer)
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
branch: claude/stance0-character-stance
issue: 162
lane_id: GAME-CHAR durability (0009 guard-function chain)
pr: null   # recorded in the FREEZE_SHA packet on #162
base_sha: 92cfc2fe
head_sha: null   # a commit cannot hold its own SHA; exact head is in the FREEZE_SHA packet
final_head_sha: null
final_head_frozen_at: null
owner: "STANCE-0 hard worker (claude-code-session-01V8C1bnpFXfj7TSUDNFgAxt)"
control_plane: session_01MnSvpbKjAZEdzEaFrwiu7D
created_at: 2026-09-29
updated_at: 2026-09-29
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/migrations/0017_character_stance.sql
  - apps/game-server/tests/character_stance_postgres.rs
  - apps/game-server/tests/support/character_stance_postgres_cases.rs
  - apps/game-server/tests/character_authority_postgres.rs   # shared: one #[path] include only
  - docs/agents/tasks/archive/OTV2-20260929-stance0-character-stance-persistence.md
public_contracts: []
depends_on: [STANCE0-CHARACTER-STANCE-PERSISTENCE-V1, "#1264 DEATH-0 (merged 2f300b25)"]
blocks: [STANCE-1]
external_repositories: []
jira: null   # sync pending (coordinator batch)
```

## Outcome

Migration `0017_character_stance.sql` implements
`reviews/OTERYN_GAME_STANCE0_CHARACTER_STANCE_PERSISTENCE_DECISION_2026-09-29.md` §4.1-§4.3 and §4.7
(D145). There is no writer: `commit_character_stance` and `reconcile_character_stance` are STANCE-1.

- `game_character_stance` (§4.1): `character_id` primary key, `stance_key` (NULL or a 1..128 B
  revision-class key), `committed_character_revision` (≥ 2), `last_stance_occurrence_id` (UUIDv7).
  No row means an empty slot; only the `standard` slot is stored; the 0009 initializer is unchanged.
- `game_character_stance_receipts` (§4.2): immutable, keyed by the toggle occurrence (UUIDv7);
  revisions (original + 1); level and experience before = after (CHECK); `stance_before` and
  `stance_after` (NULL or bounded key, CHECK distinct, so a no-op is not a receipt);
  `command_binding` (1..1024 B), `policy_digest` (32 B), the eight revision fields, `committed_at`.
- 0009/0016 XP and death tables, their CHECKs and every existing migration file are unchanged.

## Design (§4.3)

- **State guard** (replaced): the XP and death directions of 0016 plus an explicit stance branch
  (equal experience and level). The death branch already admits it; the deferred guard decides.
- **Consistency guard** (replaced), one function for all three kinds, run deferred on root, state,
  XP receipt, death receipt, stance receipt and stance row writes:
  - revision one: no receipt of any kind and no stance row;
  - XP + death + stance receipts = revision − 1 with exactly one receipt per revision; the current
    receipt matches the state; `before` = predecessor `after` across kinds; none ahead of the root;
  - stance chain: `stance_before` = previous transition's `stance_after`, NULL for the first. The
    transitions come from one `stance_chain` CTE so a later combined vocation-change receipt (§4.6)
    can join it; pruning is not implemented and not precluded;
  - the stance row equals the latest transition (key, revision, occurrence) and is absent without
    one; so a row-only write, or an XP or death commit that changes the row, fails at commit;
  - DEATH-0 transition binding kept for all three kinds: a state UPDATE needs the successor receipt
    whose `before` is the replaced row.
- **Stance row triggers:** BEFORE UPDATE OR DELETE rejects delete and `character_id` reassignment;
  a deferred constraint trigger runs the consistency guard on INSERT, UPDATE and DELETE.
- **Immutability:** stance receipts reject UPDATE/DELETE (`game_character_immutable`); both tables
  reject TRUNCATE (`game_character_reject_truncate`). Replaced and new functions get the fixed
  `search_path` again.
- **Grants:** runtime SELECT/INSERT on receipts, SELECT/INSERT/UPDATE on the row (no DELETE);
  control SELECT on both; PUBLIC revoked on tables and the new function.

## Findings for STANCE-1

- `CommandRef` (`foundation/protocol.rs`) is a (GameSessionId, CommandId) pair, not a UUIDv7. Per
  §4.2 the owner therefore issues one UUIDv7 `stance_occurrence_id` per toggle; the migration
  requires UUIDv7.
- Receipt growth and toggle latency stay UNKNOWN (§4.7); STANCE-1 measures them.

## Validation (local, isolated workspace)

- `cargo fmt --all --check`: pass. `cargo clippy --locked --workspace --all-targets -- -D warnings`:
  pass. `cargo test -p oteryn-game-server`: pass.
- PostgreSQL 17.6 (`postgres:17.6-bookworm@sha256:f3bd19c6…` via mirror.gcr.io): all 14 `*_postgres`
  targets pass, including `character_stance_postgres` (5 cases) and the same cases through
  `character_authority_postgres`, `character_progression_postgres` and
  `character_death_receipts_postgres`.
  - Positive mixed chain XP → death → stance on → XP → stance switch → death (row kept) → stance
    off → XP → stance on (revisions 2-10).
  - Negative cases assert SQLSTATE 23514 (23505 for a reused occurrence) and an unchanged snapshot.
- RED: each of 8 guard mutations (stance chain/row block, row BEFORE guard, row deferred trigger,
  receipt immutability, truncate triggers, transition binding, revision-one row check, receipt
  deferred trigger) makes at least one case fail.
- `validate_governance.py`, `validate_repository_policy.py`, `git diff --check`: pass.

## Closeout

- Review: independent exact-head review requested from the control plane after freeze.
- Merge commit/result: squash merge of the STANCE-0 PR (resolve with `git log --grep`).
