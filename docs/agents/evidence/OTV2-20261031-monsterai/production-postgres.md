# Monster AI production and durability evidence

## Execution status

The complete post-fix run on runtime [a629951e40886b1d32143f3799bbbae43e5e7a3a](https://github.com/Oteryn/Oteryn-Game/commit/a629951e40886b1d32143f3799bbbae43e5e7a3a), including merged main `045b51c2e`, recorded **6 passed, 0 failed, 0 ignored, in 316.50 seconds**. All six distinct `MONSTER_AI POSTGRES_CASE_COMPLETED` markers were observed, including `monster_attacks_rat` and `monster_attacks_wall_removal`; no case took the no-database skip path. This run proves the newest Rat movement extension and The Monster's actual durable removal alongside Hunter, Fire Devil, Fire Elemental, and Firestarter.

The preceding run on `64de132c6` recorded **5 passed, 1 failed, in 291.62 seconds**, with The Monster reaching its final assertion without a removal receipt. The root's isolated diagnostic reproduction recorded **0 passed, 1 failed, in 71.93 seconds**: 36 selected source casts all returned `Err(Rejected("Destroy current SourceMap tile"))`. Commit `a629951e40886b1d32143f3799bbbae43e5e7a3a` corrected the existing Map qualification guard. The post-fix six-case pass above validates that correction without changing source values, fixture positions or assertions; only a three-line explanatory fixture comment was added. The confirmed defect and preserved guards are recorded below.

An earlier baseline run recorded **5 passed, 0 failed, in 279.48 seconds**. That historical run predates both the Rat movement extension and the sixth removal case. All these runs used the isolated PostgreSQL 18.6 cluster and the required, dedicated `oteryn_test_admin` role with the database guard preserved. Passing monster cases reached their completion markers rather than the no-database skip path. All execution results in this record come from the root; the cross-checks below do not claim additional test runs.

## Reproduction

Run from the repository root with the existing `OTERYN_TEST_POSTGRES_ADMIN_URL` set to the isolated, dedicated test-admin connection:

```bash
cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib monster_attacks:: -- --nocapture --test-threads=1
```

The root controls `CARGO_TARGET_DIR`, toolchain 1.99.1, and build serialization. The test harness creates and cleans its own test database. Each actual completion prints `MONSTER_AI POSTGRES_CASE_COMPLETED <tag>`; absence of the admin setting instead prints an explicit `POSTGRES_CASE_SKIPPED` and must not count as PostgreSQL evidence.

For the existing real-manifest Encounter refusal regression:

```bash
cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib monster_actual_encounter_attacks_remain_eighteen_explicit_refusals -- --ignored --nocapture
```

That command also needs the root's existing `OTERYN_FULL_SPELL_TEST_MANIFEST` setting. Its execution result is recorded by the root separately.

## Six source-backed production cases

All six use `activated_loop_monster(world, false)`, the admitted native generation, canonical creature bodies, current runtime and owner fences, ordinary Channel dispatch, and the existing PostgreSQL harness. Creature damage bounds, intervals, and chances remain unchanged. The test player's health pool is increased where necessary to survive bounded area, field, or removal attempts. The removal case also prepares a wall through the existing typed Item writer, as described below.

Donor links in this table are pinned to Crystal commit `201189f871d23597b82caf88fd611454a2ddefb8`.

| Test | Execution status | Pinned donor source | Assertions |
|---|---|---|---|
| `rat_source_melee_dispatch_damages_player_once_and_checkpoints_vitals_postgres` | Passed on `a629951e4088`, including the movement extension | [rat.lua:83–85](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-crystal/monster/mammals/rat.lua#L83-L85): 2,000 ms, 100%, melee damage 0–8 | Ordinary scheduling damages the admitted player. Held-target translation then wakes exactly the next occurrence before the existing deadlines, without advancing the global AI sequence. Repeated draining preserves selection and HP. The current session-fenced writer saves actual HP/MP; an unchanged retry preserves the durable write order. |
| `hunter_source_arrow_dispatch_damages_distant_player_and_checkpoints_vitals_postgres` | Passed on `a629951e4088` | [hunter.lua:101–104](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-crystal/monster/humans/hunter.lua#L101-L104): arrow at source index 1; 2,000 ms, 50%, damage 0–100, range 7 | Source spell damage reaches the player while the actors remain beyond melee reach. The case checks source bounds, no repeated damage before the deadline, and the durable HP/MP checkpoint. |
| `fire_devil_source_area_dispatch_damages_target_and_bystander_postgres` | Passed on `a629951e4088` | [fire_devil.lua:91–95](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-crystal/monster/demons/fire_devil.lua#L91-L95): target-centered fire areas with radii 4 and 2; 2,000 ms, 20% and 15%, damage 60–90 and 20–50 | Source area casts damage both admitted players across the bounded attempts. The caster remains beyond melee reach of the first player, whose actual HP/MP are saved by the existing durability owner. This does not require both players to be hit by one cast. |
| `fire_elemental_source_field_dispatch_commits_owned_ground_once_postgres` | Passed on `a629951e4088` | [fire_elemental.lua:80–84](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-crystal/monster/elementals/fire_elemental.lua#L80-L84): fire field at index 2; 2,000 ms, 25%, range 7, radius 1, `target = true` | Ordinary dispatch reaches the existing source Item transaction. SQL joins the exact Creature receipt, Item operation, minted Item, Ground row, and audit outbox. It checks i2118, quantity 1, live lifecycle, native occurrence provenance, and unchanged identity/revision on immediate repeated drain. |
| `firestarter_zero_range_source_field_dispatch_commits_distant_ground_postgres` | Passed on `a629951e4088` | [firestarter.lua:96–100](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-crystal/monster/humanoids/firestarter.lua#L96-L100): fire field at index 2; 2,000 ms, 10%, radius 1, `target = true`, range omitted | The existing owner commits with canonical range 0 while caster and target occupy different cells. The case checks persisted provenance, physical Ground custody, audit receipt, and repeat-drain invariants. |
| `the_monster_source_removal_dispatch_retires_owned_wall_once_postgres` | Passed on `a629951e4088` after the Map qualification correction | [the_monster.lua:86–90](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-crystal/monster/bosses/the_monster.lua#L86-L90) and [destroy_magic_walls.lua:9–24](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-crystal/scripts/spells/monster/destroy_magic_walls.lua#L9-L24): source index 2; 1,000 ms, 50%, one eligible top item in the caster's 5 × 5 area | A guarded typed-writer fixture supplies i2130. Ordinary monster dispatch must produce its removal receipt, operation, and audit entry; set lifecycle 2, quantity 0, revision 2; remove the Ground row; and preserve these results on repeated drain. |

The zero-range rule is pinned separately: [monsters.hpp:40–48](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/creatures/monsters/monsters.hpp#L40-L48) defaults range to 0; [Monster::canUseSpell:1288–1292](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/creatures/monsters/monster.cpp#L1288-L1292) applies the distance bound only when range is nonzero.

## Durability and Map defects found by the real PostgreSQL runs

1. `game_character_last_positions.write_sequence` is `NUMERIC`. The new test initially decoded it as `i64`; its bounded fixture query now uses `write_sequence::bigint`. This correction changes the test query only.
2. `game_item_ground_locations.corpse_ref` is `NOT NULL BYTEA`, with an allowed length of 1–128 bytes in the unchanged `0010_item_mint_ground.sql`. Existing Creature field preparation supplied an empty `Vec`, causing PostgreSQL error `23514` and rolling back the complete receipt, Item, and outbox transaction. It now passes the already validated native `cause.occurrence` bytes into `SpellGroundTarget::placement(source_reference)`. Existing canonical occurrence validation bounds this reference and preserves Creature provenance. Its format does not grant Player loop-drop authority.
3. Existing `CreateItem` preparation also treated source range 0 as a zero-cell distance cap. It now retains exact source range equality and applies the distance bound only for nonzero range. The unchanged Firestarter source case exercises this rule through the actual durable owner and passed after the correction.
4. The real removal case exposed an existing Map qualification defect. Its source scan is centered at `(2, 12, -7)` and includes known static solid cells `(2, 10, -7)`, `(3, 10, -7)`, and `(3, 14, -7)`. `prepare_creature_source_destroy_in_transaction` reads all 25 tiles before selecting the eligible top item at `(1, 12, -7)`. `qualified_creature_source_tile_in_transaction` incorrectly rejected every movement-blocking owner as `MissingDynamicProjectileFlags`, including Source tiles with already qualified collision flags. The actual diagnostic observed 36 selected casts, all rejected as `Destroy current SourceMap tile`; the 1,000 ms / 50% source schedule was running. The correction in `a629951e4088` applies the existing player helper's distinction: reject `owner.blocks_movement(position)` only for `WorldStaticOwnerBorrow::Entry(_)`. It preserves refusal for Entry objects with insufficient dynamic collision facts and allows known Source/Bundle flags to reach the unchanged source algorithm. CreateItem still rejects solid, floor-change and protection-zone destinations; Destroy still checks the actual top stack, occupancy, provenance, current transaction and scope. No fixture relocation, source value or SQL guard was changed. Temporary adapter logging was restored byte-for-byte after diagnosis. The complete post-fix six-case PostgreSQL run passed, including removal through the unchanged scan and source recipe.

These production corrections touch `crates/gameplay/durability/src/durability/creature_source_items.rs` and `crates/gameplay/durability/src/durability/native_world_read.rs`. Shared SQL physical writers, existing IDs/markers, schemas, and merged migrations were preserved.

## Rat movement extension passed on `a629951e4088`

The added helper performs actual native Movement steps and two complete source Thinks after a successful canonical melee. It establishes a three-cell gap before each Think, rereads actual positions, and asserts no out-of-range HP change. It then moves only the held player adjacent before all three existing deadlines (shared pass, compatibility swing, full Think).

The passing assertions require the production drain to select exactly the next per-creature occurrence while preserving the held target, all three deadlines, and the global AI sequence. Canonical Rat damage remains 0–8, including a legitimate zero roll; repeated polling preserves the selection and HP. The existing actor position owner supplies the movement observation. Separate AI owner tests use fixed positive damage to check the effect receipt; this PostgreSQL extension checks the actual dispatch wake and callsite. These assertions passed in the complete post-fix six-case run.

Donor event chain: [creature.cpp:488–495](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/creatures/creature.cpp#L488-L495) responds to issuer/held-target movement with extra-swing scheduling; [game.cpp:6996–7000](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/src/game/game.cpp#L6996-L7000) invokes `onAttacking(0)`.

## Deliberate limits

The cases exercise source data through real production owner composition. They do not claim wire projectile/effect activation. Coordinator Q6A keeps that separate because the existing presentation publisher has no production consumer; no bounded but undrained queue was added.

The historical five-case baseline does not cover the Rat extension or removal; the complete post-fix six-case run covers both. A repeated connection drain differs from separately injecting the exact same durable proposal: the former is asserted here, while source Item receipt reconciliation remains in the existing guarded owner.

## Existing EQMOD regression

On the same runtime `a629951e40886b1d32143f3799bbbae43e5e7a3a`, the `equipment_mods::` filter recorded **2 passed, 0 failed, 0 ignored, in 98.45 seconds**. This comprises **one actual PostgreSQL case and one Content-only regression**, not two PostgreSQL cases.

`eqmod2_postgres_content_cleave_reflection_perfect_shot_and_amplification` emitted `PARTY-XP-1 POSTGRES_DATABASE_STARTED case=party_xp_14`, then `PARTY-XP-1 POSTGRES_CASE_COMPLETED case=party_xp_14 elapsed_seconds=53.17`, and its test result was `ok`. It exercises real fenced equipment reads and combat owners, including primary-only leech with cleave, reflection while mana shield absorbs player damage, perfect-shot damage, reflection replay handling, and durable reflected-kill rewards without duplicate award receipts. The completion marker is emitted before the scenario's result is returned; the recorded passing test result is therefore part of this evidence.

`eqmod2_content_forge_tiers_and_modifiers_are_projected` is the separate Content-only test. It checks the projected equipment modifiers and forge tiers, including rejection of tier 11. Its existing Cave Rat flee-threshold fixture overlay remains coherent with the exact bound Behavior in the attached attack closure; no source validation or assertion was removed.

```bash
cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib equipment_mods:: -- --nocapture --test-threads=1
```

## Existing Map and LOOP regressions passed on `e4fde5be5`

The four existing LOOP PostgreSQL scenarios recorded **4 passed, 0 failed, 0 ignored, in 183.03 seconds** on runtime [e4fde5be5f675c3139fd99d3e080d6d43092d6d5](https://github.com/Oteryn/Oteryn-Game/commit/e4fde5be5f675c3139fd99d3e080d6d43092d6d5). All four emitted the configured `server_version_num=180006` marker. Together with the six monster cases and the one EQMOD PostgreSQL case, this record contains **11 actual PostgreSQL scenarios**. The separate EQMOD Content-only test is excluded from that count.

The earlier actual PostgreSQL launch of `current_owner_summon_transaction_map_and_one_pass_use_actual_item_scope` on runtime `a629951e40886b1d32143f3799bbbae43e5e7a3a` reported configured `server_version_num=180006`, then **0 passed, 1 failed, in 12.48 seconds**. It failed during `activation_fixture.rs::activate` with `Content(InvalidArtifact("native monster attack Creature and Behavior binding"))`. That failed launch did not reach the transaction-abort assertions; the four-case rerun above supplies the passing evidence after fixture repair.

The existing `loop_monster_input(true)` fixture replaces Rat's Creature abilities and Behavior attacks, defenses and summons. Its attached attack closure still held the original canonical root bodies, so strict Content validation correctly rejected the mismatch. The applied repair explicitly synchronizes only those fixture roots and copies the selected real inline Item Ability with its matching source reference into the retained Rat closure. Production activation and validation are unchanged. The missing-child variant retains its intentionally absent Snake Behavior and explicitly absent execution closure, then synchronizes Rat's updated summon binding; its existing negative assertions require a runtime registration refusal with no leaked child or AI registration. The six canonical monster cases use `summons = false` and are unaffected by this fixture-only repair.

A bounded source cross-check found no additional current-HP or session fixture change necessary. The shared-budget phase intentionally supplies no target before player-state initialization. All three summon variants then initialize a live Monk state for the actual fresh actor/session before ordinary dispatch; the existing swing pass wakes idle creatures from those qualified current targets. The replay-only variant retires the owner only in its explicit stale-owner check, after its live replay and transaction checks.

The four passing cases cover source-home walkback, SQL cancellation recovery and leash behavior; shared-budget resumption, one-pass summon registration and the live cap; rejection of the inner catalogue, accepted outer-source replay and stale-owner refusal; and missing-child rollback without physical or AI leaks. The three summon variants also exercise current physical transaction/context authority and invalidation of cached tile facts after `SELECT 1/0`. The main Map case additionally proves that a real Item MINT with unqualified movement facts invalidates a former empty-tile observation. These assertions do not exercise an already populated delayed-spell reader bank. The summon variants retain their preexisting synthetic 1,000 ms / 100% fixture schedule and maximum of two summons; they are registration and authority regressions, not additional canonical Rat attack-value tests.

```bash
cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib postgres_tests::loop_monster:: -- --nocapture --test-threads=1
```

## Cross-check of production paths

### RemoveItems production regression: failure reproduced and corrected

The root requested a separate proof for the durable removal transition. Searches of mounted PostgreSQL tests and the source Item entry points found no existing case exercising `creature_source_items` removal through the ordinary monster drain. Existing Ground-capacity and Destroy Field rune cases exercise the shared SQL writer or the player rune path.

`the_monster_source_removal_dispatch_retires_owned_wall_once_postgres` reuses the same admitted generation, Channel owner, and PostgreSQL harness. Its child module is `postgres_tests/monster_attacks/removal.rs`. The canonical source is [The Monster attack index 2](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-crystal/monster/bosses/the_monster.lua#L86-L90), with an interval of 1,000 ms, a chance of 50%, and the exact [top-item removal rule](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-crystal/scripts/spells/monster/destroy_magic_walls.lua#L9-L24).

Setup uses the existing typed Spell Item SQL writer to mint one i2130 Wild Growth with the current Item fence, actual native Item policy, source collision flags, and qualified source decay chain. The setup Spell and cost are explicitly SQL-writer fixture data; they do not claim a real player rune cast. Database triggers and authority guards remain enabled. The canonical monster body and chances remain unchanged. Its other two [source attacks](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-crystal/monster/bosses/the_monster.lua#L87-L88) are bounded by 2,800 melee plus 1,200 energy damage per pass. The test player's health covers all 64 bounded attempts.

The ordinary drain must then produce a Creature removal receipt, Item line, and matching audit outbox entry. The Item must have lifecycle 2, quantity 0, and revision 2, with no remaining Ground row. A repeated drain must preserve these records and the original MINT provenance. Fire Elemental's i2118 is not an eligible removal input: neither [Anomaly's ordered item list](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-crystal/scripts/spells/monster/anomaly_break.lua#L1-L22) nor [Destroy Magic Walls' barrier list](https://github.com/zimbadev/crystalserver/blob/201189f871d23597b82caf88fd611454a2ddefb8/data-crystal/scripts/spells/monster/destroy_magic_walls.lua#L1-L24) includes it.

```bash
cargo test --locked -p oteryn-gameplay-dispatch --all-features --lib the_monster_source_removal_dispatch_retires_owned_wall_once_postgres -- --nocapture --test-threads=1
```

This case exposed and reproduced the Map qualification defect recorded above, then passed in the complete post-fix PostgreSQL run on `a629951e40886b1d32143f3799bbbae43e5e7a3a`. The canonical 5 × 5 scan, source values, seeded Item and assertions remain unchanged. The passing case verifies the removal receipt and operation, retired Item state, absent Ground custody, retained original MINT provenance, and unchanged results on an immediate repeated drain.

### Generator, qualification and condition cross-check

The root supplied the current generator scripts for this cross-check. `bind_monster_source_definitions.py` indexes the full `(family, key, revision)` identity, rejects duplicate or incomplete canonical roots and missing executable bodies, and recursively includes only Ability, Effect, and Formula dependencies. It preserves the admitted Creature and Behavior roots and refuses a closure larger than the existing limit of 128 records. It introduces no new damage, range, timing, chance, or Encounter rules. The seven retained noncanonical Canary and familiar identities stay on their existing qualification path. `register_spell_families.py` performs the final binding after the existing creature policy overlays and updates the native manifest's artifact hash.

The current `qualified_melee` iterates every authored attack index, retains an existing balance binding only on its exact matching slot, and validates agreement between the native source and definition before the existing bite owner. Native zero-damage entries use the source-qualified constructor. Current role checks admit ordinary monsters and intrinsic monster summons. The real-manifest melee census asserts that all 1,689 native melee entries resolve; its execution result is tracked by the root separately from this cross-check.

Reclassification of the supplied complete set of 116 refused spell occurrences found exactly 62 `CreateItem`, 20 `RemoveItems`, 16 existing composed sources with multiple effects, and 18 Encounter bindings. Every occurrence resolved to one exact root Ability. The 18 approved exclusions contain no unclassified ordinary single-effect shape. Existing source Item and composed owners handle the first 98 occurrences; the explicit regression over 18 tuples covers the remaining Encounter boundary.

The existing condition path is connected: `composed_admission/vitals.rs::tick_actor_vitals` calls `tick_spell_periodic`, which reads the current standing tile under the session and scope fence, then rechecks the full position before dispatching ticks originating from a Creature. `actor_spell.rs::tick_source_player_conditions` stages the canonical `ConditionStore`, checks the session, control, and reservations, prepares any death before publication, and commits HP and the condition cursor together. Unknown dynamic field facts outside a protection zone leave damage pending. Existing focused tests in `actor_spell_owned_source_tick_tests.rs` cover replay at the same time, failed death mint preserving the predecessor, malformed field provenance, protection-zone behavior, and stale predecessor rejection. This cross-check does not claim an extra test run.

`last_position.rs` tracks save deadlines by time and compares both the cell and vitals. Its periodic save therefore captures changed HP/MP for a stationary player as well as a moving one. Condition ticks use that existing session-fenced checkpoint path; no second HP writer or condition timer was introduced.
