//! Real essence USE followed by the literal public Bakragore flame USE.
use super::TestResult;
use super::fixture::{Harness, debug, id, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::MovementLocalPosition;
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
};

#[test]
fn srx3_bakragore_actual_essence_then_public_flame_use_prepares_phase_postgres() -> TestResult {
    run_flame_case(FlameCase::Normal)
}
#[test]
fn srx3_bakragore_actual_no_essence_flame_use_is_true_without_feedback_or_burn_postgres()
-> TestResult {
    run_flame_case(FlameCase::NoEssence)
}
#[test]
fn srx3_bakragore_historical_global_one_unlit_flame_keeps_essence_and_reports_activated_postgres()
-> TestResult {
    run_flame_case(FlameCase::HistoricalGlobalActive)
}
#[test]
fn srx3_rotten_essence_already_held_actual_wire_returns_true_with_literal_feedback_postgres()
-> TestResult {
    run_flame_case(FlameCase::EssenceAlreadyHeld)
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FlameCase {
    Normal,
    NoEssence,
    HistoricalGlobalActive,
    EssenceAlreadyHeld,
}
fn run_flame_case(mode: FlameCase) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("srx3-bakragore-flame", move || {
        run(async |admin| {
            super::reset();
            // Exact source walkable tile next to Murcion's literal flame.
            let scene = SourceScene::new(
                PinnedActivitySourceRoom::Bakragore,
                MovementLocalPosition {
                    x: 33084,
                    y: 32398,
                    floor: 15,
                },
            )?;
            let deps = scene.dependencies()?;
            let h =
                Harness::create_with_admission_seed(admin, "srx3_bak_flame", false, false).await?;
            let evidence = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h).await?;
            // Imported historical inventory only, before fresh authority proof.
            super::item_look::seed_main_backpack(
                &h,
                &[(
                    "oteryn:item.tibia.i43501",
                    if mode == FlameCase::EssenceAlreadyHeld {
                        2
                    } else {
                        1
                    },
                )],
            )
            .await?;
            super::boss_activity::issue_content_and_reset(&h, &scene.pin).await?;
            if mode == FlameCase::HistoricalGlobalActive {
                seed_historical_defensive_global_phase(&h, &deps).await?;
            }
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.evidence = &evidence.evidence;
            install_scene(&mut owner, &scene, PinnedActivitySourceRoom::Bakragore)?;
            let catalogue =
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?;
            owner.quest_catalogue = Some(std::sync::Arc::new(catalogue.catalogue().clone()));
            let mut admitted = deps.admitted;
            admitted.continuity.selected_capabilities =
                crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                    crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
                    &[4, 6, 7, 10, 11, 15, 18, 23, 92, 117],
                )
                .ok_or("actual source USE capabilities")?;
            prepare_player(&h, &owner, &mut admitted, 1999).await?;
            let native = scene
                .controller
                .active()
                .and_then(|g| g.native_gameplay())
                .ok_or("native policy")?;
            let essence = native
                .item_index()
                .and_then(|index| {
                    index.definition_ref("Item", "oteryn:item.tibia.i43501", "definition-r1")
                })
                .ok_or("actual source essence")?;
            let item_use =
                oteryn_protocol_oteryn::world_object::encode_item_use_intent(&UseIntent {
                    target: UseTarget::ItemByDefinition(essence),
                    use_with: None,
                    use_with_map_item: None,
                    use_with_position: None,
                })
                .map_err(debug)?;
            if mode == FlameCase::EssenceAlreadyHeld {
                let (first, _) = flame_wire(&owner, &mut admitted, 1, item_use.clone()).await?;
                assert_eq!(
                    oteryn_protocol_oteryn::world_object::decode_use_result(&first)
                        .map_err(debug)?,
                    UseDisposition::Committed
                );
                let map = owner.bundle_map().ok_or("actual source effect owner")?;
                let cursor = map
                    .facts
                    .movement_effects_since(0)
                    .last()
                    .map_or(0, |(seq, _, _)| *seq);
                let (second, feedback) = flame_wire(&owner, &mut admitted, 2, item_use).await?;
                assert_eq!(
                    oteryn_protocol_oteryn::world_object::decode_use_result(&second)
                        .map_err(debug)?,
                    UseDisposition::Committed,
                    "Crystal rottenEssences.lua16–19 returns true for the actual held flag"
                );
                let text = feedback
                    .iter()
                    .filter_map(|line| match line {
                        oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } => {
                            Some(text.as_str())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                assert_eq!(text, vec!["sorry not possible."]);
                assert!(
                    owner
                        .bundle_map()
                        .ok_or("current source effect owner")?
                        .facts
                        .movement_effects_since(cursor)
                        .is_empty(),
                    "already-held essence emits no second blue effect"
                );
                assert_eq!(
                    sqlx::query_scalar::<_, i64>(
                        "SELECT count(*) FROM game_item_use_records WHERE cause_kind=4"
                    )
                    .fetch_one(&h.pool)
                    .await?,
                    1
                );
                assert_eq!(sqlx::query_scalar::<_,i64>("SELECT quantity FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i43501' AND lifecycle=1").fetch_one(&h.pool).await?, 1);
                assert_eq!(
                    sqlx::query_scalar::<_, i64>(
                        "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1"
                    )
                    .bind(oteryn_gameplay_quests::quest::rotten_blood::ESSENCES[0].transition)
                    .fetch_one(&h.pool)
                    .await?,
                    1
                );
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            if mode != FlameCase::NoEssence {
                let reply =
                    super::quest_world_callbacks::wire(&owner, &mut admitted, 1, 2, item_use)
                        .await?;
                assert_eq!(
                    oteryn_protocol_oteryn::world_object::decode_use_result(&reply)
                        .map_err(debug)?,
                    UseDisposition::Committed,
                    "real essence USE sets its canonical flag"
                );
                assert_eq!(sqlx::query_scalar::<_,i64>("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/storage/52280'").bind(id(41).as_slice()).fetch_one(&h.pool).await?,1);
            }
            let map = owner.bundle_map().ok_or("actual captured public map")?;
            let tile = crate::map::overlay::TilePos {
                x: 33085,
                y: 32398,
                floor: -15,
            };
            let stack = crate::map::view::compose_tile(&map.overlay, &map.facts, tile)
                .map_err(debug)?
                .ok_or("actual source flame tile")?;
            let mut selected = stack
                .entries
                .iter()
                .filter(|entry| entry.facts.appearance_id == 43983);
            let entry = selected.next().ok_or("actual inactive Murcion flame")?;
            assert!(selected.next().is_none());
            let crate::map::view::EntrySource::Base {
                placement_key: key, ..
            } = entry.source
            else {
                return Err("source flame must be its actual fixed WorldObject".into());
            };
            let mut placement = map.overlay.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            let target = WorldObjectTarget {
                placement,
                expected_revision: map.facts.object_revision(key),
            };
            let payload =
                oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?;
            let flame_effect_cursor = map
                .facts
                .movement_effects_since(0)
                .last()
                .map_or(0, |(sequence, _, _)| *sequence);
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("source USE clock")?;
            let (reply, feedback) = flame_wire(
                &owner,
                &mut admitted,
                if mode == FlameCase::NoEssence { 1 } else { 2 },
                payload,
            )
            .await?;
            assert_eq!(
                oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
                UseDisposition::Committed,
                "actual source flame USE returns true for mode {mode:?}; only Normal consumes the essence"
            );
            if mode != FlameCase::Normal {
                let held=sqlx::query_scalar::<_,i64>("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/storage/52280'").bind(id(41).as_slice()).fetch_optional(&h.pool).await?.unwrap_or(0);
                assert_eq!(
                    held,
                    if mode == FlameCase::NoEssence { 0 } else { 1 },
                    "source no-op never burns the Player essence"
                );
                assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND transition_key='oteryn:quest-callback/rotten-blood/prepare-murcion'").bind(id(41).as_slice()).fetch_one(&h.pool).await?,0);
                assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_rotten_bak_preparations WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?,0);
                let lines = feedback
                    .iter()
                    .filter_map(|line| match line {
                        oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } => {
                            Some(text.as_str())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                if mode == FlameCase::NoEssence {
                    assert!(
                        lines.is_empty(),
                        "literal no-essence branch has no message: {lines:?}"
                    );
                } else {
                    assert_eq!(lines, vec!["This flame has already been activated."]);
                }
                let map = owner.bundle_map().ok_or("current defensive flame map")?;
                assert!(
                    crate::map::view::compose_tile(&map.overlay, &map.facts, tile)
                        .map_err(debug)?
                        .ok_or("actual unlit tile")?
                        .entries
                        .iter()
                        .any(|entry| entry.facts.appearance_id == 43983)
                );
                assert!(
                    map.facts
                        .movement_effects_since(flame_effect_cursor)
                        .is_empty(),
                    "no source no-op MagicBlue"
                );
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/storage/52280'").bind(id(41).as_slice()).fetch_one(&h.pool).await?,0);
            let map = owner.bundle_map().ok_or("current source map")?;
            let after = crate::map::view::compose_tile(&map.overlay, &map.facts, tile)
                .map_err(debug)?
                .ok_or("current flame tile")?;
            assert!(
                after
                    .entries
                    .iter()
                    .any(|entry| entry.facts.appearance_id == 43984),
                "the actual public flame transforms after the known commit"
            );
            let flame_effects = map.facts.movement_effects_since(flame_effect_cursor);
            assert_eq!(
                flame_effects
                    .iter()
                    .filter(|(_, at, effect)| *effect == 13
                        && at.x == 33084
                        && at.y == 32398
                        && at.floor == -15)
                    .count(),
                1,
                "source Flame USE emits one native MagicBlue at the actual activating Player"
            );
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-callback/rotten-blood/prepare-murcion'").fetch_one(&h.pool).await?,1);
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_rotten_bak_preparations WHERE character_id=encode($1,'hex')::uuid AND flavor=0 AND flame_key IS NOT NULL AND pillar_key IS NOT NULL").bind(id(41).as_slice()).fetch_one(&h.pool).await?,1,
                "actual source burn retains one immutable scoped preparation with exact reset CAS provenance");

            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    });
    eprintln!(
        "SRX3_BAKRAGORE_ACTUAL_FLAME_POSTGRES_CASE_EXECUTED duration={:?} passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

/// Defensive boundary ONLY, not a claimed normal play journey. The initial
/// historical global1/unlit state models a source activation whose public visual
/// has not been published. Its unrelated historical Character has no live actor.
/// All current USE/flag/burn/feedback work below still uses the real owners.
pub(super) async fn seed_historical_defensive_global_phase(
    h: &Harness,
    deps: &super::Dependencies,
) -> TestResult {
    let (scope, generation, activation, server, client, frame, map) = {
        let runtime = deps.runtime.lock().await;
        let pin = runtime.content_pin();
        (
            runtime.binding().runtime_scope(),
            runtime.binding().scope_generation().get(),
            pin.activation_sequence(),
            pin.server_artifact_digest(),
            pin.client_artifact_digest(),
            pin.frame_binding_digest(),
            pin.map_revision_digest(),
        )
    };
    let world = scope.world_id();
    let channel = scope.channel_id().ok_or("historical public Channel")?;
    let catalogue = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let hash = catalogue
        .catalogue()
        .definition_hash("oteryn:quest/rotten-blood")
        .ok_or("historical actual Quest hash")?;
    // Reuse the established historical bootstrap fixture: a root without its
    // immutable creation receipt is correctly rejected by global recovery.
    crate::durability::guild_chat_fixture::historical_character(
        &h.pool,
        105,
        crate::domain::CharacterId::from_bytes(id(105)).map_err(debug)?,
        id(107),
        *world.as_bytes(),
        "Historical Flame Holder",
    )
    .await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_roots SET character_revision=2 WHERE character_id=encode($1,'hex')::uuid AND character_revision=1")
        .bind(id(105).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_progression_state VALUES(encode($1,'hex')::uuid,2,1,0,'profile-1','ruleset-1','content-1','simulation-1','evidence-1','declaration-1','policy-1','reward-1')")
        .bind(id(105).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_quest_receipts(character_id,cause_kind,cause_id,cause_ordinal,transition_key,quest_key,request_binding,pinned_content_revision,definition_hash,completes,track_keys,values_before,values_after,original_character_revision,committed_character_revision,level_before,level_after,experience_before,experience_after,profile_revision,ruleset_revision,content_revision,simulation_revision,evidence_revision,declaration_revision,policy_revision,reward_revision,committed_at) VALUES(encode($1,'hex')::uuid,'use',encode($2,'hex')::uuid,1,'oteryn:quest-callback/rotten-blood/prepare-murcion','oteryn:quest/rotten-blood',$3,'content-1',$4,false,ARRAY['oteryn:quest-progress/storage/52280'],ARRAY[1]::bigint[],ARRAY[0]::bigint[],1,2,1,1,0,0,'profile-1','ruleset-1','content-1','simulation-1','evidence-1','declaration-1','policy-1','reward-1',0)")
        .bind(id(105).as_slice()).bind(id(106).as_slice()).bind([1_u8;33].as_slice()).bind(hash.as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_quest_tracks VALUES(encode($1,'hex')::uuid,'oteryn:quest-progress/storage/52280','oteryn:quest/rotten-blood',0,2)")
        .bind(id(105).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_quest_states VALUES(encode($1,'hex')::uuid,'oteryn:quest/rotten-blood','content-1',$2,2,NULL,2)")
        .bind(id(105).as_slice()).bind(hash.as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_rotten_bak_preparations(character_id,cause_id,cause_ordinal,transition_key,request_binding,world_id,source_channel_id,source_generation,reset_epoch,activation_sequence,server_digest,client_digest,frame_digest,map_digest,flavor,source_phase_ordinal,committed_revision) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,'oteryn:quest-callback/rotten-blood/prepare-murcion',$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric(20,0),0,$7::text::numeric(20,0),$8,$9,$10,$11,0,0,2)")
        .bind(id(105).as_slice()).bind(id(106).as_slice()).bind([1_u8;33].as_slice())
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(generation.to_string()).bind(activation.to_string())
        .bind(server.as_slice()).bind(client.as_slice()).bind(frame.as_slice()).bind(map.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

/// Actual command transport with decoded native feedback, including the short
/// post-result egress window. No direct callback or invented target is invoked.
async fn flame_wire(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    command: u64,
    payload: Vec<u8>,
) -> TestResult<(Vec<u8>, Vec<oteryn_protocol_oteryn::chat::ChatLine>)> {
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use oteryn_protocol_oteryn as protocol;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let frame = protocol::encode_client_command(
        1,
        &protocol::ClientCommandValue {
            command_id: command,
            command_type: 2,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client
        .write_all(&u32::try_from(frame.len())?.to_be_bytes())
        .await?;
    client.write_all(&frame).await?;
    let reader = tokio::spawn(async move {
        let result: TestResult<_> = async {
            let mut result = None;
            let mut feedback = Vec::new();
            loop {
                let mut prefix = [0; 4];
                let read = if result.is_some() {
                    match tokio::time::timeout(
                        std::time::Duration::from_millis(200),
                        client.read_exact(&mut prefix),
                    )
                    .await
                    {
                        Ok(read) => read,
                        Err(_) => {
                            client.shutdown().await?;
                            break;
                        }
                    }
                } else {
                    client.read_exact(&mut prefix).await
                };
                match read {
                    Ok(_) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
                    Err(e) => return Err(e.into()),
                };
                let mut frame = vec![0; u32::from_be_bytes(prefix) as usize];
                client.read_exact(&mut frame).await?;
                let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == protocol::MessageType::CommandResult {
                    let reply =
                        protocol::decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(reply.command_id, command);
                    result = Some(reply.payload.to_vec());
                } else if envelope.message_type() == protocol::MessageType::StateDelta {
                    let delta = protocol::decode_state_delta(envelope.payload()).map_err(debug)?;
                    if delta.domain_id == protocol::chat::STATE_DOMAIN_CHAT
                        && delta.delta_type == protocol::chat::DELTA_TYPE_CHAT_LINE_V1
                    {
                        feedback
                            .push(protocol::chat::decode_chat_line(delta.payload).map_err(debug)?);
                    }
                }
            }
            Ok((result.ok_or("no native flame result")?, feedback))
        }
        .await;
        result.map_err(|e| e.to_string())
    });
    let end = serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS).await;
    drop(server);
    let result = reader.await?.map_err(Into::into);
    if let ConnectionEnd::AdmittedThenDisconnected(a) | ConnectionEnd::AdmittedThenClosed(a, _) =
        end
    {
        *admitted = a;
    }
    result
}
