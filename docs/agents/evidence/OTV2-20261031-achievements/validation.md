# P11 / P12 validation record

This record distinguishes executed checks from planned coverage. The PR is
[#2782](https://github.com/Oteryn/Oteryn-Game/pull/2782), on
`chatgpt/achievements`, based on `ae4188a9c8d461acb152044814ea23075de02ed4`.
The final published commit is recorded in the PR, rather than embedded as a
self-referential commit identifier here.

The recovered candidate tested in `resume-11` is exactly
`28ac858ceecaa83cb383f19b584822213682e8bb`, as recorded in
`resume-11/candidate.sha`. That run started at `2026-11-01T08:41:50Z` and
finished at `2026-11-01T09:08:32Z`. Later `repair-12`,
`fishing-repair-12`, `fishing-diagnostic-13` and `repair-14` checks are explicitly
identified as checks of repairs on that base, not checks of a subsequently
authored or published commit.
The final run against the published commit follows the authoring commit and
will be identified with its exact SHA in the PR. Earlier results are not
relabeled as results for that later SHA.

Log names below are relative to the isolated workstation evidence directory
`/home/mole/chatgpt-achievements-logs/`. Each completed recovery result cited
here has a final `.log` and `.exit`; interrupted, blocked, filtered-out and
unconfigured cases are distinguished explicitly.

## Environment and boundaries

- Rust/Cargo 1.99.1, with the repository's locked dependencies.
- An isolated Ubuntu checkout and build target on the authorized workstation;
  all builds use the workstation's slot runner and a task-specific serial lock.
- A dedicated PostgreSQL 18.6 instance, separate from the user's application
  databases. Real database cases set `OTERYN_TEST_POSTGRES_ADMIN_URL` and
  `OTERYN_REQUIRE_PG=1`; connection details are deliberately not retained here.
- Ordinary package unit/doc runs unset both variables. Their successful
  unconfigured mounted cases are not counted as PostgreSQL execution.
- No full-workspace, heavy PostgreSQL partition, client UI or E2E run is
  represented as having passed. The Q7 client migration remains outside scope.

## Behavioral RED to GREEN evidence

| Case | Actual RED | Actual GREEN | Command / evidence |
|---|---|---|---|
| Earned starter addon contributes one canonical Topmodel identity | 0 passed / 1 failed; actual 0, expected 1 | 1 passed / 0 failed; same test and 552 filtered | `cargo test --locked --offline -p oteryn-gameplay-durability --lib topmodel_counts_an_earned_addon_on_an_available_starter_once -j 3 -- --nocapture --test-threads=1`; `red-topmodel-4` / `green-topmodel-6` |
| Four condition potions use the real framed item-use command | 0 passed / 4 failed; `NothingToUse` instead of `Committed` with only the final production dispatch arm disconnected | 4 passed / 0 failed; four distinct `Q6 POSTGRES_CASE_COMPLETED` markers | `cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib condition_potions_postgres_ -j 3 -- --nocapture --test-threads=1`; `red-potions-5` / `green-potions-7` |
| Q9 shared attribute slot in the existing condition owner | 0 passed / 3 failed: effects coexisted, shorter Swift Foot applied, and the zero slot increased the count | 3 passed / 0 failed, with the same 344 other tests filtered | `cargo test --locked --offline -p oteryn-gameplay-foundation --lib condition_attributes_zero_slot_ -j 3 -- --nocapture --test-threads=1`; `red-slot-6` / `green-slot-7` |
| Q10 exhaustive condition inventory after the shared-slot change | Historical `units-8.log`: full foundation library 346 passed / 1 failed; `rl_01_admits_every_exhaustive_key_at_once_and_refuses_past_32` expected `Ok(false)` but received `Ok(true)` | `resume-11/foundation.log`: full foundation library 347 passed / 0 failed, followed by 31 passing doctests (2 + 29); exit 0 | Q10=A [answer](https://github.com/Oteryn/Oteryn-Game/pull/2782#issuecomment-6548355109); candidate `28ac858ceecaa83cb383f19b584822213682e8bb`; command below |

The potion GREEN scenarios assert the live bonus and exact expiry, item burn,
98-to-99 progress, exact replay, vocation/current-session refusal, definite
rollback, lost COMMIT acknowledgement, reconciliation at the original expiry,
the hundredth-use award once, and no Potion Addict increment. They use the
production content decoder, actual actor/player reservations and database
writer. The Q9 tests also retain haste/nonzero siblings and cover the shared
dispel, suppression and Cleanse-immunity identity.

Earlier compiler errors are not RED behavioral evidence. They were repaired
before the actual failing assertions above were observed. Likewise, three
initial filename-based notice/logout filters selected zero tests; they do not
constitute regression passes. The complete package run and valid function
filters are the applicable evidence.

## Historical authoring checks

- `python3 tools/content-migration/test_condition_potion_profiles.py`: **pass**,
  6 test methods, including identity, independent binding, physical-policy and
  source-provenance mutation refusals.
- `python3 tools/content-migration/test_gathering_item_profiles.py`: **pass**,
  3 test methods, including mechanical-rod qualification and drift refusals.
- `python3 tools/content-migration/condition_potion_profiles.py --canary
  /workspace/scratch/3518815cbb43/content-predecessor-sources/canary --crystal
  /workspace/scratch/1c2a6188431a/crystalserver --check`: **pass**, exit 0.
- `python3 tools/content-schema/native-gameplay/build_gathering_item_profiles.py
  /workspace/scratch/1c2a6188431a/crystalserver --check`: **pass**, exit 0.
  The original 28 gathering records retain their semantic digest. These paths
  identify read-only local copies of the exact pinned source inputs, not a
  dependency on another task's generated data.
- `python3 tools/agents/validate_governance.py`: **pass**, 22 required policy
  documents and 9 project lanes at the authoring checkpoint.
- `python3 -m unittest discover -s tools/agents/tests`: **pass**, 129 tests at
  the same authoring checkpoint.
- `cargo fmt --all -- --check`: **pass** on the integrated runtime candidate
  (`fmt-8`); final evidence follows below.
- `git diff --check`: **pass** at each integration checkpoint.

## Recovered candidate: ordinary package checks

`resume-11/units.exit` and `resume-11/foundation.exit` are both 0. The ordinary
runs unset `OTERYN_TEST_POSTGRES_ADMIN_URL` and `OTERYN_REQUIRE_PG`.
A mounted PostgreSQL test that returns early in this configuration can be
reported as `ok` by Rust's harness; that is not evidence that it used a database.
The library counts below therefore describe package-test results, not counts
of real PostgreSQL executions.

| Package | Passed | Failed | Ignored | Evidence |
|---|---:|---:|---:|---|
| `oteryn-gameplay-content` | 285 | 0 | 1 | `resume-11/units.log` |
| `oteryn-gameplay-dispatch` | 1732 | 0 | 14 | `resume-11/units.log` |
| `oteryn-gameplay-durability` | 553 | 0 | 0 | `resume-11/units.log` |
| `oteryn-gameplay-progression` | 112 | 0 | 0 | `resume-11/units.log` |
| `oteryn-protocol-oteryn` | 598 | 0 | 0 | `resume-11/units.log` |
| `oteryn-gameplay-foundation` | 347 | 0 | 0 | `resume-11/foundation.log` |

`units.log` also contains 10 passing tests in the subsequent groups
(6 + 1 + 3) and three groups selecting zero tests. The foundation run contains
31 passing doctests (2 + 29). Zero-test groups are not coverage evidence.

Recorded commands:

```sh
cargo test --locked --offline --quiet --all-features --no-fail-fast \
  -p oteryn-protocol-oteryn -p oteryn-gameplay-content \
  -p oteryn-gameplay-progression -p oteryn-gameplay-durability \
  -p oteryn-gameplay-dispatch -- --test-threads=3
cargo test --locked --offline --quiet --all-features \
  -p oteryn-gameplay-foundation -- --test-threads=3
```

Q10 changes only the approved expected result in the fourth foundation file.
All 29 input definitions are retained. `SpellSkills(0)` alone must report
replacement, leaving 28 slots, no typed `Attributes` entry and a present
`SpellSkills(0)` entry. The 32/33 limit assertions and production implementation
remain unchanged. The historical `units-8` failure is retained above; it is
not hidden behind the later successful run.

The same historical `units-8.log` had two dispatch assertion failures:
the newly used condition-potion fixture was absent from the explicit bundle
seam inventory, and the old profile-count assertion expected 814 instead of
819. That run had 1730 passing dispatch tests, two failures and 14 ignored.
The recovered candidate includes the bounded fixture-inventory update and the
819-profile expectation; its complete dispatch run has 1732 passes, zero
failures and the same 14 ignored tests.

## Recovered candidate: executed PostgreSQL cases

These commands ran against the dedicated PostgreSQL instance with
`OTERYN_TEST_POSTGRES_ADMIN_URL` and `OTERYN_REQUIRE_PG=1`. Their logs record
the selected cases and, where provided by the fixture, `POSTGRES_STARTED`,
`POSTGRES_ASSERTED`, or `POSTGRES_CASE_COMPLETED` markers. They are separate
from the unconfigured package runs above.

| Filter / gate | Actual result on candidate `28ac858…` | Evidence under `resume-11/` |
|---|---|---|
| `p11_` | 10 passed / 1 failed; five pure notice/observation tests passed, five PostgreSQL cases passed, mechanical-rod PostgreSQL case failed; exit 101 | `pg-p11_.log` / `.exit` |
| `standard_fishing -- --skip p11_` | 8 PostgreSQL cases passed, including ordinary-rod 7158, gathering mutations/refusals and standard-fishing replay; exit 0 | `pg-standard_fishing.log` / `.exit` |
| `rust_lottery` | 4 PostgreSQL cases passed: rust success/break and lottery win/loss, with replay; exit 0 | `pg-rust_lottery.log` / `.exit` |
| `npc_paid_travel_postgres_dispatch_atomic_payment_arrival_replay_and_refusals` | 1 PostgreSQL case passed; actual-arrival, fence and saturation marker present; exit 0 | `pg-npc_paid_travel_postgres_dispatch_atomic_payment_arrival_replay_and_refusals.log` / `.exit` |
| `condition_potions_postgres_` | 4 PostgreSQL cases passed, with four distinct Q6 completion markers; exit 0 | `pg-condition_potions_postgres_.log` / `.exit` |
| `character_title` | 5 PostgreSQL cases passed, including source refresh, completion and cosmetic dispatch; exit 0 | `pg-character_title.log` / `.exit` |
| `character_info_postgres_real_dispatch` | 1 PostgreSQL case passed; exit 0 | `pg-character_info_postgres_real_dispatch.log` / `.exit` |
| `p12c_cyclopedia_combat_postgres_dispatch_snapshot_replay_and_fences` | 1 PostgreSQL case passed, with completion marker; exit 0 | `pg-p12c_cyclopedia_combat_postgres_dispatch_snapshot_replay_and_fences.log` / `.exit` |
| `cyclopedia_badges_postgres_real_dispatch` | 1 PostgreSQL case passed; exit 0 | `pg-cyclopedia_badges_postgres_real_dispatch.log` / `.exit` |
| `cyclopedia_character_tabs_postgres_real_dispatch` | 1 PostgreSQL case passed; exit 0 | `pg-cyclopedia_character_tabs_postgres_real_dispatch.log` / `.exit` |
| Fresh embedded game migration ledger | 1 PostgreSQL case passed; exit 0 | `pg-migration-ledger.log` / `.exit` |
| Runtime executes every CHECK function | 1 PostgreSQL case passed; exit 0 | `pg-check-privileges.log` / `.exit` |
| Runtime cannot change items without a use record | 1 PostgreSQL case passed; exit 0 | `pg-item-use-guard.log` / `.exit` |

The five successful PostgreSQL cases in the mixed `p11_` filter are the
account-notice commit/replay/session-fence case, failed dusting without an
award, common ice fish and the second/third rare ice species. The mechanical
rod failure is not counted as a pass. These baseline migration and privilege
results predate migration 0704, which is introduced by the repair below.

### Mechanical-rod failure and bounded repair

`resume-11/pg-p11_.log` failed the existing
`p11_mechanical_ice_fishing_grants_exquisite_taste` scenario with
`unexpected end of file`. The isolated diagnostic rerun in
`fishing-repair-12/mechanical-red.log` reproduced a behavioral RED:
0 passed / 1 failed, 1745 filtered, 13.57 s, exit 101.
The added diagnostic context located it in the first exchange while waiting
for the command result, with `craft_records=Some(0)`.

The source cause is `0549_gathering_craft.sql:23-34`: the existing gathering
admission permits a bait-consuming receipt only for rod 3483 and bait 3492.
The mechanical rod 9306 is already a native ice-hole rule but fails that SQL
guard. The caller converts the durability error to no result, explaining the
EOF before a committed receipt. Migration 0704 extends only that rod clause
to 3483/9306 while preserving bait, terrain-receipt and other admission proofs.
The temporary diagnostic changes were reversed before the repair run; they
are not part of the implementation.

The first repair attempt also completed with a failure:
`fishing-repair-12/fishing-green.exit` is 101 despite the filename.
Its complete `fishing-green.log` reports 12 passed / 1 failed,
1733 filtered, 183.65 s. The mechanical-rod case still ends with
`unexpected end of file`; the other 12 scenarios have completion markers.
This is not GREEN evidence. The command selected all 13
`standard_fishing` scenarios:

```sh
cargo test --locked --offline -p oteryn-gameplay-dispatch \
  --all-features --lib standard_fishing -- --nocapture --test-threads=1
```

The completed follow-up `fishing-diagnostic-13/test.log` and `test.exit`
record another 0 passed / 1 failed, 1745 filtered, 16.82 s, exit 101.
This run inspected the actual fixture database and reported
`installed-0704-admits-mechanical=false`, followed by
`durable-craft-result=Err(Database(Database(None)))` and
`first-exchange craft_records=Some(0)`. Thus the first repair attempt did not
establish that the new SQL admission was active. The temporary diagnostic
patch was fully removed after this run.

The fixture calls `oteryn_gameplay_durability::GAME_MIGRATOR`, embedded in
`crates/gameplay/durability/src/lib.rs:7-8`. The earlier repair log rebuilt
dispatch but did not rebuild that durability package after the new migration
file was added. The existing `migration_numbers_are_unique_and_match_embedded_migrator`
test compares migration numbers on disk with that embedded migrator.
The first `repair-14` inventory command itself compiled a fresh durability
variant and passed: `embedded-before-refresh.log` / `.exit` report 1 passed,
0 failed, 552 filtered and exit 0. This was not a RED inventory test and does
not overturn the earlier fixture-database observation from diagnostic-13.
The runner then refreshed only durability artifacts in the explicit task
target directory (`refresh-own-durability-cache.log` / `.exit`, exit 0),
removing the earlier consumer variants. The same existing inventory test
passed after recompilation as well: `embedded-after-refresh.log` / `.exit`,
1 passed, 0 failed, 552 filtered, exit 0. Both inventory results are PASS.
The actual database diagnostic supplies the evidence of the earlier missing
admission.

The same existing mechanical-rod scenario then completed **GREEN**:
`repair-14/mechanical-green.log` and `mechanical-green.exit` report
1 passed, 0 failed, 1745 filtered, 22.83 s, exit 0. The log shows durability
and dispatch recompilation and the completion marker for tool 9306,
target 7236, spent state 7237, yield 7158 and
`oteryn:achievement/exquisite_taste`. This executed PostgreSQL scenario closes
the behavioral RED recorded above; its unchanged assertions include receipt
replay, one bait debit/yield and unchanged achievement progress on replay.

The separate sibling and schema runs also completed successfully:

| Repair-14 gate | Executed result | Evidence under `repair-14/` |
|---|---|---|
| Remaining fishing scenarios, excluding the mechanical case | 12 passed, 0 failed, 1734 filtered, 163.23 s; all completion markers present; exit 0 | `remaining-fishing-green.log` / `.exit` |
| Fresh embedded migration ledger | 1 passed, 0 failed, 156 filtered, 2.12 s; exit 0 | `migration-ledger.log` / `.exit` |
| Runtime executes every CHECK function | 1 passed, 0 failed, 614 filtered, 2.13 s; exit 0 | `check-privileges.log` / `.exit` |
| Runtime cannot mutate items without a use record | 1 passed, 0 failed, 614 filtered, 2.14 s; exit 0 | `item-use-guard.log` / `.exit` |

Together with the single mechanical case, this repair run executed 13 fishing
PostgreSQL cases and three schema/privilege PostgreSQL cases successfully.
These cases overlap the earlier filters; counts are not added across runs or
presented as the final full 35-case batch for a later commit.

## Quality, source and governance checks

| Check | Executed result | Evidence |
|---|---|---|
| Formatting | PASS, exit 0 on candidate 28ac858…; PASS again after the repair, exit 0 | `resume-11/fmt.log` / `.exit`; `repair-14/fmt.log` / `.exit` |
| Whitespace / patch consistency | PASS, exit 0 on candidate 28ac858…; PASS again after the repair, exit 0 | `resume-11/diff-check.log` / `.exit`; `repair-14/diff-check.log` / `.exit` |
| Initial full selected-target Clippy | FAIL, exit 101: seven `expect_used`, one `unwrap_used`, one `needless_question_mark` | `resume-11/clippy.log` / `.exit` |
| Clippy after the three-file repair | PASS, exit 0; only three existing vendored dependency warnings remain | `repair-12/clippy.log` / `.exit`, `base.sha`, `source.patch` |
| Python condition-profile checks | PASS, 6 tests, exit 0 | `resume-11/python-condition-profiles.log` / `.exit` |
| Python gathering-profile checks | PASS, 3 tests, exit 0 | `resume-11/python-gathering-profiles.log` / `.exit` |
| Content integration tests | PASS, 123 tests, 152 filtered, exit 0 | `resume-11/content-integration.log` / `.exit` |
| World pin check | PASS, exit 0; world digest unchanged | `resume-11/pin-check.log` / `.exit` |
| Governance validation | PASS, 22 required policy documents and 9 lanes, exit 0 | `resume-11/governance.log` / `.exit` |
| Governance tests | PASS, 129 tests, exit 0 | `resume-11/governance-tests.log` / `.exit` |
| Initial isolated Python dependency installation | FAIL, exit 1: `/usr/bin/python3: No module named pip` | `resume-11/python-dependencies.log` / `.exit` |
| Repaired isolated Python dependency installation | PASS, exit 0; required packages installed into the task-specific dependency directory | `repair-12/python-dependencies.log` / `.exit` / `.command` |
| Imports using the repaired Python dependency directory | PASS, exit 0; empty diagnostic log | `repair-14/python-import.log` / `.exit` |
| Complete content validation gate | PASS, exit 0; logged import/schema/census checks and final 5 Cargo tests completed | `repair-14/content-validate.log` / `.exit` |

The Clippy repair changes error propagation and an explicitly known fixture
fact in three existing test files. It adds no lint allowance and removes no
behavioral assertion. Its recorded base is 28ac858… plus the captured
`repair-12/source.patch`, not a claim about a later publication commit.
The selected Clippy gate uses `--all-targets --all-features`, `--locked
--offline`, `-- -D warnings`, and packages `oteryn-protocol-oteryn`,
`oteryn-gameplay-content`, `oteryn-gameplay-progression`,
`oteryn-gameplay-foundation`, `oteryn-gameplay-durability`,
`oteryn-gameplay-dispatch` and `oteryn-game-server`.

The content integration command was
`cargo test --locked --offline -p oteryn-game-server --test integration_rest
content_ --quiet -- --test-threads=1`.
The pin check recorded world digest
`3bb1f0f22f3327f95e9e46ca513944c3aeba3d76068e68dc19d5e3b5c5c4997c`
and inputs digest
`7d62d10118147f849adc10bd426a485b8cf6e51938c866acf2ac545722777cd0`.

### Full content validation after dependency repair

`resume-11/content-validate.blocked` explicitly says
`isolated Python dependency install did not pass`. The full schema/content
validator was blocked at that stage; the successful profile unit tests,
123 content integration tests and world pin check do not replace it.

The subsequent full gate completed using the repaired dependency directory:
`repair-14/content-validate.exit` is **0**, and the complete 177-line
`repair-14/content-validate.log` records the executed checks on the repaired
working tree based on 28ac858…. The driver invocation is retained beside it
as `repair-14/content-validate.command`. Logged steps include:

- `python3 tools/content-migration/imbuement_catalogue.py --check` and
  `register_spell_families.py --check`: the registered spell check reports
  38 files and `activation=false`; its importer regression group passes
  9 tests.
- `python3 tools/content-migration/validate_world_project_v2_to_tree.py` and
  the associated test suite: content validation passes for 34043 item records,
  252 mounts, 28758 creature records, 1282 NPC records, 389 services,
  836 dialogues, 108 encounters, 25 charms, 443 proficiencies and
  336 reward claims. The world-project test group passes 12 tests; later
  component regression groups also finish with `OK` in the same log.
- `python3 tools/content-schema/validate_materialized_game_tree.py`:
  `PASS materialized=97/97 world_markers=10 world_successor_files=1257`.
- `python3 tools/content-census/item_key_references.py`: PASS over 244 files,
  5 history files and 287716 references.
- Craft, imbuement, charm, proficiency, reward-claim and starter-kit
  authoring checks complete; the log also records `PASS 599 checks` and
  the weapon-proficiency source check.
- `python3 tools/content-census/g4_item_crystal_binding_generator.py --check`:
  PASS with 33971 bindings.
- `cargo test --locked -q -p oteryn-game-server --test content_world_project_repository`:
  5 passed, 0 failed, 0 ignored, 0 filtered, 33.77 s.

The validator explicitly reports that the starter-kit historical seal check
has no resolvable base revision and is skipped in its smoke-check path.
The starter-kit content and smoke checks pass. This record therefore does
not claim that a historical seal comparison executed. Static import checks
reporting runtime admission blocked or `activation=false` retain those
declared boundaries; their PASS status is not a claim of runtime activation.

The dependency-install success is recorded separately from this completed
validation. `repair-14/runner.exit` is **0**, and its final formatting and
diff checks both have exit 0 with empty logs. This closes the bounded repair
run before the authoring commit; it does not substitute for the subsequent
run on the final published SHA.

## Number audit and publication identity

`numbers-resume-11.ref` records the fresh audited main commit
`1f7f76c6bc7c5014f9aa553d94a493b1d1544e19`.
The global audit exited 1 and reported existing simultaneous claims by
#2788 and #2764 for migrations 620–623 and offer order 560.
The task's reserved migration block 0704–0707 is not among those reported
clashes. The global audit is recorded as a failure with that exact scope,
not relabeled as a global pass.

`resume-11/runner.exit` and `resume-11/checks.exit` are 1 because that run
contained the Clippy failure, the mechanical-rod failure and the unavailable
Python bootstrap. Later repairs are recorded individually above.
A final run after the authoring commit, together with its exact tested and
published SHA, belongs in PR #2782. This record does not claim that a
pre-commit repair run validates an uncreated or subsequently changed commit.
