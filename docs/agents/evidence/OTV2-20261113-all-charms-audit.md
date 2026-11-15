# All 25 Charms: values, effects and actual consumer coverage

Owner request: verify every charm and what it actually gives, not just Leech. PR #3183; predecessor `a997547565218b9204752a73915f52b8537c5aa7`; base `ce45063507c22a11e0db7e9c0ff86dacbe6ff834`. This is a scoped audit and repair record, **not full charm parity, merge approval or a deployment claim**.

## Evidence scope

**PROVEN (repository):** the active catalogue has 25 entries: 14 major and 11 minor. All 75 unlocked-stage combinations are checked against independent literal expected percentages and typed outcomes through the actual character-bound/race-bound evaluator. The test uses 512 deterministic occurrences per stage, checks passive-versus-triggered behaviour, stage monotonicity, repeatable draws, wrong species and generated-charm damage exclusions. Existing independent cost/currency and source-parity tests remain in place.

**DERIVED (reference behaviour):** the numerical reference is the pinned catalogue and research #3177 (`97685d43a194010559a3a92821089067dbb147a3`), with final CipSoft Winter 2024 charm values and the subsequent Summer 2025 intrinsic-critical change. The accepted external reference cut of 2026-09-27 is not silently advanced. This task did not manually replay every charm in a live Tibia Test Server client.

Primary references: CipSoft [final charm overhaul, news 8140](https://www.tibia.com/news/?id=8140&subtopic=newsarchive), [intrinsic criticals, news 8421](https://www.tibia.com/news/?id=8421&subtopic=newsarchive), and [Low Blow common AoE behaviour, news 5268](https://www.tibia.com/news/?id=5268&subtopic=newsarchive). The old critical-weapon requirement is superseded by the 5% chance / 10% extra baseline in news 8421. The teaser values in news 8115 are not substituted for the final Scavenge values.

Catalogue: `content/charms/charms-00000-00024.json`, SHA-256 `ce118bb9933ed5fae362c819aa0c0b50c043df4c9829de9ea7d03f89d73e3afe`. This documentary hash does not alter an activation pin.

## Matrix

All rows have **D**: exact definition-stage/effect contract coverage. **T** adds a production call-path trace; it is not a full-dispatch test. **R** identifies a corresponding actual runtime/PostgreSQL consumer run on the predecessor (the exact CI list is below). **F1/F2/F3** refer to the new repairs and their regressions. **G1-G5** are remaining scope gaps, not successful qualification. Values are percentages; Low Blow, Savage Blow and leech bonuses add percentage points to the relevant skill. Costs are per-stage increments: CP for majors, minor echoes for minors.

| Charm | Stage I / II / III | Costs I / II / III | Consumer status | Effect / boundary |
|---|---|---|---|---|
| Adrenaline Burst | 6 / 9 / 12% | 100 / 150 / 225 echoes | R / G3 | 10 s haste; real incoming-source test passes. Fully blocked contact and Dodge/minor ordering remain open. |
| Bless | 6 / 9 / 12% | 100 / 150 / 225 echoes | R | Death-loss reduction; real lethal snapshot, loss transaction, rollback and replay pass. |
| Carnage | 10 / 20 / 22% | 600 / 900 / 3000 CP | R / G1 | 15% killed-creature maximum HP, cap 6 x level; physical, armor/resistance response, four cardinal neighbours. Real auto kill passes; native player-spell kill hook missing. |
| Cleanse | 6 / 9 / 12% | 100 / 150 / 225 echoes | R / G3 / G5 | One eligible negative condition, not drowning; 11 s immunity of the removed condition key. Real incoming hit passes; Hex is separately owned by #3176. |
| Cripple | 6 / 9 / 12% | 100 / 150 / 225 echoes | R / G1 | 10 s paralysis through the actual creature condition owner. Real swing passes; native player-spell outgoing hook missing. |
| Curse | 5 / 10 / 11% | 360 / 540 / 1800 CP | D / T / G1 | Death proc: 5% creature maximum HP, cap 2 x level, resistance response; auto consumer exists, native player-spell hook missing. |
| Divine Wrath | 5 / 10 / 11% | 600 / 900 / 3000 CP | D / T / G1 | Holy proc: 5% creature maximum HP, cap 2 x level, resistance response; auto consumer exists, native player-spell hook missing. |
| Dodge | 5 / 10 / 11% | 240 / 360 / 1200 CP | D / T / G2 | Avoid attack; an applied consumer exists only at CreatureBite/PlayerBiteVitals. No equivalent native-source damage-spell hook was found. |
| Enflame | 5 / 10 / 11% | 400 / 600 / 2000 CP | D / T / G1 | Fire proc: 5% creature maximum HP, cap 2 x level, resistance response; auto consumer exists, native player-spell hook missing. |
| Fatal Hold | 30 / 45 / 60% | 100 / 150 / 225 echoes | R / G1 | Suppress fleeing for 30 s, not movement/root. Real swing changes native flee behaviour; player-spell hook missing. |
| Freeze | 5 / 10 / 11% | 320 / 480 / 1600 CP | D / T / G1 | Ice proc: 5% creature maximum HP, cap 2 x level, resistance response; auto consumer exists, native player-spell hook missing. |
| Gut | 6 / 9 / 12% | 100 / 150 / 225 echoes | R | Relative creature-product chance bonus, including nested products; not a blanket bonus to every loot entry. Real death/loot test passes. |
| Low Blow | 4 / 8 / 9% | 800 / 1200 / 4000 CP | F1 / G4 | Additional critical chance in percentage points. Real assigned-race/direct-spell projection now includes innate criticals, shares one AoE draw and leaves other species unchanged. |
| Numb | 6 / 9 / 12% | 100 / 150 / 225 echoes | R / G3 | 10 s paralysis of the attacking creature. Real incoming-source test passes; zero-damage/contact ordering remains open. |
| Overflux | 5 / 10 / 11% | 600 / 900 / 3000 CP | D / T / G1 | 2.5% caster maximum mana, cap 8% creature maximum HP; physical proc ignores ordinary resistance/armor. Native player-spell hook missing. |
| Overpower | 5 / 10 / 11% | 600 / 900 / 3000 CP | D / T / G1 | 5% caster maximum HP, cap 8% creature maximum HP; physical proc ignores ordinary resistance/armor. Native player-spell hook missing. |
| Parry | 5 / 10 / 11% | 400 / 600 / 2000 CP | R / G3 | Reflect original incoming magnitude using the current accepted physical/armor policy. Native-source damage-spell and replay tests pass; modern mitigation order and Mana Drain are not promoted to proven Global rules. |
| Poison | 5 / 10 / 11% | 240 / 360 / 1200 CP | D / T / G1 | Earth proc: 5% creature maximum HP, cap 2 x level, resistance response; auto consumer exists, native player-spell hook missing. |
| Savage Blow | 20 / 40 / 44% | 800 / 1200 / 4000 CP | F1 / G4 | Additional critical extra damage in percentage points. Actual assigned-race/direct-spell projection now augments innate critical damage without requiring Strike. |
| Scavenge | 60 / 90 / 120% | 100 / 150 / 225 echoes | R | Relative skinning success bonus, capped at certainty; 120% means +120% of the base probability, not a 120% success probability. Native minted-corpse use and replay pass. |
| Vampiric Embrace | 1.6 / 2.4 / 3.2% | 100 / 150 / 225 echoes | F2 / F3 / G4 | Additional life-leech rate in percentage points; only assigned species and an existing underlying skill. Enrage, overkill, Prey exclusion and replay are covered; secondary-auto projection corrected separately. |
| Void Inversion | 20 / 30 / 40% | 100 / 150 / 225 echoes | R | Invert the qualified incoming Mana Drain amount into mana, bounded by player maximum; real source/glyph and empty-mana/replay paths pass. |
| Void's Call | 0.8 / 1.2 / 1.6% | 100 / 150 / 225 echoes | F2 / F3 / G4 | Additional mana-leech rate in percentage points; only assigned species and an existing underlying skill. Enrage and actual joined HP/MP installation are covered; secondary-auto projection corrected separately. |
| Wound | 5 / 10 / 11% | 240 / 360 / 1200 CP | D / T / G1 | Physical proc: 5% creature maximum HP, cap 2 x level; current accepted definition checks resistance but not armor. Native player-spell hook missing. |
| Zap | 5 / 10 / 11% | 320 / 480 / 1600 CP | D / T / G1 | Energy proc: 5% creature maximum HP, cap 2 x level, resistance response; auto consumer exists, native player-spell hook missing. |

## Repairs and reproduced failures

### F1: innate criticals must remain eligible for Low Blow / Savage Blow

`attack/charm_combat.rs::bonuses` previously skipped any zero equipment field, even though `EquippedOffence::player_critical` supplies 5%/10% intrinsically. A new real-assignment test failed with Low Blow I: `[10000, 10000]` instead of `[11000, 10000]` for the assigned and unrelated species. The shared auto/spell projection now permits critical bonuses independently of equipment-only fields, while preserving the no-underlying-leech rule. All three stages of both critical charms are tested with the unrelated target evaluated first and exactly one common critical draw. The old equipment-only assertion was replaced by the exact newly required `(400,400,0,0)` additions and `(900,1400)` total criticals; no inequality or skipped test masks it.

### F2: preserve native Enrage before HP clipping

`spell_leech_projection.rs::owner_damage_basis` reuses `SoulpitCreatureTag::incoming_health_change`, rather than cloning its thresholds. The spell preflight supplies the actual staged effect receipt, including preceding same-batch HP changes; DamageWithHealing uses capped post-heal HP before the callback. The auto drain calls the same projection beside the actual committed receipt. New tests use a real Instance runtime, native Soulpit boss spawn, actual player payment, physical/player joined commit and replay.

RED: at 30% boss HP, a 100-damage request becomes 60 physical damage. A 16% life skill plus a 16% mana skill with Void's Call III incorrectly produced player vitals `(36,18)` instead of `(30,11)` after payment. GREEN cases cover 30/50/70/90% HP bands, Damage Prey, lethal clipping, healing before damage and no duplicate recovery. No new persistence field, authority bypass or post-commit spell heal was introduced.

### F3: secondary-auto minor Leech must not be zeroed before eligibility

The accepted D186 contract already permits Vampiric Embrace and Void's Call on secondary auto targets. The drain zeroed the base skills before evaluating the bonus, and the existing-skill guard then suppressed the minor too. The production projection is extracted as `committed_hit_bonuses`: calculate against the genuine underlying skills first, then retain only the bonus for an off-target hit. The regression uses live assignments for every stage of both minors, lethal and nonlethal receipts, no base skill and an unrelated species. It does not claim to create a missing diamond-arrow engine or fix every secondary critical path.

### Existing PostgreSQL expectation aligned with the accepted Leech rule

On the predecessor, `delayed_strikes_postgres_outburst_partial_harmony_no_repeat` failed with `424` versus `425` at `delayed_strikes.rs:929`. Its expected mana recovery still rounded to nearest after the accepted implementation switched to per-contribution ceiling. The exact equality now uses independent integer ceiling; payment/replay assertions and the no-cost delayed callback remain intact. The actual PostgreSQL rerun on the successor is still required; a route-less local return is not proof.

## Remaining gaps: do not mark all charms complete

**G1 — PROVEN call-path gap:** `CharmHookEvent::committed_hit` and outgoing `ProcDamage`, `PreventCreatureFlee`, offensive paralysis and `AreaDamageAroundKill` consumers are in `gameplay_transport/attack/drain.rs`. The direct player-spell pipeline wires passive critical/leech projections but not this general outgoing/kill hook. This affects Wound, Poison, Freeze, Zap, Curse, Enflame, Divine Wrath, Overpower, Overflux, Cripple, Fatal Hold and Carnage for native/ordinary player casts. A valid evaluator output or an auto-attack test does not close the gap. Follow-up must join effects, replay protection, source attribution and kill credit through the existing owners, not append an unguarded after-commit callback.

**G2 — PROVEN boundary gap:** the live `DodgeAttack` consumer is `PlayerBiteVitals::dodge_creature_attack`, called by `creature_bite::apply_bite`. The native-source `CharmSourceVitals` damage-spell adapter has no equivalent attack-avoidance hook. Dodge is therefore not qualified for every creature attack path. The precise native melee-profile coverage also needs its own real-dispatch regression; do not generalise from the CreatureBite path.

**G3 — UNKNOWN/CONFLICT edge semantics:** the positive-damage and fully mana-shielded source paths are covered by existing incoming tests, but zero-damage/fully blocked contacts, Dodge/minor ordering and modern Parry mitigation/Mana Drain are not promoted to proven rules. Current `incoming_conditions` suppresses magnitude zero; a new contact owner would need a real replay-safe occurrence rather than fabricated one-point damage.

**G4 — explicit limits:** direct spells and current auto-owner projections are covered. Delayed/DoT/chain/PvP/summon special cases, diamond-arrow secondary target execution and complete secondary critical behaviour are not universally qualified by this patch. F3 repairs the reachable cleave minor projection only; it does not widen this claim.

**G5 — ownership:** Cleanse Hex work remains with #3176. Same-type immunity refresh and all future Test Server changes require separate evidence; this task does not duplicate that work.

## Concrete consumer paths

- Offensive proc/kill/flee/cripple: `crates/gameplay/dispatch/src/gameplay_transport/attack/drain.rs`; response/blocking: `attack/charm_combat.rs::damage_amount` and `paralyse_creature`.
- Incoming effects and death snapshot: `attack/charm_combat/source_vitals.rs`, `attack/player_block.rs`, `actor_conditions.rs::apply_player_charm_condition`, `attack/charm_combat.rs::capture_death_charm`.
- Gut: `gameplay_transport/kill_reward.rs::gut_bonus_hundredths` call and `apply_gut_to_loot`; only `creature_product` entries, nested children included.
- Scavenge: `crates/gameplay/durability/src/durability/item_craft/skinning.rs`; actual SQL assignment/unlock lookup bound to corpse race, native corpse provenance, exact relative chance and atomic spent-corpse/output transaction.
- Critical/leech direct-cast preparation: `ordinary_combat.rs`, `native_combat_cast/prepare.rs`, `native_combat_cast.rs`, `spell_leech_projection.rs` and `actor_spell_commit.rs`.

## Actual predecessor CI evidence

Run `43508976394`, exact head `a997547565218b9204752a73915f52b8537c5aa7`. All 24 partition logs were read; the relevant statuses below come from executed nextest jobs, not a missing-database early return. There were **22 successful and 2 failing partitions overall**, so this run is not a green merge gate.

```text
JOB 132180078531 PR early signal / Durability PostgreSQL harness (6) overall=success
PASS [  39.509s] (  6/213) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::srx2_charm_consumers::srx2_charm_void_inversion_native_source_mana_commit_and_replay
JOB 132180078542 PR early signal / Durability PostgreSQL harness (9) overall=success
PASS [  38.075s] (  3/215) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::srx2_charm_consumers::srx2_charm_parry_native_source_spell_damage_and_replay
JOB 132180078567 PR early signal / Durability PostgreSQL harness (21) overall=success
PASS [  55.475s] ( 11/213) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::srx2_charm_consumers::srx2_charm_cleanse_incoming_hit_removes_one_negative_condition
JOB 132180078567 PR early signal / Durability PostgreSQL harness (21) overall=success
PASS [  80.054s] (120/213) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::item_spell_augments::item_spell_augments_postgres_damage_critical_leech_and_replay
JOB 132180078567 PR early signal / Durability PostgreSQL harness (21) overall=success
PASS [  56.471s] (149/213) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::srx2_charm_consumers::srx2_charm_bless_lethal_snapshot_atomic_loss_rollback_and_replay
JOB 132180078598 PR early signal / Durability PostgreSQL harness (10) overall=success
PASS [  75.939s] (  1/215) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::item_spell_augments::item_spell_augments_postgres_equipped_healing_cooldown_unequip_and_replay
JOB 132180078598 PR early signal / Durability PostgreSQL harness (10) overall=success
PASS [  41.645s] (  8/215) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::srx2_charm_consumers::srx2_charm_incoming_hit_hastes_player
JOB 132180078599 PR early signal / Durability PostgreSQL harness (5) overall=success
PASS [  35.052s] (  3/215) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::w22a_loot::w22b_gut_widens_creature_product_rolls_on_the_real_kill_path
JOB 132180078599 PR early signal / Durability PostgreSQL harness (5) overall=success
PASS [  32.327s] (113/215) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::equipped_offence::equipped_offence_postgres_real_swings_critical_hit_and_leech
JOB 132180078610 PR early signal / Durability PostgreSQL harness (11) overall=success
PASS [  38.635s] (153/219) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::srx2_charm_consumers::srx2_charm_numb_incoming_hit_paralyses_attacker
JOB 132180078617 PR early signal / Durability PostgreSQL harness (7) overall=success
PASS [  52.537s] (157/218) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::srx2_charm_consumers::srx2_charm_fatal_hold_real_swing_prevents_native_flee
JOB 132180078624 PR early signal / Durability PostgreSQL harness (18) overall=failure
PASS [  78.973s] (127/221) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::item_spell_augments::dmg2_postgres_equipment_spell_leech_without_augment_and_replay
JOB 132180078628 PR early signal / Durability PostgreSQL harness (12) overall=success
PASS [  42.444s] (  5/222) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::srx2_charm_consumers::srx2_charm_cripple_real_swing_slows_living_target
JOB 132180078693 PR early signal / Durability PostgreSQL harness (19) overall=success
PASS [  23.216s] (151/218) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::srx2_scavenge::srx2_scavenge_production_use_minted_corpse_atomic_replay_postgres
JOB 132180078733 PR early signal / Durability PostgreSQL harness (23) overall=success
PASS [  39.673s] (152/215) oteryn-gameplay-dispatch gameplay_transport::monk_save::postgres_tests::srx2_charm_consumers::srx2_charm_carnage_real_lethal_swing_and_adjacent_damage
```

The other failure was job `132180078624` (partition 18): `player_death_nested_maximum_501_item_reset_makes_bounded_atomic_progress`, `Unavailable(CommitRejected)`. Its root cause is not established here. The fact that its implementation paths are unchanged does not prove it is a flake or permit ignoring the failing check. The Outburst failure was job `132180079003` (partition 14), diagnosed above.

## New tests and validation

New tests are in `crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/all_charms_contracts.rs`. Despite the directory name they are actual in-memory owner tests and do not require PostgreSQL. `build.rs` mounts this file into the production dispatch test target; no new standalone or ignored test target was created.

```sh
cargo test --offline --locked -p oteryn-gameplay-dispatch --all-features --lib all25_
cargo test --offline --locked -p oteryn-gameplay-content -p oteryn-gameplay-domain -p oteryn-gameplay-progression -p oteryn-gameplay-dispatch --all-features --lib charm
cargo test --offline --locked -p oteryn-gameplay-combat-rules -p oteryn-gameplay-progression -p oteryn-gameplay-spell -p oteryn-gameplay-dispatch --all-features --lib
cargo clippy --offline --locked -p oteryn-gameplay-combat-rules -p oteryn-gameplay-progression -p oteryn-gameplay-spell -p oteryn-gameplay-dispatch --all-features --all-targets -- -D warnings
cargo fmt --all --check
rustfmt --edition 2024 --check crates/gameplay/dispatch/src/gameplay_transport/postgres_tests/all_charms_contracts.rs
python3 tools/agents/validate_governance.py
python3 -m unittest discover -s tools/agents/tests
```

Final successor results are bound in the PR exact-head comment after publication. Local PostgreSQL administrator routing is absent and is not counted as database coverage. The two reviewed P1 findings are addressed by F1/F2; the actual-assignment regression also closes the relevant test blind spot. The remaining review P2 cases (real omitted-immune-target lowering and retained pending-transaction recovery rather than a vector clone) stay on the grouped coverage backlog. No merge or deployment is performed by this audit.

## Verified local successor tree

The final authoring source, unchanged through commit, passed:

- `all25_`: 5 new tests, including the 75-stage / 512-occurrence matrix and F1/F2/F3 regressions.
- Focused `charm` suites: 104 passed, 0 failed.
- Affected libraries: 86 + 3538 + 173 + 333 = **4130 passed, 0 failed, 16 pre-existing ignored**.
- `cargo clippy ... --all-features --all-targets -- -D warnings`: pass.
- Workspace fmt, explicit fmt of the auto-mounted test file and `git diff --check`: pass.
- Governance validator: pass; governance unittest suite: 129 tests, OK.

The focused counts overlap the full library run and are not added to it. No new
ignored test was introduced. This remains local qualification without a database
route; real PostgreSQL results above apply only to predecessor `a997547565218b9204752a73915f52b8537c5aa7`.
The successor's full SHA and fresh CI/review state are recorded in the PR.
