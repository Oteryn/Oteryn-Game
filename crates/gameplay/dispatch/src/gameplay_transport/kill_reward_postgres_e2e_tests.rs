//! Kill → reward end to end on PostgreSQL: a held auto-attack target is driven by the real
//! `drain_auto_attacks` to a lethal swing, which queues the kill, and the real
//! `drain_kill_rewards` settles it durably (XP row and a linked corpse with its loot) through
//! the composed owner. The only fabricated inputs are the clock (`clock_origin` in the past, then
//! advanced between swings) and the session fixture; the content is the committed production
//! manifest's generation (the production rat: 5 XP, `oteryn:loot.creature.rat`).
//!
//! Nested under the monk-save PostgreSQL tests to reuse their fresh-admission fixture.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

use super::fixture::{Harness, debug, id, run};
use super::{TestResult, prepare_current_session};
use crate::gameplay_transport::connection::{
    AdmittedSession, ControllerBinding, FirstEntryOutcome, SessionContinuity,
};
use crate::gameplay_transport::kill_reward::DrainOutcome;
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::time::Duration;
use tokio::sync::Mutex;
use {
    crate::foundation::{
        ChannelId, ChannelRuntimeV1, GameSessionId, MovementLocalPosition, NodeId, WorldId,
    },
    oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin,
};

/// Thalom cells next to each other, neither in a protection zone.
const OPEN_PLAYER: MovementLocalPosition = MovementLocalPosition {
    x: 2,
    y: 11,
    floor: -7,
};
const OPEN_RAT: MovementLocalPosition = MovementLocalPosition {
    x: 1,
    y: 11,
    floor: -7,
};

/// One owner swing interval is far below this; every pass of the loop advances the owner clock
/// by it, so each `drain_auto_attacks` finds the next swing due.
const PASS: Duration = Duration::from_secs(3);

#[path = "kill_reward_postgres_fixture/scenario_thread.rs"]
pub(super) mod scenario_thread;

#[path = "kill_reward_postgres_fixture/summer_settlement.rs"]
mod summer_settlement;
use summer_settlement::qualify_summer_settlement;

#[test]
fn auto_attack_kill_settles_xp_and_a_linked_corpse_with_loot_on_postgres() -> TestResult {
    scenario_thread::run("kill-reward-e2e", scenario)
}

fn scenario() -> TestResult {
    scenario_with_decay(None)
}

pub(super) fn scenario_with_decay(reconcile: Option<bool>) -> TestResult {
    scenario_with_charms(reconcile, None)
}

type CharmSamples = Vec<(i64, u32)>;
type KillScenarioSettings<'a> = (
    Option<(&'a str, &'a mut CharmSamples)>,
    Option<oteryn_gameplay_content::content::BossPolicy>,
    Option<&'a str>,
);

#[test]
fn charm_wire_missing_snapshot_preserves_combat_and_assignment_changes_damage_and_dodge()
-> TestResult {
    scenario_thread::run("charm-wire-combat", || {
        let mut plain = Vec::new();
        let mut wound = Vec::new();
        let mut dodge = Vec::new();
        let mut parry = Vec::new();
        scenario_with_charms(None, Some(("", &mut plain)))?;
        scenario_with_charms(None, Some(("wound", &mut wound)))?;
        scenario_with_charms(None, Some(("dodge", &mut dodge)))?;
        scenario_with_charms(None, Some(("parry", &mut parry)))?;
        // Route-less execution is handled by the shared harness; it supplies no samples.
        if plain.is_empty() {
            return Ok(());
        }
        assert!(
            plain.iter().any(|(outgoing, _)| *outgoing > 0),
            "no snapshot still deals damage: {plain:?}"
        );
        assert!(
            plain.iter().any(|(_, incoming)| *incoming > 0),
            "no snapshot still takes damage: {plain:?}"
        );
        assert!(
            wound.iter().zip(&plain).any(|(a, b)| a.0 > b.0),
            "assigned Wound adds damage"
        );
        assert!(
            dodge.iter().zip(&plain).any(|(a, b)| a.1 < b.1),
            "assigned Dodge cancels damage"
        );
        assert!(
            parry.iter().zip(&plain).any(|(a, b)| a.0 > b.0),
            "assigned Parry reflects creature damage"
        );
        assert!(wound.iter().zip(&plain).all(|(a, b)| a.0 >= b.0));
        assert!(dodge.iter().zip(&plain).all(|(a, b)| a.1 <= b.1));
        Ok(())
    })
}

fn scenario_with_charms(
    reconcile: Option<bool>,
    charm: Option<(&str, &mut CharmSamples)>,
) -> TestResult {
    scenario_with_creature_charmed(
        reconcile,
        false,
        None,
        false,
        false,
        false,
        (charm, None, None),
    )
}

pub(super) fn scenario_with_capacity(reconcile: Option<bool>, capacity: bool) -> TestResult {
    scenario_with_creature(reconcile, capacity, None, false)
}

#[test]
fn summer_radiant_all_six_execute_attacks_through_the_production_owner() -> TestResult {
    scenario_thread::run("summer-radiant-attacks", || {
        for name in [
            "acolyte",
            "inquisitor",
            "paragon",
            "templar",
            "warden",
            "zealot",
        ] {
            scenario_with_creature(None, false, Some(name), false)?;
        }
        Ok(())
    })
}

#[test]
fn summer_acolyte_physical_spell_hits_at_range_through_the_production_owner() -> TestResult {
    scenario_thread::run("summer-acolyte-physical-spell", || {
        scenario_with_creature_at_range(None, false, Some("acolyte"), false, true, false)
    })
}

#[test]
fn summer_acolyte_ice_area_hits_multiple_players_through_the_production_owner() -> TestResult {
    scenario_thread::run("summer-acolyte-ice-area", || {
        scenario_with_creature_at_range(None, false, Some("acolyte"), false, true, true)
    })
}

#[test]
fn summer_paragon_holy_target_hits_at_range_through_the_production_owner() -> TestResult {
    scenario_thread::run("summer-paragon-holy-target", || {
        scenario_with_creature_at_range(None, false, Some("paragon"), false, true, false)
    })
}

#[test]
fn summer_paragon_fire_area_hits_multiple_players_through_the_production_owner() -> TestResult {
    scenario_thread::run("summer-paragon-fire-area", || {
        scenario_with_creature_at_range(None, false, Some("paragon"), false, true, true)
    })
}

#[test]
fn summer_templar_energy_area_hits_multiple_players_through_the_production_owner() -> TestResult {
    scenario_thread::run("summer-templar-energy-area", || {
        scenario_with_creature_at_range(None, false, Some("templar"), false, true, true)
    })
}

#[test]
fn summer_templar_holy_target_hits_at_range_through_the_production_owner() -> TestResult {
    scenario_thread::run("summer-templar-holy-target", || {
        scenario_with_creature_at_range(None, false, Some("templar"), false, true, false)
    })
}

#[test]
fn summer_radiant_all_six_kills_settle_and_decay_on_postgres() -> TestResult {
    scenario_thread::run("summer-radiant-settlements", || {
        for name in [
            "acolyte",
            "inquisitor",
            "paragon",
            "templar",
            "warden",
            "zealot",
        ] {
            scenario_with_creature(None, false, Some(name), true)?;
        }
        Ok(())
    })
}

#[test]
fn summer_corpse_stages_recover_under_a_new_process_and_owner_generation() -> TestResult {
    scenario_thread::run("summer-corpse-handoff", || {
        scenario_with_creature(Some(true), false, Some("warden"), true)
    })
}

fn scenario_with_creature(
    reconcile: Option<bool>,
    capacity: bool,
    radiant: Option<&str>,
    settle_radiant: bool,
) -> TestResult {
    scenario_with_creature_at_range(reconcile, capacity, radiant, settle_radiant, false, false)
}

fn scenario_with_creature_at_range(
    reconcile: Option<bool>,
    capacity: bool,
    radiant: Option<&str>,
    settle_radiant: bool,
    ranged_spell: bool,
    elemental_area: bool,
) -> TestResult {
    scenario_with_creature_charmed(
        reconcile,
        capacity,
        radiant,
        settle_radiant,
        ranged_spell,
        elemental_area,
        (None, None, None),
    )
}

fn scenario_with_creature_charmed(
    reconcile: Option<bool>,
    capacity: bool,
    radiant: Option<&str>,
    settle_radiant: bool,
    ranged_spell: bool,
    elemental_area: bool,
    settings: KillScenarioSettings<'_>,
) -> TestResult {
    let (charm, boss_policy, daily_target) = settings;
    let holy_target =
        matches!(radiant, Some("paragon" | "templar")) && ranged_spell && !elemental_area;
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "kill_reward_e2e", false, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;

        // The production generation, activated for the fixture's World on the Thalom source
        // world (its tiles carry the protection-zone flags the swing reads).
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let channel = ChannelId::decode(&id(43)).map_err(debug)?;
        let session = GameSessionId::decode(&id(50)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                world,
                Some(include_bytes!(
                    "../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
                )),
            );
        let active = controller.active().expect("activated generation");
        let native = active.native_gameplay().expect("native gameplay state");
        let room = pin.qualified_room().clone();
        // The Thalom entry start is a protection-zone cell, where no swing is legal: enter the
        // Channel on the open cell beside it instead (as the owner auto-attack tests do).
        let channel_pin = ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            room.map_revision_digest(),
            (OPEN_PLAYER.x, OPEN_PLAYER.y, OPEN_PLAYER.floor),
        );

        // ATTACK-2: the swing weapon is read from the equipment owner, which checks the
        // Channel's durably active Content against the pin (the activation `serve` records) and
        // needs its empty seed (as fresh admission leaves it; no item worn, so fists).
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)")
            .bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, $3::text::numeric(20,0), NULL, $4, $5, $6)")
            .bind(id(42).as_slice())
            .bind(id(43).as_slice())
            .bind(pin.activation_sequence().to_string())
            .bind(pin.identity().server_artifact_digest().as_slice())
            .bind(pin.identity().client_artifact_digest().as_slice())
            .bind(pin.frame_binding().digest().as_slice())
            .execute(&harness.pool)
            .await?;

        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            if ranged_spell { 66 } else { 2 },
            channel_pin.clone(),
        )
        .map_err(debug)?;
        native
            .install_companion_policies(&mut runtime)
            .map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        // A2: the admitted session's Character lease bound to its slot, as first entry does; the
        // swing's attributed damage commit reads it.
        runtime
            .bind_attacker_lease(
                actor,
                session,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    // The fresh session's current Character lease generation.
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        let creature_key = radiant.map_or_else(
            || {
                if charm.is_some() {
                    "oteryn:creature.cave_rat"
                } else {
                    "oteryn:creature.rat"
                }
                .to_owned()
            },
            |name| format!("oteryn:creature.radiant_{name}"),
        );
        let expected_xp = if daily_target == Some("creature") {
            10
        } else if settle_radiant {
            native
                .reward_table()
                .row(&creature_key)
                .map_err(debug)?
                .xp_amount
                .get()
        } else {
            5
        };
        if settle_radiant {
            // Official balance override (tibia.com news 8935), content/creatures/official-balance.json.
            let exact = match radiant {
                Some("acolyte") => 37_245,
                Some("inquisitor") => 36_680,
                Some("paragon") => 39_301,
                Some("templar") => 39_008,
                Some("warden") => 40_160,
                Some("zealot") => 39_799,
                _ => return Err("unknown Summer Radiant".into()),
            };
            assert_eq!(expected_xp, exact, "{creature_key} exact native XP");
        }
        if settle_radiant {
            // Select a positive real occurrence without changing a drop probability.
            // Roll back unused player reservations to advance the reusable creature
            // slot's generation (player slot 1, creature slot 2), then spawn and kill that actual native creature.
            let reward = native.reward_table().row(&creature_key).map_err(debug)?;
            let mut generation = None;
            for candidate in 1..=20_000 {
                let death = oteryn_gameplay_items::loot_plan::LootPlanDeathKey::new(
                    id(42),
                    id(43),
                    1,
                    2,
                    candidate,
                );
                let plan = oteryn_gameplay_items::loot_plan::plan_creature_loot(
                    death,
                    &reward.loot_table_ref,
                    &reward.loot_table,
                )?;
                if !plan.entries.is_empty() {
                    generation = Some(candidate);
                    break;
                }
            }
            for _ in 1..generation.ok_or("positive bounded native loot occurrence")? {
                let unused = runtime
                    .reserve_fresh_session(GameSessionId::decode(&id(245)).map_err(debug)?)
                    .map_err(debug)?;
                runtime
                    .rollback_definitely_uncommitted(unused)
                    .map_err(debug)?;
            }
        }
        if let Some(policy) = boss_policy {
            use crate::durability::boss_encounter::{BossClockClaim, BossClockKey};
            if matches!(
                policy,
                oteryn_gameplay_content::content::BossPolicy::AtReset { .. }
            ) {
                sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_10',1,clock_timestamp(),'2026-10')")
                    .bind(id(42).as_slice()).bind([1u8;32].as_slice()).execute(&harness.pool).await?;
                sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
                    .bind(id(42).as_slice())
                    .execute(&harness.pool)
                    .await?;
            }
            sqlx::query("INSERT INTO game_boss_spawn_clocks VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'boss_spawn1_fixture:0',0,1,0,0,NULL)")
                .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
            use oteryn_world_bundle::spawn::{Creature, Direction, Period, Point, Source, Table};
            let table = Table {
                creatures: vec![Creature {
                    key: creature_key.clone(),
                    period: Period::All,
                }],
                sources: vec![Source {
                    key: "boss_spawn1_fixture".into(),
                    floor: 7,
                    x: 1,
                    y: 11,
                    points: vec![Point {
                        creature: 0,
                        x: 1,
                        y: 11,
                        direction: Direction::North,
                        respawn_ms: 0,
                        boss_policy: Some(policy),
                    }],
                }],
            };
            let boss_world = crate::map::boot::tests::door_world()?.with_spawns_for_test(table);
            let spawns = boss_world
                .boss_spawns()
                .into_iter()
                .map(
                    |(key, creature, (x, y, floor), policy)| crate::foundation::BossSpawn {
                        key,
                        creature,
                        position: MovementLocalPosition { x, y, floor },
                        policy,
                        actor: None,
                        cycle: 0,
                        pending_claim: false,
                    },
                )
                .collect();
            runtime.configure_boss_spawns(spawns).map_err(debug)?;
            let claim = harness
                .root
                .activate_boss_clock(
                    &authority,
                    &harness.node,
                    BossClockKey {
                        world,
                        channel,
                        spawn: "boss_spawn1_fixture:0".into(),
                    },
                    runtime.binding().scope_generation(),
                    policy,
                )
                .await?;
            let BossClockClaim::Claimed(clock) = claim else {
                return Err("DUE boss not claimed".into());
            };
            let before = runtime.read_actor_position(actor).map_err(debug)?;
            runtime
                .borrow_movement_position()
                .commit_cardinal(before, OPEN_RAT)
                .map_err(debug)?;
            assert!(
                runtime
                    .realize_boss_spawn("boss_spawn1_fixture:0", clock.cycle)
                    .map_err(debug)?
                    .is_none(),
                "an occupied tile retains the committed claim"
            );
            assert!(runtime.boss_spawns()[0].pending_claim);
            let before = runtime.read_actor_position(actor).map_err(debug)?;
            runtime
                .borrow_movement_position()
                .commit_cardinal(before, OPEN_PLAYER)
                .map_err(debug)?;
            assert!(
                runtime
                    .realize_boss_spawn("boss_spawn1_fixture:0", clock.cycle)
                    .map_err(debug)?
                    .is_some()
            );
            assert!(
                runtime
                    .realize_boss_spawn("boss_spawn1_fixture:0", clock.cycle)
                    .is_err()
            );
        } else {
            runtime
                .realize_native_qualification_spawn(channel_pin, &creature_key, OPEN_RAT)
                .map_err(debug)?;
        }
        if ranged_spell {
            // Unrelated committed sessions must not turn the single target cast into
            // an oversized area roster. They have no spell state or attack eligibility.
            for tag in 100..164 {
                let extra_session = GameSessionId::decode(&id(tag)).map_err(debug)?;
                let reservation = runtime
                    .reserve_fresh_session(extra_session)
                    .map_err(debug)?;
                let extra = runtime.commit_fresh_session(reservation).map_err(debug)?;
                runtime
                    .initialize_first_entry_position(extra)
                    .map_err(debug)?;
            }
        }
        if boss_policy.is_some() {
            let boss = runtime.boss_spawns()[0].actor.ok_or("boss actor")?;
            assert_eq!(
                runtime.creature_spawn_cell(boss),
                Some(OPEN_RAT),
                "boss AI retains its authored home without an ordinary respawn source"
            );
        }
        let rat = {
            let creature = runtime
                .visible_entities()
                .creatures
                .first()
                .copied()
                .expect("the rat is visible");
            EntityRef {
                identity: creature.actor.placement_identity(),
                generation: creature.generation,
            }
        };

        let door_content =
            crate::world_runtime::ReferenceContentGeneration::from_content(room.door())
                .map_err(debug)?;
        let fence = super::fixture::fence(1)?;
        let door_fence = crate::world_runtime::ScopeContentGenerationFence::for_test(
            fence.runtime_scope,
            fence.scope_ownership_generation,
            door_content,
        );
        let door = crate::world_runtime::bind_native_entry_door(
            room.door(),
            &door_fence,
            fence.runtime_scope,
            fence.scope_ownership_generation,
        )
        .map_err(debug)?;
        let descriptor = crate::native_admission_source::descriptor::ProducerDescriptor::new(
            "platform-test".into(),
            ("127.0.0.1".into(), 1),
            "source.test".into(),
            "source.test".into(),
            vec![rustls::pki_types::CertificateDer::from(
                include_bytes!("../../../../../apps/game-server/tests/support/spell-save-tls-fixture/ca.der").to_vec(),
            )],
            vec![rustls::pki_types::CertificateDer::from(
                include_bytes!("../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client.der").to_vec(),
            )],
            rustls::pki_types::PrivateKeyDer::try_from(
                include_bytes!("../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client-key.der")
                    .to_vec(),
            )?,
        )
        .map_err(debug)?;
        let evidence =
            crate::gameplay_transport::fresh_evidence::FreshEvidenceSource::new(descriptor);
        let spells = crate::spell::cast::v1_spell_book().map_err(debug)?;
        let achievements =
            crate::achievement_catalogue::AchievementCatalogue::embedded().map_err(debug)?;
        let charms =
            crate::content::charm_source::CanonicalCharmCatalogue::embedded().map_err(debug)?;
        let runtime = Mutex::new(runtime);
        let door = Mutex::new(door);
        let mut admitted = AdmittedSession {
            game_session_id: session,
            world_id: world,
            channel_id: channel,
            runtime_actor: Some(actor),
            first_entry: FirstEntryOutcome::NotApplicable,
            controller: Some(ControllerBinding {
                transport: crate::foundation::AuthenticatedTransportRefV1::decode(&[9; 16])
                    .map_err(debug)?,
                account_id: id(40),
            }),
            continuity: SessionContinuity::FRESH,
            item_fence: None,
        };
        let mut owner = crate::gameplay_transport::composed_test_owner::ComposedTestOwner {
            root: &harness.root,
            character: &authority,
            holder: &harness.node,
            evidence: &evidence,
            world_id: world,
            channel_id: channel,
            // Gameplay-only fixture: it never admits through this owner, so the ADMIT0 binding
            // declares nothing and only has to be a valid Channel boot binding.
            required_capabilities: crate::gameplay_transport::world_required_capabilities::BoundRequiredCapabilities::for_channel_boot(
                &[],
                crate::foundation::RuntimeScopeRefV1::channel(world, channel),
                1,
                "policy-1",
                crate::gameplay_transport::world_required_capabilities::offered_for_scope(false, None),
            )
            .expect("an empty declared set binds"),
            runtime: &runtime,
            door: &door,
            spells: &spells,
            achievements: &achievements,
            imported_charms: &charms,
            room: &room,
        }.build();
        owner.active_generation = Some(active);
        if let Some(target) = daily_target {
            // Bootstrap the actual durable reset identity for this isolated source-world
            // reward scenario. Production's bundle boot already owns this row.
            let digest = room.map_revision_digest();
            sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,'daily-boost-rewards',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_10',1,clock_timestamp(),'2026-10')")
                .bind(world.as_bytes().as_slice()).bind(digest.as_slice()).execute(&harness.pool).await?;
            sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
                .bind(world.as_bytes().as_slice())
                .execute(&harness.pool)
                .await?;
            let permit = owner.spell_lane_permit().await.ok_or("daily permit")?;
            let selection = owner
                .root
                .prepare_daily_boost(
                    &authority,
                    &harness.node,
                    &permit,
                    crate::durability::daily_boost::DailyBoostBinding {
                        scope: crate::foundation::RuntimeScopeRefV1::channel(world, channel),
                        generation: 1,
                        bundle: digest,
                        epoch: 0,
                    },
                    (
                        vec![if target == "creature" {
                            creature_key.clone()
                        } else {
                            "other creature".into()
                        }],
                        vec![if target == "boss" {
                            creature_key.clone()
                        } else {
                            "other boss".into()
                        }],
                    ),
                )
                .await
                .map_err(debug)?;
            owner
                .runtime
                .lock()
                .await
                .install_daily_boost(selection)
                .map_err(debug)?;
        }

        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&room));
        // In the past, so the owner clock already stands well after the swing clock's origin.
        owner.clock_origin = std::time::Instant::now()
            .checked_sub(Duration::from_secs(60))
            .expect("a monotonic clock older than a minute");
        {
            let runtime = owner.runtime.lock().await;
            assert!(
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .initialize(
                        &runtime,
                        actor,
                        session,
                        crate::spell::cast::CharacterCastFacts {
                            vocation: crate::spell::Vocation::Monk,
                            // Level 50 overcomes the rat's defence and armor. A high-level
                            // attacker kills the real Radiant in bounded swings;
                            // its native HP, defence, loot and corpse facts stay unchanged.
                            level: if settle_radiant {
                                1_000_000
                            } else if charm.is_some() {
                                30
                            } else {
                                50
                            },
                            max_health: if ranged_spell {
                                // The first unblocked source cast is lethal, so this same
                                // real dispatch case also qualifies the death-owner handoff.
                                500
                            } else if radiant.is_some() {
                                1_000_000
                            } else {
                                185
                            },
                            ..crate::gameplay_transport::actor_spell::tests::FACTS
                        },
                        (3, 7_000_000),
                        owner.owner_now(),
                    )
                    .is_some()
            );
        }

        if elemental_area || holy_target {
            let mut runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            for (tag, position) in [
                (
                    100,
                    MovementLocalPosition {
                        x: if holy_target { 3 } else { 1 },
                        ..OPEN_PLAYER
                    },
                ),
                (
                    101,
                    MovementLocalPosition {
                        x: 5,
                        ..OPEN_PLAYER
                    },
                ),
            ] {
                let extra_session = GameSessionId::decode(&id(tag)).map_err(debug)?;
                let extra = runtime
                    .positioned_melee_census()
                    .map_err(debug)?
                    .into_iter()
                    .find(|(_, _, session)| *session == Some(extra_session))
                    .ok_or("extra player")?
                    .0;
                loop {
                    let before = runtime.read_actor_position(extra).map_err(debug)?;
                    let at = before.position();
                    if at == position {
                        break;
                    }
                    let next = MovementLocalPosition {
                        x: at.x + (position.x - at.x).signum(),
                        ..at
                    };
                    runtime
                        .borrow_movement_position()
                        .commit_cardinal(before, next)
                        .map_err(debug)?;
                }
                states
                    .initialize(
                        &runtime,
                        extra,
                        extra_session,
                        crate::spell::cast::CharacterCastFacts {
                            vocation: crate::spell::Vocation::Monk,
                            max_health: 500,
                            ..crate::gameplay_transport::actor_spell::tests::FACTS
                        },
                        (3, 7_000_000),
                        owner.owner_now(),
                    )
                    .ok_or("extra spell state")?;
            }
            // A physical spell cannot qualify this test. Direct elemental protection is read
            // independently for both victims, while armor remains equipped throughout.
            for tag in [50, 100, 101] {
                let session = GameSessionId::decode(&id(tag)).map_err(debug)?;
                let target = runtime
                    .positioned_melee_census()
                    .map_err(debug)?
                    .into_iter()
                    .find(|(_, _, current)| *current == Some(session))
                    .ok_or("area player")?
                    .0;
                let state = states
                    .get_mut(&runtime, target, session)
                    .ok_or("area state")?;
                state.install_equipped_armor(5_000);
                state.install_equipped_physical_protection(Some(
                    crate::content::ReferenceRationalPercent {
                        numerator: 100,
                        denominator: 1,
                    },
                ));
                state.install_equipped_ice_protection(Some(
                    crate::content::ReferenceRationalPercent {
                        numerator: 100,
                        denominator: 1,
                    },
                ));
                if matches!(radiant, Some("paragon" | "templar")) {
                    // Qualify the circle and holy independently despite both being scheduled.
                    state.install_equipped_energy_protection(Some(
                        crate::content::ReferenceRationalPercent {
                            numerator: 100,
                            denominator: 1,
                        },
                    ));
                    state.install_equipped_holy_protection(Some(
                        crate::content::ReferenceRationalPercent {
                            numerator: 100,
                            denominator: 1,
                        },
                    ));
                    state.install_equipped_fire_protection(Some(
                        crate::content::ReferenceRationalPercent {
                            numerator: 100,
                            denominator: 1,
                        },
                    ));
                }
            }
        }

        if elemental_area {
            let mut protected_cast = false;
            let mut damaged = false;
            for _ in 0..128 {
                {
                    let mut runtime = owner.runtime.lock().await;
                    let caster = runtime.visible_entities().creatures[0].actor;
                    let before = runtime.read_actor_position(caster).map_err(debug)?;
                    runtime
                        .borrow_movement_position()
                        .commit_creature_relocation(
                            caster,
                            before,
                            MovementLocalPosition { x: 0, ..OPEN_RAT },
                        )
                        .map_err(debug)?;
                }
                owner.drain_monster_melee().await;
                let runtime = owner.runtime.lock().await;
                let mut states = owner.states.spell_states.lock().await;
                let census = runtime.positioned_melee_census().map_err(debug)?;
                let mut health = Vec::new();
                for tag in [50, 100, 101] {
                    let session = GameSessionId::decode(&id(tag)).map_err(debug)?;
                    let target = census
                        .iter()
                        .find(|(_, _, current)| *current == Some(session))
                        .ok_or("area player")?
                        .0;
                    health.push(
                        states
                            .get(&runtime, target, session)
                            .ok_or("area state")?
                            .vitals()
                            .health,
                    );
                }
                assert_eq!(health[2], 500, "outside the source circle");
                if !protected_cast
                    && states
                        .monster_spells
                        .casts
                        .iter()
                        .any(|cast| cast.entry == 1)
                {
                    assert_eq!(
                        health,
                        [500, 500, 500],
                        "current elemental protection blocks the first source cast"
                    );
                    // Keep all physical protection and armor; remove only the source element protection.
                    for tag in [50, 100] {
                        let session = GameSessionId::decode(&id(tag)).map_err(debug)?;
                        let target = census
                            .iter()
                            .find(|(_, _, current)| *current == Some(session))
                            .ok_or("area player")?
                            .0;
                        let state = states
                            .get_mut(&runtime, target, session)
                            .ok_or("area state")?;
                        if radiant == Some("paragon") {
                            // Ice protection stays installed and cannot block a fire hit.
                            state.install_equipped_fire_protection(None);
                        } else if radiant == Some("templar") {
                            state.install_equipped_energy_protection(None);
                        } else {
                            state.install_equipped_ice_protection(None);
                        }
                    }
                    protected_cast = true;
                } else if health[0] < 500 || health[1] < 500 {
                    assert!(protected_cast);
                    assert_eq!(
                        health,
                        [0, 0, 500],
                        "one elemental circle lethally hits both players through their physical armor"
                    );
                    for tag in [50, 100] {
                        let session = GameSessionId::decode(&id(tag)).map_err(debug)?;
                        let target = census
                            .iter()
                            .find(|(_, _, current)| *current == Some(session))
                            .ok_or("area player")?
                            .0;
                        assert!(states.player_death(&runtime, target, session).is_some());
                    }
                    damaged = true;
                    break;
                }
                drop(states);
                drop(runtime);
                owner.clock_origin = owner.clock_origin.checked_sub(PASS).ok_or("area clock")?;
            }
            assert!(
                damaged,
                "automatic source elemental circle must execute (protected cast {protected_cast})"
            );
            drop(owner);
            drop(authority);
            drop(seal);
            harness.cleanup().await?;
            return Ok(());
        }

        if radiant.is_some() && !settle_radiant {
            let initial = {
                let runtime = owner.runtime.lock().await;
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .get(&runtime, actor, session)
                    .ok_or("player spell state")?
                    .vitals()
                    .health
            };
            let mut damaged = false;
            let mut gated_casts = 0;
            let mut last_sequence = None;
            if ranged_spell {
                let runtime = owner.runtime.lock().await;
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .get_mut(&runtime, actor, session)
                    .ok_or("player spell state")?
                    .install_equipped_armor(5_000);
            }
            for _ in 0..if ranged_spell { 128 } else { 32 } {
                if ranged_spell {
                    // A current position input to the real owner. Reposition before each
                    // pass so that native chase cannot turn this into a melee proof.
                    let mut runtime = owner.runtime.lock().await;
                    let target = runtime.visible_entities().creatures[0].actor;
                    let before = runtime.read_actor_position(target).map_err(debug)?;
                    runtime
                        .borrow_movement_position()
                        .commit_creature_relocation(
                            target,
                            before,
                            MovementLocalPosition { x: -2, ..OPEN_RAT },
                        )
                        .map_err(debug)?;
                }
                owner.drain_monster_melee().await;
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                // Source target changes are deterministic in the full content digest. Protect
                // every candidate for the two gate casts, then observe whichever is selected.
                let mut holy_players = Vec::new();
                if holy_target {
                    let census = runtime.positioned_melee_census().map_err(debug)?;
                    for tag in [50, 100, 101] {
                        let session = GameSessionId::decode(&id(tag)).map_err(debug)?;
                        let target = census
                            .iter()
                            .find(|(_, _, current)| *current == Some(session))
                            .ok_or("holy player")?
                            .0;
                        let vitals = states
                            .get(&runtime, target, session)
                            .ok_or("holy player state")?
                            .vitals();
                        holy_players.push((target, session, vitals));
                    }
                }
                let (actor, session) = holy_players
                    .iter()
                    .find(|(_, _, vitals)| vitals.health < initial)
                    .map_or((actor, session), |(actor, session, _)| (*actor, *session));
                let state = states
                    .get(&runtime, actor, session)
                    .ok_or("player spell state")?;
                let spell_sequence = states
                    .monster_spells
                    .casts
                    .iter()
                    .find(|cast| cast.entry == 2)
                    .map(|cast| cast.sequence);
                if ranged_spell
                    && gated_casts < 2
                    && spell_sequence.is_some()
                    && spell_sequence != last_sequence
                {
                    assert_eq!(
                        state.vitals().health,
                        initial,
                        "the real spell must meet its current equipment mitigation"
                    );
                    last_sequence = spell_sequence;
                    drop(states);
                    drop(runtime);
                    let runtime = owner.runtime.lock().await;
                    let mut states = owner.states.spell_states.lock().await;
                    let state = states
                        .get_mut(&runtime, actor, session)
                        .ok_or("player spell state")?;
                    if holy_target {
                        // Keep armor and all other protection. Only holy changes, for every
                        // current candidate, so retargeting cannot bypass a protected cast.
                        for (target, session, vitals) in &holy_players {
                            assert_eq!(vitals.health, initial, "protected holy candidate");
                            states
                                .get_mut(&runtime, *target, *session)
                                .ok_or("holy player state")?
                                .install_equipped_holy_protection((gated_casts == 0).then_some(
                                    crate::content::ReferenceRationalPercent {
                                        numerator: 100,
                                        denominator: 1,
                                    },
                                ));
                        }
                    } else {
                        state.install_equipped_armor(0);
                        state.install_equipped_physical_protection((gated_casts == 0).then_some(
                            crate::content::ReferenceRationalPercent {
                                numerator: 100,
                                denominator: 1,
                            },
                        ));
                    }
                    gated_casts += 1;
                    drop(states);
                    drop(runtime);
                    owner.clock_origin =
                        owner.clock_origin.checked_sub(PASS).ok_or("attack clock")?;
                    continue;
                }
                if state.vitals().health < initial {
                    damaged = true;
                    if ranged_spell {
                        assert_eq!(
                            gated_casts, 2,
                            "both equipment gates precede the positive cast"
                        );
                        assert_eq!(state.vitals().health, 0, "the source cast is lethal");
                        if holy_target {
                            assert_eq!(state.equipped_armor(), 5_000);
                            assert!(state.equipped_physical_protection().is_some());
                            assert!(state.equipped_ice_protection().is_some());
                            assert!(state.equipped_fire_protection().is_some());
                            assert!(state.equipped_energy_protection().is_some());
                            for (bystander, _, vitals) in &holy_players {
                                if *bystander != actor {
                                    assert_eq!(
                                        vitals.health, 500,
                                        "holy target spell leaves other current players unchanged"
                                    );
                                }
                            }
                        }
                        assert!(
                            states.player_death(&runtime, actor, session).is_some(),
                            "the real player death owner retains its occurrence"
                        );
                        let target = runtime.visible_entities().creatures[0].actor;
                        let from = runtime
                            .read_actor_position(target)
                            .map_err(debug)?
                            .position();
                        let to = runtime
                            .read_actor_position(actor)
                            .map_err(debug)?
                            .position();
                        assert!(
                            from.x.abs_diff(to.x).max(from.y.abs_diff(to.y)) > 1,
                            "the damaging automatic cast must land outside melee range"
                        );
                        assert!(
                            !states.monster_spells.casts.is_empty(),
                            "the existing damage-spell owner consumed an actual scheduled proposal"
                        );
                    }
                    // A repeated owner call inside the same think window cannot apply the
                    // source attack twice. All content and attack schedules are unmodified.
                    let after = state.vitals();
                    drop(states);
                    drop(runtime);
                    owner.drain_monster_melee().await;
                    let runtime = owner.runtime.lock().await;
                    let states = owner.states.spell_states.lock().await;
                    assert_eq!(
                        states
                            .get(&runtime, actor, session)
                            .ok_or("player spell state")?
                            .vitals(),
                        after
                    );
                    for (target, session, before) in &holy_players {
                        assert_eq!(
                            states
                                .get(&runtime, *target, *session)
                                .ok_or("holy player state")?
                                .vitals(),
                            *before,
                            "repeated dispatch preserves every holy candidate's vitals"
                        );
                    }
                    break;
                }
                drop(states);
                drop(runtime);
                owner.clock_origin = owner.clock_origin.checked_sub(PASS).ok_or("attack clock")?;
            }
            assert!(
                damaged,
                "{creature_key} must execute a real damaging attack"
            );
            assert_eq!(xp_receipts(&harness).await?, 0);
            assert_eq!(corpse_receipts(&harness).await?, 0);
            drop(owner);
            drop(authority);
            drop(seal);
            harness.cleanup().await?;
            return Ok(());
        }

        // ARCH-PROGRESSION-SOURCE-0 §1.5: the session's progression binding, formed by the
        // production admission step from the Character root's stored revisions.
        owner.admit_progression_session(&mut admitted).await;
        assert!(
            owner.progression_binding(session).is_some(),
            "the fresh Character's session binds the generation's progression"
        );
        assert_eq!(xp_receipts(&harness).await?, 0);

        if boss_policy.is_some() {
            owner
                .boss_spawn_pass(&mut std::collections::BTreeMap::new())
                .await
                .map_err(debug)?;
        }
        // The target is held (attacking state), then the real owner pass swings until the rat
        // dies; each pass is one swing interval later on the owner clock.
        {
            let runtime = owner.runtime.lock().await;
            let now = owner.owner_now();
            assert_eq!(
                owner.states.attack.lock().await.set_target(
                    &runtime,
                    Some(&room),
                    actor,
                    session,
                    now,
                    Some(rat),
                    1
                ),
                AttackIntentDisposition::Ok
            );
        }
        if let Some((charm, samples)) = charm {
            eprintln!("POSTGRES_STARTED CHARM-WIRE-1 combat assignment={charm}");
            use crate::domain::charm::{BestiaryRaceKey, CharmKey, CharmStage};
            use crate::durability::charm_state::{CharacterCharmState, CombatCharmState};
            let (rat_actor, maximum) = {
                let runtime = owner.runtime.lock().await;
                let rat_actor = runtime.visible_entities().creatures[0].actor;
                (
                    rat_actor,
                    runtime
                        .companion_snapshot(rat_actor)
                        .map_err(debug)?
                        .maximum_health,
                )
            };
            if charm.is_empty() {
                let runtime = owner.runtime.lock().await;
                assert!(
                    owner
                        .states
                        .spell_states
                        .lock()
                        .await
                        .charm_state(&runtime, actor, session)
                        .is_none(),
                    "regression requires a missing snapshot"
                );
            }
            if !charm.is_empty() {
                let key = CharmKey::new(format!("oteryn:charm.{charm}")).map_err(debug)?;
                let state = CharacterCharmState {
                    unlocks: [(key.clone(), CharmStage::new(3).map_err(debug)?)].into(),
                    assignments: [(
                        key,
                        BestiaryRaceKey::new("oteryn:creature.cave_rat").map_err(debug)?,
                    )]
                    .into(),
                };
                let runtime = owner.runtime.lock().await;
                assert!(owner.states.spell_states.lock().await.install_charm_state(
                    &runtime,
                    actor,
                    session,
                    CombatCharmState::new(
                        crate::domain::CharacterId::from_bytes(id(41)).map_err(debug)?,
                        state
                    )
                ));
            }
            for _ in 0..64 {
                {
                    let mut runtime = owner.runtime.lock().await;
                    // Keep the real rat alive so the same attack lineage exercises many draws.
                    runtime
                        .set_test_creature_health(rat_actor, maximum)
                        .map_err(debug)?;
                    let mut states = owner.states.spell_states.lock().await;
                    let state = states
                        .get_mut(&runtime, actor, session)
                        .ok_or("player state")?;
                    state.restore_saved_vitals(state.facts.max_health, state.facts.max_mana);
                }
                owner.refresh_attack_weapon(actor, session).await;
                owner.drain_auto_attacks().await;
                owner.drain_monster_melee().await;
                let runtime = owner.runtime.lock().await;
                let outgoing =
                    maximum - i64::try_from(runtime.live_creature_health(rat_actor).unwrap_or(0))?;
                let states = owner.states.spell_states.lock().await;
                let player = states.get(&runtime, actor, session).ok_or("player state")?;
                samples.push((outgoing, player.facts.max_health - player.vitals().health));
                drop(states);
                drop(runtime);
                owner.clock_origin = owner.clock_origin.checked_sub(PASS).expect("advance clock");
            }
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        let mut swings = 0;
        loop {
            // ATTACK-2: the swing weapon (here fists) is read from the equipment owner first.
            owner.refresh_attack_weapon(actor, session).await;
            owner.drain_auto_attacks().await;
            let queued = owner
                .states
                .attack
                .lock()
                .await
                .kills()
                .has_session(session);
            if queued {
                break;
            }
            swings += 1;
            assert!(swings < 64, "{creature_key} dies to auto-attack swings");
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(PASS)
                .expect("a monotonic clock older than the passes");
        }
        let mut daily_expected_loot = None;
        if let Some(target) = daily_target {
            let attack = owner.states.attack.lock().await;
            let crate::gameplay_transport::kill_reward::TakeOutcome::Taken(taken) =
                attack.kills().take(session, 2)
            else {
                return Err("daily boosted kill".into());
            };
            let entry = taken.entry().ok_or("daily entry")?;
            assert_eq!(entry.row.xp_amount.get(), expected_xp);
            let death = entry.facts.death;
            let plan = oteryn_gameplay_items::loot_plan::plan_creature_loot(
                oteryn_gameplay_items::loot_plan::LootPlanDeathKey::new(
                    *death.world_id().as_bytes(),
                    *death.channel_id().as_bytes(),
                    death.scope_ownership_generation().get(),
                    death.actor_local_id(),
                    death.actor_local_generation(),
                ),
                &entry.row.loot_table_ref,
                &entry.row.loot_table,
            )
            .map_err(debug)?;
            daily_expected_loot = Some(
                plan.entries
                    .iter()
                    .map(|e| (e.item.production_key.clone(), i64::from(e.quantity)))
                    .collect::<Vec<_>>(),
            );

            match (target, entry.row.loot_table.algorithm) {
                (
                    "creature",
                    crate::combat::LootSelectionAlgorithm::DailyBoosted { extra_rolls: 1, .. },
                ) => {}
                (
                    "boss",
                    crate::combat::LootSelectionAlgorithm::DailyBoosted {
                        extra_rolls: 2 | 3,
                        boosted_entries,
                    },
                ) => {
                    assert!(
                        boosted_entries > 0,
                        "the boosted boss has equipment eligible for its extra rolls"
                    );
                    assert!(
                        boosted_entries < entry.row.loot_table.entries.len(),
                        "non-equipment remains in the boss's single base roll"
                    );
                    for (ordinal, loot) in entry.row.loot_table.entries.iter().enumerate() {
                        assert_eq!(
                            native
                                .reward_table()
                                .is_daily_boss_equipment(&loot.item.production_key),
                            ordinal < boosted_entries,
                            "only the stable equipment prefix receives boss extra rolls"
                        );
                        if ordinal < boosted_entries {
                            assert!(
                                loot.unique,
                                "production boss equipment preserves its unique-loot flag"
                            );
                            assert_eq!(
                                plan.entries
                                    .iter()
                                    .filter(|drop| drop.item == loot.item)
                                    .count(),
                                usize::from(plan.entries.iter().any(|drop| drop.item == loot.item)),
                                "a unique boss item is present at most once after extra rolls"
                            );
                        }
                    }
                }
                ("other", crate::combat::LootSelectionAlgorithm::IndependentBernoulliPpm) => {}
                _ => return Err("daily boost affected wrong race".into()),
            }
            drop(taken);
        }
        // No durable rewards exist before the reward drain.
        assert_eq!(xp_receipts(&harness).await?, 0);
        assert_eq!(corpse_receipts(&harness).await?, 0);

        let replay = if settle_radiant || boss_policy.is_some() || daily_target.is_some() {
            let attack = owner.states.attack.lock().await;
            let crate::gameplay_transport::kill_reward::TakeOutcome::Taken(taken) =
                attack.kills().take(session, 2)
            else {
                return Err("queued real kill".into());
            };
            let entry = taken.entry().ok_or("captured kill")?;
            let replay = (entry.facts, entry.target, entry.activity_observed_at);
            taken.requeue();
            Some(replay)
        } else {
            None
        };
        if boss_policy.is_some() {
            let held = owner.root.try_issue_semantic_pass().map_err(debug)?;
            assert!(
                !owner
                    .boss_spawn_pass(&mut std::collections::BTreeMap::new())
                    .await
                    .map_err(debug)?,
                "holder contention defers the pass before transaction start"
            );
            owner.runtime.lock().await.owner_fence().map_err(debug)?;
            // Drop returns the sole holder in a spawned task. Await its observed return
            // so immediate Deferred retries cannot starve that task on this runtime.
            held.run(|_, _| Box::pin(async { Ok(()) }))
                .await
                .map_err(debug)?;
        }
        let boss_death_lower = if boss_policy.is_some() {
            Some(
                sqlx::query_scalar::<_, i64>(
                    "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
                )
                .fetch_one(&harness.pool)
                .await?,
            )
        } else {
            None
        };
        let mut drains = 0;
        while owner.drain_kill_rewards(session).await != DrainOutcome::Drained {
            drains += 1;
            assert!(drains < 8, "the queued kill settles");
            // A deferral for want of an idle root connection (the pass `held` above went back
            // to the pool) only records ready demand; run the window before the next drain.
            owner.root.maintain_ready_once().await.map_err(debug)?;
        }
        assert!(
            !owner
                .states
                .attack
                .lock()
                .await
                .kills()
                .has_session(session),
            "the settled kill leaves the queue"
        );

        // XP: exactly one committed receipt for the native amount on the fenced Character.
        assert_eq!(xp_receipts(&harness).await?, 1);
        let experience: i64 =
            sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(experience, expected_xp);
        // Corpse: one materialized corpse minted for this Character's kill, and its loot rows
        // are children of that very corpse.
        assert_eq!(corpse_receipts(&harness).await?, 1);
        let children: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_corpse_container_entries e \
             JOIN game_item_mint_receipts r ON r.item_instance_id = e.parent_item_instance_id",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert!(
            children >= 1,
            "{creature_key} commits a positive native loot plan"
        );

        if let Some(expected) = daily_expected_loot {
            let actual: Vec<(String,i64)> = sqlx::query_as("SELECT i.definition_production_key,i.quantity FROM game_item_corpse_container_entries e JOIN game_item_instances i ON i.item_instance_id=e.item_instance_id JOIN game_item_mint_receipts r ON r.item_instance_id=i.item_instance_id ORDER BY r.draw_ordinal").fetch_all(&harness.pool).await?;
            assert_eq!(
                actual, expected,
                "daily loot rolls reach durable Item quantities"
            );
        }

        if let Some((facts, target, observed)) = replay {
            {
                let mut runtime = owner.runtime.lock().await;
                let attack = owner.states.attack.lock().await;
                crate::gameplay_transport::kill_reward::record_creature_kill(
                    &mut runtime,
                    crate::gameplay_transport::kill_reward::KillSink {
                        prey: None,
                        activity: &[],
                        queue: attack.kills(),
                        rewards: native.reward_table(),
                    },
                    target,
                    facts.principal,
                    facts.death_at_ms,
                    observed,
                );
                assert!(
                    attack.kills().has_session(session),
                    "the same committed kill is replayed through the router"
                );
            }
            assert_eq!(
                owner.drain_kill_rewards(session).await,
                DrainOutcome::Drained
            );
            assert_eq!(xp_receipts(&harness).await?, 1);
            assert_eq!(corpse_receipts(&harness).await?, 1);
            let replay_children: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_item_corpse_container_entries")
                    .fetch_one(&harness.pool)
                    .await?;
            assert_eq!(
                replay_children, children,
                "kill replay never duplicates loot"
            );
        }
        if settle_radiant {
            qualify_summer_settlement(
                &mut owner,
                &harness,
                &creature_key,
                expected_xp,
                children,
                reconcile == Some(true),
                replay
                    .map(|(_, target, _)| target)
                    .ok_or("real captured kill")?,
            )
            .await?;
        }
        if let Some(policy) = boss_policy {
            let row: (String,i16,Option<i64>,Option<String>) = sqlx::query_as("SELECT cycle::text,state,due_at,owner_generation::text FROM game_boss_spawn_clocks WHERE spawn_key='boss_spawn1_fixture:0'")
                .fetch_one(&harness.pool).await?;
            assert_eq!(row.0, "1", "one committed death despite reward replay");
            assert!(row.3.is_none());
            match policy {
                oteryn_gameplay_content::content::BossPolicy::Respawn { min_s, max_s } => {
                    assert_eq!(row.1, 1);
                    let now: i64 = sqlx::query_scalar(
                        "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
                    )
                    .fetch_one(&harness.pool)
                    .await?;
                    assert!(
                        (boss_death_lower.ok_or("death lower bound")? + (min_s as i64) * 1000
                            ..=now + (max_s as i64) * 1000)
                            .contains(&row.2.ok_or("due time")?)
                    );
                }
                oteryn_gameplay_content::content::BossPolicy::AtReset { .. } => {
                    assert_eq!(row.1, 3);
                    assert!(row.2.is_none());
                }
            }
        }
        let children = if capacity {
            super::summer_corpse_capacity::qualify(&mut owner, &harness).await?
        } else {
            children
        };
        let decay = match reconcile.filter(|_| !settle_radiant) {
            Some(reconcile) => {
                super::corpse_decay::qualify(&mut owner, &harness, children, reconcile)
                    .await
                    .map_err(|error| format!("corpse decay qualification: {error}").into())
            }
            None => Ok(()),
        };

        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await?;
        decay
    })
}

async fn xp_receipts(harness: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_character_xp_receipts")
            .fetch_one(&harness.pool)
            .await?,
    )
}

async fn corpse_receipts(harness: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_item_mint_receipts WHERE loot_purpose_key = $1 \
         AND materialized_at IS NOT NULL",
    )
    .bind(crate::durability::item_mint::CORPSE_MATERIALIZATION_PURPOSE_KEY)
    .fetch_one(&harness.pool)
    .await?)
}

#[test]
fn boss_spawn1_respawn_death_and_replay_through_real_dispatch_postgres() -> TestResult {
    scenario_thread::run("boss-spawn1-respawn", || {
        scenario_with_creature_charmed(
            None,
            false,
            None,
            false,
            false,
            false,
            (
                None,
                Some(oteryn_gameplay_content::content::BossPolicy::Respawn {
                    min_s: 30,
                    max_s: 60,
                }),
                None,
            ),
        )
    })
}

#[test]
fn boss_spawn1_at_reset_death_and_replay_through_real_dispatch_postgres() -> TestResult {
    scenario_thread::run("boss-spawn1-at-reset", || {
        scenario_with_creature_charmed(
            None,
            false,
            None,
            false,
            false,
            false,
            (
                None,
                Some(oteryn_gameplay_content::content::BossPolicy::AtReset {
                    chance: oteryn_gameplay_content::content::BossChance {
                        numerator: 1,
                        denominator: 1,
                    },
                }),
                None,
            ),
        )
    })
}

#[test]
fn daily_boost_real_kill_xp_loot_and_unselected_race_postgres() -> TestResult {
    scenario_thread::run("daily-boost-rewards", || {
        for target in ["creature", "other"] {
            scenario_with_creature_charmed(
                None,
                false,
                None,
                false,
                false,
                false,
                (None, None, Some(target)),
            )?;
        }
        // The production rat proves selection and ordinary loot, but has no equipment. Kill a
        // production Summer boss as the selected daily boss as well: its mixed loot table proves
        // that only equipment extra rolls survive the complete dispatch -> corpse durability path.
        scenario_with_creature_charmed(
            None,
            false,
            Some("acolyte"),
            true,
            false,
            false,
            (None, None, Some("boss")),
        )?;
        Ok(())
    })
}
