# questpc3 local qualification report

Current stop-rule pass: all 56 inventory titles now use a terminal status in
QUEST-STATUS.md and owner-final-status.json: 17 QUALIFIED, 38 implementation
BLOCKED, and the retained Dawnport exclusion recorded as one additional BLOCKED
row. No new quest is qualified by this continuation.

Fresh production-dispatch proof: current-restart-chests.log contains 23 passed tests,
24 POSTGRES_CASE_COMPLETED markers and 398.09s; current-restart-npc.log contains four
passed tests, four completion markers and 209.63s. Every individual marker and
duration_ms is linked to its exact PG test in owner-final-status.json. Both
absolute-wrapper filters below ran with the PostgreSQL route present. Exact
remaining gaps, file:line boundaries and attempts are in FINAL-BLOCKERS.md.

All12 current check commands passed; exact commands, exits and log hashes are in
current-restart-checks.json. The complete source/JSON/map/hash comparison is in
current-restart-tree-proof.json. The587 pure Rust tests, binding/coverage test,
clippy and both format checks passed; the content suite retains its one existing ignore.

The historical checkpoints below are retained evidence; current status is the
terminal inventory above, with no pending owner question or unchanged-head review
trigger. This remains a local worker tree, without a frozen candidate or PR.

Latest restart boundary sweep: no newly qualified quest. The existing four NPC
tests pass again under filter `rookgaard_exchanges`, with four completion markers
in 249.27 s (`restart-next-npc.log`). All five crates pass clippy; format checks,
587 pure Rust tests (one pre-existing content ignore) and 78 Python tests pass.
Exact commands/results are in restart-next-checks.json. Phase A's unchanged
23-test/24-claim proof remains retained in restart-current-chests.log and was
not rerun in this documentation-only continuation. Totals remain 17 qualified,
38 unfinished and 1 Dawnport exclusion. Each unfinished title now names its
concrete boundary in QUEST-STATUS.md and PHASE-BC-BLOCKERS.md.

Earlier continuation: the four renamed NPC tests passed with four completion markers
in 226.61 s; all five changed crates pass clippy, format checks pass, and the pure
unit run passes 587 tests with one pre-existing ignore. Exact fresh evidence is
`continuation-checks.json`; earlier chest/content evidence below remains saved.

Scope: source reward grants and the four restored repeatable NPC exchanges, through production dispatch. This certifies the named paths, not deployment or completion of every R–S storyline. No frozen candidate or PR exists.

Final PG evidence: `final-pg-chests.log` (23 tests,24 completion markers,415.61 seconds) and `final-pg-npc.log` (4 tests,4 completion markers,176.40 seconds). Each marker records its duration; machine-readable details are in `final-pg-evidence.json`. All five changed crates pass clippy -D warnings; format and relevant Rust/Python unit checks pass. All 5 game-server world-repository tests and all 123 content integration tests passed. The final metadata reproducibility, clippy, format and pin checks also passed; exact commands/results are in final-checks.json.

QUALIFIED: Rapier Quest - questpc3_rapier_claim_once_and_replay.

QUALIFIED: Ring Quest - questpc3_ring_time_ring_claim_once_and_replay, questpc3_ring_sword_ring_claim_once_and_replay.

QUALIFIED: Scale Armor Quest - questpc3_scale_armor_bag_once_and_replay.

QUALIFIED: Shaman Treasure Quest - questpc3_shaman_treasure_bag_once_and_replay.

QUALIFIED: Silver Amulet Quest - questpc3_charged_amulet_cohort_once_and_replay.

QUALIFIED: Silver Brooch Quest - questpc3_silver_brooch_full_bag_once_and_replay.

QUALIFIED: Six Rubies Quest - questpc3_six_rubies_claim_once_and_replay.

QUALIFIED: Skull of Ratha Quest - questpc3_skull_of_ratha_skull_and_pearl_once_and_replay, questpc3_skull_of_ratha_chain_and_ring_once_and_replay, questpc3_skull_of_ratha_witch_house_full_bag_once_and_replay.

QUALIFIED: Spike Sword Quest - questpc3_spike_sword_claim_once_and_replay.

QUALIFIED: Stealth Ring Quest - questpc3_charged_amulet_cohort_once_and_replay, questpc3_stealth_ring_ring_once_and_replay.

QUALIFIED: Steel Helmet Quest - questpc3_steel_helmet_first_coins_once_and_replay, questpc3_steel_helmet_second_coins_once_and_replay, questpc3_steel_helmet_helmet_once_and_replay, questpc3_steel_helmet_letter_once_and_replay.

QUALIFIED: Strong Potions Quest - questpc3_strong_potions_full_bag_once_and_replay.

QUALIFIED: Short Sword Quest - questpc3_short_sword_notebook_once_and_replay, rookgaard_short_sword_amber_exchange_atomic_repeatable_replay.

QUALIFIED: Small Health Potion Quest - rookgaard_small_health_potion_billy_exchange_atomic_repeatable_replay.

QUALIFIED: Studded Legs Quest - questpc3_studded_legs_honey_flower_once_and_replay, rookgaard_studded_legs_lee_delle_exchange_atomic_repeatable_replay.

QUALIFIED: Studded Shield Quest - questpc3_studded_shield_free_banana_once_and_replay, questpc3_studded_shield_premium_banana_once_and_replay, rookgaard_studded_shield_willie_exchange_atomic_repeatable_replay.

QUALIFIED: Small Sapphire Quest - questpc3_small_sapphire_full_recipe_once_and_replay.

Exact PG command filters (absolute wrapper, one cargo at a time):

```sh
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked -j 4 -p oteryn-gameplay-dispatch --all-features --lib questpc3_reward_chests -- --nocapture --test-threads=2
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked -j 4 -p oteryn-gameplay-dispatch --all-features --lib rookgaard_exchanges -- --nocapture --test-threads=1
CARGO_BUILD_JOBS=4 /home/mole/oteryn-cache/codex-bin/cargo test --locked -j 4 -p oteryn-gameplay-dispatch --all-features --lib quest_callbacks_binding_and_coverage_require_exact_source_witnesses -- --nocapture
```

The complete validation commands/results are in final-checks.json. Small Health Potion's pan comes from the separately owned Goblin Temple quest; this lane qualifies Billy's exchange, not that other quest's acquisition path. Written rewards retain both exact source texts in durable custody; the existing carried-book READ/TextWindow limitation remains explicit in repository coverage. All unfinished titles and the Dawnport exclusion are listed in QUEST-STATUS.md.

Continuation evidence supersedes the saved NPC run: `continuation-keywords-pg-fixed.log`
contains the four current `rookgaard_` tests, four completion markers and 226.61 s.
The source rule unit filter is `rookgaard` in `oteryn-gameplay-quests --lib`.
The saved `final-pg-npc.log` uses the earlier test/module names. No additional
quest is counted; the 24 chest claim proofs are unchanged saved evidence.

Latest restart evidence supersedes those saved runs: restart-current-chests.log
has 23 passed tests and 24 distinct completed claims in 549.11 seconds;
restart-greeting-green.log has all 4 current Rookgaard tests and 4 completion markers
in 201.71 seconds. The shared test now refuses mixed greeting/offer messages before
focus, retains the source greeting and proves zero inventory mutation. The Lee'Delle
case first failed on `hi honey scented flowers` (restart-greeting-red.log).
All 17 paths above are requalified; no additional quest is counted. Clippy for
the five changed crates, both format checks and the `rookgaard` unit filter pass.
Exact wrapper commands, results and per-case durations: restart-current-checks.json.


Active restart qualification: Sinatuki Quest adds the eighteenth qualified title.
QUALIFIED: Sinatuki Quest - sinatuki_mixed_fish_then_final_fish_atomic_replay
(POSTGRES_CASE_COMPLETED, duration_ms=47473);
sinatuki_whole_hundred_stack_still_needs_final_fish
(POSTGRES_CASE_COMPLETED, duration_ms=57999). Log: sinatuki-final-pg.log.
The four current Rookgaard cases are recorded in rookgaard-final-pg.log
(64317/54470/66220/68322ms; 4 markers; 253.34 seconds).
The unchanged 89-file input snapshot is active-inputs.json. Pure library tests
finished in active-pure-tests.log: content312 passed/1 existing ignored; NPC24
passed; quests251 passed. No ignored case is counted as quest qualification.
