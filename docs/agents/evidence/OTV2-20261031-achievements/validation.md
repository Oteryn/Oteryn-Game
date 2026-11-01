# P11 / P12 validation record

This record distinguishes executed checks from planned coverage. The PR is
[#2782](https://github.com/Oteryn/Oteryn-Game/pull/2782), on
`chatgpt/achievements`, based on `ae4188a9c8d461acb152044814ea23075de02ed4`.
The final published commit is recorded in the PR, rather than embedded as a
self-referential commit identifier here.

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

## Executed small checks

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

## Final combined checks

Final package, content, PostgreSQL, migration, number and documentation results
are still being collected. This section must be completed before publication
of the finished implementation; no pending run is claimed as a pass.
