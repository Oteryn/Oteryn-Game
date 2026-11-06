# Run7 root validation checkpoint

All commands run from the worktree. No frozen candidate or CI claim. Cargo uses the absolute queued wrapper and explicit mechanic PG filters. The tree remains PARTIAL pending Newhaven capacity choice; its actual source PG is not qualified.

| Check | Exit | Wall seconds |
|---|---:|---:|
| r7-quest-generation-source-stable | 0 | 987.801 |
| r7-authoring-checks-verified | 0 | 202.971 |
| r7-donor-check-final | 0 | 449.361 |
| r7-authoring-units-green | 0 | 215.957 |
| r7-map-decoder-installed-sdk | 0 | 0.806 |
| r7-native-content-check-final | 0 | 39.134 |
| r7-clippy-final-clean | 0 | 116.295 |
| r7-fmt-checkpoint | 0 | 10.035 |
| r7-content-world-rewards-final | 0 | 1.502 |
| r7-durable-reward-unit-final | 0 | 93.935 |
| r7-quests-unit-final | 0 | 12.617 |
| r7-server-reward-binding-green | 0 | 44.733 |
| r7-boot-verified | 0 | 30.407 |
| r7-client-tracker-integration | 0 | 5.994 |
| r7-root-fluid-owner-pg | 0 | 73.651 |
| r7-root-newhaven-unit | 0 | 89.046 |
| r7-root-combined-pg | 101 | 743.603 |
| r7-root-stag-guard-alone | 0 | 345.692 |

Exact completed commands:

```sh
# r7-quest-generation-source-stable
python3 .questpc/regenerate_stable.py
# r7-authoring-checks-verified
python3 tools/content-schema/quest-authoring/run_checks.py --part checks
# r7-donor-check-final
/home/mole/oteryn-cache/venv-ci312/bin/python tools/content-schema/quest-authoring/quest_donor_source_authoring.py --check
# r7-authoring-units-green
python3 tools/content-schema/quest-authoring/run_checks.py --part unit-tests --jobs 4
# r7-map-decoder-installed-sdk
python3 -m unittest discover -s tools/content-schema/quest-authoring -p test_donor_source_maps.py
# r7-native-content-check-final
python3 tools/content-migration/register_spell_families.py --check
# r7-clippy-final-clean
/home/mole/oteryn-cache/codex-bin/cargo clippy --locked --offline -p oteryn-gameplay-content -p oteryn-gameplay-durability -p oteryn-gameplay-quests -p oteryn-gameplay-social -p oteryn-gameplay-dispatch -p oteryn-gameplay-combat -p oteryn-gameplay-encounters -p oteryn-gameplay-npc -p oteryn-game-server -p oteryn-client --all-targets --all-features --quiet -- -D warnings
# r7-fmt-checkpoint
/home/mole/oteryn-cache/codex-bin/cargo fmt --all
# r7-content-world-rewards-final
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-content --lib world_reward_claims -- --nocapture
# r7-durable-reward-unit-final
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-durability --lib reward_claim_mint::tests -- --nocapture
# r7-quests-unit-final
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-quests --lib --quiet
# r7-server-reward-binding-green
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-game-server --test integration_rest world_reward_claims_the_embedded_shards_decode -- --nocapture
# r7-boot-verified
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-game-server --lib quest_catalogue_boot_tests -- --nocapture
# r7-client-tracker-integration
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-client --test play_features quest_log_recorded_domain_16_frames_reach_the_play_panel -- --nocapture
# r7-root-fluid-owner-pg
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-game-server --test character_authority_postgres filled_fluid_claims_commit_root_and_children_and_replay_without_refilling -- --nocapture
# r7-root-newhaven-unit
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib -- newhaven_cached_plan_reserves_only_shortfall_and_releases_it newhaven_corruptor_guarantee_survives_low_factor_and_keeps_independent_ordinary_drop --nocapture
# r7-root-combined-pg
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib -- quest_catalogue_chests_medusa_exact_crystal_reward_use_and_replay quest_catalogue_chests_full_charge_rewards_use_replay_and_refusal quest_catalogue_chests_exact_partial_charges_use_replay_and_refusal quest_catalogue_chests_exact_fluid_subtypes_use_replay_and_refusal quest_world_callbacks_literal_use_accepts_clicked_nonzero_ordinal quest_world_callbacks_read_only_stag_uses_gate_without_quest_writes falcon_chests_postgres_real_use_frozen_draw_refusal_restart_and_fences catalogue_npc_source_acceptance_is_atomic_and_replay_bound soul_war_postgres_ endgame_entry_postgres_real_dispatch_cannot_allocate_or_reset_without_w02 --test-threads=2 --nocapture
# r7-root-stag-guard-alone
/home/mole/oteryn-cache/codex-bin/cargo test --locked --offline -p oteryn-gameplay-dispatch --all-features --lib quest_world_callbacks_read_only_stag_uses_gate_without_quest_writes -- --test-threads=1 --nocapture
```

Map decoder and final93-module authoring run also used `CPLUS_INCLUDE_PATH=/opt/node24/include/node` and `LIBRARY_PATH=$PWD/.questpc/native-libs`. Local `libz.so` points to existing system `libz.so.1`; no downloads/installs. Cargo slots were2/3 for root validation,6 for final formatting; all workers used their assigned1..6slots.

The integrated dispatch run was10/1 (398.09s test). Its Stag failure occurred in unchanged fresh-session fixture setup before the wallcallback. Exact isolated rerun1/0,9markers,304.38s test; concurrent fixture qualification remains CI-only. No deadline/guard/test weakened. Fluid-owner1/0,completion3.833802782s. Full authoring93/93modules; mapdecoder23tests.

Worker snapshots and source deltas: r7-callbacks-validation.md, r7-falcon-validation.md, r7-fluids-soulwar-validation.md, r7-npc-validation.md, r7-newhaven-validation.md.

Final formatter verification: `/home/mole/oteryn-cache/codex-bin/cargo fmt --all -- --check`, exit0, 11.638s. Final productdiffcheck exit0; conflictmarker scan empty.
