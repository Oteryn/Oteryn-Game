//! RANGED-ATTACK-1: real admission, equipment owner, auto-attack and durable reload.
#![allow(clippy::expect_used, clippy::unwrap_used)]
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use super::fixture::{Harness, debug, id, run};
use super::{TestResult, prepare_current_session};
use oteryn_protocol_oteryn::attack::AttackIntentDisposition;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::time::Duration;
use tokio::sync::Mutex;
use crate::foundation::{ChannelId, ChannelRuntimeV1, GameSessionId, MovementLocalPosition, NodeId, WorldId};
use oteryn_gameplay_content::content::channel_content_pin::ChannelContentPin;
const OPEN_RAT: MovementLocalPosition = MovementLocalPosition { x: 1, y: 11, floor: -7 };
const PASS: Duration = Duration::from_secs(3);
#[test]
fn ranged_attack_postgres_bow_kills_at_four_consumes_and_reloads() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("ranged-bow", || scenario(3350, 4, true, 100, false, false))
}
#[test]
fn ranged_attack_postgres_wand_pays_mana_and_deals_death() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("ranged-wand", || scenario(25760, 3, false, 100, false, false))
}
#[test]
fn ranged_attack_postgres_out_of_range_no_ammunition_and_no_mana_are_free() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("ranged-gates", || {
        scenario(3350, 7, true, 100, false, false)?;
        scenario(3350, 4, false, 100, false, false)?;
        scenario(25760, 3, false, 0, false, false)
    })
}
#[test]
fn ranged_attack_postgres_throwing_breaks_in_the_hand_by_source_chance() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("ranged-throwing", || scenario(3287, 3, false, 100, false, false))
}
#[test]
fn ranged_attack_postgres_projectile_wall_refuses_without_cost() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("ranged-wall", || scenario(3350, 6, true, 100, true, false))
}
#[test]
fn ranged_attack_postgres_reconcile_spends_once_without_late_damage_or_training() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("ranged-reconcile", || scenario(3350, 4, true, 100, false, true))
}
fn scenario(weapon: u32, distance: i32, ammo: bool, mana: u32, blocked: bool, reconcile: bool) -> TestResult {
    // restore_saved_vitals clamps to the owned maximum; all payment assertions start there.
    let mana = mana.min(crate::gameplay_transport::actor_spell::tests::FACTS.max_mana);
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "ranged_attack", false, false).await?;
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
                    "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
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
            if blocked { (7, 13, -7) } else { (OPEN_RAT.x + distance, OPEN_RAT.y, OPEN_RAT.floor) },
        );

        // Test-only custody provisioning. Every tested count change uses the real owner.
        {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
            for (seed, item, count, slot) in [(70, weapon, if weapon == 3287 { 100 } else { 1 }, 6), (71, 3447, 100, 10)] {
                if slot == 10 && !ammo { continue; }
                sqlx::raw_sql(sqlx::AssertSqlSafe(crate::durability::item_fee_burn::test_support::item(seed, &format!("oteryn:item.tibia.i{item}"), "definition-r1", count))).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($4,'hex')::uuid)")
                    .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(slot as i16).bind(id(seed).as_slice()).execute(&mut *tx).await?;
            }
            tx.commit().await?;
        }
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
            2,
            channel_pin.clone(),
        )
        .map_err(debug)?;
        if matches!(weapon, 25760 | 3287) {
            // An elemental wand must pass physical immunity. This distinguishes its element
            // from a fist/physical hit through the real dispatch.
            let mut rat = native.policies().policies().iter().find(|p| p.definition_key == "oteryn:creature.rat").ok_or("rat policy")?.as_ref().clone();
            rat.damage_immunities.push("physical".into());
            let policies = crate::content::creature_policy::CompiledCreaturePolicies::from_active_artifact(native.source_digest(), vec![rat]).map_err(debug)?;
            runtime.install_companion_policies(policies).map_err(debug)?;
        } else { native.install_companion_policies(&mut runtime).map_err(debug)?; }
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
        runtime
            .realize_native_qualification_spawn(channel_pin, "oteryn:creature.rat", OPEN_RAT)
            .map_err(debug)?;
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
                include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/ca.der").to_vec(),
            )],
            vec![rustls::pki_types::CertificateDer::from(
                include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client.der").to_vec(),
            )],
            rustls::pki_types::PrivateKeyDer::try_from(
                include_bytes!("../../../../../../apps/game-server/tests/support/spell-save-tls-fixture/client-key.der")
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
                            vocation: crate::spell::Vocation::Paladin,
                            ..crate::gameplay_transport::actor_spell::tests::FACTS
                        },
                        (0, 0),
                        owner.owner_now(),
                    )
                    .is_some()
            );
        }

        {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            let state = states.get_mut(&runtime, actor, session).ok_or("player")?;
            state.restore_saved_vitals(1000, mana);
            let build = crate::durability::character_build::DurableBuildState::new("paladin", (0, 0), [(80, 0); 7]).map_err(debug)?;
            state.enable_owned_training(&build, native.training_formula().ok_or("training formula")?).map_err(debug)?;
        }
        {
            let runtime = owner.runtime.lock().await;
            assert_eq!(owner.states.attack.lock().await.set_target(&runtime, Some(&room), actor, session, owner.owner_now(), Some(rat), 1), AttackIntentDisposition::Ok);
        }
        if reconcile {
            owner.refresh_attack_weapon(actor, session).await;
            owner.park_committed_ranged_shot_for_test(actor, session).await?;
            owner.drain_auto_attacks().await;
            let quantity: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(quantity, 99);
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts WHERE swing_ordinal IS NOT NULL").fetch_one(&harness.pool).await?;
            assert_eq!(count, 1, "reconciliation never burns a second arrow");
            let runtime = owner.runtime.lock().await;
            let creature = runtime.visible_entities().creatures[0].actor;
            let snapshot = runtime.companion_snapshot(creature).map_err(debug)?;
            assert_eq!(snapshot.health, snapshot.maximum_health, "unknown acknowledgement delivers no late hit");
            let states = owner.states.spell_states.lock().await;
            let checkpoint = states.get(&runtime, actor, session).ok_or("caster")?.prepare_training_checkpoint(owner.owner_now().get(), native.training_formula().ok_or("formula")?, crate::durability::character_build::BuildOccurrence::from_bytes(id(93)).map_err(debug)?, true).map_err(debug)?;
            assert!(checkpoint.is_none(), "reconciliation credits no skill tries");
            drop(states); drop(runtime); drop(owner); drop(authority); drop(seal);
            return harness.cleanup().await;
        }
        let mut shots = 0;
        let mut killed = false;
        for _ in 0..if weapon == 3287 { 100 } else { 30 } {
            owner.refresh_attack_weapon(actor, session).await;
            owner.drain_auto_attacks().await;
            killed = owner.states.attack.lock().await.kills().has_session(session);
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_spell_item_receipts WHERE swing_ordinal IS NOT NULL").fetch_one(&harness.pool).await?;
            shots = count;
            if killed || blocked || distance > 6 || (weapon == 3350 && !ammo) || (weapon == 25760 && mana == 0) { break; }
            owner.clock_origin = owner.clock_origin.checked_sub(PASS).ok_or("clock")?;
        }
        if weapon == 3350 && distance <= 6 && ammo && !blocked {
            assert!(killed, "bow kills from range {distance}");
            assert!(shots > 0);
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let checkpoint = states.get(&runtime, actor, session).ok_or("bow caster")?
                .prepare_training_checkpoint(owner.owner_now().get(), native.training_formula().ok_or("formula")?, crate::durability::character_build::BuildOccurrence::from_bytes(id(91)).map_err(debug)?, true).map_err(debug)?.ok_or("distance training")?;
            assert!(checkpoint.request.after.skills()[4].1 > 0, "bow trains distance");
            drop(states);
            drop(runtime);
            let remaining: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(71).as_slice()).fetch_one(&harness.pool).await?;
            assert_eq!(remaining, 100-shots, "durable reload sees exactly one arrow per shot");
        } else if weapon == 3287 {
            assert!(!killed, "physical immunity blocks the throwing damage");
            let remaining: i64 = sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(70).as_slice()).fetch_one(&harness.pool).await?;
            assert!(shots > 0 && shots < 100, "source break chance breaks some, retains others");
            assert_eq!(remaining, 100-shots, "only broken throwing stars leave the hand");
        } else if weapon == 25760 && mana > 0 && distance <= 3 {
            assert!(killed, "wand damage reaches the monster");
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let caster = states.get(&runtime, actor, session).ok_or("wand caster")?;
            assert!(caster.vitals().mana < mana);
            let checkpoint = caster.prepare_training_checkpoint(owner.owner_now().get(), native.training_formula().ok_or("formula")?, crate::durability::character_build::BuildOccurrence::from_bytes(id(92)).map_err(debug)?, true).map_err(debug)?.ok_or("mana training")?;
            // Crossing a magic level consumes its required mana; compare both level and remainder.
            let expected = oteryn_gameplay_progression::mana_training::advance(native.training_formula().ok_or("formula")?, "paladin", 0,
                checkpoint.request.before.magic(), u64::from(mana-caster.vitals().mana));
            assert_eq!(checkpoint.request.after.magic(), expected, "mana spent trains magic");
        } else {
            assert!(!killed);
            assert_eq!(shots, 0, "a refused shot consumes nothing");
            let runtime = owner.runtime.lock().await;
            let visible = runtime.visible_entities();
            let rat = visible.creatures.first().ok_or("live refused target")?;
            let snapshot = runtime.companion_snapshot(rat.actor).map_err(debug)?;
            assert_eq!(snapshot.health, snapshot.maximum_health);
        }
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
