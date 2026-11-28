//! The existing tracker commands and domain 16 across a fresh character sign-in.

use std::{sync::Arc, time::Duration};

use oteryn_protocol_oteryn::quest_log::{
    CAPABILITY_QUEST_LOG_V1, CAPABILITY_QUEST_MISSION_TRACKING_V1, COMMAND_TYPE_QUEST_LOG_QUERY,
    COMMAND_TYPE_QUEST_MISSION_TRACK, QuestLogQuery, STATE_DOMAIN_QUEST_LOG, TrackedMission,
    decode_quest_log, encode_quest_log_query, encode_quest_mission_track,
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, MessageType, decode_snapshot_chunk, decode_state_delta,
    decode_wire_envelope, encode_client_command, logout::LogoutResult,
};

use super::fixture::{Harness, debug};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::gameplay_transport::capabilities::{
    BUNDLE_WORLD_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::{
    AdmittedSession, FirstEntryOutcome, LogoutReply, SessionContinuity,
};
use crate::gameplay_transport::quest_log::QuestLogLines;

#[test]
fn quest_tracker_selection_survives_fresh_sign_in() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e_fixtures::scenario_thread::run_async(async move |_| {
        scenario(&admin, false).await?;
        scenario(&admin, true).await
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED quest_tracker_selection_survives_fresh_sign_in duration={:?} success={}",
        started.elapsed(),
        result.is_ok(),
    );
    result
}

async fn scenario(admin: &str, mission_tracking: bool) -> TestResult {
    let harness =
        Harness::create_with_admission_seed(admin.to_owned(), "quest_tracker_relogin", true, false)
            .await?;
    prepare_current_session(&harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let mut owner = deps.owner(&harness, &authority);
    let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let loaded = oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&lowered)?;
    let (quest_index, quest) = loaded
        .content
        .active()
        .quests()
        .iter()
        .enumerate()
        .find(|(_, quest)| quest.key == "oteryn:quest.a_father_s_burden_quest")
        .ok_or("quest index")?;
    let quest_index = u32::try_from(quest_index)?;
    let mission_index = u32::try_from(
        quest
            .missions
            .iter()
            .position(|mission| mission.name == "The Birthday Presents")
            .ok_or("mission index")?,
    )?;
    owner.quest_log_content = Some(QuestLogLines::new(loaded.content).ok_or("journal bounds")?);
    let catalogue = Arc::new(lowered.catalogue().clone());
    owner.quest_catalogue = Some(Arc::clone(&catalogue));
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let store = FreshAdmissionStore::from_root(harness.root.clone());
    let session = admitted.game_session_id;
    let current = store.current_session(session).await?;
    {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(deps.actor, session, current.current_character_lease())
            .map_err(debug)?;
    }
    admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
    let capability_ids = [
        CAPABILITY_QUEST_LOG_V1,
        CAPABILITY_QUEST_MISSION_TRACKING_V1,
    ];
    let selection = if mission_tracking {
        &capability_ids[..]
    } else {
        &capability_ids[..1]
    };
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(BUNDLE_WORLD_OFFERED_CAPABILITIES, selection)
            .ok_or("selection")?;
    owner.admit_progression_session(&mut admitted).await;
    owner.admit_quest_session(&admitted).await;
    deps.initialize(&owner).await;

    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    for (command_id, transition) in [
        (1, super::quest_runtime_fixture::TRANSITION),
        (
            2,
            "oteryn:quest-transition/quest/u8_6/afathers_burden/status/npc_2",
        ),
    ] {
        let outcome = harness
            .root
            .commit_character_quest_transition(
                &authority,
                &harness.node,
                owner
                    .current_quest_fence(session)
                    .await
                    .map_err(debug)?
                    .ok_or("quest fence")?,
                QuestTransitionRequest {
                    transition_key: transition.into(),
                    cause: QuestCause::Command(crate::foundation::CommandRef::new(
                        session,
                        crate::foundation::CommandId::new(command_id).map_err(debug)?,
                    )),
                },
                Arc::clone(&catalogue),
            )
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
    }
    owner.refresh_quest_session(session, false).await;

    let (command_type, payload) = if mission_tracking {
        (
            COMMAND_TYPE_QUEST_MISSION_TRACK,
            encode_quest_mission_track(&[TrackedMission {
                quest_index,
                mission_index,
            }]),
        )
    } else {
        (
            COMMAND_TYPE_QUEST_LOG_QUERY,
            encode_quest_log_query(&QuestLogQuery::Track {
                quest_indexes: vec![quest_index],
            }),
        )
    };
    let payload = payload.map_err(debug)?;
    let command = encode_client_command(
        1,
        &ClientCommandValue {
            command_id: admitted.continuity.next_command_id,
            command_type,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    let mut input = u32::try_from(command.len())?.to_be_bytes().to_vec();
    input.extend(command);
    let (_, output) =
        super::weather_wire_fixtures::fixture_access::serve_input(&owner, admitted, &input).await?;
    let before = tracked(&output, MessageType::StateDelta)?;
    assert_eq!(before.len(), 1, "TRACK matched a visible journal entry");
    assert_eq!(before[0].quest_index, quest_index);
    if mission_tracking {
        let indexes: Vec<_> = before[0]
            .missions
            .iter()
            .map(|mission| mission.mission_index)
            .collect();
        assert_eq!(indexes, [mission_index]);
    }

    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(120))
        .ok_or("clock")?;
    assert_eq!(
        owner.logout_session(admitted).await,
        LogoutReply::Result(LogoutResult::Accepted)
    );
    let next = super::full_world_support::relogin(&harness).await?;
    assert_ne!(next, session);
    let current = store.current_session(next).await?;
    let fence = owner
        .current_quest_fence(next)
        .await
        .map_err(debug)?
        .ok_or("successor quest fence")?;
    let position = owner.read_admission_position(fence).await.map_err(debug)?;
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(next).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        owner
            .initialize_admitted_position(&mut runtime, actor, next, &position)
            .await
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(actor, next, current.current_character_lease())
            .map_err(debug)?;
        actor
    };
    assert!(owner.track_restored(next, actor, &position, fence));
    admitted = AdmittedSession {
        game_session_id: next,
        runtime_actor: Some(actor),
        item_fence: crate::gameplay_transport::item_fence_of(current),
        continuity: SessionContinuity {
            selected_capabilities: admitted.continuity.selected_capabilities,
            ..SessionContinuity::FRESH
        },
        ..admitted
    };
    owner.admit_progression_session(&mut admitted).await;
    owner.admit_quest_session(&admitted).await;
    deps.initialize_player(&owner, actor, next).await;
    let (_, output) =
        super::weather_wire_fixtures::fixture_access::serve_input(&owner, admitted, &[]).await?;
    assert_eq!(
        tracked(&output, MessageType::SnapshotChunk)?,
        before,
        "the fresh GameSession snapshot restores the character's exact tracker selection"
    );
    drop(owner);
    drop(authority);
    drop(seal);
    harness.cleanup().await
}

fn tracked(
    mut output: &[u8],
    message: MessageType,
) -> TestResult<Vec<oteryn_protocol_oteryn::quest_log::QuestLine>> {
    // Framing is a fixed four-byte big-endian length.
    while !output.is_empty() {
        let (size, rest) = output.split_first_chunk::<4>().ok_or("frame header")?;
        let size = usize::try_from(u32::from_be_bytes(*size))?;
        let (frame, rest) = rest.split_at_checked(size).ok_or("frame body")?;
        output = rest;
        let envelope = decode_wire_envelope(frame).map_err(debug)?;
        if envelope.message_type() != message {
            continue;
        }
        let payload = if message == MessageType::SnapshotChunk {
            let (_, domains) = decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
            let Some(domain) = domains
                .into_iter()
                .find(|domain| domain.domain_id == STATE_DOMAIN_QUEST_LOG)
            else {
                continue;
            };
            domain.payload
        } else {
            let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
            if delta.domain_id != STATE_DOMAIN_QUEST_LOG {
                continue;
            }
            delta.payload
        };
        return Ok(decode_quest_log(payload).map_err(debug)?.tracked);
    }
    Err("quest log domain missing".into())
}
