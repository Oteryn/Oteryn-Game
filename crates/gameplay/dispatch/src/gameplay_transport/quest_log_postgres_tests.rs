//! Capability 16 through the real composed owner's connection and admitted session copy.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::quest_log::{QuestLogLines, QuestLogObservation};
use oteryn_protocol_oteryn::quest_log::{
    QuestLogQuery, QuestLogView, decode_quest_log, encode_quest_log_query,
};
use oteryn_protocol_oteryn::{MessageType, decode_snapshot_chunk, decode_wire_envelope};

#[test]
fn quest_log_real_session_source_serves_selected_domain_16_only() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario().map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "quest log e2e panicked")?
        .map_err(Into::into)
}

fn scenario() -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "quest_log_wire", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let mut owner = dependencies.owner(&harness, &authority);
        let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?;
        let loaded = oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&lowered)?;
        owner.quest_log_content = Some(QuestLogLines::new(loaded.content).ok_or("line bounds")?);
        owner.quest_catalogue = Some(std::sync::Arc::new(lowered.catalogue().clone()));
        dependencies.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(dependencies.actor)
            .map_err(debug)?;
        let mut admitted = dependencies.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        owner.admit_quest_session(&admitted).await;
        let session = admitted.game_session_id;
        let QuestLogObservation::Changed(source) = owner
            .observe_quest_log(dependencies.actor, session, None)
            .await
        else {
            return Err("session copy unavailable".into());
        };
        assert!(matches!(
            owner
                .observe_quest_log(dependencies.actor, session, Some(source.version))
                .await,
            QuestLogObservation::Unchanged
        ));

        // Real transitions reveal a quest and a mission, preserving their production hashes.
        for (command_id, key) in [
            (1, super::quest_runtime_fixture::TRANSITION),
            (
                2,
                "oteryn:quest-transition/quest/u8_6/afathers_burden/status/npc_2",
            ),
        ] {
            let fence = owner
                .current_quest_fence(session)
                .await
                .map_err(|_| "fence")?
                .ok_or("live")?;
            let outcome = owner
                .root
                .commit_character_quest_transition(
                    &authority,
                    &harness.node,
                    fence,
                    crate::durability::quest_state::QuestTransitionRequest {
                        transition_key: key.into(),
                        cause: crate::durability::quest_state::QuestCause::Command(
                            crate::foundation::CommandRef::new(
                                session,
                                crate::foundation::CommandId::new(command_id).map_err(debug)?,
                            ),
                        ),
                    },
                    std::sync::Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?),
                )
                .await
                .map_err(debug)?;
            assert!(matches!(
                outcome,
                crate::durability::quest_state::QuestTransitionOutcome::Committed(_)
            ));
        }
        owner.refresh_quest_session(session, false).await;
        assert!(matches!(
            owner
                .observe_quest_log(dependencies.actor, session, Some(source.version))
                .await,
            QuestLogObservation::Changed(_)
        ));

        for selected in [true, false] {
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(
                owner.offered_capabilities(),
                if selected { &[16] } else { &[] },
            )
            .ok_or("selection")?;
            let mut framed = Vec::new();
            for (command_id, query) in
                [QuestLogQuery::List, QuestLogQuery::Quest { quest_index: 0 }]
                    .into_iter()
                    .enumerate()
            {
                let query = encode_quest_log_query(&query).map_err(debug)?;
                let command = oteryn_protocol_oteryn::encode_client_command(
                    1,
                    &oteryn_protocol_oteryn::ClientCommandValue {
                        command_id: command_id as u64 + 1,
                        command_type: 22,
                        payload: &query,
                    },
                )
                .map_err(debug)?;
                framed.extend(u32::try_from(command.len())?.to_be_bytes());
                framed.extend(command);
            }
            let (_, output) =
                super::weather_wire::fixture_access::serve_input(&owner, admitted, &framed).await?;
            let mut reader = output.as_slice();
            let mut journals = Vec::new();
            let mut lists = Vec::new();
            while !reader.is_empty() {
                let frame = crate::gameplay_transport::tcp_tls::read_frame(&mut reader).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == MessageType::SnapshotChunk {
                    let (_, domains) = decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
                    for domain in domains {
                        if domain.domain_id == 16 {
                            journals.push(decode_quest_log(domain.payload).map_err(debug)?);
                        }
                    }
                } else if envelope.message_type() == MessageType::StateDelta {
                    let delta = oteryn_protocol_oteryn::decode_state_delta(envelope.payload())
                        .map_err(debug)?;
                    if delta.domain_id == 16 {
                        lists.push(decode_quest_log(delta.payload).map_err(debug)?);
                    }
                }
            }
            assert_eq!(journals.len(), usize::from(selected));
            if selected {
                assert!(journals[0].view.is_none());
                assert!(lists.iter().any(|log| matches!(&log.view, Some(QuestLogView::List(quests)) if quests.iter().any(|quest| quest.quest_index == 0))));
                assert!(lists.iter().any(|log| matches!(&log.view, Some(QuestLogView::Line(line)) if line.quest_index == 0 && line.missions.iter().any(|mission| mission.name == "The Birthday Presents" && mission.text.len() == 121))));
            } else {
                assert!(lists.is_empty());
            }
        }
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
