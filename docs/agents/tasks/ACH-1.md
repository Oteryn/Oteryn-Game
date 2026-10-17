# ACH-1 — achievements, badges, titles (skeleton task record)

```yaml
task_id: ACH-1
title: "ACH-1: achievements, badges, titles"
mode: IMPLEMENT
status: NOT_FULLY_QUALIFIED
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 4
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 6 row ACH-1 (rows 125, 129); §7 'ACH-1'"
briefs:
  - docs/architecture/reviews/OTERYN_GAME_ACHIEVEMENT0_ACHIEVEMENT_GRANT_RUNTIME_DECISION_2026-10-03.md
base_branch: main
base_sha: 5742f30f37f5ae4e21b9c03ad25211ec5349eed6
branch: skeleton/ach-1
needs_migration: true
needs_registry: false
needs_content_regeneration: true
merge_authority: CP_ONLY (builder never marks ready)
```

## Goal

Achievement grant runtime: quest-transition grant, encounter-outcome grant with durable receipt, batch grant API and counter thresholds, coverage report. ACH-NOTIFY-1 is MERGED.

Children (allocate from the brief's child graph; the plan row is only the family entry gate):

ACH-QUEST-1, ACH-ENC-1 (hard), ACH-COUNTER-1, ACH-COVERAGE-1 (content tooling). ACH-NOTIFY-1 MERGED.

## Dependencies (state on main@5742f30f3, 2026-10-15)

| Dependency | State |
|---|---|
| LOOP-PROGRESS-1 (family entry gate) | BLOCKING (Wave 1 sequential) |
| QUEST-STATE-1 | MERGED |
| ENC-OUTCOME-1 (ACH-ENC-1) | BLOCKING |
| XP receipt; SKILLS-0 build receipts (ACH-COUNTER-1) | verify |
| FND-02 / ACH-NOTIFY-1 | MERGED |
| acceptance of ACHIEVEMENT0 | ACCEPTED by ACCEPT-BATCH-2026-10-15 §1 (#2209, integrated main `e2da9805d296a27b06b32335bf67394509d4910b`, control-plane verified) |

Rows marked BLOCKING must merge (or be accepted) before the builder's slot commit; code in owned_paths may be drafted earlier but must not assume unmerged schemas.

ACH-COVERAGE-1 has **no dependencies** in the accepted brief's child graph. The
family/runtime dependencies above do not block this separately allocated tooling
slice and remain applicable to their respective runtime children.

## owned_paths (builder edits freely)

- `apps/game-server/src/achievement/grant.rs`
- `apps/game-server/src/achievement/counters.rs`
- `apps/game-server/src/achievement/grant_tests.rs`
- `apps/game-server/src/durability/achievement_outcome_receipt.rs`
- `apps/game-server/tests/achievement_grant_postgres.rs`
- `tools/content-schema/achievement-authoring/validate_achievements.py and test_validate_achievements.py (ACH-COVERAGE-1 coverage report)`
- `docs/agents/tasks/ACH-1.md` (this record: status and notes only)

To compile on the branch the builder may add only the module registration lines in shared `mod.rs`/`lib.rs` files; they are re-applied in the slot commit after merging main, together with every other slot-only change.

## slot_only_paths (ONLY in the final commit at the merge-queue turn, after merging main)

- tools/content-migration/regenerate_content.py outputs
- content/project.json
- content/manifest.json, content/world/manifest.json
- content/content.lock.json, content/world/content.lock.json
- content/interactions/index.json
- content/world/pins/*
- Rust pins (pinned registries / fixtures regenerated from content)
- docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json, GAME_EVENT_FOUNDATION_REGISTRY.json, RESOURCE_LIMITS_REGISTRY.json, OTERYN_GAME_ERROR_CODE_REGISTRY.json
- apps/game-server/migrations/NNNN_*.sql number (next free on main at queue turn; main ends at 0087)
- apps/game-server/src/gameplay_transport/connection.rs, mod.rs, capabilities.rs wiring
- crates/protocol-oteryn/src/lib.rs `pub mod` + registry constants
- apps/game-server/src/durability/mod.rs, domain/mod.rs, lib.rs module registration

## Wiring points (verified on origin/main 5742f30f3)

- apps/game-server/src/durability/account_achievement.rs (single writer of the account fact; call its grant entry point only)
- apps/game-server/src/achievement_catalogue.rs
- quest transition transaction (quest/)
- crates/protocol-oteryn/src/account_achievements.rs and achievement_notices.rs (existing; no new capability)

## Inseparable bundles (never split across PRs)

- migration (outcome receipt table) + writer + durability
- each granter calls the one grant entry point from its own transaction

## Acceptance criteria

- [ ] Grant idempotent per (account, achievement).
- [ ] Encounter outcome receipt prevents double grant after restart.
- [ ] Notice delta emitted after `Granted` commit only.
- [ ] Every concurrent transition serialized (named lock or deterministic order) and losing writer typed.
- [ ] Durable state sufficient to resume after restart / session-generation change.

## Validation commands

```bash
cargo fmt --all -- --check
cargo clippy -p oteryn-game-server -p oteryn-protocol-oteryn --all-targets -- -D warnings
cargo test -p oteryn-game-server ach
cargo test -p oteryn-game-server --test <task>_postgres   # needs DATABASE_URL
python3 tools/content-migration/regenerate_content.py   # slot commit only; runs --check tools
```

## Reference evidence pointers

- docs/architecture/reviews/OTERYN_GAME_ACHIEVEMENT0_ACHIEVEMENT_GRANT_RUNTIME_DECISION_2026-10-03.md §3-§5
- apps/game-server/src/durability/account_achievement.rs
- docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md §4, §5 (Wave 4), §7
- docs/agents/cp/CP_WAVES.md (branch claude/beautiful-franklin-1tp8kj), 'Model pracy równoległej'

## Builder instructions

- Work only in owned_paths until CP gives the queue-turn GO; then merge main, run regeneration/registry/migration numbering/wiring in ONE final slot commit.
- Push rarely and batched: one push per fix round (heavy builds per push). No status-only commits.
- Never mark the PR ready and never post `@codex review`; CP marks ready (Codex auto-reviews each push).
- Max 2 repair rounds on the PR; then CP closes and splits.
- A required edit outside owned_paths is a `cross_domain_finding` (REPORT_ONLY), not an edit.

## ACH-COVERAGE-1 local preparation (2026-10-15, draft #2154)

- One writer, branch `skeleton/ach-1`, starting HEAD
  `1c8d0dbdd9ac7e2ecd6c7e03eb189b4b74996d46`; only the two allocated Python
  files and this record changed. No runtime, migration, registration, generated
  content, catalogue or pin changes; publication remains with the control plane.
- Effective authority: ACHIEVEMENT0 §4, accepted through #2209. Its report
  "lists every catalogue key with no granter in any admitted content (claim,
  transition, encounter binding or threshold)"; "Such a key is
  `UNEARNABLE_PENDING`"; "No admin, script or migration grant substitutes for
  content." §2 names quest/quest-step, boss/encounter, counter, and one-time-action
  earning families. The accepted source was read on main; this older draft's
  historical CANDIDATE header does not override the integrated acceptance.
- The CLI now always adds `coverage`, scoped explicitly to the supplied supported
  inputs. Existing record counts, validation errors, slug/explicit-owner bindings,
  chest-ref counts and failure exit semantics remain. Missing content bindings
  are reportable pending records and do not fail an otherwise valid catalogue.
- `bound_keys` means a matching RewardClaim placement reference only, never runtime
  earnability or world admission. `retired_keys` remains explicit even when a key
  binds: retired grants remain no-ops under the existing contract. Every key with
  no supplied binding, including a retired key, is listed as `UNEARNABLE_PENDING`.
  An invalid/duplicate-key catalogue reports `INVALID_CATALOGUE` instead of
  positive coverage. Sorted keys and deduplicated bindings make output stable.
- **Scope limit / cross_domain_findings:** the current CLI admits catalogue and
  RewardClaim documents only. The existing quest transition schema has no
  achievement field; the Tirecz manifest has name-only `outcome_evidence`, not a
  keyed admitted achievement consumer; this validator has no admitted threshold
  input. Transition, encounter-binding and counter-threshold granters therefore
  remain explicitly unsupported. No new schemas, counter owners, thresholds,
  event maps or name-derived bindings were invented.
- The catalogue schema carries no earning-family/condition field. Descriptions,
  names, Premium/secret flags and external provenance cannot supply that authority.
  Pending reasons are explicitly `NO_ADMITTED_GRANTER;
  EARNING_FAMILY_UNCLASSIFIED`, with all four §2 families listed in the report.
  Exact per-key family reasons and whole-world coverage need the owning admitted
  content/family facts; they are not claimed complete by this supported-input slice.
- Local evidence and reproducible commands live outside the repository in
  `/workspace/oteryn-prep/ach-1-evidence/`. No Rust build is needed for this slice.


## ACH-COVERAGE-1 invalid-key repair (2026-10-16)

Returned to AUTHORING for a concrete defect in the allocated coverage tooling:
with `--chest-claims`, an invalid catalogue key represented as a JSON list or
object raised `TypeError` before the CLI could emit its invalid-catalogue report.
Only string keys now enter reference binding, matching catalogue key validation.
Schema rejection, unbound-reference failure and `INVALID_CATALOGUE` coverage
remain intact; malformed values cannot count as content coverage.

The CLI regression reproduced both crashes before the fix. The complete offline
validator suite now passes 23 tests, including both malformed-key shapes. This
change requires no runtime grant writer, schema/content admission, CP slot,
PostgreSQL job or Rust source change. Existing runtime ownership blockers above
remain applicable to their separate children.


### CP D1153/D1160: batch grant writer (2026-10-16)

AUTHORING resumed from published coverage repair 9d877bf37a180a516523489a0f2eada00bf070e0.
CP comment 6224947337 assigns `durability/account_achievement.rs` exclusively
to this PR and migration 0098 for the separate receipt increment.

The existing writer now accepts 1..64 requests with one consumed same-TX
fenced witness and one Character-root lock. It rejects duplicate
(source kind, event id, key) pairs and validates all inputs before storage,
writes in event/key order and returns outcomes in input order. The single
grant delegates to this API. An all-retired batch still verifies the witness.
Errors require the earning owner to discard its transaction; no independent
transaction or replacement writer is added. The existing test-only owner
harness also calls the batch once, matching the production witness rule.

Four pure validation tests and seven real PostgreSQL case bodies are added.
PG cases live in the owned `achievement/grant_tests.rs` and are registered
only by `account_achievement_postgres.rs` and the canonical aggregate
`character_authority_postgres.rs` under D1160. They are not mounted by lib
or transitive durability copies. Missing PG is an explicit error.

NOT_FULLY_QUALIFIED (D1122): package Clippy and package/PG execution NOT_RUN.
The earlier coverage repair retains its separate 23/23 PASS and review evidence.
No compilation, runtime batch PASS or full ACH-1 completion is claimed.
Quest-transition schema belongs to #2198; counter receipts, encounter receipt
migration0098, true earning callers and post-commit notices remain unfinished.
Shared slot ordering and migration/registry wiring remain subject to CP.


## Native batch evidence persistence — 2026-10-17

This appendix preserves and corrects the historical qualification boundary above.
The earlier `NOT_RUN` statements describe the 2026-10-16 checkpoint; they do not
describe the native runs recorded here. This is a task-record amendment only.
The batch source, original test bodies and existing registrations already belong
to the published implementation.

### Exact post-merge native candidate

The current batch qualification is bound to complete source head
`f43eb51dbf12650d744b993f8fa847893edd5939`, including its merged runtime dependencies.
A separate clean checkout used a fresh Cargo target directory and the original
registered `account_achievement_postgres` target. The PostgreSQL server reported
`server_version_num=170006` (17.6), with
`OTERYN_TEST_POSTGRES_ADMIN_URL` configured for the unchanged native harness.
No source overlay, altered URL guard or process-wide `RUST_MIN_STACK` was used.

| Check on this exact source | Observed result |
|---|---|
| Original `achievement_batch_` selection | PASS: 11/11, four pure and seven PostgreSQL cases; zero failed or ignored; 1,151 unrelated filtered; 8.54 seconds |
| Strict game-server all-target Clippy | PASS, exit 0 |
| Workspace format check | PASS, exit 0 |
| Exact HEAD, clean worktree and source/build-input hashes before and after | PASS |

The selected tests completed at **2026-10-17T09:19:31Z**. Clippy and formatting
completed at **2026-10-17T09:22:37Z**. Clippy's only emitted warning was the
existing vendor Tokio `missing_docs` warning; the strict command exited 0.

```sh
cargo test --locked -p oteryn-game-server --test account_achievement_postgres achievement_batch_ --quiet -- --test-threads=1
cargo clippy --locked -p oteryn-game-server --all-targets --quiet -- -D warnings
cargo fmt --all -- --check
```

The source readback included the account-achievement writer, original test leaf,
dedicated native target, `Cargo.lock` and server `Cargo.toml`. The selected batch
files retain these exact Git blobs at the tested head:

| Source | Git blob |
|---|---|
| `apps/game-server/src/durability/account_achievement.rs` | `242d663b2946e51cdbff2459cccdcf62330d1022` |
| `apps/game-server/src/achievement/grant_tests.rs` | `ae5c41fc21908157be49c9c011b21015526d5699` |
| `apps/game-server/tests/account_achievement_postgres.rs` | `b8240fd304de4c2368c9e34e4370871a37df2c21` |

The evidence is retained on Molehill-PC under
`/root/oteryn-six-pr-20261017/evidence-ach-current/`: `source-head-before.txt`,
`source-before.sha256`, `achievement-batch.log`, `achievement-batch-result.txt`,
`clippy-all-targets.log`, `clippy-fmt-result.txt` and `fmt.log`.
This was a fresh run after the main merge, rather than an extrapolation from
unchanged batch blobs.

### Earlier evidence and remaining delivery

The earlier exact source `40163df019b45f0af136139ffba0fda6109be2f5` separately
passed the same 11 selected cases (1,105 unrelated filtered, 7.93 seconds), strict
all-target Clippy and formatting. Its evidence remains under
`/root/oteryn-six-pr-20261017/evidence-ach/`. The initial noncanonical-port
attempt correctly refused the seven database cases before execution; that setup
failure remains separate from the later successful canonical-port run. The
harness guard was unchanged.

Only the existing selected batch component is qualified by these native runs.
Full package execution, quest-transition schema and earning callers, durable
counter earning receipts, encounter receipt/grant composition with allocated
migration 0098, post-commit notices and complete gameplay/E2E delivery remain
unfinished. The earlier 23-case offline coverage result remains a separate
tooling result. `NOT_FULLY_QUALIFIED`, the shared-slot gates and CP-only
integration/merge authority remain in force.
