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

## ACH-COUNTER-1: pending catalogue loader — CP D1331–D1334 (2026-10-17)

Prepared against exact source `52007b10f28991b9eb4fa4d4eb0ea86fd67e45a7` under
[CP D1331–D1334](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233177640).
This is a held D1332 source slice. Its publication slot follows #2181 merged and
fresh main integration; the coordinator remains the sole branch publisher.

### owned_paths amendment under D1332

In addition to the already owned `apps/game-server/src/achievement/counters.rs`,
D1332 grants these exact paths for this slice:

- `content/achievements/counter-thresholds.json` — new table; the initial empty
  version is superseded here by exactly one CP-permitted, evidence-qualified row.
- `tools/content-schema/achievement-authoring/counter-thresholds.schema.json` — table schema.
- `apps/game-server/src/achievement_catalogue.rs` — the real catalogue lookup binding;
  this candidate reuses its existing `lookup` unchanged, without a duplicate wrapper.

The new loader accepts only `level`, `magic_level`, `fist`, `club`, `sword`,
`axe`, `distance`, `shielding` and `fishing`. It rejects malformed/unknown fields,
unknown versions, any status other than `PENDING`, nonpositive or unrepresentable
values, exact duplicate `(counter, value, achievement)` triples, and catalogue keys
that are absent or retired. The revision comes from the actual
`AchievementCatalogue::lookup`, never from the threshold document or a supplied
lookup callback. Different achievements may share one counter/value event; the
existing batch writer retains the separate per-earning-transaction request limit.

The result is explicitly `PendingCounterThresholdCatalogue`: pending authoring
bindings only, with no grant request, activation variant or authority witness.
The table's `PENDING` status remains unchanged. Qualification of a threshold datum
does not activate its earning path.

### Exactly one independently reviewed Reference datum

[CP capture 6233358050](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233358050)
permits exactly `oteryn:achievement/seasoned_adventurer`, counter `level`, value
`50`. The [raw transfer 6233403966](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233403966)
was independently reviewed against the actual catalogue record at this source head:

| Binding | Exact evidence |
|---|---|
| Catalogue key/revision | `oteryn:achievement/seasoned_adventurer` / `1`; the record is not retired |
| Catalogue source | `content/achievements/achievements-00000-00499.json` at `52007b10f28991b9eb4fa4d4eb0ea86fd67e45a7`, Git blob `fa42f0dc1d90a8863515f1c1ed95d24c6662386b` |
| MediaWiki page/revision | pageid `68951`, revid `753341`, timestamp `2019-01-11T21:20:04Z`; cut `2026-09-27T23:59:59Z` |
| Revision content SHA1 | `31b7fd5f97c0f45159e7bfbb6c62aac04d97cd56` |
| Raw response SHA256 | `c1ace44aae3025b4db8228272a8239a9bad95410cf6f3ce98997353adf48d1b4` |
| Independent review receipt SHA256 | `daf2dc77e825db186b0e3c83916f2cf6d2ed8113cc98a6b185c5b8ca189a05c8` |

The independent review binds the registered MediaWiki request, response/content
hashes, full acquisition condition, and the actual catalogue identity/provenance.
Its scope is this one positive datum; it is not native loader/runtime qualification.
The earlier local API HTTP 402 is retained in
[6233309005](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6233309005)
and was superseded for this datum by CP's successful capture and raw review.

The other eight counter keys receive no rows. Discovery screened current text;
historical completeness at the cut is not established, so this slice makes no
global claim that other thresholds never existed.

The acquisition condition also describes obtaining the achievement on another
level-up after already surpassing level 50. That catch-up wording is a separate
future caller question against ACHIEVEMENT0 §3.4's accepted strict crossing
`before < threshold <= after`. It does not authorize login-time or historical
automatic grants. This pending loader implements neither earning behavior and
does not change the current crossing contract.

### Qualification boundary

Eight original Rust tests live in the owned loader leaf. The embedded-table test
requires exactly the one real `level / 50 / seasoned_adventurer / revision 1` row,
resolved through the actual embedded Achievement catalogue. The other seven test
bodies remain unchanged: all nine keys, unowned keys, closed JSON shape/status/version,
lossless positive values, absent/retired bindings, duplicate triples and distinct
achievements sharing a threshold event. Their synthetic fixtures add no other
Reference claim or production row.

The coordinator qualified the original v1 JSON Schema driver on PC: **50/50 PASS**.
V2 changes the schema description only, with every validation rule preserved;
the checked-in table now carries the one qualified row. Schema execution against
this exact v2, native Cargo tests, strict Clippy and Rust formatting are **NOT_RUN**
at this held checkpoint. No prior batch/CI/native result is transferred to the
new loader.

No module registration, production activation, counter receipts, XP/build writer
hooks or proof extension is part of this source slice. D1333 still defers the two
writer hooks until #2181, #2192 and #2154 have merged and CP grants fresh pinned
hunks. D1334 requires a separate exact proof-type/consumer decision. The existing
batch, encounter/quest earning, post-commit notice, full-package and E2E gates
remain as recorded above; this slice does not complete ACH-1.

## ACH-COUNTER-1 — D1363 compiling draft checkpoint (2026-10-17)

This appendix supersedes the preceding D1332 **publication timing** hold and the
selected loader's NOT_RUN checkpoint. The complete preceding record is preserved.
[CP D1363](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234142275)
performed the ordinary main merge and permits the complete compiling draft source
checkpoint now. Its exact base is `5a309b1a89ad42481e03e273ea83808034751279`, with
parents `52007b10f28991b9eb4fa4d4eb0ea86fd67e45a7` and main
`f010472bcb6794a2cbc56dfaf755bd122c10225e`.

### D1363 owned_paths supplement

The checkpoint contains exactly these five paths within the existing D1332/D1363
scope:

- `apps/game-server/src/achievement/counters.rs` — the PENDING loader and eight original cases.
- `content/achievements/counter-thresholds.json` — the one independently qualified D1340 row.
- `tools/content-schema/achievement-authoring/counter-thresholds.schema.json` — the closed table schema.
- `apps/game-server/tests/account_achievement_postgres.rs` — exactly the granted two-line mount of the owned loader in the existing canonical test target.
- `docs/agents/tasks/ACH-1.md` — this additive allocation and evidence record.

D1363 permits publication of the actual test mount in this draft. The original
entry, catalogue registration and test bodies are preserved. The existing
`AchievementCatalogue::lookup` is reused unchanged. The loader/table remain
PENDING and provide no grant, runtime consumer, XP/build hook or authority proof.

### Preserved attempt 1: failed compilation, no counter test result

The first assembly used exact `52007b10f28991b9eb4fa4d4eb0ea86fd67e45a7`.
Its fresh locked fetch passed, but the selected test command exited **101 before
test execution** with three unresolved protocol `logout` imports. No eight-case,
Clippy or workspace fmt PASS is attributed to that attempt.

Actual current source already exported `logout` unconditionally. Root retained
Cargo's `fresh:true` protocol artifact and its dependency text omitting
`logout.rs`. These facts establish the incompatible reused dependency artifact;
they do not identify a particular sccache failure mechanism. No counter or
protocol source repair was used for the retry.

Historical evidence remains in
`/root/oteryn-six-pr-20261017/evidence-ach-counter-native-1017/`:

- Raw failed `results.json`: SHA256 `154baac0d172bfea737912236242d54c7bd83f1b89b3be70d6f3b4d3161304a2`.
- Failed `ach-counter-eight.log`: SHA256 `3801c713bbacfdb714c2884c299f1ac2c3205945abd12a497fc5458f0590f7f4`.
- Retained diagnosis `/mnt/c/Oteryn/tmp/six-pr-ach-cache-diagnosis.json`: SHA256 `6297f6669fa24c22f4fc3b4bd23e986e5c34fe45992bc7248cb79a8680740808`.
- Attempt-1 driver: SHA256 `216a8ba38031dc2fab5fa1ac0f4984be231e0cef64c819cbcdea29a74ed2d03f`.

### Exact current-source native qualification

The coordinator's attempt 2 qualified **8 passed, 0 failed, 0 ignored, 0 measured,
1163 filtered out**, in **0.01 seconds**, at `2026-10-17T13:23:01.170836+00:00`.
These are the eight original pending-loader/schema/catalogue cases, through
`--test account_achievement_postgres`; they are not eight PostgreSQL grant cases.
Explicit target strict Clippy, game-server all-target strict Clippy and workspace
Cargo fmt also passed. The terminal run record reports PASS and source UNCHANGED;
the qualified before/after snapshots are equal at exact base `5a309b1a`.

D1363c's repair was ordinary `cargo clean` only in the run's held D1342 slot
**A**, target `/root/oteryn-target-A`, followed by a fresh locked fetch and the unchanged
selected checks. Each recorded Cargo command used the pinned shared build
environment and a fresh MemAvailable check of at least 12 GiB. The shared sccache
configuration remained unchanged. No cache purge, source-mtime or compiler-flag
workaround, peer action, or outside-target ELF action was used.
The original tests used their default Rust stack and one test thread.

Fresh source preparation is bound by
`/root/oteryn-six-pr-20261017/evidence-ach-counter-current-prep-v2-1017/prep.json`,
SHA256 `8e54e2a6966e25395c3a05f90ec6e2f3a03f1ef8da88fe31fe5e2c0d456c0ec4`;
it records the exact source/build pins and **50/50** schema cases passing.
The four source/content publication files are byte-identical to native input:

| File | SHA256 |
|---|---|
| `achievement/counters.rs` | `cb326fe554de199f034124be70e984313ebc5ae2b9a8a21c42f0ef7848c22e69` |
| `tests/account_achievement_postgres.rs` | `0fce1e7aa54c396a6ca8f9d893ae5b19a33b26c92e2563fbd29bcb36658f92a3` |
| `content/achievements/counter-thresholds.json` | `e943343675780fa2ce0ea52f8f769b5797c4b0aa380f0f5e5e980a59537800f1` |
| `achievement-authoring/counter-thresholds.schema.json` | `d0fd833d9e3df61c0bf1fb913ee9f0c673fa92ad23a7feac26ea664f3f23f09d` |

The native input task-card bytes (`46afaf7ce1e7eb1f87144939f869f8fd4eb5844a15bb485f778edacbcdaf8c57`)
remain the complete prefix before this appendix. The publication adds only this
truthful documentation after those qualified inputs.

Actual requested commands below were each wrapped by the recorded pinned build
environment; the native receipts retain the complete outer argv:

```sh
cargo clean --target-dir /root/oteryn-target-A
cargo fetch --locked
/root/.oteryn/in-pg-netns.sh cargo test --locked --offline -p oteryn-game-server --test account_achievement_postgres achievement_counter_thresholds_ --quiet --message-format=json-render-diagnostics -- --test-threads=1
/root/.oteryn/in-pg-netns.sh cargo clippy --locked --offline -p oteryn-game-server --test account_achievement_postgres --quiet -- -D warnings
/root/.oteryn/in-pg-netns.sh cargo clippy --locked --offline -p oteryn-game-server --all-targets --quiet -- -D warnings
cargo fmt --all -- --check
```

| Check | Result | Finished at UTC | Actual log SHA256 |
|---|---|---|---|
| ach-counter-clean | PASS | 2026-10-17T13:16:51.697749+00:00 | 34054be9ba0ac00767ebd8904e92ae5e5734eff85b5029bc965d1ff0684bd9de |
| ach-counter-fetch | PASS | 2026-10-17T13:16:55.471312+00:00 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| ach-counter-eight | PASS | 2026-10-17T13:19:16.908264+00:00 | ec9b20d8f210e8959100e5968a7d7190c218b247560655bd697957c0d9dccd45 |
| ach-counter-clippy-target | PASS | 2026-10-17T13:21:08.895005+00:00 | 984a0050d7b0091cd2c5701c8ec6e6b74b8587417704637c594d8849234b24a1 |
| ach-counter-clippy | PASS | 2026-10-17T13:22:28.070901+00:00 | 984a0050d7b0091cd2c5701c8ec6e6b74b8587417704637c594d8849234b24a1 |
| ach-counter-fmt | PASS | 2026-10-17T13:23:00.733713+00:00 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |

Native evidence directory:
`/root/oteryn-six-pr-20261017/evidence-ach-counter-native-v2-1017/`.

| Receipt or source binding | SHA256 |
|---|---|
| `ach-counter-qualified.json` | `b2b01a3501a5a248c34e387fa57ff3eabc9ea548e53cb54565909cfaec85bd17` |
| Terminal `results.json` | `27e3cbfed5f64415e37dc3a07efbfb9aa680e6bedf7740a02387304bd49b4361` |
| `clean-qualified.json` | `56e6e219ca0429bb489439790eabb66305d2d61ff3157fbdd7d03e70e29a17b3` |
| `fetch-qualified.json` | `c867ea47a07d50b86c4cd5c853dc4fdfe67c5b8e9d580a6cc5e84e9603fbedb4` |
| `ach-counter-artifact.json` | `2052522c9b4bb866cc15f0fabc8ddb4e1af53223cdc8596bb26359ea4dc2bce8` |
| `protocol-artifact.json` | `fbda21e8d526e29c14fc36078c07ba1c14e21c5ec5470854480eab4706a372f8` |
| Actual test ELF, checked under the native slot lease | `265341f1f34a8afc95220ddf4aa940922ebaaaaa0ab18b092e21d0383b5c6cdf` |
| Actual test dependency file | `b1012024a6385240ebda03a3740b722f68053f207a1edf3f60a5fc5efbf90c73` |
| Retained current protocol dependency file | `20916bdf5c82ee33919b88b9abaadf8d1ff0a7cd6275c66526d1649f6c697806` |
| Attempt-2 driver | `8a81aa75b380842b2684aa53b01be336d4c4b9b86bf96e36537dba679b95299b` |
| Shared slot helper | `851870ed67accd2f66bfb5f0ecce2a81662e908509cfe742daaebb7b0cacb3e3` |
| Shared build environment | `afc2f04924c412b5965a1c4f6a2b30dfcd9e80ac0a185af737d070ebedbefd82` |

The actual Cargo JSON and test dependency receipt bind the canonical entry,
owned loader, real Achievement catalogue and threshold table to the native
checkout. The protocol receipt records a rebuilt (`fresh:false`) actual library
and retained dependency text containing both current `lib.rs` and `logout.rs`.
The driver verified these artifacts while it still held its slot; publication
rehashes their saved receipts and logs without reading a later user's live slot
ELF or dependency files.

### Remaining integration gates

The source checkpoint does not complete ACH-1. Under D1363b, merge and production
activation remain **after #2181 merges**. The shared test-file integration order
is still #2181 → #2154, and #2154 merges then-current main again after #2181 lands.
Ready-for-review and queue actions remain CP-only.

D1331, D1333 and D1334 remain DEFERRED. The separately qualified single D1340 row
does not establish a complete threshold catalogue; historical absence of other
thresholds has not been proved. The captured level-50 catch-up wording remains a
future caller question against the accepted strict crossing contract. Earning
receipts/callers, XP/build hooks, proof-type extension, encounter/quest composition,
notices, full-package execution and gameplay/E2E retain their existing gates.

Frozen-commit diff/governance checks and publication readback are required after
this source freeze. Their actual outcomes belong to the separate publication
record and PR evidence; this appendix contains no claim about a future commit.

## 2026-10-17 — Existing ACH batch PostgreSQL tests in the non-PG CI lane

This appendix preserves the complete task-card history at predecessor
`a24c919b475bf88146faee320becbfc839073bdd` (SHA256
`f9b44235259bed9869e13346b5f48a4a5a4a8179fe54c99c547ad9063359a1a6`).
The current correction changes only the missing-configuration branch in
`apps/game-server/src/achievement/grant_tests.rs`, an existing ACH-owned test file.
It keeps the existing shared `configured_admin()` diagnostic and returns `Ok(())`
after that helper reports `PRE-ROUTING / NONCANONICAL`.

The original CI integration-rest job `117409280578` at the predecessor failed
all eight batch cases before PostgreSQL routing because the URL was absent
(aggregate result: 1,584 passed, eight failed, exit 101). The retained root-saved
failure excerpt has SHA256
`f2ca0bedecec5380befa5f05b8b8554bb16eaf42ef17618494ba1fc14f64c9e7`;
it is an excerpt, not a full-job log. The separate canonical durability job
`117409280834` passed but was not substituted for the selected eight-case result.
The existing repository convention was described for HOUSE by CP D1366
([decision](https://github.com/Oteryn/Oteryn-Game/pull/2131#issuecomment-6234335395));
this appendix applies the same convention within the already-owned ACH test file.

### Source and scope

The source preimage is Git blob `34f5188bb31c7069b2c65b08136394781621a1ef`.
The reviewed repair is SHA256
`25aabd1098e4936b984c04112bb39fb982a401b5882156e83bdc39f8cf3c18bf`
and changes one branch by two added lines and one removed line. Its complete
configured execution path, PostgreSQL 17.6 assertion, all eight case bodies and
fixture cleanup are byte-identical to the predecessor. Present malformed URL
values still reach and fail the original canonical harness guard.

The original `account_achievement_postgres` Cargo target and its existing
`achievement_batch_postgres_cases` mount are unchanged. No harness, fixture, test
body, Cargo configuration, workflow, ignore/skip policy, counter source, threshold
table, schema, writer or migration changed. No counter cases were selected for a
repeat run. The existing protected pipeline and merge gates remain in force.

### Source-bound qualification

At `2026-10-17T14:32:59.059417+00:00`, the original eight batch cases completed through the shared
canonical PostgreSQL 17.6 wrapper: **8 passed, 0 failed, 0 ignored, 0 measured,
1163 filtered, 12.85 seconds**. The log contains exactly eight
`ACHIEVEMENT-BATCH-NATIVE-PG server_version_num=170006` execution markers and no
noncanonical marker.

The same eight selected tests were then invoked through ordinary Cargo with
inherited `OTERYN_TEST_POSTGRES_ADMIN_URL` and `DATABASE_URL` removed:
**8 passed, 0 failed, 0 ignored, 0 measured, 1163 filtered,
0.00 seconds**. This run records eight explicit
`PRE-ROUTING / NONCANONICAL` early returns and zero PostgreSQL execution markers.
It qualifies the missing-configuration behavior only and is not a PostgreSQL
test pass. Both invocations are bound by their actual Cargo JSON to the same
original test ELF and current-worktree dependency receipt.

Fresh `cargo fetch --locked`, strict explicit-target Clippy, strict all-targets
Clippy and workspace fmt check also passed. The explicit target check is retained
because the existing focused target has `test = false`. All commands sourced the
pinned D1342 build environment while the driver held slot `B` and target
`/root/oteryn-target-B`; the minimum memory floor was 12 GiB. The shared sccache configuration
remained unchanged. No cache purge, source-mtime or compiler-flag workaround,
peer action, or outside-target ELF action was used. The native checkout and all
15 bound source/build/card inputs were unchanged before and after qualification.

The actual requested commands were:

```sh
cargo fetch --locked
/root/.oteryn/in-pg-netns.sh cargo test --locked --offline -p oteryn-game-server --test account_achievement_postgres achievement_batch_postgres_cases:: --quiet --message-format=json-render-diagnostics -- --nocapture --test-threads=1
cargo test --locked --offline -p oteryn-game-server --test account_achievement_postgres achievement_batch_postgres_cases:: --quiet --message-format=json-render-diagnostics -- --nocapture --test-threads=1
/root/.oteryn/in-pg-netns.sh cargo clippy --locked --offline -p oteryn-game-server --test account_achievement_postgres --quiet -- -D warnings
/root/.oteryn/in-pg-netns.sh cargo clippy --locked --offline -p oteryn-game-server --all-targets --quiet -- -D warnings
cargo fmt --all -- --check
```

| Check | Result | Finished at UTC | Actual log SHA256 |
|---|---|---|---|
| ach-batch-fetch | PASS | 2026-10-17T14:27:39.331550+00:00 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |
| ach-batch-pg-eight | PASS | 2026-10-17T14:29:44.292135+00:00 | 26dec17e452b863028b998b65e46dce2df176ec9ab2c9978d046fd06c0621b26 |
| ach-batch-nonpg-eight | PASS | 2026-10-17T14:29:48.332728+00:00 | 0fef987011ffb0e81f63894ac35eef3d88a4310c57fa71128c1df565f5566ca3 |
| ach-batch-clippy-target | PASS | 2026-10-17T14:31:06.933811+00:00 | 984a0050d7b0091cd2c5701c8ec6e6b74b8587417704637c594d8849234b24a1 |
| ach-batch-clippy | PASS | 2026-10-17T14:32:22.410948+00:00 | 984a0050d7b0091cd2c5701c8ec6e6b74b8587417704637c594d8849234b24a1 |
| ach-batch-fmt | PASS | 2026-10-17T14:32:58.337747+00:00 | e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 |

Native evidence is retained at
`/root/oteryn-six-pr-20261017/evidence-ach-batch-ci-native-1017/`.
The original CI excerpt and preparation are retained separately at
`/root/oteryn-six-pr-20261017/evidence-ach-batch-ci-prep-1017/`.

| Receipt or source binding | SHA256 |
|---|---|
| `ach-batch-qualified.json` | `583cd1575c29e694a05a6cc1a846ca44f9936f34b3b9fbc0cc3d570c7474a4f8` |
| Terminal `results.json` | `74fe0d1931db5de78f240b9453f73b0e63c9f8b6fecdc2b998646a9b110ecc78` |
| Actual exact-source `prep.json` | `6f4eabaff039fe3bd814d83aa36a14d3b64c82d8cab3e11bd294cebe06ef28e7` |
| `fetch-qualified.json` | `648d8d9a51d086dabe1da8f66443553a11bd904f5ba6f6b30a8ce3ebba2201b2` |
| Canonical PG Cargo artifact receipt | `7e951cf9449f404303063f568ffbe1264d6f667add7ebe7e78e42acea546ef18` |
| Non-PG Cargo artifact receipt | `83a4621103db5c1f81320bdc42600f34781487c713fb38a60f3955f1af18f2d9` |
| Protocol Cargo artifact receipt | `5e532fbb4b0477a898f2df9517d4158b204203ce3672863e274fe5da13a26894` |
| Shared actual test ELF, checked while the native lease was held | `df8e3dec5bb7cca3e27cd8e61996af2e4cc89d1285724cf73534af5771f5ceb7` |
| Shared actual test dependency file | `0ce0a7938ec72e4c1519600c545892c218bc4bedd819a389e8bcf1afb5c559e1` |
| Retained current protocol dependency file | `32b503ac482cf65dd4952ee9be886f65800f2a468e4ce7fe5252452e57adee11` |
| Native driver | `b296cdd8741874663add3086a84916db8f3d03990330df85392a0728358bcd2a` |
| Shared slot helper | `851870ed67accd2f66bfb5f0ecce2a81662e908509cfe742daaebb7b0cacb3e3` |
| Shared build environment | `afc2f04924c412b5965a1c4f6a2b30dfcd9e80ac0a185af737d070ebedbefd82` |

The source receipts include the unchanged original test entry and canonical
harness, the existing achievement writer and the corrected test file. Actual
Cargo JSON records a newly built test artifact in the PG invocation (`fresh:
false`) and reuse of that same artifact in the non-PG invocation (`fresh: true`).
The protocol artifact also records `fresh: false`; its retained dependency text
contains both current `lib.rs` and `logout.rs`.
Publication rehashes saved receipts and logs without opening a later slot user's
live ELF, library or dependency files.

### Remaining gates

This test-routing repair does not complete ACH-1. The predecessor's counter
qualification remains historical evidence on its recorded source. D1363b still
requires merge and production activation after #2181 and then-current main
integration in the prescribed order. D1331/D1333/D1334, earning callers and
receipts, XP/build hooks, the threshold catch-up interpretation, encounter/quest
composition, notices, full-package execution and gameplay/E2E retain their
previous gates. This selected qualification is not an aggregate CI rerun.

The final source/card commit still requires frozen-commit diff/governance checks
and remote publication readback. Their actual outcomes belong to the publication
record and PR evidence; this appendix does not claim a future commit result.
