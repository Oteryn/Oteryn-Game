//! Real source Hare AI -> production observer dictionary -> serialized native frames.
use super::*;
use crate::gameplay_transport::connection::{
    FreshAdmissionAuthority, seasonal_appearance_capture::SeasonalAppearanceCapture,
};
use sha2::{Digest, Sha256};
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

pub(super) async fn run(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
) -> TestResult {
    deps.initialize(owner).await;
    let session = deps.admitted.game_session_id;
    let fence = owner
        .current_monk_fence(session)
        .await
        .map_err(|_| "actual Hare observer fence")?
        .ok_or("Hare observer current session")?;
    // The actual observer owner requires the Character lease bound by fresh admission.
    deps.runtime
        .lock()
        .await
        .bind_attacker_lease(
            deps.actor,
            session,
            crate::foundation::CharacterLease::new(
                oteryn_protocol_oteryn::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    assert!(owner.track_restored(
        session,
        deps.actor,
        &crate::gameplay_transport::last_position::AdmissionPosition::FirstLogin,
        fence
    ));
    let mut admitted = deps.admitted;
    admitted.item_fence = Some(
        owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "Hare item fence")?
            .ok_or("Hare current item session")?
            .item_fence(),
    );
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            &offered,
            &[4, 6, 18, 25],
        )
        .ok_or("actual native appearance capabilities")?;
    assert!(
        admitted.continuity.selected_capabilities.contains(25),
        "production appearance offer must select25"
    );
    assert!(!admitted.continuity.selected_capabilities.contains(104));
    let accepted =
        crate::foundation::encode_server_accepted(&crate::foundation::ServerAcceptedValue {
            game_session_id: session,
            world_id: admitted.world_id,
            channel_id: admitted.channel_id,
            connection_generation: admitted.continuity.connection_generation,
            current_server_sequence: admitted.continuity.server_sequence,
            next_command_id: admitted.continuity.next_command_id,
            schema_revision: crate::gameplay_transport::connection::SERVER_SCHEMA_REVISION,
            selected_capabilities: admitted.continuity.selected_capabilities.as_slice(),
        })
        .map_err(debug)?;
    let visible = owner
        .observe_visible_entities(deps.actor)
        .await
        .ok_or("actual Hare initial visibility")?;
    let (mut recorder, domains) =
        SeasonalAppearanceCapture::begin(owner, deps.actor, &mut admitted, &visible)
            .await
            .ok_or("production Hare appearance snapshot")?;
    let map = recorder
        .initial_map(owner, deps.actor, &mut admitted)
        .await
        .ok_or("actual shared Hare initial map")?;
    let mut borrowed = domains
        .iter()
        .map(|d| oteryn_protocol_oteryn::DomainSnapshot {
            domain_id: d.domain_id,
            revision: d.revision,
            snapshot_type: d.snapshot_type,
            payload: &d.payload,
        })
        .collect::<Vec<_>>();
    borrowed.push(oteryn_protocol_oteryn::DomainSnapshot {
        domain_id: map.domain_id,
        revision: map.revision,
        snapshot_type: map.snapshot_type,
        payload: &map.payload,
    });
    let snapshots = oteryn_protocol_oteryn::encode_single_chunk_snapshot(
        admitted.continuity.connection_generation,
        1,
        admitted.continuity.server_sequence,
        &borrowed,
    )
    .map_err(debug)?;
    let mut initial = Vec::new();
    crate::gameplay_transport::tcp_tls::write_frame(&mut initial, &accepted).await?;
    for frame in snapshots {
        crate::gameplay_transport::tcp_tls::write_frame(&mut initial, &frame).await?;
    }
    let hare = {
        let runtime = deps.runtime.lock().await;
        let census = runtime.positioned_melee_census().map_err(debug)?;
        let hare = census
            .iter()
            .find_map(|(actor, _, session)| {
                if session.is_some() {
                    return None;
                }
                runtime
                    .companion_snapshot(*actor)
                    .ok()
                    .filter(|s| {
                        s.state.policy.definition_key == "oteryn:creature.the_halloween_hare"
                    })
                    .map(|_| *actor)
            })
            .ok_or("actual source Hare in native AI census")?;
        let snapshot = runtime.companion_snapshot(hare).map_err(debug)?;
        let native = owner
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
            .ok_or("actual Hare native source")?;
        let reference = crate::content::ProjectV2DefinitionRef {
            family: crate::content::ProjectV2Family::Creature,
            key: snapshot.state.policy.definition_key.clone(),
            revision: snapshot.state.policy.definition_revision.clone(),
        };
        let mut probe = crate::ai_think::CreatureAiTable::default();
        let admitted = probe.admit_native_creature(
            &runtime,
            native,
            hare,
            &reference,
            owner.owner_now().get(),
        );
        let observer = runtime
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position();
        let at = runtime.read_actor_position(hare).map_err(debug)?.position();
        let room = owner
            .rooms
            .gate_monster_ai()
            .ok_or("actual Hare combat room")?;
        let tile = room.movement_cells().spell_tiles().lookup(
            room.movement_cells().scope(),
            crate::content::LogicalCell {
                x: observer.x,
                y: observer.y,
                z: i32::from(observer.floor),
            },
        );
        eprintln!(
            "SRV7_HARE_AI_PROBE source={at:?} observer={observer:?} native_admission={admitted:?} observer_tile_known={} census={} generation_match={} protection={:?}",
            tile.is_ok(),
            census.len(),
            room.compiled().server_digest() == runtime.content_pin().server_artifact_digest(),
            runtime.current_player_monster_target_protection(
                deps.actor,
                session,
                owner.owner_now().get()
            )
        );
        admitted.map_err(debug)?;
        hare
    };
    let ground_capture = std::fs::read(
        super::hare_production_activation::repository_root()
            .join("tools/content-schema/native-gameplay/seasonal-ground-physics.json"),
    )?;
    let room = owner
        .rooms
        .gate_monster_ai()
        .ok_or("Hare actual qualified AI room")?;
    let mut qualified_ground_cells = Vec::new();
    for x in [32624, 32625] {
        let tile = room
            .movement_cells()
            .spell_tiles()
            .lookup(
                room.movement_cells().scope(),
                crate::content::LogicalCell { x, y: 32018, z: 7 },
            )
            .map_err(debug)?;
        assert_eq!(tile.ground_source_id(), Some(4515));
        assert!(
            tile.ground_present() && !tile.flags().block_solid && !tile.flags().block_projectile,
            "actual Hare/observer source ground must carry captured projectile fact"
        );
        qualified_ground_cells.push(serde_json::json!({"runtime_cell":[x,32018,7],"source_ground_id":4515,"ground_present":tile.ground_present(),"block_solid":tile.flags().block_solid,"block_projectile":tile.flags().block_projectile}));
    }
    let baseline = live_player_look(owner, deps.actor, session).await?;
    if std::env::var_os("OTERYN_SRV7_HARE_DIAGNOSTIC").is_some() {
        eprintln!("SRV7_HARE_OBSERVER_BASELINE {baseline:?}");
        let visible = owner
            .observe_visible_entities(deps.actor)
            .await
            .ok_or("Hare diagnostic visibility")?;
        let runtime = deps.runtime.lock().await;
        let snapshot = runtime.companion_snapshot(hare).map_err(debug)?;
        eprintln!(
            "SRV7_HARE_BODY key={} default={:?} finite={} appearance25={} visible={:?}",
            snapshot.state.policy.definition_key,
            snapshot.state.policy.outfit_appearance(),
            crate::gameplay_transport::seasonal_appearance::creature(&runtime, hare),
            admitted.continuity.selected_capabilities.contains(25),
            visible
                .entities
                .iter()
                .find(|e| e.identity == hare.placement_identity())
                .map(|e| e.kind)
        );
    }
    let initial_observer = deps
        .runtime
        .lock()
        .await
        .read_actor_position(deps.actor)
        .map_err(debug)?
        .position();
    let mut follow_due = 0;
    let mut paced_steps = 0_u32;
    let mut seen = std::collections::BTreeSet::new();
    let mut frames = Vec::new();
    for attempt in 0..4096 {
        owner.clock_origin -= std::time::Duration::from_millis(200);
        if super::hare_follow::step(owner, deps.actor, session, hare, &mut follow_due).await? {
            paced_steps += 1;
        }
        let drain_started = std::time::Instant::now();
        let event_now = owner.owner_now().get();
        owner.drain_monster_melee().await;
        if std::env::var_os("OTERYN_SRV7_HARE_DIAGNOSTIC").is_some() && attempt < 33 {
            let now = owner.owner_now().get();
            let runtime = deps.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            let spell = states
                .actors
                .iter()
                .find(|(actor, held_session, _)| *actor == deps.actor && *held_session == session)
                .map(|(_, _, state)| {
                    state
                        .owned_conditions()
                        .displayed_temporary_appearance_at(now)
                });
            let owned = runtime
                .actor_conditions_at(deps.actor, Some(session), now)
                .map(|conditions| conditions.displayed_temporary_appearance_at(now));
            let spell_outfit = states
                .actors
                .iter()
                .find(|(actor, held_session, _)| *actor == deps.actor && *held_session == session)
                .map(|(_, _, state)| {
                    format!(
                        "{:?}",
                        state
                            .owned_conditions()
                            .get(oteryn_gameplay_domain::domain::condition::ConflictKey::Outfit)
                    )
                });
            let owned_outfit = runtime
                .actor_conditions_at(deps.actor, Some(session), now)
                .map(|conditions| {
                    format!(
                        "{:?}",
                        conditions
                            .get(oteryn_gameplay_domain::domain::condition::ConflictKey::Outfit)
                    )
                });
            eprintln!(
                "SRV7_HARE_CONDITIONS turn={attempt} event_now={event_now} now={now} drain_ms={} spell={spell:?} owned={owned:?} spell_conflict={spell_outfit:?} owned_conflict={owned_outfit:?} casts={}",
                drain_started.elapsed().as_millis(),
                states.monster_spells.casts.len()
            );
        }
        let display = live_player_look(owner, deps.actor, session).await?;
        let look = (display != baseline).then(|| format!("{display:?}"));
        if let Some(look) = look
            && seen.insert(look.clone())
        {
            let visible = owner
                .observe_visible_entities(deps.actor)
                .await
                .ok_or("actual transformed Hare visibility")?;
            let mut bytes = recorder
                .refresh(owner, deps.actor, &mut admitted, &visible)
                .await
                .ok_or("production Hare transformed mapping")?;
            bytes.extend(
                recorder
                    .refresh_map(owner, deps.actor, &mut admitted)
                    .await
                    .ok_or("production Hare moved map")?,
            );
            if bytes.is_empty() {
                return Err("Hare visible transformation produced no native frames".into());
            }
            frames.push(serde_json::json!({"look":look,"owner_time_us":owner.owner_now().get(),"frames_hex":hex(&bytes),"sha256":hex(&Sha256::digest(&bytes))}));
        }
        if std::env::var_os("OTERYN_SRV7_HARE_DIAGNOSTIC").is_some() && attempt >= 32 {
            break;
        }
        if seen.len() == 15 {
            break;
        }
    }
    if seen.len() != 15 {
        let runtime = deps.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        eprintln!(
            "SRV7_HARE_AI_FINAL source={:?} observer={:?} table_present={} target={:?} casts={} ai_sequence={} pending={} control_current={}",
            runtime.read_actor_position(hare),
            runtime.read_actor_position(deps.actor),
            states.monster_melee.creature_ai.state(hare).is_some(),
            states
                .monster_melee
                .creature_ai
                .state(hare)
                .and_then(|s| s.target()),
            states.monster_spells.casts.len(),
            states.monster_ai_sequence,
            states.has_pending_spell_commit(deps.actor, session),
            runtime
                .player_control_facts(deps.actor, session)
                .is_ok_and(|f| f.control_loss.is_none())
        );
    }
    owner.clock_origin -= std::time::Duration::from_millis(6001);
    assert_eq!(
        live_player_look(owner, deps.actor, session).await?,
        baseline,
        "productionobserver restoresactualbaseafterexpiry acrossbothcondition owners"
    );
    let visible = owner
        .observe_visible_entities(deps.actor)
        .await
        .ok_or("actual Hare expiry visibility")?;
    let mut cleanup = recorder
        .refresh(owner, deps.actor, &mut admitted, &visible)
        .await
        .ok_or("production Hare expired mapping")?;
    cleanup.extend(
        recorder
            .refresh_map(owner, deps.actor, &mut admitted)
            .await
            .ok_or("production Hare cleanup map")?,
    );
    let observer = deps
        .runtime
        .lock()
        .await
        .read_actor_position(deps.actor)
        .map_err(debug)?
        .position();
    let bundle =
        std::fs::read(std::env::var_os("OTERYN_SRV7_HARE_BUNDLE").ok_or("Hare Bundle path")?)?;
    let native = owner
        .active_generation
        .and_then(crate::content::ActiveGeneration::native_gameplay)
        .ok_or("actual Hare native content")?;
    let item2096 = native
        .item_index()
        .ok_or("actual native Item index")?
        .definition_ref("Item", "oteryn:item.tibia.i2096", "definition-r1")
        .ok_or("actual appearance-only Item2096 typed reference")?
        .get();
    let keys = std::fs::read(
        super::hare_production_activation::repository_root()
            .join("tools/content-schema/native-gameplay/item-keys.json"),
    )?;
    let client_content_generation = deps
        .runtime
        .lock()
        .await
        .content_pin()
        .client_artifact_digest();
    let character = admitted
        .item_fence
        .ok_or("actual admitted character fence")?
        .character_id;
    if let Some(path) = std::env::var_os("OTERYN_SRV7_HARE_CAPTURE") {
        std::fs::write(
            path,
            serde_json::to_vec_pretty(
                &serde_json::json!({"schema":"OTERYN_SEASONAL_HARE_NATIVE_CAPTURE/v1","complete":seen.len()==15,"expected_transformations":15,"source":{
            "ground_physics_capture_sha256":hex(&Sha256::digest(&ground_capture)),"qualified_ground_cells":qualified_ground_cells,"producer_test":"srv7_halloween_hare_native_map_runtime_e2e","b3_sha256":"6ca7aa92d56ae6175c091bc8476817e5d8cadfa9ca4bc10af5dc2a8905b6b047",
            "bundle_sha256":hex(&Sha256::digest(&bundle)),"world_id_hex":hex(admitted.world_id.as_bytes()),"character_id_hex":hex(character.as_bytes()),"item_keys_sha256":hex(&Sha256::digest(&keys)),"client_content_generation":hex(&client_content_generation),"item_2096_definition_ref":item2096,"actor_hex":hex(&deps.actor.placement_identity()),
            "observer_runtime":{"x":initial_observer.x,"y":initial_observer.y,"floor":initial_observer.floor},"observer_native":{"x":initial_observer.x,"y":initial_observer.y,"floor":-i32::from(initial_observer.floor)},"final_observer_runtime":{"x":observer.x,"y":observer.y,"floor":observer.floor},"paced_steps":paced_steps,"think_tick_ms":200},
            "initial_frames_hex":hex(&initial),"initial_sha256":hex(&Sha256::digest(&initial)),"transformations":frames,"cleanup_frames_hex":hex(&cleanup),"cleanup_sha256":hex(&Sha256::digest(&cleanup))}),
            )?,
        )?;
    }
    assert_eq!(
        seen.len(),
        15,
        "actual source AI must expose all fifteen native looks"
    );
    Ok(())
}

async fn live_player_look(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
) -> TestResult<oteryn_protocol_oteryn::appearance::Look> {
    let entity = oteryn_protocol_oteryn::appearance::EntityRef {
        identity: actor.placement_identity(),
        generation: actor.actor_local_generation(),
    };
    owner
        .appearance_observation(actor, session, vec![entity])
        .await
        .ok_or("actual production player appearance observation")?
        .bindings
        .into_iter()
        .find(|b| b.entity == entity)
        .map(|b| b.look)
        .ok_or_else(|| "actual observed player appearance binding".into())
}
