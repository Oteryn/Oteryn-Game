//! Production Soulpit flows with authored lobby geometry and the exact Crystal Instance room.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult};
use crate::content::bosses::soulpit_template::QualifiedSoulpitRoomTemplate;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::runtime_actor_spell_types::{
    OwnerCombatBatch, OwnerCombatChange, OwnerCombatEffect, SpellOccurrenceBinding,
};
use crate::foundation::{
    ChannelContentPin, CharacterLease, CommandId, CommandRef, MovementLocalPosition,
    RuntimeScopeRefV1,
};
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{AdmittedSession, FirstEntryOutcome, UseCommand};
use crate::gameplay_transport::kill_reward::{KillSink, record_batch_kills};
use crate::gameplay_transport::{ComposedFreshAdmission, SeamRooms, item_fence_of};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
};
#[path = "soulpit_production/proofs.rs"]
mod proofs;
#[path = "soulpit_production/room.rs"]
mod room;
use room::{bundle, issue_content, object};

#[derive(Clone, Copy)]
enum Case {
    Exit,
    Complete,
    DeathOrder,
    AtomicCommit,
    AtomicRollback,
}

pub(super) fn completion_atomicity_case(inject_failure: bool) -> TestResult {
    scenario_thread(if inject_failure {
        Case::AtomicRollback
    } else {
        Case::AtomicCommit
    })
}

#[test]
fn soulpit_production_use_atomic_entry_first_wave_and_authenticated_exit() -> TestResult {
    scenario_thread(Case::Exit)
}
#[test]
fn soulpit_production_four_physical_waves_complete_animus_and_return_once() -> TestResult {
    scenario_thread(Case::Complete)
}
#[test]
fn soulpit_production_boss_lethal_xp_precedes_player_death_without_global_tick() -> TestResult {
    scenario_thread(Case::DeathOrder)
}
fn scenario_thread(case: Case) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        eprintln!("SV8 SOULPIT_PRODUCTION_PG_SKIPPED no_admin_url");
        assert!(
            std::env::var_os("OTERYN_REQUIRE_PG").is_none(),
            "Soulpit production PG lane requires PostgreSQL"
        );
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(case).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "production Soulpit scenario panicked")??;
    Ok(())
}

fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        let began = std::time::Instant::now();
        super::reset();
        let h =
            Harness::create_with_admission_seed(admin, "soulpit_production", false, false).await?;
        let evidence = super::house_transfer::RecoverySource::start(&h).await?;
        super::house_transfer::prepare_owned_session(&h).await?;
        let world = super::fixture::fence(1)?.runtime_scope.world_id();
        let (controller, source_pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let active = controller.active().ok_or("active content")?;
        let native = active.native_gameplay().ok_or("native gameplay")?;
        let index = native.item_index().ok_or("native item index")?;
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, pins) = bundle(
            reference("oteryn:item.tibia.i103")
                .ok_or("ground ref")?
                .get()
                - 1,
            reference("oteryn:item.tibia.i47367")
                .ok_or("obelisk ref")?
                .get()
                - 1,
        )?;
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let map = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 32375,
                    y: 31158,
                    floor: -8,
                },
            },
            world,
            super::fixture::fence(1)?
                .runtime_scope
                .channel_id()
                .ok_or("channel")?,
            |key| {
                // Canary 47dfd51f items.xml:33,77475 and pinned appearances:
                // genuine map-only dirt/obelisk facts, without inventory admission.
                let solid = match key {
                    "oteryn:item.tibia.i103" => false,
                    "oteryn:item.tibia.i47367" => true,
                    _ => return None,
                };
                Some(crate::map::facts::ItemDefinition {
                    reference: reference(key)?,
                    solid: Some(solid),
                    blocks_projectile: false,
                    pickupable: false,
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let cells = map
            .movement_cells(source_pin.qualified_room().movement_cells())
            .map_err(debug)?;
        let room = source_pin
            .qualified_room()
            .bundle_view(
                cells.clone(),
                map.frame_binding_digest(world),
                map.map_revision_digest(),
                crate::content::NativeEntryStart {
                    x: 32375,
                    y: 31158,
                    floor: 8,
                },
            )
            .ok_or("bundle combat room")?;
        let soulpit_room = crate::map::soulpit_room::qualify_soulpit_room(
            source_pin.qualified_room(),
            active,
            world,
            super::fixture::fence(1)?
                .runtime_scope
                .channel_id()
                .ok_or("channel")?,
        )?;
        let template =
            QualifiedSoulpitRoomTemplate::qualify(&soulpit_room, active).map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            world,
            active.activation_sequence(),
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            map.frame_binding_digest(world),
            map.map_revision_digest(),
            (32375, 31158, 8),
        );
        let mut deps = Dependencies::with_content(
            pin.clone(),
            source_pin.qualified_room().clone(),
            native.spell_book().clone(),
        )?;
        deps.combat_room = Some(room.clone());
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.evidence = &evidence.evidence;
        owner.active_generation = Some(active);
        owner.proficiency_catalogue =
            crate::gameplay_transport::weapon_proficiency::ProductionProficiencyCatalogue::embedded(
            );
        owner.movement_cells = &cells;
        owner.rooms = SeamRooms::new(None, Some(&room));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let current = FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(admitted.game_session_id)
            .await?
            .0;
        admitted.item_fence = item_fence_of(current);
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            BUNDLE_WORLD_OFFERED_CAPABILITIES,
            &[
                4,
                6,
                11,
                15,
                18,
                oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1,
            ],
        )
        .ok_or("production capabilities")?;
        let fence = admitted.item_fence.ok_or("source item fence")?;
        {
            let mut runtime = owner.runtime.lock().await;
            runtime
                .bind_attacker_lease(
                    deps.actor,
                    admitted.game_session_id,
                    CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            runtime
                .initialize_pinned_test_position(
                    deps.actor,
                    MovementLocalPosition {
                        x: 32375,
                        y: 31158,
                        floor: 8,
                    },
                )
                .map_err(debug)?;
        }
        owner.admit_progression_session(&mut admitted).await;
        room::qualify_entry_level(&h, &owner, admitted).await?;
        owner
            .register_current_runtime()
            .await
            .map_err(|_| "register origin")?;
        owner
            .activity_runtimes
            .register_soulpit_template(&template)
            .map_err(|_| "register Soulpit template")?;
        owner
            .activity_runtimes
            .remember(admitted)
            .map_err(|_| "remember origin")?;
        let account: String = sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        evidence
            .evidence
            .refresh_account(&h.root, &h.node, &account)
            .await
            .map_err(debug)?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING")
            .bind(fence.character_id.as_bytes().as_slice()).execute(&h.pool).await?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *tx)
            .await?;
        sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i3264','definition-r1',1,1,encode($1,'hex')::uuid)")
            .bind(id(117).as_slice()).bind(world.as_bytes().as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,6,encode($3,'hex')::uuid,encode($3,'hex')::uuid)")
            .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(id(117).as_slice()).execute(&mut *tx).await?;
        tx.commit().await?;
        issue_content(&h, &pin).await?;
        super::item_look::seed_main_backpack(&h, &[("oteryn:item.tibia.i47398", 2)]).await?;
        let instance = super::activity_runtime::allocate_ready_instance(&h, 88).await?;
        proofs::bind_identity(&owner, &template, instance).await?;
        let obelisk = object(map.base(), 32375, 31157, 1)?;
        let intent = UseIntent {
            target: UseTarget::ItemByDefinition(
                reference("oteryn:item.tibia.i47398").ok_or("core ref")?,
            ),
            use_with: None,
            use_with_map_item: Some(MapUseTarget::Object(obelisk)),
            use_with_position: None,
        };
        let command = UseCommand {
            game_session_id: admitted.game_session_id,
            command_id: 1,
            item_fence: Some(fence),
        };
        if matches!(case, Case::Exit) {
            room::probe_entry_rollback(&h, &owner, deps.actor, command, &intent).await?;
        }
        let result = owner
            .use_soulpit_obelisk(deps.actor, command, &intent, None, None)
            .await
            .map_err(|_| "production Soulpit use")?
            .ok_or("obelisk dispatch not selected")?;
        assert_eq!(result.disposition, UseDisposition::Committed);
        let replay = owner
            .use_soulpit_obelisk(deps.actor, command, &intent, None, None)
            .await
            .map_err(|_| "entry replay")?
            .ok_or("entry replay selection")?;
        assert_eq!(replay.disposition, UseDisposition::Committed);
        let amount:i64=sqlx::query_scalar("SELECT quantity FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid")
            .bind(id(71).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(amount, 1, "one core across exact retry");
        let (handoff, started): (Vec<u8>, i64) = sqlx::query_as(
            "SELECT uuid_send(handoff_id),started_at_micros FROM game_soulpit_encounters",
        )
        .fetch_one(&h.pool)
        .await?;
        let handoff: [u8; 16] = handoff.try_into().map_err(|_| "handoff id")?;
        let destination = owner.activity_runtimes.current_connection(admitted);
        let target_fence = destination.item_fence.ok_or("destination fence")?;
        assert_eq!(target_fence.runtime_scope, instance);
        assert_ne!(destination.game_session_id, admitted.game_session_id);
        assert_ne!(destination.runtime_actor, admitted.runtime_actor);
        assert_eq!(
            target_fence.character_lease_generation,
            fence.character_lease_generation + 1
        );
        let instance_view = owner
            .resolve_runtime_view(destination)
            .await
            .map_err(|_| "resolve destination")?
            .ok_or("destination view")?;
        let actor = destination.runtime_actor.ok_or("destination actor")?;
        let effects = instance_view
            .read_admission_effects(destination.game_session_id)
            .await
            .map_err(|_| "Instance equipment effects")?;
        {
            let runtime = instance_view.runtime.lock().await;
            instance_view
                .install_admission_suppressions(
                    &runtime,
                    actor,
                    destination.game_session_id,
                    effects,
                )
                .await
                .ok_or("Instance equipment admission")?;
        }
        room::prove_equipment_entry_fences(&instance_view, admitted, destination).await?;
        assert_eq!(
            instance_view
                .runtime
                .lock()
                .await
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            MovementLocalPosition {
                x: 32373,
                y: 31151,
                floor: 8
            }
        );
        if matches!(case, Case::AtomicCommit) {
            super::soulpit_lifecycle::completion_atomicity::prove_instance_proficiency_fences(
                &h,
                &instance_view,
                destination,
            )
            .await?;
            super::maintain_ready(owner.root).await?;
        }
        if matches!(case, Case::AtomicCommit | Case::AtomicRollback) {
            super::soulpit_lifecycle::completion_atomicity::assert_shared_completion(
                &h,
                &instance_view,
                destination,
                handoff,
                matches!(case, Case::AtomicRollback),
            )
            .await?;
            drop(instance_view);
            drop(owner);
            drop(authority);
            drop(seal);
            drop(evidence);
            return h.cleanup().await;
        }
        let mut now = u64::try_from(started)? + 4_000_000;
        owner
            .tick_soulpits(now, false)
            .await
            .map_err(|_| "first physical wave")?;
        let first = instance_view
            .runtime
            .lock()
            .await
            .visible_entities()
            .creatures
            .len();
        assert!(
            (1..=7).contains(&first),
            "source spawn attempts admit physical actors in the actual Instance"
        );
        if matches!(case, Case::Exit) {
            room::probe_return_uncertainty(&h, &instance_view, destination).await?;
        } else {
            let before_xp:i64=sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            let mut actual_bosses = 0_i64;
            for wave in 1..=4_u8 {
                actual_bosses += kill_wave(
                    &instance_view,
                    destination,
                    u64::from(wave) * 100,
                    now + 1_000,
                )
                .await?;
                if wave == 4 && matches!(case, Case::DeathOrder) {
                    proofs::probe_boss_death_order(
                        &h,
                        &owner,
                        &instance_view,
                        destination,
                        before_xp,
                        actual_bosses,
                        now,
                    )
                    .await?;
                    drop(instance_view);
                    drop(owner);
                    drop(authority);
                    drop(seal);
                    drop(evidence);
                    return h.cleanup().await;
                }
                now += 5_000_000;
                owner
                    .tick_soulpits(now, true)
                    .await
                    .map_err(|_| "physical wave clearance")?;
                if wave < 4 {
                    now += 4_000_000;
                    owner
                        .tick_soulpits(now, false)
                        .await
                        .map_err(|_| "subsequent physical wave")?;
                    let spawned = instance_view
                        .runtime
                        .lock()
                        .await
                        .visible_entities()
                        .creatures
                        .len();
                    assert!((1..=7).contains(&spawned));
                }
            }
            let terminal:bool=sqlx::query_scalar("SELECT terminal FROM game_soulpit_encounters WHERE handoff_id=encode($1,'hex')::uuid")
                .bind(handoff.as_slice()).fetch_one(&h.pool).await?;
            assert!(terminal);
            let mastery:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_animus_masteries WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(mastery, 1);
            let xp:i64=sqlx::query_scalar("SELECT total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            proofs::verify_xp(&h, fence.character_id, before_xp, actual_bosses, xp).await?;
            let beast: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_bestiary_kill_receipts")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(
                beast, 0,
                "Soulpit physical kills do not enter ordinary Bestiary"
            );
            owner
                .tick_soulpits(now + 1_000_000, true)
                .await
                .map_err(|_| "completed replay cadence")?;
            let after:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_animus_masteries WHERE character_id=encode($1,'hex')::uuid")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(after, 1);
            let points: i64 = sqlx::query_scalar("SELECT coalesce((SELECT progress FROM game_character_proficiency WHERE character_id=encode($1,'hex')::uuid AND item_key='oteryn:item.tibia.i3264'),0)")
                .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                points,
                1500 * actual_bosses,
                "only actual final Boss grants source weapon points, once"
            );
        }
        let returned = owner.activity_runtimes.current_connection(destination);
        assert!(matches!(
            returned.item_fence.ok_or("returned fence")?.runtime_scope,
            RuntimeScopeRefV1::Channel { .. }
        ));
        assert_ne!(returned.game_session_id, destination.game_session_id);
        let returned_view = owner
            .resolve_runtime_view(returned)
            .await
            .map_err(|_| "return view")?
            .unwrap_or(owner.clone());
        assert_eq!(
            returned_view
                .runtime
                .lock()
                .await
                .read_actor_position(returned.runtime_actor.ok_or("returned actor")?)
                .map_err(debug)?
                .position(),
            MovementLocalPosition {
                x: 32373,
                y: 31158,
                floor: 8
            }
        );
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_activity_runtime_returns WHERE handoff_id=encode($1,'hex')::uuid")
            .bind(handoff.as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(count, 1);
        eprintln!(
            "SV8 SOULPIT_PRODUCTION_PG_EXECUTED completion={} duration={:?}",
            matches!(case, Case::Complete),
            began.elapsed()
        );
        drop(returned_view);
        drop(instance_view);
        drop(owner);
        drop(authority);
        drop(seal);
        drop(evidence);
        h.cleanup().await
    })
}

async fn kill_wave(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    base: u64,
    now_us: u64,
) -> TestResult<i64> {
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native")?;
    let fence = admitted.item_fence.ok_or("Instance fence")?;
    let caster = admitted.runtime_actor.ok_or("Instance player")?;
    let now = oteryn_simulation_determinism::SemanticTimeMicros::from_micros(now_us);
    let mut runtime = owner.runtime.lock().await;
    let targets = runtime
        .visible_entities()
        .creatures
        .iter()
        .map(|creature| creature.actor)
        .collect::<Vec<_>>();
    let mut bosses = 0;
    for (ordinal, target) in targets.into_iter().enumerate() {
        bosses += i64::from(runtime.soulpit_variant(target).is_some_and(|tag| {
            tag.stack == crate::domain::soulpit::SoulpitStack::Boss && !tag.is_summon
        }));
        let batch = OwnerCombatBatch {
            caster,
            attacker: crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                .map_err(debug)?,
            current_lease_generation: fence.character_lease_generation,
            command: CommandRef::new(
                admitted.game_session_id,
                CommandId::new(base + ordinal as u64).map_err(debug)?,
            ),
            occurrence: SpellOccurrenceBinding {
                id: format!("soulpit-production:{base}:{ordinal}"),
                revisions: ["r1", "r1", "r1", "r1", "r1"].map(str::to_owned),
            },
            binding: format!("soulpit-production:{base}:{ordinal}").into_bytes(),
            anchor: None,
            now_us: Some(now),
            now_ms: now.get() / 1000,
            effects: vec![OwnerCombatEffect {
                target,
                sub_ordinal: 0,
                change: OwnerCombatChange::Damage {
                    target_atom: runtime.creature_spell_target_atom(target).map_err(debug)?,
                    magnitude: 1_000_000,
                },
            }],
            deferred: None,
        };
        let staged = runtime.stage_spell_batch(&batch).map_err(debug)?;
        let receipt = runtime.commit_spell_batch(staged).map_err(debug)?;
        assert!(receipt.effects.iter().any(|effect| effect.target == target
            && effect.health.is_some_and(|health| health.health_after == 0)));
        let attack = owner
            .states
            .attack
            .try_lock()
            .map_err(|_| "attack owner busy")?;
        record_batch_kills(
            &mut runtime,
            KillSink {
                prey: None,
                activity: &[],
                queue: attack.kills(),
                rewards: native.reward_table(),
                charms: None,
            },
            &receipt,
            now.get() / 1000,
        );
    }
    Ok(bosses)
}
