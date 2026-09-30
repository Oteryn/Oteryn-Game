# OTV2-20260930-char-build-1b

```yaml
task_id: OTV2-20260930-char-build-1b
title: "CHAR-BUILD-1b Character build writer, reconcile, admission load and req(L) arithmetic"
mode: IMPLEMENT
status: completed
repository: Oteryn/Oteryn-Game
base_branch: main
issue: 162
branch: claude/char-build-1b
pr: "exact PR in the #162 FREEZE_SHA entry"
base_sha: a6a054e
head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_sha: "exact frozen head in the #162 FREEZE_SHA entry"
final_head_frozen_at: null
owner: claude-code-session_01F9kKrvvtHptXoZLv9KNGHk (oteryn-hard-worker)
created_at: 2026-09-30
updated_at: 2026-09-30
execution_policy: continuous_progress
owned_paths:
  - apps/game-server/src/durability/character_build.rs
  - apps/game-server/src/durability/character_progression.rs
  - apps/game-server/src/durability/mod.rs
  - apps/game-server/tests/support/character_build_postgres_cases.rs
  - docs/agents/tasks/archive/OTV2-20260930-char-build-1b.md
public_contracts: []
depends_on: [CHAR-BUILD-1a (#1393)]
blocks: [W2b, DAWNPORT-1, DEATH ML loss]
cross_repository_coordination_id: null
external_repositories: []
```

## Outcome

The writer half of CHAR-BUILD-1 (A13 §4.1, §4.2, §4.6; SKILLS-0 §3.1-§3.3). No migration.

- `durability::character_build`:
  - `commit_character_build`: one CharacterRevision with one build receipt, the build row
    upsert and, for a vocation change that prunes the stance, the stance row. The lock order
    matches the XP writer: recovery fence, admission relations,
    `oteryn:character-build:<occurrence>`, the replay lookup (the binding is compared first), then
    the gameplay fence (session generation, lease, scope, node incarnation, `character_root` FOR
    UPDATE). The progression row, the build row and the stance row are taken FOR UPDATE.
  - The writer refuses before any write: the wrong cause direction (it mirrors the 0030 CHECK), a
    training prune, a `before` that is not the stored build or a pruned stance that is not the
    stored key (`BuildStateMismatch`, a new `CharacterProgressionError` variant), and a pending
    respawn.
  - The binding covers the occurrence, character, expected revision, cause, before and after, the
    stance fields and the policy digest. A revision mismatch fails closed (QUEST-STATE-0 §5.2,
    CHAR-REV-SEQ-1).
  - `reconcile_character_build`: resolves an outcome by occurrence and never reacquires authority.
  - `read_character_build_state`: the admission load. It returns the row, or the seed when there
    is no row.
  - `skill_tries_required` (`req(L)`: f64 `powf`, truncated, `None` above 2^63 - 1),
    `cumulative_progress` (checked u128, saturating at 2^63 - 1) and `relevel`.
- Tests (in the CI-run `character_authority_postgres` target):
  - the writer: seed load, stale revision, stance and before mismatch, prune, replay at a stale
    revision, conflict, reconcile hit and miss, direction and prune rejections, three stale-fence
    facts with an unchanged snapshot, pending respawn, promotion after a death, reload after a
    restart;
  - #1393 MEDIUM: the verifier test adds a death with build fields and training after it, plus
    three planted death build-field corruptions. With the verifier's death arm disabled, the test
    goes red;
  - #1393 LOW: a correct death whose build row is not updated is rejected by the build guard.

## Assumptions (for the control plane)

- The admission load is the durability read. Wiring it into `CasterState` is W2b, which adds
  `Vocation::None` (A13 §5).
- The writer does not recompute a `vocation_choice` re-level. The caller computes `after` with
  `cumulative_progress` and `relevel` from the content formula table (W2b, DAWNPORT-1) and binds
  that table in `policy_digest`. SQL checks only the direction (SKILLS-0 §3.3).
- `req(L)` is the SKILLS-0 skill formula. The magic-level mana formula is W2b's (A13 §4.5). The
  arithmetic takes `req` as a closure for that reason.
- No revision sequencer exists on `main` (#1373 is the decision). The writer follows its rule:
  it includes the revision in the binding and does not retry on a mismatch.
- Size: `character_build.rs` has about 610 non-comment, non-test lines after rustfmt, above the
  ~500 guide. The #1393 scope and its carried findings are one unit. The arithmetic (about 60
  lines plus unit tests) can move to its own PR if the control plane wants that.

## Acceptance criteria

- [ ] Exact frozen head with passing CI.
- [ ] Independent persistence review (control plane).
- [ ] Protected Merge Queue integration.

## Excluded scope

- The runtime caller, the owner queue and training accumulation (W2b); Dawnport (DAWNPORT-1);
  death build fields in the DEATH-1 writer (DEATH ML loss); the sequencer (CHAR-REV-SEQ-1); any
  migration, wire or Platform change.

## Validation

- `cargo fmt --check`; `cargo clippy --locked -p oteryn-game-server --all-targets -- -D warnings`.
- PostgreSQL 17.6 (docker `postgres:17.6-bookworm`): `character_authority_postgres` 908 passed;
  `--lib` 1196 passed; the full crate run is in the PR body.
- Mutation: `verify_character_build_chain` death arm set to `WHERE false` turns
  `build_grants_and_admission_verifier` red (restored).
- `python3 tools/agents/validate_governance.py`; `python3 tools/repository/validate_repository_policy.py`;
  `git diff --check`.

```yaml
last_progress: final authoring commit; archived before freeze
status: completed
branch: claude/char-build-1b
owner_action_required: null
blocker: null
next_action: null   # control plane: persistence review of the frozen head, then Merge Queue
```
