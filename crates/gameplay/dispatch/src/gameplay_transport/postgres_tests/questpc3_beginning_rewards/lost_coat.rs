//! Missing-coat replacement is source value, not a state-only clothing check.
use super::super::classic_quests::install;
#[path = "lost_coat/readback.rs"]
mod readback;
use super::super::fixture::{Harness, debug, id, run};
use super::super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use readback::progress;
use std::collections::BTreeMap;
use std::sync::Arc;
const QUEST: &str = "oteryn:quest.the_beginning_quest";
const SANTIAGO: &str = "oteryn:npc.santiago";
const BASE: &str = "oteryn:quest-progress/quest/u8_2/the_beginning_quest/";
const GREET: &str = oteryn_gameplay_quests::quest::beginning::GREET;
const COAT: &str = "oteryn:item.tibia.i3562";
const SEED: &str = "oteryn:quest-transition/fixture/beginning-lost-coat/prior-chest";
const REPLACE: &str = "oteryn:quest-reward/questpc3/beginning/lost-coat";
fn position((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x, y, floor }
}
#[test]
fn beginning_missing_coat_replaces_only_absent_owned_item_and_repeats_after_loss() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("BEGINNING_LOST_COAT POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256<<20).spawn(||{let started=std::time::Instant::now();match std::panic::catch_unwind(scenario){Ok(result)=>{eprintln!("BEGINNING_LOST_COAT POSTGRES_CASE_COMPLETED duration_ms={} success={}",started.elapsed().as_millis(),result.is_ok());result.map_err(|e|e.to_string())},Err(panic)=>{eprintln!("BEGINNING_LOST_COAT POSTGRES_CASE_COMPLETED duration_ms={} success=false assertion_panicked=true",started.elapsed().as_millis());std::panic::resume_unwind(panic)}}})?.join().map_err(|_|"lost coat scenario panicked")??;
    Ok(())
}
fn fixture_catalogue() -> TestResult<QuestStateCatalogue> {
    let source = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let mut tracks = BTreeMap::new();
    let mut transitions = Vec::new();
    for c in oteryn_gameplay_quests::quest::reward_callbacks::load()?
        .into_iter()
        .filter(|c| c.quest == QUEST)
    {
        for storage in &c.storage {
            tracks.insert(
                storage.track.clone(),
                source
                    .catalogue()
                    .track(&storage.track)
                    .ok_or("track")?
                    .clone(),
            );
        }
        transitions.push(
            source
                .catalogue()
                .transition(&c.key)
                .ok_or("transition")?
                .clone(),
        );
    }
    transitions.push(QuestTransition {
world_change: None,
        key: SEED.into(),
        quest: QUEST.into(),
        effects: vec![
            QuestEffect {
                track: GREET.into(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(3),
            },
            QuestEffect {
                track: format!("{BASE}santiago_quest_log"),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(2),
            },
        ],
        completes: false,
        experience: None,
    });
    QuestStateCatalogue::new("content-1", tracks.into_values().collect(), transitions)
        .map_err(|e| format!("{e:?}").into())
}
fn scenario() -> TestResult {
    let catalogue = Arc::new(fixture_catalogue()?);
    run(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin.clone(), "beginning_lost_coat", false, false)
                .await?;
        let version: String = sqlx::query_scalar("SHOW server_version")
            .fetch_one(&h.pool)
            .await?;
        eprintln!("BEGINNING_LOST_COAT POSTGRES_CASE_STARTED server_version={version}");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let mut geometry = oteryn_gameplay_quests::quest::world_callbacks::load()?
            .into_iter()
            .next()
            .ok_or("map prototype")?;
        let first = (32600, 31800, -7);
        geometry.selector.position = Some(position(first));
        geometry.selector.action = None;
        geometry.selector.unique = None;
        geometry.selector.appearance = Some(100);
        geometry.lever = None;
        geometry.teleport = None;
        geometry.item_on_target = None;
        geometry.carried_use = None;
        // These fields construct fixture geometry, never grants or executable offerings.
        geometry.passages = vec![];
        geometry.offerings = vec![];
        let world = super::super::quest_world_callbacks::world(&geometry, position(first))?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let start = Position {
            x: first.0 - 1,
            y: first.1,
            floor: first.2,
        };
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
            (
                i32::from(start.x),
                i32::from(start.y),
                -i16::from(start.floor),
            ),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                super::super::quest_world_callbacks::local(start),
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = &cells;
        owner.quest_catalogue = Some(catalogue);
        let mut admitted = deps.admitted;
        admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let fresh = super::super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence.item_fence(),
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
        super::super::item_look::seed_main_backpack(&h, &[]).await?;
        owner
            .states
            .chat
            .admit(admitted.game_session_id, "Quest witness", owner.owner_now());
        owner
            .states
            .quest_callbacks
            .fixture_clock_ms
            .store(3_600_000, std::sync::atomic::Ordering::Relaxed);
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let result = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: SEED.into(),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(37000).map_err(debug)?,
                    )),
                },
                Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?),
            )
            .await
            .map_err(debug)?;
        assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
        drop(slot);
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        let [santiago] = install(&owner, &admitted, [SANTIAGO]).await?;
        admitted.item_fence = Some(fence.item_fence());
        admitted.continuity.selected_capabilities =
            crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
                &[oteryn_protocol_oteryn::npc_service::CAPABILITY_NPC_SERVICE_V1],
            )
            .ok_or("NPC service selection")?;
        // Keep the admitted transport alive across the complete conversation.
        // Reconnects discard the connection-local reply cache and emit new snapshots.
        use crate::gameplay_transport::connection::{IDLE_LIVENESS, serve_admitted};
        use tokio::io::AsyncWriteExt;
        let (mut server, mut client) = tokio::io::duplex(1 << 20);
        let driving = async {
            let result: TestResult<(u32, i64, i64)> = async {
        assert_eq!(
            speak(&mut client, santiago, 1, "hi").await?,
            NpcIntentDisposition::Ok,
            "native Santiago source offer must open"
        );
        let first = speak(&mut client, santiago, 2, "yes").await?;
        let mut completed = 0;
        if first == NpcIntentDisposition::Ok && quantity(&h).await? == 1 {
            completed += 1;
            assert_eq!(
                progress(&owner, &admitted, &authority, GREET).await?,
                3,
                "source replacement preserves greeting storage"
            );
            assert_eq!(quantity(&h).await?, 1);
            speak(&mut client, santiago, 3, "bye").await?;
            speak(&mut client, santiago, 4, "hi").await?;
            speak(&mut client, santiago, 5, "yes").await?;
            if quantity(&h).await? == 1
                && progress(&owner, &admitted, &authority, GREET).await? == 3
            {
                completed += 1;
            }
            drop_owned_coat(&owner, &admitted).await?;
            speak(&mut client, santiago, 6, "bye").await?;
            speak(&mut client, santiago, 7, "hi").await?;
            let second = speak(&mut client, santiago, 8, "yes").await?;
            if second == NpcIntentDisposition::Ok && quantity(&h).await? == 2 {
                completed += 1;
            }
        }
        let coats = quantity(&h).await?;
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_quest_reward_sources WHERE transition_key=$1",
        )
        .bind(REPLACE)
        .fetch_one(&h.pool)
        .await?;
        eprintln!(
            "BEGINNING_LOST_COAT POSTGRES_EVIDENCE completed={completed} coats={coats} receipts={receipts}"
        );
                Ok((completed, coats, receipts))
            }.await;
            client.shutdown().await?;
            result
        };
        let serving = async {
            let ended = serve_admitted(&mut server, admitted, &owner, IDLE_LIVENESS).await;
            drop(server);
            ended
        };
        let (_, result) = tokio::time::timeout(
            std::time::Duration::from_secs(60),
            super::super::fixture::join_two(serving, driving),
        )
        .await?;
        let (completed, coats, receipts) = result?;
        // Read the final committed state after the transport releases its root work.
        assert_eq!(progress(&owner, &admitted, &authority, GREET).await?, 3);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        assert_eq!(
            completed, 3,
            "first missing coat, owned duplicate refusal, and actual-loss replacement must all work"
        );
        assert_eq!(coats, 2);
        assert_eq!(receipts, 2);
        Ok(())
    })
}
async fn quantity(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1").bind(COAT).fetch_one(&h.pool).await?)
}
async fn speak(
    client: &mut tokio::io::DuplexStream,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    command: u64,
    text: &str,
) -> TestResult<NpcIntentDisposition> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    // NPC0-RL-04 admits four intents per second.
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    let payload = oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
        npc_actor: npc,
        text: text.into(),
    })
    .map_err(debug)?;
    let frame = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id: command,
            command_type: oteryn_protocol_oteryn::npc_service::COMMAND_TYPE_NPC_TALK_INTENT,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    let copies = if command == 2 { 2 } else { 1 };
    for _ in 0..copies {
        client
            .write_all(&u32::try_from(frame.len())?.to_be_bytes())
            .await?;
        client.write_all(&frame).await?;
    }
    let mut result = None;
    let mut replies = 0;
    while replies < copies {
        let mut prefix = [0; 4];
        client.read_exact(&mut prefix).await?;
        let mut frame = vec![0; u32::from_be_bytes(prefix) as usize];
        client.read_exact(&mut frame).await?;
        let envelope = crate::foundation::decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == crate::foundation::MessageType::CommandResult {
            let reply =
                oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(reply.command_id, command);
            let payload = reply.payload.to_vec();
            if let Some(previous) = &result {
                assert_eq!(previous, &payload, "exact reward reply replays");
            }
            result = Some(payload);
            replies += 1;
        }
    }
    oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(
        &result.ok_or("no command result")?,
    )
    .map_err(debug)
}
async fn drop_owned_coat(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
) -> TestResult {
    use crate::durability::item_loop_move::{LoopDestination, LoopGround, LoopMoveRequest};
    let budget = crate::gameplay_transport::AdmissionRetryBudget::for_command(owner.root);
    let fence = owner
        .current_quest_fence_within(&budget, admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let inventory = budget
        .retry_unavailable(async || {
            owner
                .root
                .read_item_use_inventory(owner.character, owner.holder, fence.item_fence())
                .await
                .map_err(|error| match error {
                    crate::durability::item_use::ItemUseError::Unavailable(error) => {
                        readback::classify(error)
                    }
                    _ => crate::gameplay_transport::connection::AdmissionRefusal::Rejected,
                })
        })
        .await
        .map_err(debug)?;
    let item = inventory
        .backpack
        .as_ref()
        .ok_or("backpack")?
        .entries
        .iter()
        .find(|e| e.item.definition.production_key == COAT)
        .ok_or("actual granted coat")?
        .item
        .item_instance_id;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    let (digest, frame) = {
        let runtime = owner.runtime.lock().await;
        (
            runtime.content_pin().server_artifact_digest(),
            runtime.content_pin().frame_binding_digest(),
        )
    };
    let revision = format!(
        "sha256:{}",
        digest
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect::<String>()
    );
    let permit = owner.spell_lane_permit().await.ok_or("permit")?;
    let map = owner.bundle_map().ok_or("map")?;
    let result = owner
        .root
        .commit_item_loop_move(
            &permit,
            owner.character,
            owner.holder,
            &fence.item_fence(),
            CommandRef::new(
                admitted.game_session_id,
                CommandId::new(37009).map_err(debug)?,
            ),
            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
            LoopMoveRequest {
                item,
                count: None,
                pickup_cell: None,
                destination: LoopDestination::Ground(LoopGround {
                    spatial_position:
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            32599, 31800, -7,
                        ),
                    map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                    content_revision: revision,
                    placement_context: frame.to_vec(),
                }),
            },
        )
        .await
        .map_err(debug)?;
    assert!(result.is_ok(), "actual coat drop: {result:?}");
    Ok(())
}
