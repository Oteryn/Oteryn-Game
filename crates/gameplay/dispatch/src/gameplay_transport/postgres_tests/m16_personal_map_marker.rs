//! M16: Sterling confirmation -> fenced receipt -> domain 16 delta and reconnect snapshot.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::gameplay_transport::quest_log::QuestLogLines;
use oteryn_protocol_oteryn::quest_log::{QuestLog, QuestMapMark, decode_quest_log};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const PREFIX: &str = "oteryn:quest-progress/crystalserver/quest/u15_24/targuna/long_lost_treasure/";
const QUEST: &str = "oteryn:quest.targuna_quest";

#[test]
fn m16_sterling_commits_personal_marker_and_delivers_delta_and_reconnect_snapshot() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("m16-marker", || {
        super::quest_world_callbacks::scenario(
            "m16-marker",
            oteryn_gameplay_quests::quest::world_callbacks::Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    owner.quest_log_content = Some(
        QuestLogLines::new(
            oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&lowered)?.content,
        )
        .ok_or("log bounds")?,
    );
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[3, 16])
            .ok_or("capabilities")?;
    assert!(admitted.continuity.selected_capabilities.contains(16));
    let [npc] = super::classic_quests::install(owner, admitted, ["oteryn:npc.sterling"]).await?;
    // No predecessor progress: confirmation cannot create a personal marker.
    let (snapshot, _) =
        conversation(owner, admitted, npc, 1, &["hi", "mission", "yes"], false).await?;
    assert!(snapshot.map_marks.is_empty());
    assert_eq!(counts(h).await?, (0, 0));

    // Establish a real revision/receipt chain before installing later predecessor facts.
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "predecessor fence")?
        .ok_or("predecessor fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let predecessor = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            crate::durability::quest_state::QuestTransitionRequest {
                transition_key: super::quest_runtime_fixture::TRANSITION.into(),
                cause: crate::durability::quest_state::QuestCause::Command(
                    crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(1000).map_err(debug)?,
                    ),
                ),
            },
            std::sync::Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?),
        )
        .await
        .map_err(debug)?;
    assert!(matches!(
        predecessor,
        crate::durability::quest_state::QuestTransitionOutcome::Committed(_)
    ));
    drop(slot);

    // Fixture privileges establish only predecessor progress. The confirmation below
    // uses the production dispatcher, runtime role, callback and receipt guard chain.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (track, value) in [("mission", 3_i64), ("morla_done", 1)] {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,$4,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id,track_key) DO UPDATE SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision")
            .bind(id(41).as_slice()).bind(format!("{PREFIX}{track}")).bind(QUEST).bind(value)
            .execute(&mut *tx).await?;
    }
    tx.commit().await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let expected = vec![QuestMapMark {
        x: 31948,
        y: 31924,
        floor: -7,
        text: "Treasure".into(),
    }];
    let (before, delta) =
        conversation(owner, admitted, npc, 4, &["hi", "mission", "yes"], true).await?;
    assert!(before.map_marks.is_empty());
    assert_eq!(delta.ok_or("marker delta")?.map_marks, expected);
    assert_eq!(counts(h).await?, (1, 1));
    for (track, value) in [("mission", 4), ("treasure_marked", 1)] {
        let stored: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
            .bind(id(41).as_slice()).bind(format!("{PREFIX}{track}")).fetch_one(&h.pool).await?;
        assert_eq!(stored, value);
    }
    let coupled: bool = sqlx::query_scalar("SELECT m.created_xact_id=r.created_xact_id AND r.created_xact_id=s.created_xact_id AND (SELECT count(*) FROM game_character_quest_tracks t WHERE t.character_id=m.character_id AND t.track_key=ANY($1) AND t.committed_character_revision=m.committed_character_revision)=2 FROM game_character_quest_map_marks m JOIN game_character_quest_receipts r ON (r.character_id,r.committed_character_revision)=(m.character_id,m.committed_character_revision) JOIN game_quest_reward_sources s ON (s.character_id,s.cause_id,s.cause_ordinal,s.transition_key)=(r.character_id,r.cause_id,r.cause_ordinal,r.transition_key)")
        .bind(vec![format!("{PREFIX}mission"), format!("{PREFIX}treasure_marked")])
        .fetch_one(&h.pool).await?;
    assert!(
        coupled,
        "marker, progress, native source and receipt commit together"
    );
    // A new connection restores the marker without a query; replaying the exact
    // confirmation and then repeating it with a fresh command creates no second fact.
    let (restored, _) = conversation(owner, admitted, npc, 6, &["yes"], false).await?;
    assert_eq!(restored.map_marks, expected);
    let (repeated, _) =
        conversation(owner, admitted, npc, 7, &["hi", "mission", "yes"], false).await?;
    assert_eq!(repeated.map_marks, expected);
    assert_eq!(counts(h).await?, (1, 1));
    eprintln!("M16_STERLING_DISPATCH_PG_EXECUTED marker_delta_reconnect_replay");
    Ok(())
}

async fn counts(h: &Harness) -> TestResult<(i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_map_marks),(SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-reward/targuna/sterling-mark-treasure')")
        .fetch_one(&h.pool).await?)
}

async fn conversation(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    npc: EntityRef,
    first: u64,
    words: &[&str],
    wait_marker: bool,
) -> TestResult<(QuestLog, Option<QuestLog>)> {
    use crate::gameplay_transport::connection::{ConnectionEnd, IDLE_LIVENESS, serve_admitted};
    use crate::gameplay_transport::tcp_tls::{read_frame, write_frame};
    use oteryn_protocol_oteryn::npc_service::{NpcTalkIntent, encode_npc_talk_intent};
    use oteryn_protocol_oteryn::{
        ClientCommandValue, CommandStatus, MessageType, decode_command_result,
        decode_snapshot_chunk, decode_state_delta, decode_wire_envelope, encode_client_command,
    };
    use tokio::io::AsyncWriteExt;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let before = *admitted;
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let serving = async {
        let end = serve_admitted(&mut server, before, owner, IDLE_LIVENESS).await;
        drop(server);
        end
    };
    let driving = async {
        let mut snapshot = None;
        let mut marker_delta = None;
        loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::SnapshotChunk {
                for domain in decode_snapshot_chunk(envelope.payload()).map_err(debug)?.1 {
                    if domain.domain_id == 16 {
                        snapshot = Some(decode_quest_log(domain.payload).map_err(debug)?);
                    }
                }
            }
            if envelope.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        for (offset, text) in words.iter().enumerate() {
            let command_id = first + u64::try_from(offset)?;
            let payload = encode_npc_talk_intent(&NpcTalkIntent {
                npc_actor: npc,
                text: (*text).into(),
            })
            .map_err(debug)?;
            write_frame(
                &mut client,
                &encode_client_command(
                    before.continuity.connection_generation,
                    &ClientCommandValue {
                        command_id,
                        command_type: 7,
                        payload: &payload,
                    },
                )
                .map_err(debug)?,
            )
            .await?;
            loop {
                let frame = read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame).map_err(debug)?;
                if envelope.message_type() == MessageType::CommandResult {
                    let result = decode_command_result(envelope.payload()).map_err(debug)?;
                    assert_eq!(result.command_id, command_id);
                    assert_eq!(
                        result.status,
                        if command_id < before.continuity.next_command_id {
                            CommandStatus::DuplicateReplay
                        } else {
                            CommandStatus::Accepted
                        },
                        "{text}"
                    );
                    break;
                }
            }
        }
        while wait_marker && marker_delta.is_none() {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame).map_err(debug)?;
            if envelope.message_type() == MessageType::StateDelta {
                let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
                if delta.domain_id == 16 {
                    let log = decode_quest_log(delta.payload).map_err(debug)?;
                    if !log.map_marks.is_empty() {
                        marker_delta = Some(log);
                    }
                }
            }
        }
        client.shutdown().await?;
        Ok((snapshot.ok_or("quest snapshot")?, marker_delta))
    };
    let (ended, result): (_, TestResult<_>) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::fixture::join_two(serving, driving),
    )
    .await?;
    if let ConnectionEnd::AdmittedThenDisconnected(current)
    | ConnectionEnd::AdmittedThenClosed(current, _) = ended
    {
        *admitted = current;
    }
    result
}
