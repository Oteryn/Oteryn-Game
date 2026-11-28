//! Real essence USE followed by the literal public Bakragore flame USE.
use oteryn_gameplay_world::map::view::MapFacts;
use super::srx3_rotten_preparation_fixtures::{seed_historical_defensive_global_phase};

use super::TestResult;
use super::fixture::{Harness, debug, id, run};
use super::srx3_activity_source_fixture::{SourceScene, install_scene, prepare_player};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::MovementLocalPosition;

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
#[test]
fn srx3_partial_flame_publication_recovers() -> TestResult {
    for index in 0..4 {
        run_flame_case(FlameCase::Interrupted(index))?;
    }
    Ok(())
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FlameCase {
    Normal,
    Interrupted(usize),
    NoEssence,
    HistoricalGlobalActive,
    EssenceAlreadyHeld,
}
fn run_flame_case(mode: FlameCase) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run("srx3-bakragore-flame", move || {
        run(async |admin| {
            super::reset();
            use oteryn_gameplay_quests::quest::rotten_blood::{ESSENCES, flames};
            let index = match mode { FlameCase::Interrupted(i) => i, _ => 0 };
            let source = flames::FLAMES[index];
            let item = ESSENCES.iter().find(|item| item.track == source.storage).ok_or("source essence")?.item;
            let [x, y, floor] = source.flame;
            let start = MovementLocalPosition {
                x: x + (source.pillar[0] - x).signum(),
                y: y + (source.pillar[1] - y).signum(),
                floor: floor.try_into()?,
            };
            let scene = SourceScene::new(PinnedActivitySourceRoom::Bakragore, start)?;
            let deps = scene.dependencies()?;
            let h =
                Harness::create_with_admission_seed(admin, "srx3_bak_flame", false, false).await?;
            let evidence = super::house_transfer_fixtures::RecoverySource::start(&h).await?;
            super::house_transfer_fixtures::prepare_owned_session(&h).await?;
            // Imported historical inventory only, before fresh authority proof.
            super::item_look_fixtures::seed_main_backpack(
                &h,
                &[(
                    item,
                    if mode == FlameCase::EssenceAlreadyHeld {
                        2
                    } else {
                        1
                    },
                )],
            )
            .await?;
            super::boss_activity_fixtures::issue_content_and_reset(&h, &scene.pin).await?;
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
                    index.definition_ref("Item", item, "definition-r1")
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
                    super::quest_world_callbacks_fixtures::wire(&owner, &mut admitted, 1, 2, item_use)
                        .await?;
                assert_eq!(
                    oteryn_protocol_oteryn::world_object::decode_use_result(&reply)
                        .map_err(debug)?,
                    UseDisposition::Committed,
                    "real essence USE sets its canonical flag"
                );
                assert_eq!(sqlx::query_scalar::<_,i64>("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(id(41).as_slice()).bind(source.storage).fetch_one(&h.pool).await?,1);
            }
            let map = owner.bundle_map().ok_or("actual captured public map")?;
            let tile = crate::map::overlay::TilePos {
                x: x.try_into()?,
                y: y.try_into()?,
                floor: -15,
            };
            let stack = crate::map::view::compose_tile(&map.overlay, &map.facts, tile)
                .map_err(debug)?
                .ok_or("actual source flame tile")?;
            let mut selected = stack
                .entries
                .iter()
                .filter(|entry| entry.facts.appearance_id == source.inactive);
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
            let use_flame = flame_wire(
                &owner,
                &mut admitted,
                if mode == FlameCase::NoEssence { 1 } else { 2 },
                payload,
            );
            let interrupted = matches!(mode, FlameCase::Interrupted(_));
            let (reply, feedback) = crate::gameplay_transport::quest_callbacks::INTERRUPT_FLAME_VISUAL
                .scope(std::cell::Cell::new(interrupted), use_flame).await?;
            if interrupted {
                assert!(owner.reconcile_quest_callbacks(admitted.game_session_id).await);
            }
            assert_eq!(
                oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
                if interrupted { UseDisposition::Rejected } else { UseDisposition::Committed },
                "actual source flame USE returns true for mode {mode:?}; only Normal consumes the essence"
            );
            if !matches!(mode, FlameCase::Normal | FlameCase::Interrupted(_)) {
                let held=sqlx::query_scalar::<_,i64>("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(id(41).as_slice()).bind(source.storage).fetch_optional(&h.pool).await?.unwrap_or(0);
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
                        .any(|entry| entry.facts.appearance_id == source.inactive)
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
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2").bind(id(41).as_slice()).bind(source.storage).fetch_one(&h.pool).await?,0);
            let map = owner.bundle_map().ok_or("current source map")?;
            let after = crate::map::view::compose_tile(&map.overlay, &map.facts, tile)
                .map_err(debug)?
                .ok_or("current flame tile")?;
            assert!(
                after
                    .entries
                    .iter()
                    .any(|entry| entry.facts.appearance_id == source.active),
                "the actual public flame transforms after the known commit"
            );
            let pillar = crate::map::overlay::TilePos {
                x: source.pillar[0].try_into()?,
                y: source.pillar[1].try_into()?,
                floor: -15,
            };
            assert!(crate::map::view::compose_tile(&map.overlay, &map.facts, pillar)
                .map_err(debug)?.ok_or("source pillar")?.entries.iter()
                .any(|entry| entry.facts.appearance_id == flames::PILLAR_ACTIVE));
            assert!(!owner.has_pending_quest_callback(admitted.game_session_id));
            let flame_effects = map.facts.movement_effects_since(flame_effect_cursor);
            assert_eq!(
                flame_effects
                    .iter()
                    .filter(|(_, at, effect)| *effect == 13
                        && i32::from(at.x) == start.x
                        && i32::from(at.y) == start.y
                        && at.floor == -15)
                    .count(),
                1,
                "source Flame USE emits one native MagicBlue at the actual activating Player"
            );
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1").bind(source.key).fetch_one(&h.pool).await?,1);
            assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM game_rotten_bak_preparations WHERE character_id=encode($1,'hex')::uuid AND flavor=$2 AND flame_key IS NOT NULL AND pillar_key IS NOT NULL").bind(id(41).as_slice()).bind(i16::try_from(index)?).fetch_one(&h.pool).await?,1,
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

/// Actual command transport with native feedback observed through a CHAT egress marker.
async fn flame_wire(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    command: u64,
    payload: Vec<u8>,
) -> TestResult<(Vec<u8>, Vec<oteryn_protocol_oteryn::chat::ChatLine>)> {
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use oteryn_protocol_oteryn::{self as protocol, chat::ChatLine};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    const CHAT_MARKER: &str = "SRX3 flame test egress marker";
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
    let admitted_session = *admitted;
    let session = admitted_session.game_session_id;
    let reader = async move {
        tokio::time::timeout(std::time::Duration::from_secs(5), async move {
            let mut result = None;
            let mut feedback = Vec::new();
            loop {
                let mut prefix = [0; 4];
                client.read_exact(&mut prefix).await?;
                let mut frame = vec![0; u32::from_be_bytes(prefix) as usize];
                client.read_exact(&mut frame).await?;
                let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == protocol::MessageType::CommandResult {
                    let reply =
                        protocol::decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(reply.command_id, command);
                    result = Some(reply.payload.to_vec());
                    // The command has already queued its feedback. The same FIFO
                    // delivers the marker after every line queued by the command.
                    owner.states.chat.push_quest_feedback(session, CHAT_MARKER);
                } else if envelope.message_type() == protocol::MessageType::StateDelta {
                    let delta = protocol::decode_state_delta(envelope.payload()).map_err(debug)?;
                    if delta.domain_id == protocol::chat::STATE_DOMAIN_CHAT
                        && delta.delta_type == protocol::chat::DELTA_TYPE_CHAT_LINE_V1
                    {
                        let line =
                            protocol::chat::decode_chat_line(delta.payload).map_err(debug)?;
                        if matches!(&line, ChatLine::Private { speaker_name, text }
                            if speaker_name == "Game" && text == CHAT_MARKER)
                        {
                            let reply =
                                result.ok_or("CHAT marker arrived before the flame result")?;
                            return Ok::<_, Box<dyn std::error::Error>>((reply, feedback));
                        }
                        feedback.push(line);
                    }
                }
            }
        })
        .await
        .map_err(|_| "timed out waiting for flame command and CHAT egress")?
    };
    let serving =
        async move { serve_admitted(&mut server, admitted_session, owner, IDLE_LIVENESS).await };
    let (end, result) = tokio::join!(serving, reader);
    if let ConnectionEnd::AdmittedThenDisconnected(a) | ConnectionEnd::AdmittedThenClosed(a, _) =
        end
    {
        *admitted = a;
    }
    result
}
