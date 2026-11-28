//! Drive the real Grizzly, STEP, and visibility phases in a fixed order on one connection.
use super::*;
use crate::gameplay_transport::tcp_tls::read_frame;
use oteryn_protocol_oteryn::npc_service::{
    COMMAND_TYPE_NPC_TALK_INTENT, NpcIntentDisposition, NpcTalkIntent, decode_npc_intent_result,
    encode_npc_talk_intent,
};
use oteryn_protocol_oteryn::world_spatial::{decode_step_result, encode_step_intent};

pub(in crate::gameplay_transport) async fn regression_grizzly_step_capacity(
    authority: &impl FreshAdmissionAuthority,
    mut admitted: AdmittedSession,
    npc: WireEntityRef,
) -> Result<(), Box<dyn std::error::Error>> {
    let actor = admitted.runtime_actor.ok_or("Grizzly test actor")?;
    let baseline = authority
        .observe(actor)
        .await
        .ok_or("Grizzly step baseline")?;
    let (mut stream, mut client) = tokio::io::duplex(1 << 22);
    let mut session = setup::prepare(
        &mut stream,
        &mut admitted,
        authority,
        IDLE_LIVENESS,
        actor,
        baseline,
        integrity::Integrity::default(),
    )
    .await
    .map_err(|_| "Grizzly step setup")?;
    let initial_vitals = session.admitted.continuity.vitals_revision;
    assert_ne!(initial_vitals, 0, "join must snapshot the pre-level vitals");

    while decode_wire_envelope(&read_frame(&mut client).await?)?.message_type()
        != MessageType::SnapshotCommit
    {}
    inbound::receive(&mut stream, authority, &mut session, poll::Next::Visibility)
        .await
        .map_err(|_| "initial visibility")?;
    let before = session.inventory_capacity.ok_or("initial capacity")?;
    let original_refresh = session
        .inventory_capacity_refresh_at
        .ok_or("initial capacity deadline")?;
    for text in [Some("hi"), Some("tasks"), Some("promotion"), None] {
        let (kind, payload) = if let Some(text) = text {
            (
                COMMAND_TYPE_NPC_TALK_INTENT,
                encode_npc_talk_intent(&NpcTalkIntent {
                    npc_actor: npc,
                    text: text.into(),
                })
                .map_err(|_| "Grizzly talk payload")?,
            )
        } else {
            assert_eq!(session.admitted.continuity.vitals_revision, initial_vitals);
            assert_eq!(
                session.inventory_capacity_refresh_at,
                Some(original_refresh),
                "Grizzly's task and promotion did not move any items"
            );
            (
                COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                encode_step_intent(StepDirection::West),
            )
        };
        let command_id = session.admitted.continuity.next_command_id;
        let frame = oteryn_protocol_oteryn::encode_client_command(
            session.generation,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: kind,
                payload: &payload,
            },
        )?;
        command::execute(
            &mut stream,
            authority,
            &mut session,
            inbound::Input {
                frame,
                arrived_while_buffered: false,
                buffered_original: false,
                received_at: tokio::time::Instant::now(),
            },
        )
        .await
        .map_err(|_| "Grizzly or STEP dispatch ended")?;
        loop {
            let frame = read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame)?;
            if envelope.message_type() == MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
                assert_eq!(result.command_id, command_id);
                assert_eq!(result.status, CommandStatus::Accepted);
                if text.is_some() {
                    assert_eq!(
                        decode_npc_intent_result(result.payload).map_err(|_| "Grizzly result")?,
                        NpcIntentDisposition::Ok
                    );
                } else {
                    assert_eq!(
                        decode_step_result(result.payload).map_err(|_| "STEP result")?,
                        StepDisposition::Moved
                    );
                }
                break;
            }
        }
    }
    assert!(
        session.admitted.continuity.vitals_revision > initial_vitals,
        "STEP published the Grizzly level-up revision"
    );
    assert!(
        tokio::time::Instant::now() < original_refresh,
        "the original five-second deadline is still in the future"
    );
    inbound::receive(&mut stream, authority, &mut session, poll::Next::Visibility)
        .await
        .map_err(|_| "visibility after STEP")?;
    let after = session.inventory_capacity.ok_or("capacity after STEP")?;
    assert!(
        after.capacity > before.capacity,
        "the next visibility tick must publish the higher capacity"
    );
    Ok(())
}
