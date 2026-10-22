# QUEST-RT-1 — quest runtime on the bundle World: XP admission caller, chest rewards, capability 16 (consolidated task card)

```yaml
task_id: QUEST-RT-1
title: "QUEST-RT-1: quest runtime on the bundle World, quest XP admission caller, chest rewards, capability 16 offer"
mode: IMPLEMENT
status: in_progress  # integrator admitted explicit journal exclusions
repository: Oteryn/Oteryn-Game
issue: 1622
wave: 2
plan: "docs/architecture/reviews/OTERYN_GAME_MECHANICS_COMPLETION_PLAN_2026-10-13.md"
plan_row: "§4 Epic 2 QUEST-RT-1 (rows 86, 87); App. 7 QUEST-RT-1"
brief: "docs/architecture/reviews/OTERYN_GAME_ARCH_PROGRESSION_SOURCE_PACKETS_2026-10-05.md §2.3 (QUEST-XP-ADMISSION-1 caller); QUEST_GATE0 / QUEST_STATE0 decisions"
class: "hard (rewards are mints)"
base_branch: main
branch: w2/quest-rt-1
pr: null
needs_migration: false  # existing quest/chest/XP owners suffice
needs_registry: true     # capability 16 QUEST_LOG_V1 offer flip
merge_authority: CP_ONLY (builder never marks ready)
```

Reference implementation (pre-ADR-0022, do not copy paths or structure): draft #2198, branch skeleton/quest-rt-1

## Goal

Quest runtime on the bundle World: pending quest XP obligations paid exactly once at
quest admission/refresh (the missing production caller), chest rewards minted once per
Character and replay-safe, and capability 16 (QUEST_LOG_V1) offered once its gate
holds. The draft delivered the XP admission caller patch (held, never landed) and six
native PostgreSQL scenarios; chest-use E2E and the capability offer remain open.

## Dependencies (re-verified against origin/main 59785d76)

| Dependency | State |
|---|---|
| QUEST-LOWER-1 | MERGED (#1727) |
| QUEST-XP-1 / QUEST-XP-1b (obligation + XP writer, migration 0069) | MERGED (#1724, #1731) |
| PROGRESSION-OWNER-1 / PROG-BIND-1 (session progression binding) | MERGED (#1964) |
| QUEST-LOG-WIRE-1 (capability 16 wire, domain 16; `offered: false`) | MERGED (#1741); offer gate still needs QUEST-CONTENT-2 journal content |
| QUEST_GATE0 / QUEST_STATE0 acceptance | ACCEPTED (#2209) |
| NPC-ECON-1 (#2197, ordered before the D1150 slot) | MERGED (#2197) |
| ITEM-LOOP-1 | MERGED (#2190) |
| MAP-PICKUP-1 | MERGED — #2290 (migration 0130) |
| QUEST-XP-ADMISSION-1 caller | not on main: `request_pending_quest_experience` still has no production caller — this card owns it (CP D1150) |

## Accepted decisions

- **ARCH_PROGRESSION_SOURCE_PACKETS §2.3:** "This packet adds the call" — the XP caller
  lives in the transport owner's `refresh_quest_session`; no second loader, admission
  owner, XP writer or retry loop.
- **CP D1150:** this task owns the `refresh_quest_session` → `request_pending_quest_experience`
  call, landed in the ordered shared slot after #2197 on then-current main.
- **CP D1316:** grants the native monk-save fixture mount for the admission test leaf,
  atomically with the D1150 caller.
- **CP D1329 / D1342:** native PG execution policy (evidence only for the draft).
- Capability 16 stays **unoffered** until its QUEST-LOG-WIRE-1 offer gate holds; journal
  text/wire ownership is QUEST-LOG-WIRE-1's, not this task's.
- Quest content pins are provenance only, never the active progression reward revision;
  the award uses `obligation.award(policy)`.

## Reference behaviour from the draft (re-implement, do not copy)

Existing pieces to reuse unchanged: `durability/quest_state.rs`
`read_character_quest_state` (bounded coherent copy under root share lock, preserves
`WAITING_MIGRATION`), `admit_character_quest_state` (requests pending claim obligations
through the sequencer), `request_pending_quest_experience` (iterates the bounded pending
XP copy, calls `RevisionSlot::commit_quest_experience`, settles on success/replay,
requests backoff only for unknown `Unavailable`).

Caller behaviour in `refresh_quest_session`:

1. After admission returns a usable copy, get this session's progression binding via
   `progression_binding` (clone the `Arc`, release the map lock before awaiting); call
   the XP helper with the independently resolved `current_quest_fence`, existing
   root/authority/node/sequencer and `&binding.policy`. Never build a policy from quest
   data or reconstruct authority from the copy. Heap-pin the helper future
   (`Box::pin`, following `admit_progression_session`) — the full policy carries its
   threshold table by value; the draft measured the refresh future 202,504 → 8,376 bytes.
2. OR the helper's retry with the existing load/claim retry; keep
   `QUEST_OBLIGATION_RETRY` (60 s). Terminal installs no copy and no retry; an unknown
   fence read retains the prior copy and schedules backoff; `insert: false` never
   recreates an entry removed during the await.
3. No progression binding → no XP request, obligations stay pending, log
   `quest_experience_deferred reason=no_progression_binding` once per session (flag on
   the existing `QuestSession` entry, logged after releasing the mutex), no retry solely
   for that reason.
4. Existing callers (fresh admission, resume, cadence retries, chest refresh after a
   committed transition) then cover every path.

Six native PostgreSQL scenarios (draft: 6/6 pass, run on a 64 MiB test thread; default
stack overflowed — keep the explicit test-thread recipe from
`kill_reward_postgres_e2e_tests.rs`, never change production stack or `RUST_MIN_STACK`):

1. Pending XP from an earlier native owner pays once at bound admission; repeated
   admission and transition refreshes do not pay twice.
2. Repeated unbound admission keeps pending XP and the one-time deferral marker with no
   retry; a later binding pays once.
3. Injected XP-receipt transaction failure leaves the obligation pending, schedules
   backoff, pays once when the cadence is due after recovery.
4. Replacing only the durable connection generation after the fence read refuses the
   award, keeps the obligation, adds no retry.
5. Removing the local session and binding while a refresh is paused: `insert: false`
   recreates neither.
6. Refresh after a real expired-loss release in FreshAdmissionStore: fence `Ok(None)`;
   two refreshes discard the copy and retry state; durable XP/receipts/obligations stay
   `(0, 0, 1)`.

The fixture's quest catalogue pin is the admitted `content-1` context and its
reward-revision inequality is asserted.

## Remaining (not done in the draft)

- Land the caller and the fixture mount (they were never merged).
- Literal chest-use command → refresh E2E; complete fresh-admission/transport E2E;
  unknown-fence-read and terminal-lifecycle regressions.
- Chest reward minted once per Character, replay-safe, through the existing chest MINT
  (`crates/gameplay/social/src/interaction/chest_use.rs`).
- Capability 16 offer flip in `capabilities.rs` and the registry once its gate holds.

## Target layout (ADR-0022)

- Caller: `crates/gameplay/dispatch/src/gameplay_transport/mod.rs`
  (`refresh_quest_session`, ~line 2050; call-site + `QuestSession` flag only).
- Durable helpers (exist, unchanged): `crates/gameplay/durability/src/durability/quest_state.rs`.
- Admission scenarios: `crates/gameplay/dispatch/src/gameplay_transport/quest_admission_postgres_tests.rs`
  (new), mounted from `crates/gameplay/dispatch/src/gameplay_transport/monk_save_postgres_tests.rs`
  (registration line only).
- Chest: `crates/gameplay/social/src/interaction/chest_use.rs` (exists); PG cases
  `apps/game-server/tests/support/chest_use_postgres_cases.rs` (exists) or a new child
  case file; quest XP cases `apps/game-server/tests/support/quest_xp_postgres_cases.rs` (exists).
- Quest log projection: `crates/gameplay/dispatch/src/gameplay_transport/quest_log.rs`,
  `crates/protocol-oteryn/src/quest_log.rs` (exist).
- Capability: `crates/gameplay/dispatch/src/gameplay_transport/capabilities.rs` +
  `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` (kept equal by `capabilities_tests.rs`).

## Acceptance criteria

- [x] Pending quest XP paid exactly once at bound quest admission/refresh; six scenarios above pass.
- [x] Chest reward minted once per Character; replay safe; literal chest-use E2E.
- [x] Capability 16 offered only when its QUEST-LOG-WIRE-1 gate holds; registry and offer sets equal.
- [x] Every concurrent transition serialized and the losing writer typed.
- [x] Structure: one mechanic per module file in its ADR-0022 layer.
- [x] Structure: new tests live in their own child files, not appended to existing files.
- [x] Structure: edits to `mod.rs`/`lib.rs`/`connection.rs`/`capabilities.rs` are wiring-only.
- [x] Structure: no file over ~1000 lines grows by more than ~150 lines (`gameplay_transport/mod.rs` takes the call site only).

## Validation

### Current PostgreSQL validation repair (2026-10-22)

Returned to `AUTHORING` from integrator candidate `214d409da` for the three
reported `character_authority_postgres` quest failures. This local repair is
uncommitted; publication and independent candidate review remain with the
integrator.

The unchanged `character_authority_postgres -- quest --nocapture` selection
passed all 40 tests against PostgreSQL 17.6 before any production repair
(91.09 s). The three reported failures did not reproduce in that run. These
fixtures use the admin role, so their intermittent `CommitOutcomeUnknown`
failures are not proven to have the runtime-role privilege cause below.

The new admission caller exposes an existing production privilege defect:
both the quest transition's pending-XP capacity read and the XP writer's
obligation read use `FOR UPDATE` on `game_character_quest_xp_obligations`.
Migration 0069 deliberately grants runtime SELECT/INSERT/DELETE, without
UPDATE. A new child regression reuses the composed admission scenario with
the root's sole connection switched to `oteryn_game_runtime`; the final
scenario also asserts that this role has no UPDATE privilege. Before the
repair it failed, and temporary local diagnostics captured SQLSTATE `42501`,
`permission denied for table game_character_quest_xp_obligations`. The
diagnostics were removed.

Both reads now rely on the already-held Character root `FOR UPDATE` lock,
which every production quest-obligation insert and consumption holds. This
preserves serialization, the RL-10 capacity boundary and the receipt/consume
transaction without expanding grants or changing schema, guards, deadlines
or existing test expectations. The composed runtime-role regression then
passed (one test, 50.52 s), covering transition, admission payout and replay.

Execution uses `/home/mole/oteryn-cache/codex-bin/cargo +1.95.0`. An initial
unqualified PATH `cargo` run had no PostgreSQL environment and early-returned;
it is excluded from qualification. A pre-repair full-target run was interrupted
to prioritize the focused privilege reproduction; it has no passing result.
Final test arguments after the wrapper's `+1.95.0 test --locked`:

| Arguments | Result |
|---|---|
| `-p oteryn-game-server --test character_authority_postgres --quiet` | 953 passed, 0 failed/ignored/filtered; 1326.62 s |
| `-p oteryn-gameplay-dispatch --lib --quiet -- quest_admission_runtime_role --nocapture` | 1 passed after the production fix |
| `-p oteryn-gameplay-durability --lib --quiet` | 384 passed |
| `-p oteryn-game-server --lib --quiet` | 306 passed, 3 existing ignored |
| `-p oteryn-gameplay-dispatch --lib --quiet` | 914 passed, 14 existing ignored; includes the final runtime-role regression |
| `-p oteryn-gameplay-quests --quiet` | 34 passed; doc tests also pass |

`clippy --locked -p oteryn-gameplay-durability -p oteryn-gameplay-dispatch
-p oteryn-game-server -p oteryn-gameplay-quests --all-targets --quiet -- -D warnings`
passed. Cargo still emits the unchanged vendored Tokio `missing_docs` warning;
no vendored dependency was edited. `fmt --all -- --check` passed. Governance
validation, all 124 governance unit tests and `git diff --check` passed.

The whole implementation and local repair diff were reread against
`~/oteryn-cache/CODEX-CHECKLIST.md`. Every added type/field/variant's constructor,
match and implementation sites were searched in `apps/`, `crates/`, `tools/`;
this repair adds only the test-local `Case::RuntimeRole` variant and no public
production function, struct field, trait method, persisted code or migration.
The regression is registered once beneath the existing monk-save fixture and
runs in CI-PG-LIB-1's dispatch `--lib` target. Existing admission scenarios and
test assertions are preserved. In this repair no production file grows by more than two lines;
the child regression is eight lines and reuses the existing scenario.

The three reported tests pass within the complete 953-test target, but their
original intermittent failure cause remains unverified: they also passed in
the unchanged, configured 40-test selection. The verified `42501` repair must
not be presented as proof that those admin-role failures had the same cause.
The integrator still owns candidate freeze, independent review and publication.

### Worktree preflight (2026-10-22)

- Baseline: `9f814e72a3a32ad2a6ed67787c2d67a2d39d4af3`; no implementation edits.
- Planned module layout: `gameplay_transport/quest_admission.rs` would own the
  refresh orchestration extracted from `mod.rs`; admission regressions would live
  in `quest_admission_postgres_tests.rs`, mounted once under the existing
  monk-save fixture. Chest dispatch regressions would use a separate child test
  file and the existing chest MINT and native PostgreSQL fixtures.
- Blocking prerequisite: the capability 16 gate in
  `docs/contracts/PROTOCOL_OTERYN_V1_REGISTRY.json` requires composed quest-log
  content and a production session-copy source. Workspace Rust searches find
  `QuestLogCatalogue::new`, `QuestLogContent::new` and `QuestLogLines::new` only
  in tests. `ComposedFreshAdmission` has no `observe_quest_log` implementation;
  the trait default in `connection.rs` returns `QuestLogObservation::Unavailable`.
  QUEST-CONTENT-2's merged status does not establish this runtime composition.
- The accepted decisions above exclude journal content ownership from this task.
  Enabling capability 16 now would offer queries that always fail closed;
  implementing its missing content loader would expand the allocated scope.
  Stopped under the coordinator's blocking-dependency/scope rule. The owning
  lane must deliver the quest-log content loader, or the coordinator must
  explicitly revise the allocation before end-to-end implementation resumes.
- Capability 16 remains unoffered. XP admission, chest dispatch, new suites and
  runtime validation remain unfinished. No migration or Rust API was added.
- File sizes: this task record is 189 lines (baseline 157, net +32);
  no file grew by more than 200 lines. No helper, fixture, harness or module
  registration was added or copied.
- Documentation checks: governance validation and all 124 agent unit tests pass;
  `git diff --check` passes.
- Full task-record diff reviewed against the owner's D1456 checklist: no hits.
- Cargo test filters: none executed; no Rust crate was changed. The commands
  below remain required after implementation and are not recorded as passes.

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-gameplay-dispatch quest_admission -- --test-threads=1
OTERYN_TEST_POSTGRES_ADMIN_URL=... cargo test --locked -p oteryn-game-server --test chest_use_postgres
cargo test --locked -p oteryn-game-server capabilities
python3 tools/agents/validate_governance.py
```

### Integrator decision 2026-10-22: prerequisite implemented in this lane (allocated; blocked below)

The integrator authorizes the missing composition before the runtime work. Planned
modules: `quest/log/loader.rs` and `loader_tests.rs` for existing source journal
interpretation; `gameplay_transport/quest_log_source.rs` for session projection;
`node/quest_log_content.rs` for boot composition; `quest_admission.rs` for the
extracted refresh orchestration and `quest_admission_postgres_tests.rs` for native
admission regressions. Existing state loader, source-text registry, projection,
XP sequencer/writer and monk-save fixture are the planned reuse sites. No migration is planned.

### Additional prerequisite audit: incomplete admitted journal counter (2026-10-22)

The integrator's option B authorizes implementing composition, so work resumed
with status `in_progress`. A trial loader reused `LoweredQuestState`, the existing
`QuestLogCatalogue` validator, `samples/questlog/quests.json` and the captured
`samples/source_texts/source_texts.json` registry. It checked registry hashes,
resolved source references through the admitted track map, and validated the whole
catalogue. This exposed a different, concrete blocking content dependency:

- Quest `oteryn:quest.killing_in_the_name_of_quest`, mission
  `paw_and_fur_hunting_elite`: journal reference
  `519d69f9b699afac8b48c55edf5c4d7f6d05b0caf568e76e41aff5ed5a3c7dd3`
  contains **two** `%d` counters. The source quest metadata declares only one read:
  `quest/u8_5/killing_in_the_name_of/boss_points`.
- The exact pinned Canary source at revision
  `04b83b512114bfd888000d6e1433ed8ecaec7c5b`,
  `data-otservbr-global/lib/core/quests/catalog/011_killing_in_the_name_of.lua:13`,
  reads `POINTSSTORAGE` for hunting points, then `BossPoints` for boss points.
  Both source values are clamped with `math.max(value, 0)`. Repeating the single
  supplied read for both placeholders would display false progress.
- The production `content/quests/missions/quest-state.json` admits 96 quests and
  1,328 tracks. This quest has only the `boss_points` points track; it has no
  `POINTSSTORAGE` binding. The existing coverage evidence also names that source
  storage in `samples/quest-coverage-2026-09-27.json` as a task/points system gap.
- `ots_questlog.py:127-128` records a sorted set of template reads, not an ordered
  placeholder binding. It cannot supply the missing global storage identity,
  argument order or clamping semantics to the composition loader.

This cannot be repaired by composition alone. It needs a content-owner correction
that admits the hunting-points source and its ownership/writer, preserves the two
ordered counter bindings and their clamping, and addresses existing quest
`definition_hash`/pin compatibility when adding a track. This packet authorizes
loading the merged content, not inventing a progress source or repinning persisted
quests. Stopped under the explicit blocking-dependency/other-lane-path rule.
Capability 16 remains unoffered; no placeholder or partial journal was installed.

The exploratory Rust edits were removed, preserving the original task-card audit
and leaving the executable baseline unchanged. The prerequisite is **not** marked
implemented. XP admission and the original acceptance criteria remain unfinished.
No migration, new Rust API, helper, fixture copy or duplicate registration remains.

Validation of the exploratory loader: `cargo +1.95.0 test --locked -p
oteryn-gameplay-quests quest_log_content --quiet` ran two tests: the corrupted-text
control passed; the full-catalogue load failed with `unbound journal counter`.
Those exploratory tests were removed with the incomplete implementation; this is
blocker evidence, not passing delivery evidence. Final documentation validation
and whole-diff checklist results are recorded below after execution.

### Final validation and file-size record (2026-10-22 continuation)

- Only this task card differs from the executable baseline. No production
  enum/field/trait method or public function remains added, so the constructor,
  match, implementation and real-dispatch checks introduce no new sites.
- `python3 tools/agents/validate_governance.py`: PASS.
- `python3 -m unittest discover -s tools/agents/tests`: PASS (124 tests).
- Whole final diff reread against `~/oteryn-cache/CODEX-CHECKLIST.md`: no hits.
  No existing test was modified or removed; capability 16 remains gated.
- `git diff --check`: PASS. No file grew by more than 200 lines (the task card
  grows by fewer than 150 lines against HEAD, including the prior audit).
- Cargo test filters covering the remaining change: none (documentation only).
  No Rust clippy/rustfmt or PostgreSQL pass is claimed for this continuation.
- Next step: the integrator resolves the missing admitted hunting-points source
  and journal argument-binding contract; then resume the authorized loader,
  production session source, XP caller and all acceptance tests in this worktree.

### Integrator decision 2026-10-22

CP confirms that POINTSSTORAGE does not block this lane. Quests/missions without
an admitted progress track or an ordered counter binding are excluded from the
capability-16 journal with machine-checked reason `no admitted progress track`.
No counter is invented; merged QUEST-CONTENT-2 ordering is preserved. All other
entries use real content. This supersedes the prerequisite blocker above.

### Integrator decision 2026-10-22: prerequisite implemented in this lane

Layout planned before coding and implemented: `quest/log/loader.rs`
loads the existing source catalogue, `quest/log/journal_source.rs` interprets
its text references, `quest/log/source_texts.rs` verifies captured prose; child
`loader_tests.rs` tests the mechanic. `gameplay_transport/quest_log_source.rs`
projects session copies and `quest_admission.rs` owns extracted refresh logic.
`node/quest_log_content.rs` composes boot content. Admission/chest regression
children reuse the existing monk-save fixture, mounted once. No migration needed.
Offer assertions moved to `capability_quest_log_tests.rs`; the old large file only
registers that child.

CP scope resolution: the new library PG cases would otherwise skip in CI's
non-PG library job. CI-PG-LIB-1 (#2300) already runs the whole
`oteryn-gameplay-dispatch --lib` suite against PostgreSQL in the `rest` leg of
both the PR harness and the merge-group gate, so this task adds no workflow step.
No new test harness, duplicated registration, migration or capability beyond 16.

### Delivery validation and file-size record (2026-10-22)

This supersedes the historical blocked/validation sections above. Status remains
`in_progress` for CP review/publication; no commit, push, remote review or CI run.
Targeted D1466 checks supersede the obsolete workspace command in the audit.
All following cargo commands used the queued wrapper and `+1.95.0 --locked`:

| Package / target | Exact filter | Passed |
|---|---|---|
| `oteryn-gameplay-quests` | all | 34 |
| `oteryn-gameplay-dispatch` | `quest_` | 27 |
| `oteryn-gameplay-dispatch` | `capabilities` | 60 |
| `oteryn-gameplay-dispatch` | `weather_wire` | 4 |
| `oteryn-game-server --lib` | `quest_catalogue_boot_tests` | 8 |
| `oteryn-game-server --test character_authority_postgres` | `quest_state_postgres_cases` | 15 |
| `oteryn-game-server --test chest_use_postgres` | all | 441 |

- Clippy `--all-targets -- -D warnings`: PASS for all three touched packages.
- `cargo +1.95.0 fmt --all -- --check`: PASS;
  `monk_save::postgres_tests::quest_ -- --list` selects all ten new PG cases.
- Governance + 124 unit tests, repository policy, seven qualification-placement unit tests,
  13 duplicate-source guard tests and PR routing regressions: PASS. Broader policy
  canaries reached the unchanged Windows check but cannot finish without `pwsh`.
- Whole diff checked against D1456; all new fields/variants/callers searched in
  `apps/ crates/ tools/`; tests are mounted once, with no deleted/weakened tests.
- Files growing >200 lines: `quest_admission_postgres_tests.rs` (347) and
  `quest_chest_postgres_tests.rs` (213): L5 tests of the real composed dispatch.
- Reuse: state loader/catalogue, captured source registry, log projection/wire,
  quest/XP durability and revision slot, progression binding, chest MINT, starter
  grant, Content fact resolver, monk-save Harness/Dependencies/fresh Source/native
  generation. The existing weather connection helper was moved to
  `weather_wire_fixture.rs` and shared; its body was not copied or remounted.
- Next: CP independent review and publication; no migration or new owner decision.

### Integrator validation repair (2026-10-22)

Baseline: integrator commit `0262148d22f90a9fd325102cb6228389cc3f9ac4`.
Status remains `in_progress` for CP review/publication of this uncommitted repair.

Layout planned before coding: repair the existing revision-sequencer fixture's
recovery-directory binding in its owner, with a separate
`gameplay_transport/quest_recovery_postgres_tests.rs` regression. Its existing
monk-save parent adds only the single module registration. No production module,
capability, durable table/column/function, migration or guard change is needed.

Root cause: all eight admission scenarios reuse the `quest_admission` tag. The
existing fixture gave each database a unique name, but called `fence_parent(tag)`;
that helper removes and recreates the retained directory. Concurrent fixtures
therefore deleted each other's recovery records while their stores still held
directory descriptors, producing `Unavailable` and occasional missing-file errors.
The owner now passes the already-unique database name to `fence_parent`. The new
regression creates two same-tag fixtures, opens both real recovery authorities,
cleans up one fixture and proves the other's authority remains usable. It reuses
the harness without copying or adding fixture constructors, helpers or SQL.

The pre-repair full dispatch run reproduced seven admission failures (905 passed,
7 failed, 14 existing ignored). The two originally reported unknown-COMMIT failures
did not recur in that run or the repaired full run. A first parallel
character-authority `quest` run separately returned two unknown-COMMIT errors;
without any further semantic edit or deadline change, both its serial and default
parallel reruns passed all 40 tests. No test was weakened, ignored or removed.

All cargo commands below used `/home/mole/oteryn-cache/codex-bin/cargo +1.95.0`.
These are the exact test arguments (no name filter unless shown):

| Arguments after `test --locked` | Result |
|---|---|
| `-p oteryn-gameplay-dispatch --lib --quiet` | 913 passed, 14 existing ignored |
| `-p oteryn-game-server --lib --quiet` | 306 passed, 3 existing ignored |
| `-p oteryn-gameplay-quests --quiet` | 34 passed; doc tests also pass |
| `-p oteryn-game-server --test chest_use_postgres --quiet` | 441 passed |
| `-p oteryn-game-server --test character_authority_postgres quest --quiet` | 40 passed |
| `-p oteryn-game-server --test character_authority_postgres quest --quiet -- --test-threads=1` | 40 passed |
| `-p oteryn-game-server --test durability_postgres --quiet` | 527 passed, including process-replacement coverage |
| `-p oteryn-gameplay-dispatch --lib monk_save::postgres_tests::quest_recovery --quiet -- --nocapture` | 1 passed on final wiring, 8.55 s |

PostgreSQL tests actually executed through the configured wrapper, including the
reused harness's PostgreSQL 17.6 assertion; they did not early-return for missing
configuration. The final uncaptured regression has no noncanonical/skip message.
The new regression is mounted once and runs in CI through CI-PG-LIB-1's
`oteryn-gameplay-dispatch --lib` PostgreSQL step.

- Final `clippy --locked -p oteryn-gameplay-dispatch -p oteryn-game-server -p
  oteryn-gameplay-quests --all-targets --quiet -- -D warnings`: PASS.
- Final `fmt --all -- --check`: PASS. Only module registration ordering needed repair.
- Whole implementation and repair diff reread against
  `~/oteryn-cache/CODEX-CHECKLIST.md`; no remaining hits. Workspace constructor,
  match, implementation and production-caller sites searched in `apps/ crates/ tools/`.
  This repair adds no enum variant, struct field, trait method or public function.
- Reuse: `Harness::create`/`cleanup`, existing unique `Database::name`,
  `fence_parent`, `run`, `debug`, `CharacterRecoveryStore::seal_current` and
  `DurabilityRoot::open_character_authority`. No duplicate test registration.
- File sizes: fixture 1,241 lines (net +1), monk-save parent 1,375 (net +2), new
  regression 31. No file grows by more than 200 lines in this repair; the task
  record adds fewer than 150 lines. Earlier large delivery files remain unchanged.
- Repository policy and all 13 duplicate-source guard unit tests: PASS.
- Next: CP review/publication. No remaining implementation or Rust validation;
  no Git mutation, credential access, external write or migration was performed.
