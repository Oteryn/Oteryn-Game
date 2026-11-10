//! A selected native callback must apply its damage through the production owner.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::foundation::{ChannelContentPin, ChannelRuntimeV1, MovementLocalPosition};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

/// One current death: at most one detached turn per copied event, its completion,
/// and the retained uncredited corpse ACK. Never advance the source clock here.
pub(super) async fn drain_source_death(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    now: oteryn_simulation_determinism::SemanticTimeMicros,
) -> TestResult {
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("actual native death content")?;
    let profile = owner
        .runtime
        .lock()
        .await
        .companion_snapshot(actor)
        .map_err(debug)?
        .state
        .policy;
    let definition = crate::content::ProjectV2DefinitionRef {
        family: crate::content::ProjectV2Family::Creature,
        key: profile.definition_key.clone(),
        revision: profile.definition_revision.clone(),
    };
    let event_count = native
        .native_death_events(&definition)
        .map_or(0, |events| events.events().len());
    for _ in 0..event_count + 2 {
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let head = runtime.next_source_creature_death().map_err(debug)?;
        if let Some(head) = &head {
            assert_eq!(
                head.actor(),
                actor,
                "only this exact retained death may be drained"
            );
        }
        let pending = head.is_some() || states.source_death_corpse.is_some();
        drop(states);
        drop(runtime);
        if !pending {
            break;
        }
        owner.drain_monster_melee_at(now).await;
    }
    assert!(
        !owner
            .runtime
            .lock()
            .await
            .has_pending_source_creature_deaths(),
        "all copied source events must finish without advancing time"
    );
    assert!(
        owner
            .states
            .spell_states
            .lock()
            .await
            .source_death_corpse
            .is_none(),
        "the retained physical corpse must have its real writer ACK"
    );
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "independent actual source play case facts"
)]
fn source_cast_case(
    source_kind: crate::content::bosses::activity_template::PinnedActivitySourceRoom,
    slug: &'static str,
    ability: &'static str,
    origin: MovementLocalPosition,
    caster_at: MovementLocalPosition,
    expect_damage: bool,
    teleport: Option<MovementLocalPosition>,
    master_mode: bool,
) -> TestResult {
    source_cast_case_inner(
        source_kind,
        slug,
        ability,
        origin,
        caster_at,
        expect_damage,
        teleport,
        master_mode,
        false,
        false,
    )
}

#[allow(
    clippy::too_many_arguments,
    reason = "independent actual source play facts plus the physical uncredited-death assertion"
)]
fn source_cast_case_inner(
    source_kind: crate::content::bosses::activity_template::PinnedActivitySourceRoom,
    slug: &'static str,
    ability: &'static str,
    origin: MovementLocalPosition,
    caster_at: MovementLocalPosition,
    expect_damage: bool,
    teleport: Option<MovementLocalPosition>,
    master_mode: bool,
    prove_null_corpse: bool,
    prove_heart_death: bool,
) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
        run(async |admin| {
            let mut deps = Dependencies::new()?;
            let (controller, pin) = super::death_ml_loss_composed::production_generation_with_pin(deps.admitted.world_id, "content-1")?;
            let active = controller.active().ok_or("native activation")?;
            let native = active.native_gameplay().ok_or("native gameplay")?;
            let record = native.creature_profiles().records.iter()
                .find(|record| record.profile.target.key == format!("oteryn:creature.{slug}"))
                .ok_or("actual callback caster")?;
            if slug != "charging_outburst" {
                assert!(record.monster_melee.is_none(), "the source explosion is the sole HP-changing attack");
            }
            let crate::content::ProjectV2AuthoringProfileData::Behavior(behavior) =
                &record.behavior.as_ref().ok_or("actual source behavior")?.data else {
                    return Err("actual source behavior family".into());
                };
            let callback_index = behavior.attacks.iter().position(|entry| entry.ability.key == ability)
                .ok_or("actual source callback entry")?;
            for entry in behavior.attacks.iter().filter(|_| teleport.is_none() && slug != "charging_outburst") {
                if entry.ability.key == ability { continue; }
                let definition = record.attack_definitions.as_ref().ok_or("actual definitions")?.profiles.iter()
                    .find(|profile| profile.target == entry.ability).ok_or("other source attack")?;
                let crate::content::ProjectV2AuthoringProfileData::Ability(other) = &definition.data else { return Err("other source ability family".into()); };
                assert!(other.details.as_ref().is_some_and(|details| !details.effects.is_empty() && details.effects.iter().all(|effect| {
                    matches!(effect, crate::content::ProjectV2AbilityEffect::Inline(inline)
                        if matches!(inline.operation, crate::content::ProjectV2InlineEffectOperation::Condition { .. }))
                })), "no other attack may change HP in this callback scenario");
            }
            let source_cast = oteryn_gameplay_encounters::bosses::source_callbacks::source_callback_cast::SourceCallbackCast::qualify_native(
                native, &record.profile.target, callback_index, crate::foundation::profile_schedule::ScheduleList::Attack,
            ).map_err(debug)?.ok_or("closed actual callback source")?;
            assert_eq!(source_cast.parent().key, ability);
            let kind = source_kind;
            let source = active.qualify_activity_source_room(pin.qualified_room(), kind).map_err(debug)?;
            let room = crate::gameplay_transport::activity_runtime::source_bundle::qualify(
                &source, kind, active, deps.admitted.world_id, deps.admitted.channel_id,
            ).map_err(|_|"Brain Head exact source Bundle")?;
            let cells = room.movement_cells().clone();
            deps.room = pin.qualified_room().clone();
            deps.combat_room = Some(room.clone());
            let content = ChannelContentPin::from_activation(deps.admitted.world_id, 1,
                active.identity().server_artifact_digest(), active.identity().client_artifact_digest(),
                room.frame_binding().digest(), room.map_revision_digest(), (origin.x,origin.y,origin.floor));
            // Current SQL/session/lease proofs follow the expensive native catalogue load.
            let h = Harness::create_with_admission_seed(admin, &format!("srx3_native_cast_{slug}"), false, false).await?;
            super::reset();
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(deps.admitted.world_id, deps.admitted.channel_id,
                crate::foundation::NodeId::decode(&id(1)).map_err(debug)?, 1,1,1,"runtime-scope-assignment:1",32,content).map_err(debug)?;
            native.install_companion_policies(&mut runtime).map_err(debug)?;
            let reservation = runtime.reserve_fresh_session(deps.admitted.game_session_id).map_err(debug)?;
            deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime.bind_attacker_lease(deps.actor,deps.admitted.game_session_id,
                crate::foundation::CharacterLease::new(crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,2).map_err(debug)?).map_err(debug)?;
            runtime.initialize_pinned_test_position(deps.actor,
                origin).map_err(debug)?;
            let key = format!("oteryn:creature.{slug}");
            let initial_key = if master_mode { "oteryn:creature.razzagorn" } else { key.as_str() };
            let mut caster = runtime.spawn_arena_opponent(initial_key,caster_at,|_,_|Some(true)).map_err(debug)?;
            let master = master_mode.then_some(caster);
            if let Some(master) = master {
                let maximum = runtime.companion_snapshot(master).map_err(debug)?.maximum_health;
                runtime.set_test_creature_health(master, maximum / 10).map_err(debug)?;
            }

            deps.admitted.runtime_actor = Some(deps.actor);
            deps.runtime = tokio::sync::Mutex::new(runtime);
            let mut owner = deps.owner(&h,&authority);
            owner.active_generation = Some(active);
            owner.movement_cells = (&cells).into();
            if slug == "charging_outburst" {
                let quests = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1").map_err(debug)?;
                owner.quest_catalogue = Some(std::sync::Arc::new(quests.catalogue().clone()));
                owner.refresh_quest_session(deps.admitted.game_session_id, true).await;
            }
            {
                let runtime = owner.runtime.lock().await;
                owner.states.spell_states.lock().await.initialize(&runtime,deps.actor,deps.admitted.game_session_id,
                    crate::spell::cast::CharacterCastFacts {max_health:oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL,
                        ..crate::gameplay_transport::actor_spell::tests::FACTS},(0,0),owner.owner_now()).ok_or("player spell state")?;
            }
            let heart_vortex = if prove_heart_death {
                let runtime = owner.runtime.lock().await;
                let context = active.qualify_callback_source_context(&room, runtime.content_pin(), source_kind)
                    .ok_or("actual Outburst callback source context")?;
                let (x,y,floor) = context.source_frame().physical_position([32225,31285,14])
                    .ok_or("actual source vortex projection")?;
                let tile = crate::map::overlay::TilePos {
                    x:u16::try_from(x)?, y:u16::try_from(y)?, floor:i8::try_from(floor)?.checked_neg().ok_or("native source floor")?,
                };
                let map = owner.bundle_map().ok_or("actual Outburst Bundle map")?;
                let stack = crate::map::view::compose_tile(&map.overlay,&map.facts,tile).map_err(debug)?
                    .ok_or("actual source vortex tile")?;
                let mut candidates = stack.entries.iter().filter(|entry|entry.facts.appearance_id==23483);
                let original = candidates.next().ok_or("actual retained source23483")?;
                assert!(candidates.next().is_none(),"source fixture must have one current23483");
                let crate::map::view::EntrySource::Base{placement_key,..}=original.source else {
                    return Err("real captured source vortex required".into());
                };
                assert_eq!(map.facts.action_at(placement_key),None,"pinned initial current23483 has no action");
                Some((tile,placement_key))
            } else {None};
            owner.clock_origin -= std::time::Duration::from_secs(10);
            let mut selected = false;
            let mut entered_source_owner = false;
            let mut damaged = false;
            let mut selected_at = None;
            for _ in 0..128 {
                owner.drain_monster_melee().await;
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                if let Some(master) = master.filter(|_| !selected)
                    && let Some(child) = runtime.positioned_actor_census().map_err(debug)?.into_iter()
                        .map(|(actor, _, _)| actor).find(|actor|
                            runtime.native_summon_master(*actor) == Some(master)
                                && runtime.matches_live_creature_identity(*actor, key.as_bytes()))
                {
                    caster = child;
                }
                if let Some(report) = states.monster_melee.creature_ai.last_report_for_test(caster) {
                    selected |= report.proposals.iter().any(|proposal| proposal.ability.key == ability);
                }
                damaged = states.get(&runtime, deps.actor, deps.admitted.game_session_id)
                    .ok_or("current player HP owner")?.vitals().health
                    < oteryn_protocol_oteryn::actor_spell::MAX_VITAL_POOL;
                if selected {
                    selected_at.get_or_insert(owner.owner_now().get());
                    entered_source_owner |= states.native_callbacks.source_cast_status_for_test(caster).0;
                }
                let due_elapsed = selected_at.is_some_and(|at| owner.owner_now().get().saturating_sub(at) >= 8_000_000);
                let master_finished = master_mode && selected && !runtime.contains_live_creature(caster);
                drop(states);
                drop(runtime);
                if selected && (slug == "charging_outburst" || teleport.is_some() || master_finished || (!master_mode && damaged) || (!expect_damage && due_elapsed)) {break;}
                owner.clock_origin -= std::time::Duration::from_secs(2);
            }
            assert!(selected,"actual native scheduler must select {ability}");
            if slug == "charging_outburst" {
                let runtime = owner.runtime.lock().await;
                assert_eq!(runtime.read_actor_position(deps.actor).map_err(debug)?.position(),
                    MovementLocalPosition{x:32234,y:31280,floor:14},
                    "selected native Charging Outburst must immediately teleport current source players");
                let states=owner.states.spell_states.lock().await;
                let receipt=states.native_callbacks.outburst_cast_receipt_for_test(caster)
                    .expect("actual source birth and canonical NULL health receipt");
                let replacement = receipt.replacement.expect("actual normal Outburst replacement");
                assert_ne!(replacement,caster);
                assert_eq!(receipt.global_health,0,"no historical normal Outburst exists in this source case");
                assert_eq!(receipt.synchronized_health,Some(0),"must use literal source global default, not Charging HP");
                assert!(receipt.committed_null_lethal,"real canonical lethal receipt required, not absence inference");
                assert!(receipt.no_player_credit,"NULL synchronization cannot invent player contribution");
                drop(states);
                assert!(runtime.contains_live_creature(caster), "source removal waits one second");
                assert!(!runtime.positioned_actor_census().map_err(debug)?.iter().any(|(actor,_,_)|
                    runtime.matches_live_creature_identity(*actor,b"oteryn:creature.outburst")),
                    "default global zero must never leave a live replacement with Charging HP");
                drop(runtime);
                drain_source_death(&owner, replacement, owner.owner_now()).await?;
                owner.clock_origin -= std::time::Duration::from_millis(1001);
                owner.drain_monster_melee().await;
                assert!(!owner.runtime.lock().await.contains_live_creature(caster),
                    "same source callback owner must remove its original Charging caster at one second");
                if let Some((tile,placement_key)) = heart_vortex {
                    let map = owner.bundle_map().ok_or("current Outburst Bundle map")?;
                    let stack = crate::map::view::compose_tile(&map.overlay,&map.facts,tile).map_err(debug)?
                        .ok_or("current source vortex tile")?;
                    let current = stack.entries.iter().find(|entry|matches!(entry.source,
                        crate::map::view::EntrySource::Base{placement_key:key,..} if key==placement_key))
                        .ok_or("same actual source WorldObject")?;
                    assert_eq!(current.facts.appearance_id,23482,"NULL-killed actual Outburst must run copied HeartBossDeath current-vortex transform");
                    assert_eq!(map.facts.action_at(placement_key),Some(14325),"Heart death publishes its literal action atomically with current23482");
                    let progress:Option<i64> = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/storage/14332'")
                        .bind(id(41).as_slice()).fetch_optional(&h.pool).await?;
                    assert_eq!(progress,Some(1),"current in-room Player receives Heart storage14332 even when NULL lethal has no contribution credit");
                    owner.drain_monster_melee().await;
                    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/storage/14332'")
                        .bind(id(41).as_slice()).fetch_one(&h.pool).await?,1,"copied source death replay must retain canonical progress");
                }
                if prove_null_corpse {
                    let materialized:Vec<(String,Option<String>)>=sqlx::query_as(
                        "SELECT i.item_instance_id::text,m.corpse_top_damage_character_id::text FROM game_item_mint_receipts m JOIN game_item_instances i USING(item_instance_id) JOIN game_item_ground_locations g USING(item_instance_id) WHERE m.loot_purpose_key='CORPSE_MATERIALIZATION' AND m.draw_ordinal=0 AND i.definition_production_key='oteryn:item.tibia.i23564' AND i.lifecycle=1 AND i.quantity=1")
                        .fetch_all(&h.pool).await?;
                    assert_eq!(materialized.len(),1,"actual canonical NULL-killed normal Outburst must materialize its physical source23564 corpse without a fabricated player");
                    assert!(materialized[0].1.is_none(),"uncredited source death cannot invent a top-damage Character");
                    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT (SELECT count(*) FROM game_item_corpse_container_entries)+(SELECT count(*) FROM game_item_nested_entries)").fetch_one(&h.pool).await?,0,"rewardBoss NULL corpse is empty, not ordinary46-entry loot");
                    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_boss_reward_items").fetch_one(&h.pool).await?,0,"no contributor means no private boss reward entitlement");
                }

            } else if master_mode {
                let runtime = owner.runtime.lock().await;
                let states = owner.states.spell_states.lock().await;
                let receipt = states.native_callbacks.eruption_master_receipt_for_test(caster)
                    .expect("actual intrinsic Eruption must execute its native master-heal/Demon prelude before removal");
                assert!((20_000..=30_000).contains(&receipt.requested));
                let maximum = runtime.companion_snapshot(receipt.master).map_err(debug)?.maximum_health;
                assert!(receipt.health_after > receipt.health_before);
                assert_eq!(receipt.health_after, maximum.min(receipt.health_before + i64::from(receipt.requested)));
                let demon = runtime.companion_snapshot(receipt.demon.expect("actual source Demon birth")).map_err(debug)?;
                assert_eq!(demon.state.policy.definition_key, "oteryn:creature.demon");
                assert_eq!(demon.maximum_health, 8200);
                assert!(!runtime.contains_live_creature(caster), "completed native explosion must retire Eruption");
            } else if let Some(expected) = teleport {
                let runtime = owner.runtime.lock().await;
                assert_eq!(runtime.read_actor_position(deps.actor).map_err(debug)?.position(), expected,
                    "selected native Tenebris source must immediately teleport the current player");
                assert_eq!(runtime.read_actor_position(caster).map_err(debug)?.position(), expected,
                    "selected native Tenebris source must immediately teleport every Lady in its source room");
                assert!(runtime.native_callback_movement_locked(caster).map_err(debug)?, "source lock is movement-only");
                drop(runtime);
                owner.clock_origin -= std::time::Duration::from_secs(4);
                owner.drain_monster_melee().await;
                let runtime = owner.runtime.lock().await;
                assert!(!runtime.native_callback_movement_locked(caster).map_err(debug)?, "actual source 4s event unlocks its original caster");

            } else {
            assert_eq!(damaged, expect_damage,
                "selected {ability} must preserve its source HP behavior; this native creature has no other damaging attack");
            }
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            assert!(entered_source_owner,
                "the selected source proposal must enter its actual callback owner before source retirement prunes the live scheduling memo");
            if !expect_damage && teleport.is_none() && !master_mode && slug != "charging_outburst" {
                assert!(runtime.contains_live_creature(caster), "nil-master Lua failure must not remove Eruption");
                assert!(!states.native_callbacks.source_cast_status_for_test(caster).1,
                    "the attempted nil-master source event must complete without leaving a phantom delayed cast");
            }
            drop(states);
            drop(runtime);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        }).map_err(|error|error.to_string())
    })?.join().map_err(|_|"native cast callback thread panicked")??;
    eprintln!(
        "SRX3_NATIVE_SOURCE_CAST POSTGRES_CASE_COMPLETED duration={:?}",
        started.elapsed()
    );
    Ok(())
}

#[test]
fn srx3_energy_pulse_actual_native_proposal_applies_source_explosion_postgres() -> TestResult {
    source_cast_case(
        crate::content::bosses::activity_template::PinnedActivitySourceRoom::BrainHead,
        "energy_pulse",
        "oteryn:ability.spell.energy_pulse_explosion",
        MovementLocalPosition {
            x: 31953,
            y: 32325,
            floor: 10,
        },
        MovementLocalPosition {
            x: 31953,
            y: 32324,
            floor: 10,
        },
        true,
        None,
        false,
    )
}

#[test]
fn srx3_eruption_actual_native_missing_master_aborts_delayed_lua_without_damage_postgres()
-> TestResult {
    source_cast_case(
        crate::content::bosses::activity_template::PinnedActivitySourceRoom::BrainHead,
        "eruption_of_destruction",
        "oteryn:ability.spell.eruption_of_destruction_explosion",
        MovementLocalPosition {
            x: 31953,
            y: 32325,
            floor: 10,
        },
        MovementLocalPosition {
            x: 31953,
            y: 32324,
            floor: 10,
        },
        false,
        None,
        false,
    )
}

#[test]
fn srx3_tenebris_actual_native_ultimate_teleports_current_players_and_ladies_postgres() -> TestResult
{
    source_cast_case(
        crate::content::bosses::activity_template::PinnedActivitySourceRoom::CallbackLadyTenebris,
        "lady_tenebris",
        "oteryn:ability.spell.tenebris_ultimate",
        MovementLocalPosition {
            x: 32913,
            y: 31599,
            floor: 14,
        },
        MovementLocalPosition {
            x: 32913,
            y: 31600,
            floor: 14,
        },
        false,
        Some(MovementLocalPosition {
            x: 32912,
            y: 31599,
            floor: 14,
        }),
        false,
    )
}

#[test]
fn srx3_eruption_actual_intrinsic_razzagorn_master_heal_and_demon_postgres() -> TestResult {
    source_cast_case(
        crate::content::bosses::activity_template::PinnedActivitySourceRoom::CallbackRazzagorn,
        "eruption_of_destruction",
        "oteryn:ability.spell.eruption_of_destruction_explosion",
        MovementLocalPosition {
            x: 33424,
            y: 32466,
            floor: 14,
        },
        MovementLocalPosition {
            x: 33424,
            y: 32467,
            floor: 14,
        },
        false,
        None,
        true,
    )
}

#[test]
fn srx3_charging_outburst_actual_native_compound_default_global_zero_postgres() -> TestResult {
    source_cast_case(
        crate::content::bosses::activity_template::PinnedActivitySourceRoom::CallbackOutburst,
        "charging_outburst",
        "oteryn:ability.spell.outburst_explode",
        MovementLocalPosition {
            x: 32234,
            y: 31285,
            floor: 14,
        },
        MovementLocalPosition {
            x: 32234,
            y: 31284,
            floor: 14,
        },
        true,
        None,
        false,
    )
}

#[test]
fn srx3_outburst_actual_null_death_materializes_empty_uncredited_corpse_postgres() -> TestResult {
    source_cast_case_inner(
        crate::content::bosses::activity_template::PinnedActivitySourceRoom::CallbackOutburst,
        "charging_outburst",
        "oteryn:ability.spell.outburst_explode",
        MovementLocalPosition {
            x: 32234,
            y: 31285,
            floor: 14,
        },
        MovementLocalPosition {
            x: 32234,
            y: 31284,
            floor: 14,
        },
        true,
        None,
        false,
        true,
        false,
    )
}

#[test]
fn srx3_outburst_actual_null_death_runs_copied_heart_vortex_and_current_player_progress_postgres()
-> TestResult {
    source_cast_case_inner(
        crate::content::bosses::activity_template::PinnedActivitySourceRoom::CallbackOutburst,
        "charging_outburst",
        "oteryn:ability.spell.outburst_explode",
        MovementLocalPosition {
            x: 32234,
            y: 31285,
            floor: 14,
        },
        MovementLocalPosition {
            x: 32234,
            y: 31284,
            floor: 14,
        },
        true,
        None,
        false,
        false,
        true,
    )
}
