# questpc4 validation

Continuation: 2026-11-06. Product/tool paths are listed in CHANGED-FILES.txt. Mounted Rust modules now use mechanic names; TEST-RENAMES.json maps the historical names retained in older logs. No test was removed, skipped or weakened.

## PostgreSQL qualification

All commands used the absolute queued wrapper, CARGO_BUILD_JOBS=4 and explicit -j4, one cargo command at a time. Cases created their own databases. Every qualified cohort emitted POSTGRES_CASE_COMPLETED with duration; no absent-URL return was counted.

| Run | Result | Log |
|---|---|---|
| Five simple reward cohorts | 5 passed, 0 failed; 297.75s | logs/continuation-simple-rewards.log |
| Next five reward cohorts | 5 passed, 0 failed; 404.41s | logs/continuation-next-rewards.log |
| Last two rewards and Waterfall access | 3 passed, 0 failed; 214.64s | logs/continuation-final-rewards-access.log |
| Troll Cave after teardown repair | 1 passed, 0 failed; 88.20s | logs/continuation-troll-cleanup.log |

Together the first three commands qualify 12 reward cohorts and 26 source placements, plus Waterfall's three STEP destinations. Waterfall verifies non-empty durable control tracks and receipts are unchanged. Troll Cave additionally proves alternate-box refusal and replay in both source orientations. BATCH-01.md records each named cohort and duration.

The final three-line test repair replaces an unconditional Ok return with release of the authority/seal and the existing Harness::cleanup. The first three runs predate this teardown-only repair; production content and every reward/access assertion are identical. The focused Troll Cave rerun passed both placements and cleanup before its cohort marker. No historical/shared databases were enumerated or removed.

Exact executed PostgreSQL commands:

```sh
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --quiet -j 4 -p oteryn-gameplay-dispatch --lib -- throwing_star_rewards time_ring_rewards troll_cave_rewards waterfall_rewards wedding_ring_rewards --nocapture --test-threads=2
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --quiet -j 4 -p oteryn-gameplay-dispatch --lib -- thais_lighthouse_rewards triangle_tower_rewards vampire_shield_rewards vampire_hunter_rewards tomes_of_knowledge_rewards --nocapture --test-threads=2
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --quiet -j 4 -p oteryn-gameplay-dispatch --lib -- to_blind_the_enemy_rewards to_outfox_a_fox_rewards waterfall_access_steps_preserve_quest_state --nocapture --test-threads=2
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --quiet -j 4 -p oteryn-gameplay-dispatch --lib troll_cave_rewards -- --nocapture --test-threads=1
```

## Fresh Rust unit/content checks

```sh
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo check --locked --quiet -p oteryn-gameplay-dispatch
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo clippy --locked --quiet -j 4 -p oteryn-gameplay-dispatch -p oteryn-gameplay-quests -p oteryn-game-server --all-targets --all-features -- -D warnings
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --quiet -j 4 -p oteryn-gameplay-quests --lib world_callbacks
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --quiet -j 4 -p oteryn-gameplay-content --lib world_reward_claims
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --quiet -j 4 -p oteryn-game-server --test integration_rest world_reward_claims
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo fmt --all --check
```

- Recovery dispatch check passed before further code work (continuation-check.log).
- Final clippy passed after the teardown repair (continuation-clippy-final.log).
- Quest world_callbacks: 11 passed, 0 failed, 3.18s (continuation-quest-rust-unit.log).
- Content world_reward_claims: 6 passed, 0 failed, 0.11s (continuation-content-rust-unit.log).
- Game-server integration_rest world_reward_claims: 12 passed, 0 failed, 0.18s (continuation-integration-rewards.log), including the strict 312-candidate/eight-held-plain binder count.
- Final cargo fmt passed (continuation-fmt-final.log, exit 0 after the shared-PC load queue). Explicit rustfmt --edition 2024 --check already passes for all three mounted changed Rust files.
- git diff --check passes for apps/, crates/, tools/ and content/. Raw wiki reference scratch retains upstream whitespace.

## Python/source checks and repaired failure

- Reward authoring: 25 focused tests plus 12 core/container tests passed (continuation-reward-unit.log, continuation-reward-core-unit.log).
- Batch-four and SOURCE-readiness: 15 passed (continuation-quest-unit.log).
- Native-provider module: 13 exercised, with 12 passing and one exact-count failure, 131 != 130 (continuation-native-unit.log). HEAD/current recipe sets prove the only added Item is Garlic Necklace i3083 from Triangle Tower. The test now asserts exactly 131 plus the necklace explicitly; all per-Item identity/custody/weight/stack assertions remain. Its isolated rerun passed, one test in 4.103s (continuation-native-count-green.log). The full module was not rerun after this test-only correction.
- Source capture and reward fixture generation both pass --check against retained Crystal201189f inputs, 12 source cohorts.
- D1462 review read both checklists and all handwritten source changes. CONTINUATION-AUDIT.json independently confirms exactly three Waterfall callbacks were added, none changed/removed, and production QuestState changes only authoring_sources. Constructor/impl/variant searches across apps/, crates/ and tools/ found no production enum/field/trait additions or orphan production pub fn. Shared repairs are in SHARED-FIXES.md.

## Earlier unchanged-content evidence

The preceding checkpoint's final ordered whole Quest checks, full offline donor replay, native-provider 41-file --check and world pin-check passed (restart-quest-checks-ordered.log, restart-donor-check-final.log, native-check-final.log, restart-pin-check.log). The 146 pinned Quest-tree inputs remained unchanged after final donor regeneration. This continuation changed no production content or authoring producer, so those expensive unchanged-input checks were not repeated. Earlier RED evidence remains in charge-red.log, embedded-claims-red.log and waterfall-access-red.log.

## Unfinished

Full quest completion is not claimed. Triangle Tower needs its asymmetric entrance lever; Thais Lighthouse needs pressure-plate step-out and paired portals; Annihilator needs its party/encounter access path; Tinder Box/Torch/Voodoo Doll/Waterskin need native delivery/placement and their recorded access/repeat semantics. Waterfall facing/splash is unqualified. The owner's coordination note says to mark missing engine features BLOCKED rather than add them. NPC exchanges, storylines and outfits in phases B/C remain unfinished. See PHASE-A.md and the named BLOCKED records in RESUME.md. No migration, command/capability number, commit, push, candidate freeze, PR or external review was created.

## Recovery expansion: four source claim cohorts
Initial real-dispatch RED:4 failed/0 passed in96.50s (logs/recovery-four-rewards.log). Missing native profiles caused Ape City, White Raven and Threatened Dreams to be unbound; the shared fixture omitted Shattered Isles existing quest catalogue/session. Source-backed admission adds exactly i3205,i4827,i4831,i25235,i25729 through the existing native provider generator, with no canonical Item/claim mutation. The fixture now mounts the existing quest catalogue only for a linked claim and asserts one quest receipt, one consumed obligation and no pending obligation after both warm and reconstructed-owner execution. GREEN:4 passed/0 failed,163.59s (logs/recovery-four-rewards-green.log), each with completion/duration markers. Native registration/check passed41 files; all13 native profile tests and21 focused reward/batch-four/SOURCE tests passed. Clippy is pending.
```sh
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --quiet -j 4 -p oteryn-gameplay-dispatch --lib -- the_ape_city_rewards the_shattered_isles_rewards the_white_raven_monastery_rewards threatened_dreams_rewards --nocapture --test-threads=1
```

## Recovery second reward batch
Initial run:3 passed/2 failed,588.50s (recovery-five-rewards-red.log). New Frontier (5 placements), Outlaw Camp (4 numbered keys) and Paradox Tower (4 rewards/key3899) passed with duration markers. The expected native-admission repair adds exactly i9172 (Beregar prison-cell key) and i112 (Foolish magical watch); existing i8895 gains only its canonical marketability=true fact in previously UNKNOWN trade_restrictions. Other profiles are unchanged. Isolated repair run:2 passed/0 failed,162.56s (recovery-second-repair-green.log), including all6 Beregar and2 Foolish placements; markers162.555661458s/54.845499787s. The full63 reward-authoring tests and native registration/check passed. Batch clippy is running before qualification.
```sh
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --quiet -j 4 -p oteryn-gameplay-dispatch --lib -- the_hidden_city_of_beregar_rewards the_new_frontier_rewards the_outlaw_camp_rewards the_paradox_tower_rewards what_a_foolish_rewards --nocapture --test-threads=1
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --quiet -j 4 -p oteryn-gameplay-dispatch --lib -- the_hidden_city_of_beregar_rewards what_a_foolish_rewards --nocapture --test-threads=2
```
Second-batch completion: all13 native-profile tests passed (44.027s); all-target/all-feature clippy passed for dispatch/quests/game-server with-D warnings in recovery-second-clippy.log. The five qualified cohorts are checkpointed in RESUME.md.

## Recovery authored books
Ice Islands passed2 placements,60.324657138s, including exact255-character authored text, revision1/no player writer, complete5-child bag, stale fence and warm/reconstructed-owner replay (recovery-books-green.log). That initial command had1 passed/1 failed: Inquisition UID6270 lacked native i7874 after its source book passed. The generator added only i7874, without modifying an existing profile; its41-file --check and all13 profile tests passed (57.110s). Focused Inquisition GREEN passed2 placements,59.322988829s (recovery-inquisition-green.log), including exact2042-character text. Final clippy pending.
```sh
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --quiet -j 4 -p oteryn-gameplay-dispatch --lib -- the_ice_islands_rewards the_inquisition_rewards --nocapture --test-threads=2
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --quiet -j 4 -p oteryn-gameplay-dispatch --lib the_inquisition_rewards -- --nocapture --test-threads=1
```
Fresh capture, canonical RewardClaim content check, batch-four22-packet check, source fixture23-cohort check and product/tool diff whitespace check passed. All146 pinned Quest-tree inputs match the retained exact snapshot; unchanged-input donor/Quest consistency results are reused rather than re-run. New generated native delta vs HEAD is12 profiles added, none removed, with only i3287/i3372/i8895 existing profiles modified; recovery itself adds8 of those12 profiles. RECOVERY-SEMANTIC-AUDIT.json records this exact scope.
Final clippy passed (recovery-final-clippy.log), dispatch+quests+game-server all targets/features with-D warnings. Fresh game-server integration_rest content_ / world_reward_claims passed135 tests/0 failed in1.50s (recovery-integration-content.log). Exact invocation:
```sh
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --quiet -j 4 -p oteryn-game-server --test integration_rest -- content_ world_reward_claims
```

Fresh final units: content world_reward_claims6/0,0.03s; Quest world_callbacks11/0,1.47s; World repository5/0,40.93s. Post-rename Python discovery preserves63 reward tests (45.588s) and15 SOURCE/batch-four tests (0.549s), all passing; module/class rename mappings are in TEST-RENAMES.json.
```sh
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --quiet -j 4 -p oteryn-gameplay-content --lib world_reward_claims
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --quiet -j 4 -p oteryn-gameplay-quests --lib world_callbacks
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --quiet -j 4 -p oteryn-game-server --test content_world_project_repository
```

Final gates passed: World pin-check (recovery-pin-check.log,exit0), cargo fmt --all --check (recovery-final-fmt.log,exit0), explicit mounted-file rustfmt, and product/tool git diff --check. The recovery is locally validated for the stated reward/access scope; full quest completion and publication readiness are not claimed. Exact consolidated coverage filters are source_quest_rewards and waterfall_access_steps_preserve_quest_state; actual subset runs and inherited unchanged-scope results are listed above.

## Shared reward choices (current continuation)

Initial dispatch recovery check passed before edits (phase-b-recovery-check.log). Initial PG RED:2 failures/0 passes,62.70s (claims-choice-initial.log), both missing native Item admission. Second RED found key3700/i2971 and a discarded mixed USE/STEP session cursor (claims-choice-green.log,0 passed/2 failed,296.77s). Focused native RED: KeyError i2971 (claims-key-profile-red.log). Exact Item admission and minimal fixture repairs are in SHARED-FIXES.md. A superseded PG run was cancelled exit130; a transient shared-helper signature edit failed compilation for3 legacy callers before test execution. Their interface was restored with a wrapper; these runs provide no gameplay qualification.

Final-code GREEN:3 passed/0 failed,349.13s (claims-choice-retained-pg.log). Pits of Inferno12 placements:349.127009281s; Thieves Guild7 placements:213.217412644s; existing Troll Cave2 placements:57.353195072s. All3 cohort completion markers follow successful fixture cleanup. Every orientation of each shared claim visits/refuses/replays every sibling via actual STEP/USE, asserts the actor's position, returns, and reconstructs the owner for durable replay. Exact key3700, source text, native children and audit assertions pass unchanged.

```sh
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline --quiet -p oteryn-gameplay-dispatch --lib -- the_pits_of_inferno_rewards the_thieves_guild_rewards troll_cave_rewards --nocapture --test-threads=2
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline --quiet -p oteryn-gameplay-dispatch -p oteryn-gameplay-quests -p oteryn-game-server --all-targets --all-features -- -D warnings
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline --quiet -p oteryn-gameplay-content --lib world_reward_claims
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline --quiet -p oteryn-game-server --test integration_rest -- content_ world_reward_claims
```

Fresh clippy passed for all3 packages/all targets/all features (claims-choice-clippy.log). Content6/0 failed,0.03s (claims-choice-content-unit.log); game-server content/binder135/0 failed,1.43s (claims-choice-integration.log). Python reward-authoring63/0 failed,61.133s; native-profile13/0 failed,75.218s after extending the existing canonical-admission test to all9 new profiles; SOURCE/batch-four15/0 failed,0.748s. Native41-file --check passed after the key repair. Exact25-cohort source fixture --check passed. All146 pinned Quest-tree inputs remain byte-for-byte unchanged, so the earlier ordered Quest/donor evidence is reused within that unchanged-input scope. CHOICE-REWARD-AUDIT.json and CHOICE-REWARD-REVIEW.md record the current semantic delta and whole-diff/caller review.


The inherited extra coverage regression was RED at308 manifest candidates vs312 actual binder candidates (claims-choice-source-coverage.log). Regenerated the report through its existing tool, registering exactly the4 already-qualified changed claims; its supported-unqualified297 denominator and ledger remain unchanged. GREEN: strict exact-set test1 passed/0 failed,0.14s (claims-choice-source-coverage-green.log); all7 qualification unit tests passed,0.254s. Generator --check and ledger --check both pass. No assertion or denominator was weakened.

Explorer Society: exact owner Q5=A re-read from PR2770 comment6542716086, keeping canonical12510 over donor3208 with an explicit cited deviation. All7 source placements enroll in the shared template. Initial PG RED0 passed/1 failed,124.90s, at absent native12510 (explorer-rewards-red.log). Added only native12510, no existing profile change. GREEN1 passed/0 failed,228.20s with completion marker after cleanup (explorer-rewards-green.log). The existing native-profile module passes all13 tests in82.068s, and the41-file native --check passes. Final dispatch+quests+game-server all-target/all-feature clippy passed after this final native addition (questpc4-final-clippy.log).

```sh
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline --quiet -p oteryn-gameplay-dispatch --lib the_explorer_society_rewards -- --nocapture --test-threads=1
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline --quiet -p oteryn-gameplay-dispatch --lib quest_callbacks_binding_and_coverage_require_exact_source_witnesses
```


Final sequential Rust gates all passed after native12510 and the coverage-report repair: dispatch/quests/game-server all-target/all-feature clippy with-D warnings; content world_reward_claims6/0 in0.04s plus Quest world_callbacks11/0 in1.28s (questpc4-final-units.log); game-server content/binder135/0 in1.67s (questpc4-final-integration.log); World repository5/0 in48.09s (questpc4-final-world-repository.log); strict source binding/coverage1/0 (questpc4-final-source-coverage.log); World pin-check exit0 (questpc4-final-pin-check.log); cargo fmt --all --check exit0 plus explicit rustfmt for all3 mounted files. No final code/content mutation followed these gates.

```sh
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline --quiet -p oteryn-gameplay-content -p oteryn-gameplay-quests --lib -- world_reward_claims world_callbacks
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline --quiet -p oteryn-game-server --test integration_rest -- content_ world_reward_claims
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline --quiet -p oteryn-game-server --test content_world_project_repository
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo run --locked --offline --quiet -p oteryn-world-bundle-compiler -- pin-check . .questpc4/tmp/final-questpc4-pin-check
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo fmt --all --check
```

Final source fixture26/84, coverage generator --check and unchanged297-key qualification ledger --check pass. All146 pinned Quest-tree inputs still match their earlier qualified snapshot. Product/tool diff whitespace passes. D1462 whole-diff/constructor/caller and semantic reviews are complete for this reward scope; no production API, migration, protocol/capability number or CI/protection change. No test was removed, skipped or weakened.

## Final-status restart and Thawing repair

Initial dispatch check: `CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo check --locked --offline --quiet -p oteryn-gameplay-dispatch`, exit0 (`logs/current-restart-check.log`).

Thawing boot RED: `cargo test --locked --offline --quiet -p oteryn-game-server --lib thawing_boot_installs_harvestable_source_placements -- --nocapture`,0/1; source tile(32255,31079,-5) had only ground432 rather than [432,13842]. GREEN same command1/0,0.56s (`logs/thawing-boot-green.log`).

Real projected-flower PG: `CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked --offline --quiet -p oteryn-gameplay-dispatch --lib w19_ice_flower_harvest_wire -- --nocapture --test-threads=1`,2/0,59.87s (`logs/thawing-projected-pg.log`). After-cleanup completion durations28.342751454s(no-seed) and31.522283517s(yield/ten-seed achievement/eleventh seed). Earlier static-fixture run was2/0,55.62s; it does not substitute for this final projected path. Raw PG fixture contains neither flower nor harvest palette states. Actual B3 source tiles independently decoded:14 non-House ground tiles (`THAWING-NATIVE-CENSUS.json`). Native14 placements/25% yield are preserved, with the wiki15/~20% discrepancy explicit (`THAWING-REVIEW.md`).

Clippy: `CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline --quiet -p oteryn-gameplay-world -p oteryn-gameplay-dispatch -p oteryn-gameplay-quests -p oteryn-game-server --all-targets --all-features -- -D warnings`, exit0 (`logs/current-final-clippy.log`).

Server daily regressions: `cargo test --locked --offline --quiet -p oteryn-game-server --lib -- thawing_boot_installs_harvestable_source_placements w05b_apply_items_without_a_bundle_places_nothing sp02_fungus_boot_changes_only_the_source_rectangle_and_rebuilds_the_same_day sp02_trader_boot_loads_each_ship_before_placing_yasir sp02_grimvale_boot_activates_only_the_days_creatures`,5/0,23.49s (`logs/current-daily-map-regressions.log`). No unrelated PostgreSQL case was selected.

Fresh ordered Quest consistency `python3 tools/content-schema/quest-authoring/run_checks.py --part checks --jobs 1` completed exit0 (`logs/current-quest-consistency.log`). Content/Quest Rust filters `world_reward_claims world_callbacks`:6/0 and11/0 (`logs/current-content-quest-units.log`). Game-server integration_rest filters `content_ world_reward_claims`:135/0,1.53s (`logs/current-content-integration.log`). All cargo invocations used the absolute wrapper, --locked/--offline and CARGO_BUILD_JOBS=4, one at a time.

Whole handwritten product/tool diff re-read against owner and repository checklists:2201 lines/25 files before final fmt-only line wrap. New production field DailyMap.items has exactly2 constructors(server and PG), both updated. No new production enum/trait method/pub fn, no migrations/command/capability allocation. Mounted rustfmt and product/tool whitespace pass. First workspace fmt check caught one line wrap in channel_boot; repaired with cargo fmt; fresh check follows.

Final remaining gates passed: strict source-binding1/0,0.14s (`logs/current-source-binding-unit.log`); World repository5/0,33.25s (`logs/current-world-repository.log`); `cargo run --locked --offline --quiet -p oteryn-world-bundle-compiler -- pin-check . .questpc4/tmp/current-final-pin-check` exit0 (`logs/current-final-pin-check.log`), unchanged World digest3af394d0f4e68a8485d4655e3109ceaf867320c5ca053290a29e8d297faf7afd; `cargo fmt --all --check` exit0 (`logs/current-final-fmt-green.log`). Final product/tool diff whitespace and mounted rustfmt passed. The78-path manifest matches `git diff HEAD --name-only` outside lane records; no CI/protection, Cargo, migration, command or capability delta. Final status files agree across100 titles;6 QUALIFIED entries all point to actual completion markers/durations and94 BLOCKED entries name source anchors/attempts. No claim of full current-Tibia source parity, external review, protected CI, publication or mergeability.

## 2026-11-06 batch02 continuation

Whole-title checkpoint:9 QUALIFIED /10 BLOCKED(WORLDCHG) /81 ACTIONABLE(questpc4). Batch02 is3/5 qualified; the whole task remains incomplete. No full PostgreSQL target/nextest/workspace test run, Git write command, external mutation or subagent was used.

Focused PostgreSQL commands (absolute queued wrapper; all completion markers follow cleanup):
```
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --lib waterfall_access:: -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline --quiet -p oteryn-gameplay-dispatch --lib triangle_tower_source_wall_cycle_over_use_wire -- --nocapture
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline --quiet -p oteryn-gameplay-dispatch --lib vampire_shield_placed_level_doors_require_seventy_over_wire -- --nocapture
```
Waterfall2/0 including the native-selector unit, real STEP marker116.573114609s (resume-waterfall-pg.log); Triangle1/0 including actual native basement STEP exit34.271905343s (batch02-triangle-qualified-pg.log); Vampire1/0 covering both source doors60.632619611s (batch02-vampire-current-pg.log). No absent-URL return counted. Earlier Waterfall RED was0 splash occurrences versus1 (batch02-waterfall-red.log,30.41s). Startup source admission RED had7 missing attributes. Triangle source authoring RED rejected the unregistered quest. Vampire PG RED reached ItemReference after31.42s, and native-profile RED independently named missing5102/5103. Later fixture-only failures were corrected, not loosened; all final markers are actual execution.

Compiler gates, exit0 (batch02-current-check.log,batch02-current-clippy.log):
```
/home/mole/oteryn-cache/codex-bin/cargo check --locked --offline --quiet -p oteryn-gameplay-world -p oteryn-gameplay-quests -p oteryn-gameplay-dispatch -p oteryn-game-server --all-targets
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline --quiet -p oteryn-gameplay-world -p oteryn-gameplay-quests -p oteryn-gameplay-dispatch -p oteryn-game-server --all-targets --all-features -- -D warnings
```
The three subsequent legacy-layout restorations have byte-identical canonical rustfmt output to the validated Rust, proven in BATCH02-FORMAT-EQUIVALENCE.json; required new fields/call arguments and the new scenario branch remain. Workspace fmt --all --check passes after restoration (batch02-fmt-restored-layout.log). No Rust semantic change followed the final compiler/PG gates. Existing vendor warnings were emitted by dependencies; workspace clippy exits0 with-D warnings.

Other exact Rust test commands, all through the same absolute wrapper with --locked --offline --quiet:
- `-p oteryn-gameplay-quests --lib world_callbacks::`:14/0,1.56s (batch02-quest-units.log).
- `-p oteryn-gameplay-world --lib map::door::tests::`:1/0 (batch02-world-door-units.log).
- `-p oteryn-gameplay-world --lib map::door::key_tests::`:6/0 (batch02-world-key-door-units.log).
- `-p oteryn-gameplay-content --lib world_reward_claims`:6/0,0.03s (batch02-content-reward-units.log).
- `-p oteryn-game-server --test integration_rest -- content_ world_reward_claims`:135/0,1.94s (batch02-content-integration.log).
- `-p oteryn-game-server --test content_world_project_repository`:5/0,50.38s (batch02-world-repository.log).

Python source/authoring: batch-four16/0; startup3/0 (CI-independent literal source-shaped parser inputs); original quest-door overlay3/0; native key/level-door profiles2/0. Full retained Crystal capture plus `--apply --check` validates7 selectors and0 changed files (batch02-startup-source-check.log). `register_spell_families.py --check` passes41 files. Source cache hashes were verified before generation; no dependency downloaded.

Complete ordered `run_checks.py --part checks` exits0 (batch02-quest-consistency.log). The first pass correctly caught source-provenance and dependent completion/rollout packets; repaired only with owning generators. World compiler pin-refresh and pin-check exit0, output inside .questpc4/tmp/batch02-pin-check. Product/tool whitespace passes; raw retained source whitespace is preserved. BATCH02-SEMANTIC-AUDIT.json proves7 selector-only map changes, two profile additions/no existing-profile delta, and provenance-only Quest-record changes.

External owner/control-plane WIP snapshots moved HEAD from c1e4ee612 through a1e0dc28b to dcf46eba9 while this agent worked; neither commit was issued by this agent. They preserved the authored product bytes. The current work remains AUTHORING with no frozen candidate or PR. Review is BATCH02-REVIEW.md, current manifest BATCH02-CHANGED-FILES.txt, remaining work/owners MECHANISM-GROUPS.json.81 Game quests are unfinished and actionable; no new CI-only failure is left over.
