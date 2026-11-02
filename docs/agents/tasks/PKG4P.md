# PKG4P — Prey draft takeover

```yaml
task_id: PKG4P
title: Prey difficulty draft takeover against fresh main
type: mechanic
mode: IMPLEMENT
status: AUTHORING
repository: Oteryn/Oteryn-Game
base_branch: main
branch: null
pr: 2797
base_sha: 96d802bc83300820a4de3ab47ae37ceacf3f6224
head_sha: null
owner: coordinator-delegated-pkg4p
created_at: 2026-11-02
updated_at: 2026-11-02
owned_paths:
  - apps/client/src/combat_input.rs
  - apps/client/src/features/0140-prey/input.rs
  - apps/client/src/features/0140-prey/native.rs
  - apps/client/src/features/0140-prey/native_tests.rs
  - apps/client/src/features/0140-prey/protocol.rs
  - apps/client/src/features/0140-prey/tests/play.rs
  - apps/client/src/features/0140-prey/view.rs
  - apps/client/src/play/session_loop.rs
  - apps/client/src/play/session_stepper.rs
  - apps/client/src/windows_shell/input.rs
  - apps/game-server/src/qualification/client_phases/prey.rs
  - apps/game-server/migrations/0752_prey_difficulty.sql
  - apps/game-server/migrations/0753_prey_pick_again_fee.sql
  - apps/game-server/migrations/0754_prey_measurements.sql
  - apps/game-server/migrations/0755_prey_list_expansion.sql
  - apps/game-server/src/lifted_tests/auto/prey_postgres_cases.rs
  - apps/game-server/tests/support/stamina_postgres_cases.rs
  - content/content.lock.json
  - content/creatures/definitions/index.json
  - content/creatures/definitions/spell-native-profiles.json
  - content/creatures/prey-slot-values.json
  - content/creatures/prey-tiers.json
  - content/manifest.json
  - content/spells.manifest.json
  - crates/client-runtime/src/features/0140-prey/protocol.rs
  - crates/client-ui/src/features/0140-prey/input.rs
  - crates/client-ui/src/features/0140-prey/mod.rs
  - crates/client-ui/src/features/0140-prey/protocol.rs
  - crates/client-ui/src/features/0140-prey/state.rs
  - crates/client-ui/src/features/0140-prey/tests.rs
  - crates/client-ui/src/features/0140-prey/tests/state.rs
  - crates/client-ui/src/features/0140-prey/view.rs
  - crates/gameplay/content/src/content/native_gameplay.rs
  - crates/gameplay/dispatch/examples/prey_time_sim.rs
  - crates/gameplay/dispatch/src/gameplay_transport/actor_spell.rs
  - crates/gameplay/dispatch/src/gameplay_transport/attack/drain.rs
  - crates/gameplay/dispatch/src/gameplay_transport/bundle_gates_tests.rs
  - crates/gameplay/dispatch/src/gameplay_transport/hunting_stamina.rs
  - crates/gameplay/dispatch/src/gameplay_transport/kill_reward.rs
  - crates/gameplay/dispatch/src/gameplay_transport/kill_reward_tests.rs
  - crates/gameplay/dispatch/src/gameplay_transport/monster_ai_cycle.rs
  - crates/gameplay/dispatch/src/gameplay_transport/monster_profile_cycle/physical_spell.rs
  - crates/gameplay/dispatch/src/gameplay_transport/native_combat_cast.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/prey.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/prey_difficulty.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/prey_expansion.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/prey_loot_measurement.rs
  - crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/prey_store_retry.rs
  - crates/gameplay/dispatch/src/gameplay_transport/prey.rs
  - crates/gameplay/dispatch/src/gameplay_transport/prey/kill.rs
  - crates/gameplay/dispatch/src/gameplay_transport/prey/live.rs
  - crates/gameplay/dispatch/src/gameplay_transport/prey/live_tests.rs
  - crates/gameplay/dispatch/src/gameplay_transport/store.rs
  - crates/gameplay/dispatch/src/gameplay_transport/store/catalogue.rs
  - crates/gameplay/dispatch/src/gameplay_transport/store/prey_retry.rs
  - crates/gameplay/dispatch/src/gameplay_transport/store/tests.rs
  - crates/gameplay/durability/src/durability/character_authority.rs
  - crates/gameplay/durability/src/durability/item_fee_burn.rs
  - crates/gameplay/durability/src/durability/item_fee_burn/purchase_cause.rs
  - crates/gameplay/durability/src/durability/item_fee_burn_audit/purchase_cause.rs
  - crates/gameplay/durability/src/durability/prey.rs
  - crates/gameplay/durability/src/durability/prey/command.rs
  - crates/gameplay/durability/src/durability/prey/decode.rs
  - crates/gameplay/durability/src/durability/prey/draw.rs
  - crates/gameplay/durability/src/durability/prey/expansion.rs
  - crates/gameplay/durability/src/durability/prey/loot_value.rs
  - crates/gameplay/durability/src/durability/prey/storage.rs
  - crates/gameplay/durability/src/durability/prey/types.rs
  - crates/gameplay/durability/src/durability/store/mod.rs
  - crates/gameplay/durability/src/durability/store/prey_expansion.rs
  - crates/gameplay/durability/src/durability/store/prey_purchase.rs
  - crates/gameplay/progression/src/prey.rs
  - crates/gameplay/progression/src/prey/difficulty.rs
  - crates/gameplay/spell/src/creature_damage_spell/activity_time.rs
  - crates/gameplay/spell/src/creature_damage_spell/mod.rs
  - crates/protocol-oteryn/src/prey.rs
  - crates/protocol-oteryn/src/prey_tests.rs
  - crates/session/src/capability_support.rs
  - crates/session/src/lib.rs
  - crates/session/src/session_tests_capabilities.rs
  - docs/agents/tasks/archive/PREY-DIFF-20261101.md
  - docs/balance/PREY-SLOT-VALUE-2026-11-01.md
  - docs/balance/PREY-TIME-SIM-2026-11-01.md
  - docs/contracts/protocol-oteryn/v1/prey_v1.proto
  - docs/contracts/protocol-oteryn/v1/registry/command_types/0160-prey.json
  - docs/contracts/resource-limits/entries/1692-PREY-DIFF-RL-REQUEST.json
  - docs/contracts/resource-limits/entries/1693-PREY-DIFF-RL-REPLY.json
  - docs/contracts/resource-limits/entries/1694-PREY-DIFF-RL-LIST.json
  - docs/contracts/resource-limits/entries/1695-PREY-DIFF-RL-PAGE.json
  - rulesets/economy/store/catalogue.json
  - rulesets/economy/store/prey-list-expansion.json
  - rulesets/prey/README.md
  - rulesets/prey/difficulty.json
  - tools/content-migration/prey_difficulty.py
  - tools/content-migration/regenerate_content.py
  - tools/content-migration/register_spell_families.py
  - tools/content-migration/test_monster_seven_spell_overlay.py
  - tools/content-migration/test_prey_difficulty.py
  - tools/content-migration/world_project_v2_to_tree.py
  - tools/content-schema/native-gameplay/prey-model-results.json
  - tools/content-schema/store-authoring/build_runtime_catalogue.py
  - docs/agents/tasks/PKG4P.md
```

The coordinator prepared the merge of draft `0af7c8adee0fae674dc2e91444ab83e7bbe08d69`.
Its merge base with main is `17f035e04c317803cfd1b3f410a4e1b53d3ad6f1`.
The worktree is detached, with MERGE_HEAD retained. No commit, branch publication,
real-index staging, fetch or merge is authorized for this worker.

## Review and repairs

The draft supplies five tiers, daily offer exclusion, quoted gold repeat selection,
per-race activity clocks, immutable selection measurements, and Store expansion
reservation/delivery/retry under the existing session and Character fences.

Resolved nine marker-conflicted paths. Reconciled additional marker-free duplicate
feature modules, trait methods, mailbox fields, enum variants, imports and routing
created by the additive merge. Main's capability set, extracted session Stepper,
Animus input chain and Ctrl+P shortcut are retained. Expanded UI keeps main's
SelectAny, option commands, projected candidate text and existing test assertions;
old tests are adapted to the expanded typed reply instead of removed.
The game-server qualification client is also updated to consume the confirmed
projection; its old field reference failed the focused game-server test build.

The no-stage content regeneration guard now accepts marker-free resolved files in
an unmerged index, with red/green coverage; marker-bearing and missing paths refuse.
The normal pipeline rebuilt content and model, then its pin step refused the real
unmerged index. The compiler was rerun against a temporary index snapshot of the
resolved files. Both compiler pin-refresh and pin-check passed; the real index's
SHA-256 was unchanged. No pin or generated content digest was edited manually.
The Creature-provider digest changes because generated runtime Prey rules are
now embedded; the two new managed tables account for the strict 38-to-40
registration pin update (commented in its test). World pins themselves remain
unchanged after regeneration. The world digest remains main's `e312602df8b804962542142bd8a02087b22392d187a09739a35a2a86890346fa`.

Production paths reviewed: capability admission and command160, automatic attacks,
spell damage, monster melee and native spell attacks, kill/corpse settlement,
checkpoint/expiry/rebind, durable command fencing and receipt replay, quoted fees,
and Store reservation/retained-answer/delivery/outbox recovery. Migrations0752–0755
have no number collision on the allocated main; their new tables/views/functions
revoke PUBLIC, and explicit grants are confined to runtime/control roles.
This is self-review; coordinator-owned independent frozen-head review is outstanding.

## Donor evidence and boundaries

Crystal `201189f871d23597b82caf88fd611454a2ddefb8`, class `OTS_HYPOTHESIS_ONLY`:
- `src/io/ioprey.cpp:65` / `:120`: nine-card draw and ordinary-pool eligibility.
- `src/io/ioprey.cpp:263`: expiry, automatic bonus reroll and locked repeat.
- `src/io/ioprey.cpp:314`: paid/free reroll and retained bonus.
- `src/io/ioprey.cpp:338` / `:366` / `:378` / `:400`: direct selection, bonus
  reroll, ordinary selection and option affordability.
- `src/creatures/players/player.cpp:9704`: level-based gold fee.
Five tiers, daily exclusions, paid repeat pricing and selected-race time are the
explicit draft OWNER-DEVIATION PREY-DIFF, not donor-parity claims. Existing unresolved
balance/price questions in the draft record are not treated as owner approval.

## Qualification

Focused evidence (all exit 0; full command logs are temporary worker evidence in `/tmp/pkg4p-test-*.log`):

| Check | Result / elapsed seconds |
|---|---|
| prey-units | PASS / 90.04 |
| dispatch-unit | PASS / 140.17 |
| prey_postgres_real_dispatch_draw_select_reroll_expiry_and_replay | PASS / 23.01 |
| prey_diff_set_tier_preserves_list_and_refuses_locked_or_above_level | PASS / 18.93 |
| prey_diff_offers_survive_reload_exclude_three_slots_and_reset_without_charging_exhaustion | PASS / 17.15 |
| prey_diff_pick_again_debits_once_and_refuses_unoffered_occupied_locked_or_unfunded | PASS / 16.45 |
| prey_diff_chosen_race_checkpoint_is_independent_per_slot | PASS / 16.92 |
| prey_diff_checkpoint_preserves_admitted_activity_after_premium_lapses | PASS / 15.47 |
| prey_diff_end_history_commits_measured_counters_with_transition | PASS / 14.97 |
| prey_diff_expansion_purchase_reservation_serializes_ids_and_requires_current_fence | PASS / 14.97 |
| prey_diff_expansion_purchase_release_requires_retained_definitive_refusal | PASS / 14.84 |
| prey_diff_expansion_malformed_refusal_cannot_delete_reservation_as_runtime_role | PASS / 14.85 |
| prey_diff_expansion_grant_replays_exact_receipt_and_refuses_another_operation | PASS / 21.92 |
| prey_diff_expansion_grant_rejects_forged_binding_and_stale_replay_authority | PASS / 17.39 |
| prey_diff_expansion_entitlement_receipts_and_outbox_commit_or_roll_back_together | PASS / 15.15 |
| prey_diff_expansion_survives_cold_admission_and_only_next_draw_expands | PASS / 16.69 |
| prey_diff_real_kill_values_committed_quantities_and_replay_keeps_one_history_measurement | PASS / 31.59 |
| prey_diff_store_accepted_purchase_resumes_after_catalogue_change_without_another_debit | PASS / 17.11 |
| prey_diff_store_lost_platform_answer_reconciles_original_id_after_catalogue_change | PASS / 22.92 |
| prey-storage | PASS / 19.77 |
| prey-model | PASS / 6.18 |
| bundle-inventory | PASS / 1.63 |

Final checks, all exit 0:

| Check | Elapsed seconds |
|---|---:|
| stamina-regression | 1.86 |
| world-inventory | 5.38 |
| world-materialization | 20.34 |
| architecture | 3.68 |
| check-final | 14.43 |
| clippy | 120.17 |
| fmt | 6.5 |

Additional evidence: 22 Python Prey/registration tests in 29.311s; world converter
checks/tests in 156.55s; 6 regeneration-owner fixture checks; 3 Store converter
checks in 0.18s. Prey generation check passes in 31.16s and registration byte replay
in 30.56s. Governance validation and changed-JSON parsing pass.

All 17 named Prey PostgreSQL cases used PostgreSQL18.6 (180006) and printed
`PREY_DIFF POSTGRES_CASE_COMPLETED` without `route_less`; combined elapsed 310.33s,
individual elapsed 14.84–31.59s. The two storage cases used the same configured
server. The touched Stamina case printed `POSTGRES_STARTED STAMINA-1 logout/login`.
Unit counts: 41 across the client/protocol/progression/durability/session packages,
10 dispatch live tests, 2 storage cases, 5 model tests and 1 inventory test.

The content rebuild/model ran for 511.55s before its original-index pin step refused
unmerged entries. Separate compiler-tool pin refresh passed in 21.37s and pin-check
in 32.06s using the temporary resolved index. The final main world artifact is
30,948,286 bytes with its original digest. No world-pin byte changed.

The initial PostgreSQL invocation refused the local URL's missing password segment;
its configuration was corrected without changing the harness. The initial server
test build failed on the stale qualification-client field described above; its fixed
build and both storage cases then passed. No failure was skipped or weakened.

Full suites, Windows execution, publication and independent paid review remain
outside this worker's packet. The working tree is locally validated, not frozen or
claimed mergeable. The coordinator must stage the resolved files, freeze its own
candidate and obtain required exact-head review/CI before integration.

## P03 completion limits

The ledger's P03 entry remains PARTIAL until its owning coordinator reconciles
fresh production-path evidence. This worker does not edit the re-audit ledger.
Owner balance/price decisions and Store enablement remain pending. Reward admission
still covers only a small subset of preyable creatures; accepted R4's compile-time
oversized-loot rejection is still absent (the draft retains the runtime guard).
Production node/live-session qualification and independent review are not claimed.
