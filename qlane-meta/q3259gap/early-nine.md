## w05d_cake_bites_use_native_map_and_commit_exact_world_receipts

### w05d_cake_bites_use_native_map_and_commit_exact_world_receipts

Elapsed 351s exit=0

```bash
CARGO_BUILD_JOBS=2 OTERYN_DISPATCH_PG_TESTS=cake_phases cargo test -p oteryn-gameplay-dispatch --lib w05d_cake_bites_use_native_map_and_commit_exact_world_receipts -- --test-threads=1 --nocapture
```

```text
running 1 test
test gameplay_transport::monk_save::postgres_tests::cake_phases::w05d_cake_bites_use_native_map_and_commit_exact_world_receipts ... PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
W05D_CAKE_PHASE_POSTGRES_COMPLETED native_bite_world_receipt_replay_cooldown_collision_projection elapsed=18.09453227s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3019 filtered out; finished in 18.10s
```

[Full stdout and stderr](/workspace/scratch/pr3259-pg/w05d_cake_bites_use_native_map_and_commit_exact_world_receipts.log)

## w05d_cake_baker_

### w05d_cake_baker_all_carried_equipment_source_caps_and_replays_postgres

Elapsed 388s exit=0

```bash
CARGO_BUILD_JOBS=2 OTERYN_DISPATCH_PG_TESTS=cake_contributions cargo test -p oteryn-gameplay-dispatch --lib w05d_cake_baker_all_carried_equipment_source_caps_and_replays_postgres -- --test-threads=1 --nocapture
```

```text
running 1 test
test gameplay_transport::monk_save::postgres_tests::cake_contributions::w05d_cake_baker_all_carried_equipment_source_caps_and_replays_postgres ... PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
W05D_CAKE_BAKER_PG_EXECUTED equipment_source=true tiny200=false donation_elapsed_ms=186 elapsed_ms=15254
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3021 filtered out; finished in 16.35s
```

[Full stdout and stderr](/workspace/scratch/pr3259-pg/w05d_cake_baker_all_carried_equipment_source_caps_and_replays_postgres.log)

### w05d_cake_baker_all_carried_partial_equipment_extra_postgres

Elapsed 23s exit=0

```bash
CARGO_BUILD_JOBS=2 OTERYN_DISPATCH_PG_TESTS=cake_contributions cargo test -p oteryn-gameplay-dispatch --lib w05d_cake_baker_all_carried_partial_equipment_extra_postgres -- --test-threads=1 --nocapture
```

```text
running 1 test
test gameplay_transport::monk_save::postgres_tests::cake_contributions::w05d_cake_baker_all_carried_partial_equipment_extra_postgres ... PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
W05D_CAKE_BAKER_PG_EXECUTED equipment_source=false tiny200=false donation_elapsed_ms=280 elapsed_ms=19677
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3021 filtered out; finished in 20.20s
```

[Full stdout and stderr](/workspace/scratch/pr3259-pg/w05d_cake_baker_all_carried_partial_equipment_extra_postgres.log)

### w05d_cake_baker_two_hundred_nested_inputs_use_one_compact_audit_postgres

Elapsed 22s exit=0

```bash
CARGO_BUILD_JOBS=2 OTERYN_DISPATCH_PG_TESTS=cake_contributions cargo test -p oteryn-gameplay-dispatch --lib w05d_cake_baker_two_hundred_nested_inputs_use_one_compact_audit_postgres -- --test-threads=1 --nocapture
```

```text
running 1 test
test gameplay_transport::monk_save::postgres_tests::cake_contributions::w05d_cake_baker_two_hundred_nested_inputs_use_one_compact_audit_postgres ... PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
W05D_CAKE_BAKER_PG_EXECUTED equipment_source=false tiny200=true donation_elapsed_ms=1090 elapsed_ms=18864
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3021 filtered out; finished in 19.20s
```

[Full stdout and stderr](/workspace/scratch/pr3259-pg/w05d_cake_baker_two_hundred_nested_inputs_use_one_compact_audit_postgres.log)

## m25_galthen_real_native_deaths_require_predecessors_and_never_complete_postgres

### m25_galthen_real_native_deaths_require_predecessors_and_never_complete_postgres

Elapsed 367s exit=0

```bash
CARGO_BUILD_JOBS=2 OTERYN_DISPATCH_PG_TESTS=m25_galthen_native_death cargo test -p oteryn-gameplay-dispatch --lib m25_galthen_real_native_deaths_require_predecessors_and_never_complete_postgres -- --test-threads=1 --nocapture
```

```text
running 1 test
test gameplay_transport::monk_save::postgres_tests::m25_galthen_native_death::m25_galthen_real_native_deaths_require_predecessors_and_never_complete_postgres ... PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
M25_ACTUAL_CORPSE_CHAIN_REACHED creature=oteryn:creature.ahau schedules=3 lifetime_ms=670000
M25_REAL_NATIVE_DEATH_REACHED creature=oteryn:creature.ahau predecessor=s4 target=s5
M25_ACTUAL_CORPSE_CHAIN_REACHED creature=oteryn:creature.mitmah_vanguard schedules=3 lifetime_ms=670000
M25_REAL_NATIVE_DEATH_REACHED creature=oteryn:creature.mitmah_vanguard predecessor=s7 target=s8
POSTGRES_CASE_COMPLETED qengine-rewards-m25-galthen-native-deaths duration=36.247958377s success=true
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3018 filtered out; finished in 36.25s
```

[Full stdout and stderr](/workspace/scratch/pr3259-pg/m25_galthen_real_native_deaths_require_predecessors_and_never_complete_postgres.log)

## m47_native_use_commits_random_outcome_feedback_blackout_and_replays

### m47_native_use_commits_random_outcome_feedback_blackout_and_replays

Elapsed 140s exit=0

```bash
CARGO_BUILD_JOBS=2 OTERYN_DISPATCH_PG_TESTS=qm47_mead_dispatch cargo test -p oteryn-gameplay-dispatch --lib m47_native_use_commits_random_outcome_feedback_blackout_and_replays -- --test-threads=1 --nocapture
```

```text
running 1 test
test gameplay_transport::monk_save::postgres_tests::qm47_mead_dispatch::m47_native_use_commits_random_outcome_feedback_blackout_and_replays ... PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
M47_NATIVE_ROUNDS_POSTGRES_EXECUTED passed=true exhausted=true late_replay=true
M47_NATIVE_POSTGRES_EXECUTED failed=true won=true uses=3
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3018 filtered out; finished in 35.14s
```

[Full stdout and stderr](/workspace/scratch/pr3259-pg/m47_native_use_commits_random_outcome_feedback_blackout_and_replays.log)

## m47_postgres_mead_roll_is_fenced_and_exactly_replayed

### m47_postgres_mead_roll_is_fenced_and_exactly_replayed

Elapsed 135s exit=0

```bash
CARGO_BUILD_JOBS=2 OTERYN_DISPATCH_PG_TESTS=qm47_mead cargo test -p oteryn-gameplay-dispatch --lib m47_postgres_mead_roll_is_fenced_and_exactly_replayed -- --test-threads=1 --nocapture
```

```text
running 1 test
test gameplay_transport::monk_save::postgres_tests::qm47_mead::m47_postgres_mead_roll_is_fenced_and_exactly_replayed ... PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
M47_POSTGRES_EXECUTED duration=30.156638461s success=true
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3018 filtered out; finished in 30.16s
```

[Full stdout and stderr](/workspace/scratch/pr3259-pg/m47_postgres_mead_roll_is_fenced_and_exactly_replayed.log)

## m44_native_children_readset_reset_manual_close_reopen_and_stale_timer

### m44_native_children_readset_reset_manual_close_reopen_and_stale_timer

Elapsed 138s exit=0

```bash
CARGO_BUILD_JOBS=2 OTERYN_DISPATCH_PG_TESTS=m44_children_lifecycle cargo test -p oteryn-gameplay-dispatch --lib m44_native_children_readset_reset_manual_close_reopen_and_stale_timer -- --test-threads=1 --nocapture
```

```text
running 1 test
test gameplay_transport::monk_save::postgres_tests::m44_children_lifecycle::m44_native_children_readset_reset_manual_close_reopen_and_stale_timer ... PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
POSTGRES_CASE_COMPLETED M44 children_native_readset commit_barrier replay close reopen stale_timer exact_expiry cells_preserved
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3018 filtered out; finished in 28.91s
```

[Full stdout and stderr](/workspace/scratch/pr3259-pg/m44_native_children_readset_reset_manual_close_reopen_and_stale_timer.log)

## m44_native_lions

### m44_native_lions_first_offering_consumption_progress_and_fenced_pagoda_reset

Elapsed 134s exit=0

```bash
CARGO_BUILD_JOBS=2 OTERYN_DISPATCH_PG_TESTS=m44_lions_lifecycle cargo test -p oteryn-gameplay-dispatch --lib m44_native_lions_first_offering_consumption_progress_and_fenced_pagoda_reset -- --test-threads=1 --nocapture
```

```text
running 1 test
test gameplay_transport::monk_save::postgres_tests::m44_lions_lifecycle::m44_native_lions_first_offering_consumption_progress_and_fenced_pagoda_reset ... PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
POSTGRES_CASE_COMPLETED M44 lions_native_pagoda phase=first offering_atomic replay exact_reset once_only
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3018 filtered out; finished in 28.65s
```

[Full stdout and stderr](/workspace/scratch/pr3259-pg/m44_native_lions_first_offering_consumption_progress_and_fenced_pagoda_reset.log)

### m44_native_lions_later_offering_preserves_existing_progress_and_resets_pagoda

Elapsed 24s exit=0

```bash
CARGO_BUILD_JOBS=2 OTERYN_DISPATCH_PG_TESTS=m44_lions_lifecycle cargo test -p oteryn-gameplay-dispatch --lib m44_native_lions_later_offering_preserves_existing_progress_and_resets_pagoda -- --test-threads=1 --nocapture
```

```text
running 1 test
test gameplay_transport::monk_save::postgres_tests::m44_lions_lifecycle::m44_native_lions_later_offering_preserves_existing_progress_and_resets_pagoda ... PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
POSTGRES_CASE_COMPLETED M44 lions_native_pagoda phase=later offering_atomic replay exact_reset once_only
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3018 filtered out; finished in 22.64s
```

[Full stdout and stderr](/workspace/scratch/pr3259-pg/m44_native_lions_later_offering_preserves_existing_progress_and_resets_pagoda.log)

### m44_native_lions_unknown_commit_reserves_both_objects_until_reconciled

Elapsed 23s exit=0

```bash
CARGO_BUILD_JOBS=2 OTERYN_DISPATCH_PG_TESTS=m44_lions_lifecycle cargo test -p oteryn-gameplay-dispatch --lib m44_native_lions_unknown_commit_reserves_both_objects_until_reconciled -- --test-threads=1 --nocapture
```

```text
running 1 test
test gameplay_transport::monk_save::postgres_tests::m44_lions_lifecycle::m44_native_lions_unknown_commit_reserves_both_objects_until_reconciled ... PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
POSTGRES_CASE_COMPLETED M44 lions_unknown_commit objects_reserved original_cause_reconciled no_double_consumption
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3018 filtered out; finished in 21.79s
```

[Full stdout and stderr](/workspace/scratch/pr3259-pg/m44_native_lions_unknown_commit_reserves_both_objects_until_reconciled.log)

## m07_dreamer

### m07_dreamer_fire_bug_retains_input_and_resets_shared_entrance

Elapsed 140s exit=0

```bash
CARGO_BUILD_JOBS=2 OTERYN_DISPATCH_PG_TESTS=m07_dreamer_fire_bug cargo test -p oteryn-gameplay-dispatch --lib m07_dreamer_fire_bug_retains_input_and_resets_shared_entrance -- --test-threads=1 --nocapture
```

```text
running 1 test
test gameplay_transport::monk_save::postgres_tests::m07_dreamer_fire_bug::m07_dreamer_fire_bug_retains_input_and_resets_shared_entrance ... PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
M07 POSTGRES_CASE_COMPLETED dreamer_ignition_accepts_bundle_channel_frame
M07 POSTGRES_CASE_COMPLETED shared_fire_bug real_USE real_STEP retained_item no_progress five_minute_reset replay_no_extension
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3018 filtered out; finished in 26.93s
```

[Full stdout and stderr](/workspace/scratch/pr3259-pg/m07_dreamer_fire_bug_retains_input_and_resets_shared_entrance.log)

### dreamer_ignition_accepts_bundle_channel_frame

Elapsed 26s exit=0

```bash
CARGO_BUILD_JOBS=2 OTERYN_DISPATCH_PG_TESTS=m07_dreamer_fire_bug cargo test -p oteryn-gameplay-dispatch --lib dreamer_ignition_accepts_bundle_channel_frame -- --test-threads=1 --nocapture
```

```text
running 1 test
test gameplay_transport::monk_save::postgres_tests::m07_dreamer_fire_bug::dreamer_ignition_accepts_bundle_channel_frame ... PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
M07 POSTGRES_CASE_COMPLETED dreamer_ignition_accepts_bundle_channel_frame
M07 POSTGRES_CASE_COMPLETED shared_fire_bug real_USE real_STEP retained_item no_progress five_minute_reset replay_no_extension
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3018 filtered out; finished in 23.18s
```

[Full stdout and stderr](/workspace/scratch/pr3259-pg/dreamer_ignition_accepts_bundle_channel_frame.log)

### m07_dreamer_native_lever_conditional_walls_and_distinct_deadlines_postgres

Elapsed 142s exit=0

```bash
CARGO_BUILD_JOBS=2 OTERYN_DISPATCH_PG_TESTS=m07_dreamer_levers cargo test -p oteryn-gameplay-dispatch --lib m07_dreamer_native_lever_conditional_walls_and_distinct_deadlines_postgres -- --test-threads=1 --nocapture
```

```text
running 1 test
PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
POSTGRES_DATABASE_READY m07/dreamer-lever-north version=18.6 (Debian 18.6-1.pgdg13+2)
POSTGRES_CASE_COMPLETED m07/dreamer-lever-north duration=26.686650815s success=true
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3018 filtered out; finished in 26.69s
```

[Full stdout and stderr](/workspace/scratch/pr3259-pg/m07_dreamer_native_lever_conditional_walls_and_distinct_deadlines_postgres.log)

### lost_ack_dreamer_ring_replay_republishes_trees

Elapsed 128s exit=0

```bash
CARGO_BUILD_JOBS=2 OTERYN_DISPATCH_PG_TESTS=m07_dreamer_ring_lost_ack cargo test -p oteryn-gameplay-dispatch --lib lost_ack_dreamer_ring_replay_republishes_trees -- --test-threads=1 --nocapture
```

```text
running 1 test
test gameplay_transport::monk_save::postgres_tests::m07_dreamer_ring_lost_ack::lost_ack_dreamer_ring_replay_republishes_trees ... PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
M07_DREAMER_RING_LOST_ACK_REPLAY_PUBLISHED
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3018 filtered out; finished in 28.06s
```

[Full stdout and stderr](/workspace/scratch/pr3259-pg/lost_ack_dreamer_ring_replay_republishes_trees.log)

### m07_carrot_premium_entry_consumes_exactly_one_and_replays

Elapsed 139s exit=0

```bash
CARGO_BUILD_JOBS=2 OTERYN_DISPATCH_PG_TESTS=m07_dreamer_carrot cargo test -p oteryn-gameplay-dispatch --lib m07_carrot_premium_entry_consumes_exactly_one_and_replays -- --test-threads=1 --nocapture
```

```text
running 1 test
test gameplay_transport::monk_save::postgres_tests::m07_dreamer_carrot::m07_carrot_premium_entry_consumes_exactly_one_and_replays ... PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
M07 POSTGRES_CASE_COMPLETED carrot real_STEP premium=true missing_before_premium carried_atomic_debit origin_fire replay_no_extra_effects no_progress
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3018 filtered out; finished in 28.71s
```

[Full stdout and stderr](/workspace/scratch/pr3259-pg/m07_carrot_premium_entry_consumes_exactly_one_and_replays.log)

### m07_carrot_free_account_keeps_its_offering_and_gets_source_refusal

Elapsed 22s exit=0

```bash
CARGO_BUILD_JOBS=2 OTERYN_DISPATCH_PG_TESTS=m07_dreamer_carrot cargo test -p oteryn-gameplay-dispatch --lib m07_carrot_free_account_keeps_its_offering_and_gets_source_refusal -- --test-threads=1 --nocapture
```

```text
running 1 test
test gameplay_transport::monk_save::postgres_tests::m07_dreamer_carrot::m07_carrot_free_account_keeps_its_offering_and_gets_source_refusal ... PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
M07 POSTGRES_CASE_COMPLETED carrot real_STEP premium=false missing_before_premium carried_atomic_debit origin_fire replay_no_extra_effects no_progress
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3018 filtered out; finished in 21.50s
```

[Full stdout and stderr](/workspace/scratch/pr3259-pg/m07_carrot_free_account_keeps_its_offering_and_gets_source_refusal.log)

### m07_death_ring_real_step_restores_only_missing_trees_and_replays

Elapsed 134s exit=0

```bash
CARGO_BUILD_JOBS=2 OTERYN_DISPATCH_PG_TESTS=m07_dreamer_ring_reset cargo test -p oteryn-gameplay-dispatch --lib m07_death_ring_real_step_restores_only_missing_trees_and_replays -- --test-threads=1 --nocapture
```

```text
running 1 test
test gameplay_transport::monk_save::postgres_tests::m07_dreamer_ring_reset::m07_death_ring_real_step_restores_only_missing_trees_and_replays ... PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
M07 POSTGRES_CASE_COMPLETED Death Ring UID9234 real_STEP missing_only_tree_reset source_teleport replay
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3018 filtered out; finished in 25.86s
```

[Full stdout and stderr](/workspace/scratch/pr3259-pg/m07_death_ring_real_step_restores_only_missing_trees_and_replays.log)

### m07_ring_real_additem_consumes_once_then_step_resets_shared_trees

Elapsed 143s exit=0

```bash
CARGO_BUILD_JOBS=2 OTERYN_DISPATCH_PG_TESTS=m07_dreamer_ring_offering cargo test -p oteryn-gameplay-dispatch --lib m07_ring_real_additem_consumes_once_then_step_resets_shared_trees -- --test-threads=1 --nocapture
```

```text
running 1 test
test gameplay_transport::monk_save::postgres_tests::m07_dreamer_ring_offering::m07_ring_real_additem_consumes_once_then_step_resets_shared_trees ... PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
M07 POSTGRES_CASE_COMPLETED Death Ring real_ADDITEM wrong_input preserved absent_trees preserved atomic_ring_debit replay real_STEP shared_reset
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3018 filtered out; finished in 36.31s
```

[Full stdout and stderr](/workspace/scratch/pr3259-pg/m07_ring_real_additem_consumes_once_then_step_resets_shared_trees.log)

## qnpcfx_koshei

### qnpcfx_koshei_paid_pieces_atomic_replay_and_clock

Elapsed 162s exit=0

```bash
CARGO_BUILD_JOBS=2 OTERYN_DISPATCH_PG_TESTS=qnpcfx_koshei cargo test -p oteryn-gameplay-dispatch --lib qnpcfx_koshei_paid_pieces_atomic_replay_and_clock -- --test-threads=1 --nocapture
```

```text
running 1 test
test gameplay_transport::monk_save::postgres_tests::qnpcfx_koshei::qnpcfx_koshei_paid_pieces_atomic_replay_and_clock ... PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
POSTGRES_CASE_COMPLETED qengine-rewards-qnpcfx-koshei duration=45.272759654s success=true
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3018 filtered out; finished in 45.27s
```

[Full stdout and stderr](/workspace/scratch/pr3259-pg/qnpcfx_koshei_paid_pieces_atomic_replay_and_clock.log)

### qnpcfx_koshei_mixed_carried_bank_payment_atomic

Elapsed 45s exit=0

```bash
CARGO_BUILD_JOBS=2 OTERYN_DISPATCH_PG_TESTS=qnpcfx_koshei cargo test -p oteryn-gameplay-dispatch --lib qnpcfx_koshei_mixed_carried_bank_payment_atomic -- --test-threads=1 --nocapture
```

```text
running 1 test
test gameplay_transport::monk_save::postgres_tests::qnpcfx_koshei::qnpcfx_koshei_mixed_carried_bank_payment_atomic ... PGTPL reused oteryn_tpl_9541d0b8f2fcab0e
DURABILITY_ROOT_REESTABLISHED
POSTGRES_CASE_COMPLETED qengine-rewards-qnpcfx-koshei-bank duration=42.909708691s success=true
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 3018 filtered out; finished in 42.91s
```

[Full stdout and stderr](/workspace/scratch/pr3259-pg/qnpcfx_koshei_mixed_carried_bank_payment_atomic.log)

