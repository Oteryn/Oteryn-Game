# W01a / native melee evidence for PR 2766

This is the AI/melee portion of the implementation evidence. It does not replace the spell, Item or PostgreSQL evidence owned by the other implementation packets. Test names below identify regressions; their presence is not a claim that the latest combined packet has passed.

**Implementation and verification status:** the AI, session-damage, shared melee and position code is complete, including the House-exit translation stamp. The final eight-library run passed on `9c72fa2558915c417831d3d8d374320e70a3d9fa`; actual package counts and the separate PostgreSQL proof are recorded below.

## Pinned donors

- Primary: [Crystal 201189f871d23597b82caf88fd611454a2ddefb8](https://github.com/zimbadev/crystalserver/tree/201189f871d23597b82caf88fd611454a2ddefb8).
- Secondary: [Canary 47dfd51f45280a59a1d3e50ba7edd573d7234446](https://github.com/opentibiabr/canary/tree/47dfd51f45280a59a1d3e50ba7edd573d7234446).

`monster.cpp` below means `src/creatures/monsters/monster.cpp` at the stated pin. The primary file was re-read at the exact pin while preparing this table.

## Implemented rules and regression evidence

| Rule | Crystal evidence | Oteryn owner and regression |
|---|---|---|
| Wild creatures flee at `health <= runAwayHealth`; intrinsic monster summons do not use the wild flee/retarget path. An existing active forced-target/distance hold suppresses fleeing. If an existing owner raises HP above the threshold, ordinary melee eligibility returns; this does not activate automatic monster self-healing. | `monster.cpp:1001–1003`, wild timed retarget `1295–1349`. | `crates/gameplay/monster-ai/src/ai_think/summon_plan.rs`; `think_tests/native_parity.rs`: `native_summon_does_not_flee_or_run_wild_timed_target_changes`, `native_fleeing_keeps_ranged_spells_and_returns_to_fighting_after_heal`. |
| Fleeing suppresses melee only. Other due attack entries continue. | `monster.cpp:1204–1205,1263–1268`. | `ai_think/profile_schedule.rs`, `ai_monster_melee.rs`; `native_melee_clock::fleeing_skips_only_melee_and_preserves_pending_extra_for_healing`. This follows the primary donor despite the task's broader phrase “no attack”. |
| Native strategy weights use a fixed percentile: nearest, health, damage, then random remainder. Health input is live absolute HP. The literal primary damage selection omits the first candidate from comparisons and keeps it as fallback. | `monster.cpp:771–789,853–895`. | `ai_think/targeting.rs`, `summon_plan.rs`, dispatch `monster_ai_cycle.rs::think_candidates`; `native_percentile_strategy_retains_crystal_remainder_and_damage_quirk`, `monster_candidates_read_live_health_and_refuse_dead_or_stale_players`. |
| Damage strategy reads the current attacker's contribution to this particular creature, fenced by current session and lease. Reconnect cannot inherit the old actor's damage subtotal. Reward totals remain unchanged. | `monster.cpp:885–888` uses this creature's `damageMap` keyed by creature actor ID. | Existing `crates/gameplay/entities/src/carrier/damage_ledger.rs` gains a session subtotal/read view; `targeting::with_current_damage` joins exact current actors. Tests: `current_session_ai_damage_shares_are_creature_scoped_and_reconnect_fenced`, `current_session_damage_reconnect_resets_ai_without_losing_rewards`, `current_session_damage_does_not_invent_a_session_for_unsequenced_credit`, `current_session_subtotal_does_not_change_reward_winner`. No second damage ledger. |
| Native projection preserves every authored attack/defence slot and source index. One unresolved callback does not erase resolved neighbors. Zero-damage and multiple melee entries remain admitted. Exact duplicate Ability bodies deduplicate; conflicting bodies refuse. | Authored list iteration `monster.cpp:1201–1234`; source creature definitions are the admission input. | `ai_think/behaviour_profile.rs`, `native_profile.rs`; `native_missing_callback_does_not_erase_known_attack_or_change_source_index`, `native_malformed_unknown_entry_is_still_refused`, `native_melee_source_entries_may_be_multiple_or_have_zero_direct_damage`, `admitted_native_profiles_keep_every_source_attack_and_defence_slot`. The census collects all refused keys before failing and still checks every successful source slot. |
| Repeated summon names retain separate authored schedule slots/chances. | `data-crystal/monster/quests/cults_of_tibia/bosses/eshtaba_the_conjurer.lua:62–70`: five Dread Minion entries with 20/30/20/30/20 percent chances. | `ai_think/behaviour_profile/summon_projection.rs`; `native_repeated_summon_names_retain_distinct_authored_slots_and_chances`. Strict caller-authored admission keeps its prior unique-name contract. |
| A native melee entry authored in the defence list cannot invalidate the creature's ordinary attacks. Its slot, self-targeted proposal and defence clock remain intact; executing defence effects remains excluded under the accepted scope decision. | Both `data-crystal/monster/reptiles/seacrest_serpent.lua:111–116` and the corresponding `data-global` file contain zero-damage melee in defenses. | `ai_think/behaviour_profile.rs`, `profile_schedule.rs`; `native_defence_melee_retains_its_slot_without_disabling_attack_schedules`. The actual admitted Seacrest record has five attack slots and two defence slots. Strict caller-authored defence-melee rejection is unchanged. |
| Attack facing uses the dominant axis; diagonal ties preserve the source's prior-direction rules. New creatures start south. | `monster.cpp:2467–2523`; `src/creatures/creature.hpp:806`. | `ai_think/attack_facing.rs`, `creature_steps/creature_step_timer.rs`; `crystal_attack_facing_uses_dominant_axis_and_prior_diagonal_direction`. The existing spell reader consumes this state. |
| Melee and spells share the existing attack ticks. Waiting authored entries retain the clock; completed rounds reset it. Native melee consumes the recorded source-index selection and does not independently redraw chance or add a cooldown. | `monster.cpp:1188–1242,1276–1285`. | `ai_think/profile_schedule.rs`, `ai_monster_melee.rs`; `melee_participates_in_the_same_longest_attack_window_as_spells`, `native_short_interval_uses_the_shared_think_without_a_second_1500ms_cooldown`. |
| A due out-of-reach melee grants an extra swing; a merely waiting interval does not. Target disappearance resets attack ticks and grants extra. | `monster.cpp:306–308,1230–1232,1276–1291`. | Same schedule owner; `only_a_due_out_of_reach_melee_grants_an_extra_swing`, `disappearing_target_resets_the_clock_and_grants_extra_on_reacquisition`, `disappearance_extra_reaches_vitals_before_the_previous_authored_interval_expires`. |
| The 1500 ms check is measured from the last extra attempt, not from every ordinary cast. While extra is pending even an intervening non-melee entry updates that timestamp. A failed extra chance stays pending; a selected melee clears extra. | `monster.cpp:1222–1225,1270–1285`. | `profile_schedule.rs`; `threshold_is_since_the_last_extra_attempt_not_each_regular_cast`, `a_failed_extra_chance_remains_pending_and_refreshes_its_timestamp`, `fleeing_skips_only_melee_and_preserves_pending_extra_for_healing`. |
| All selected melee indices have independent occurrence receipts. Native bite construction requires immutable source proof and binds exact issuer, target, session, sequence, source index and digest. Protection/current actor checks remain in the effect owner. | Source ordered casts `monster.cpp:1201–1234`; Oteryn's existing commit/replay authority contract remains mandatory. | `ai_monster_melee.rs`, combat `ability/creature_bite.rs`, `creature_auto_attack.rs`, domain `melee_definition.rs`, dispatch `monster_ai_cycle.rs`; `native_melee_entries_have_independent_receipts_in_one_committed_think`, `native_source_proof_refuses_modified_public_source_fields_and_definition`, `scheduled_bite_definition_is_bound_to_its_exact_occurrence_and_target_session`. |
| A zero melee roll does not install attached melee conditions. The zero entry remains selected/receipted without a vitals write. | `src/creatures/combat/combat.cpp:1077–1080` returns from melee condition application when both resulting damage values are zero. | Existing zero bite path retained; `native_zero_melee_is_retained_without_damage_or_a_second_replay`. No invented zero-damage condition callback. |
| Movement of issuer or held target may retry pending extra through interval zero; it neither advances nor resets attack ticks and does not run defence, summon or non-melee schedules. Time alone and unrelated movement cannot redraw it. | `src/creatures/creature.cpp:488–495`, `src/game/game.cpp:6996–7000`, `monster.cpp:1195–1196,1276–1285`. | `ai_think/movement_attack.rs`, `profile_schedule.rs`, dispatch `monster_ai_cycle.rs`. Tests: `movement_interval_zero_keeps_both_clocks_and_skips_spells_defences_and_summons`, `target_or_issuer_translation_retries_pending_extra_before_next_think`, `out_and_back_translation_is_observed_once_but_unrelated_motion_is_not`, `movement_does_not_grant_extra_before_the_source_interval_becomes_due`. |
| Position translation is distinguished from turning in place; an out-and-back translation is still observable. Current sequence/replay is shared, while full-think and legacy swing deadlines remain unchanged. | The source trigger is `onCreatureMove`, not turning (`creature.cpp:488–495`). | Existing position owner exposes a derived `last_translation_revision()`; AI stores exact snapshots and compares that stamp. `turning_either_actor_in_place_never_wakes_a_pending_extra_swing`. No new counter, timer or movement ledger. |
| A House exit that changes Channel coordinates or floor also updates the derived translation stamp. Returning to the same stored Channel cell preserves it. | Existing position-owner contract, covered by the accepted Q5A extension. | `crates/gameplay/entities/src/carrier/house_custody.rs`; `house_exit_updates_translation_stamp_only_for_coordinate_or_floor_changes` uses actual pinned movement and custody commits. The merge audit at main `a29cedbe74c0f1cb1f87334e5102907d9de97d4e` found this existing direct revision writer; no House mechanic, authority or counter is added. |
| Movement retries require both actors' exact positions to belong to the current Content pin. A synthetic/foreign context on either actor cannot create an attack wake. | Oteryn's existing current-owner Content/position authority contract; no donor context is fabricated. | `ai_think/movement_attack.rs`; `a_foreign_position_context_on_either_actor_cannot_wake_native_melee` covers foreign issuer, foreign target and both. Native owner fixtures use the existing pinned player initializer and a parameterized pinned creature test helper; legacy synthetic fixtures stay separate. |
| Commit normal selection N before preparing movement selection N+1. Native melee from actual due thinks is consumed even between compatibility swing passes. | Ordered direct casts in `monster.cpp:1201–1234` and subsequent queued `onAttacking(0)`. | Dispatch takes actual `run_due_current_thinks` actors, executes their normal batch, then prepares/executes movement retries. `regular_melee_selection_can_be_consumed_before_a_second_range_grants_extra` is an explicit scheduler fixture; it is not claimed as an admitted two-range creature because native `MeleeSource` currently requires range 1. |

## Health multiplier boundary

Crystal scales initial current HP, max HP and flee threshold from the same type multiplier (`monster.cpp:52–54`). `src/creatures/monsters/monsters.cpp:330–343` chooses the boss or ordinary configured health rate. The admitted Oteryn runtime has no corresponding global rate owner. The coordinator's **ANSWER A** keeps the admitted **1x** policy and leaves configurable rates to W14a. The implementation therefore uses the authored threshold without deriving a multiplier from mutable maximum HP, forge state or unrelated values. This is an explicit accepted boundary, not a claim that arbitrary configured donor rates are implemented.

## Verified secondary difference

Crystal `monster.cpp:876–890` initializes `mostDamage` to zero only after advancing past the first candidate. Canary `monster.cpp:1021–1038` initializes from the first candidate's actual damage and faction offset. The implementation deliberately follows Crystal's literal first-candidate exclusion, per primary-donor priority. The percentile/quirk regression gives the first candidate damage 1000 and later candidates damage 1, and expects the later candidate.

The compared fleeing predicate is identical in Canary `monster.cpp:1460–1462`; its extra/1500 ms/range logic at `2109–2138` also matches the cited Crystal rule. Canary `1754–1766` routes attacks through combat intentions; no Canary scheduling architecture was imported into Oteryn.

## Accepted scope decisions

- **Health rates:** admitted 1x; configurable boss/ordinary rates deferred to W14a.
- **Think callbacks:** coordinator ANSWER A (comment 6543173780). Preserve source bindings; do not create an empty callback hook or a Lua interpreter. Type-level `onThink` override (`monster.cpp:1099–1122`) and non-overriding CreatureEvent callbacks remain distinct future typed Interaction/Encounter admission work.
- **Encounter attack callbacks:** the 18 admitted Encounter entries keep explicit refusal, per coordinator ANSWER A. The dedicated census/refusal test belongs to the production-test packet; this AI evidence does not claim execution of those encounters.
- **Defence actions:** separate explicit Left out under coordinator ANSWER A (comment 6543503050). Automatic ordinary self-heal, haste and invisibility are not newly activated. Projection, clocks and proposals are preserved; their presence does not claim production execution of those defence effects. Ordinary attack execution stays required.
- **Translation observation:** coordinator ANSWER A (comment 6543529998) authorizes the minimal existing-position-owner derived fact. No event interpreter, second movement store, counter or timer is introduced.
- **Network spell presentation:** coordinator ANSWER A (comment 6543702206) keeps cross-client projectile/impact activation in SPELL-PRESENT-0. Retained source presentation facts do not prove a running publisher, new wire capability or playable FX. The existing HP publication path below is a separate, already active path.
- **Damage distribution:** coordinator ANSWER A (comment 6544219482) retains the existing deterministic uniform damage policy and shared randomness. Crystal truncated-normal damage-distribution parity is explicitly Left out; source values, chances, intervals and one decision/replay remain required.
- Respawn W01b remains out of scope. Existing actor, session, content, protection and effect-commit fences are retained.

## Left out: Crystal truncated-normal damage sampling (Q7 A)

[Q7, PR comment 6544184071](https://github.com/Oteryn/Oteryn-Game/pull/2766#issuecomment-6544184071) received **ANSWER A in [6544219482](https://github.com/Oteryn/Oteryn-Game/pull/2766#issuecomment-6544219482)**. The existing deterministic uniform damage policy is retained; shared randomness is unchanged. **Crystal truncated-normal damage-distribution parity is explicitly Left out.** Source values and schedules, bounded chance, and one decision/replay remain required. The previous six A decisions are unchanged; all seven questions are answered A.

Crystal `201189f871d23597b82caf88fd611454a2ddefb8` uses a different distribution: [`src/utils/tools.cpp:442–450`](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/utils/tools.cpp#L442-L450) draws a normal value with mean 0.5 and standard deviation 0.25, **redraws values outside [0,1]**, then sorts the bounds and returns the minimum plus `std::lround` of the scaled span. This is rejection sampling of a truncated normal, not endpoint clipping. [`src/creatures/combat/combat.cpp:211–219`](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/creatures/combat/combat.cpp#L211-L219) applies that sampler; [`src/creatures/monsters/monster.cpp:2457–2464`](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/creatures/monsters/monster.cpp#L2457-L2464) only supplies the damage bounds.

The uniform melee expression predates this PR: [`ai_monster_melee.rs:164–166` at a52549e3](https://github.com/Oteryn/Oteryn-Game/blob/a52549e3b9230c7486a24df66f4e8ec59d1f96cb/crates/gameplay/monster-ai/src/ai_monster_melee.rs#L164-L166) already uses `minimum + draw % width`. For the admitted bounds this agrees with the existing [`crates/gameplay/domain/src/domain/uniform_draw.rs:3–14`](https://github.com/Oteryn/Oteryn-Game/blob/a29cedbe74c0f1cb1f87334e5102907d9de97d4e/crates/gameplay/domain/src/domain/uniform_draw.rs#L3-L14), also used by other current combat owners. Thus this is a pre-existing project projection newly applied to more admitted attacks, rather than a newly introduced sampling-law regression. Matching bounds, intervals, chances or deterministic replay does **not** establish distribution parity.

Earlier policy documents establish the history, rather than distribution equivalence: [`OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md:79`, D12](https://github.com/Oteryn/Oteryn-Game/blob/a29cedbe74c0f1cb1f87334e5102907d9de97d4e/docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md#L79) assigns damage distribution to a world combat rule, and [`217–222`](https://github.com/Oteryn/Oteryn-Game/blob/a29cedbe74c0f1cb1f87334e5102907d9de97d4e/docs/architecture/OTERYN_MONSTER_AUTHORING_SCHEMA_V1.md#L217-L222) records the donor's truncated-normal law. The accepted [`ATTACK0 decision:4–6`](https://github.com/Oteryn/Oteryn-Game/blob/a29cedbe74c0f1cb1f87334e5102907d9de97d4e/docs/architecture/reviews/OTERYN_GAME_ATTACK0_ATTACK_TARGET_AND_AUTO_ATTACK_DECISION_2026-09-30.md#L4-L6) accepts its implemented slice while carrying the uniform-distribution issue into the `ATTACK-PARITY-1` backlog. Q7 A now explicitly authorizes retention for this task with that limitation. Nonidentical random streams alone would understate the remaining difference in distribution.

## Completed canonical Rat movement proof

`crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/monster_attacks.rs::rat_source_melee_dispatch_damages_player_once_and_checkpoints_vitals_postgres` includes the `rat_movement_retry` helper. The root reported this extended case passing in the completed six-case monster PostgreSQL run at its `a629` checkpoint. This is an executed production-dispatch proof, not just an owner fixture; the shared verification record supplies the complete revision and command.

The helper preserves canonical Rat bounds **0–8**, interval **2000 ms** and chance **100%**. Actual qualified movement keeps the held target out of melee reach while ordinary thinks advance the source clock. Moving only that target into reach then triggers the existing drain before the AI-pass, legacy-swing and full-think deadlines. Assertions require the source occurrence to advance once, select the held player, retain all three deadlines and the global think sequence, and consume the movement observation. An unchanged repeated drain preserves HP and the selected occurrence. The extra's HP delta is bounded to **0–8**: a zero roll is valid, so this test does not claim a strictly positive hit on that particular extra attempt. Separate native owner regressions verify positive damage commits and replay. The containing Rat case also verifies ordinary damage and the durable vitals checkpoint.

## Final verification and recorded checkpoints

On merged runtime `9c72fa2558915c417831d3d8d374320e70a3d9fa`, the complete AI library passed
**135 tests, zero failed, 4 ignored**, in **8.97s**.
It includes the complete 1870-profile census, Eshtaba repeated summons, Seacrest
Defence-melee preservation, shared clocks, target strategies, interval-zero
movement, out-and-back translation, turn-only suppression and foreign-context refusal.
The entities library passed **196 tests, zero failed**, in
**3.24s**, including
`house_exit_updates_translation_stamp_only_for_coordinate_or_floor_changes` and
`current_session_ai_damage_shares_are_creature_scoped_and_reconnect_fenced`.

The explicitly invoked actual-manifest melee census passed **one test, zero failed,
zero ignored**, in **28.17s**, resolving 1689 melee entries, 24 multi-melee
profiles and 13 zero-damage entries. The default ignored count above is not counted
as execution; this named gate ran separately with `--ignored`. Complete source
projection does not mean every creature ran through PostgreSQL. The canonical Rat
case and its movement extension passed in the separate six-case database run at
`a629951e40886b1d32143f3799bbbae43e5e7a3a`. Exact commands, all package results and
the documentation-only final delta are recorded in README and the PR.

Relevant existing crate commands from the repository root (the final combined commands and executed counts are in the shared README/PR):

```sh
cargo test --locked -p oteryn-gameplay-monster-ai --all-features --lib
cargo test --locked -p oteryn-gameplay-entities --all-features --lib
```

The existing full-melee gate requires the actual regenerated manifest and explicit ignored-test selection:

```sh
OTERYN_FULL_SPELL_TEST_MANIFEST="$PWD/content/spells.manifest.json" cargo test --locked -p oteryn-gameplay-monster-ai --all-features --lib ai_monster_melee::tests::native_tests::actual_full_admitted_melee_sources_resolve_every_authored_index -- --ignored --exact --nocapture
```

The completed House-exit implementation is covered by the entities library command above. The exact standalone selection of that existing test is recorded for reference, not as an additional test requirement:

```sh
cargo test --locked -p oteryn-gameplay-entities --all-features --lib carrier::house_custody::translation_tests::house_exit_updates_translation_stamp_only_for_coordinate_or_floor_changes -- --exact
```

## Existing health publication path

The Oteryn line references in this trace were inspected at main `1d292a25fc2e7db466e5893ff69c6716870d6d38`. The named functions are the lookup anchors after merging later main changes; the line references do not claim final-branch line numbers.

Own-player health needs no new monster-specific network bridge. The melee and generic source-damage adapters commit into the existing `ChannelSpellStates` owner (`crates/gameplay/dispatch/src/gameplay_transport/actor_spell.rs::apply_creature_damage_native`, `PlayerLethalVitals::apply_attack_damage` and the composite-condition adapter). A real hit advances `PlayerSpellState`'s vitals revision; replay returns its existing receipt without another mutation.

The production chain is:

1. `crates/gameplay/dispatch/src/gameplay_transport/composed_admission/vitals.rs:23–34`, `tick_actor_vitals`, drains monster attacks on the existing cadence.
2. `crates/gameplay/dispatch/src/gameplay_transport/connection/authority/composed.rs:704–709,773–778` delegates `observe_vitals` and `tick_vitals` to that owner.
3. `crates/gameplay/dispatch/src/gameplay_transport/connection/serve/refresh.rs:5–45`, `serene`, calls `tick_vitals` and then independently `observe_vitals`. It compares the actual revision with the last transmitted revision, builds the existing `ACTOR_VITALS` delta, writes the frame and updates connection continuity. A different connection's prior monster drain is therefore still observed; a regen result is not needed to publish damage.
4. `crates/session/src/domain_dispatch.rs` applies the existing actor-vitals domain. Existing tests cover unsolicited idle publication and coalesced revisions: dispatch `connection/tests/spell_vitals.rs::the_serene_cadence_publishes_a_changed_vitals_delta`, `connection/tests/pending_spell.rs::vitals_publication_coalesces_real_changes_from_last_transmitted_revision`, and session `session_tests_push.rs::a_pushed_vitals_delta_during_idle_applies_and_is_queued_once`. These are traced existing regressions, not a claim that this worker executed them.

Creature observer bars also already read current owner HP. `crates/gameplay/dispatch/src/gameplay_transport/composed_admission/world_map.rs:41–68` obtains `companion_snapshot` health and maximum health. `crates/gameplay/dispatch/src/gameplay_transport/world_spatial.rs:469–492` compares health percentages separately from movement revisions and emits stationary actor updates, using the existing snapshot fallback at the delta ceiling. `connection/serve/refresh.rs:336–373` runs this visibility publication. Existing regressions are `forge_creature_stationary_health_is_sent_once_without_a_position_revision` and `forge_creature_health_updates_resync_at_the_existing_delta_ceiling` in `world_spatial.rs`.

**Preexisting observer boundary:** `composed_admission/world_map.rs:60–68` currently tries `companion_snapshot` for every actor and falls back to 100 percent for player slots. Consequently a player's own `ACTOR_VITALS` changes correctly, while that player's health percentage shown to other observers remains at 100. Root explicitly kept this preexisting general player-bar projection outside the W01 owner change. No fix or full observer-player HP parity is claimed here.
