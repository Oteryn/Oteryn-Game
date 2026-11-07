//! Isolated native NPC/hazard-child qualification. Unlocked6 is a controlled
//! historical prerequisite, not proof of the separate real Primal boss producer.
use super::*;
pub(super) async fn exercise(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    below_prize: bool,
) -> TestResult {
    assert!(
        owner.admit_hazard(*admitted).await,
        "existing fenced hazard read/cache owner"
    );
    let mut setup = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *setup)
        .await?;
    sqlx::query("UPDATE game_character_hazard SET unlocked=6,selected=1 WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).execute(&mut *setup).await?;
    setup.commit().await?;
    assert!(owner.admit_hazard(*admitted).await);
    let desired = if below_prize { "level 3" } else { "6" };
    let frames = super::super::grizzly_npc::wire::talk_appearance(
        owner,
        admitted,
        npc,
        &[(1, "hi"), (2, "hazard"), (3, desired)],
    )
    .await?;
    let expected = if below_prize { 3_i16 } else { 6_i16 };
    assert_selected(h, owner, *admitted, expected).await?;
    let replies = conversation_lines(&frames)?;
    assert!(
        replies
            .iter()
            .any(|line| line.contains("current level is set to 1")
                && line.contains("maximum unlocked level is {6}")),
        "actual source ask shows the locked current and maximum values"
    );
    assert_eq!(
        advance_lines(&frames)?
            .iter()
            .filter(
                |line| line.as_str() == "Congratulations you received the Noxious Ripptor mount."
            )
            .count(),
        usize::from(!below_prize),
        "actual source EVENT_ADVANCE is delivered only for the first prize"
    );
    if below_prize {
        assert_rewards(h, 0).await?;
    } else {
        assert_rewards(h, 1).await?;
        // Reconnect genuinely greets again; inside this one connection the
        // source retains topic1, so direct next-number3 works without another ask.
        super::super::grizzly_npc::wire::talk(
            owner,
            admitted,
            npc,
            &[(4, "hi"), (5, "hazard"), (6, "6"), (7, "3")],
        )
        .await?;
        assert_selected(h, owner, *admitted, 3).await?;
        assert_rewards(h, 1).await?;
        let frames = super::super::grizzly_npc::wire::talk_appearance(
            owner,
            admitted,
            npc,
            &[(8, "hi"), (9, "hazard"), (10, "7")],
        )
        .await?;
        assert_selected(h, owner, *admitted, 3).await?;
        assert!(conversation_lines(&frames)?.iter().any(|line| line
            == "You can't set your hazard level higher than your maximum unlocked level."));
        // Invalid0 moves the source to topic2. A following number cannot select
        // another level until the player asks about hazard again.
        let frames = super::super::grizzly_npc::wire::talk_appearance(
            owner,
            admitted,
            npc,
            &[(11, "hi"), (12, "hazard"), (13, "0"), (14, "2")],
        )
        .await?;
        assert_selected(h, owner, *admitted, 3).await?;
        assert!(conversation_lines(&frames)?.iter().any(|line| line
            == "I'm sorry, I don't understand. What hazard level would you like to set?"));
        assert_rewards(h, 1).await?;
    }
    Ok(())
}
async fn assert_selected(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: crate::gameplay_transport::connection::AdmittedSession,
    expected: i16,
) -> TestResult {
    let actual:(i16,i16)=sqlx::query_as("SELECT unlocked,selected FROM game_character_hazard WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        actual,
        (6, expected),
        "native source changes only the selected unlocked level"
    );
    assert_eq!(
        owner.runtime.lock().await.hazard_level(
            admitted.runtime_actor.ok_or("actual hazard actor")?,
            admitted.game_session_id
        ),
        Some(u16::try_from(expected)?),
        "known source commit refreshes the real native actor hazard cache"
    );
    Ok(())
}
async fn assert_rewards(h: &Harness, count: i64) -> TestResult {
    let program =
        oteryn_gameplay_quests::quest::npc_cosmetics::source_program::embedded().map_err(debug)?;
    let mount = program
        .mounts
        .iter()
        .find(|mount| mount.id == 202)
        .ok_or("source Noxious Ripptor binding")?;
    assert_eq!(mount.look, 1577);
    let owned:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlocks WHERE account_id=encode($1,'hex')::uuid AND unlock_key=$2")
        .bind(id(40).as_slice()).bind("oteryn:appearance-grant/npc/gnomadness/noxious_ripptor/mount").fetch_one(&h.pool).await?;
    assert_eq!(owned, count, "source mount202 is granted only once");
    let bags:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i39546' AND i.quantity=1 AND e.character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        bags, count,
        "source ordinary Primal bag39546 is fresh native carried custody, once"
    );
    let claims:i64=sqlx::query_scalar("SELECT count(*) FROM game_npc_primal_prize_claims WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(claims, count, "source per-character KV is one atomic claim");
    Ok(())
}
fn conversation_lines(frames: &[Vec<u8>]) -> TestResult<Vec<String>> {
    let mut lines = Vec::new();
    for frame in frames {
        let envelope = crate::foundation::decode_wire_envelope(frame).map_err(debug)?;
        if envelope.message_type() != crate::foundation::MessageType::StateDelta {
            continue;
        }
        let delta =
            oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
        if delta.domain_id == oteryn_protocol_oteryn::npc_service::STATE_DOMAIN_NPC_CONVERSATION {
            lines.extend(
                oteryn_protocol_oteryn::npc_service::decode_npc_conversation(delta.payload)
                    .map_err(debug)?
                    .lines,
            );
        }
    }
    Ok(lines)
}

fn advance_lines(frames: &[Vec<u8>]) -> TestResult<Vec<String>> {
    use oteryn_protocol_oteryn::zones_nologout::{
        STATE_DOMAIN_SERVER_MESSAGES, ServerMessage, decode_server_message,
    };
    let mut notices = Vec::new();
    for frame in frames {
        let envelope = crate::foundation::decode_wire_envelope(frame).map_err(debug)?;
        if envelope.message_type() != crate::foundation::MessageType::StateDelta {
            continue;
        }
        let delta =
            oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
        if delta.domain_id == STATE_DOMAIN_SERVER_MESSAGES
            && let ServerMessage::Advance(text) =
                decode_server_message(delta.payload).map_err(debug)?
        {
            notices.push(text);
        }
    }
    Ok(notices)
}
