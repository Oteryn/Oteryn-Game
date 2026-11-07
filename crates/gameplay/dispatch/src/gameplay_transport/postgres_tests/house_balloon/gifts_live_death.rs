//! Actual USE, observed HP0, and the existing HouseExit on one live connection.
use super::wire::HouseWireCapture;
use super::*;
pub(super) async fn qualify(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    capture: &std::sync::Arc<std::sync::Mutex<HouseWireCapture>>,
) -> TestResult {
    let (status, payload, saw_zero) = {
        let data = capture.lock().map_err(|_| "live death capture")?;
        (
            data.exit_status,
            data.exit_ack.clone(),
            data.vitals
                .iter()
                .filter_map(|v| oteryn_protocol_oteryn::actor_spell::decode_actor_vitals(v).ok())
                .any(|v| v.health == 0),
        )
    };
    assert!(
        saw_zero,
        "source-framed lethal USE must publish real native HP0 before live exit"
    );
    assert_eq!(
        status,
        Some(CommandStatus::Accepted),
        "genuine live HP0 House actor must settle ordinary death through existing HouseExit102"
    );
    let payload = payload.ok_or("actual death transfer ACK")?;
    let receipt = owner
        .root
        .house_transfer_recovery(admitted.game_session_id)
        .await?
        .ok_or("actual committed death transfer receipt")?;
    let ack = receipt.ack;
    assert_eq!(
        payload,
        oteryn_protocol_oteryn::house_transfer::encode_house_transfer_ack(&ack).map_err(debug)?,
        "live ACK is exactly the mature committed transfer receipt"
    );
    assert!(
        ack.house.is_none(),
        "death crosses actual House→Channel barrier"
    );
    assert_eq!(ack.session, *admitted.game_session_id.as_bytes());
    let fence = admitted.item_fence.ok_or("Temple actual fence")?;
    assert!(matches!(
        fence.runtime_scope,
        crate::foundation::RuntimeScopeRefV1::Channel { .. }
    ));
    let row:(Vec<u8>,Vec<u8>,i64)=sqlx::query_as("SELECT uuid_send(death_occurrence_id),respawn_position,(SELECT count(*) FROM game_character_death_receipts WHERE character_id=d.character_id) FROM game_character_death_receipts d WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(row.2, 1, "one real lethal gift prices one ordinary death");
    let encoded: Vec<u8> = ack.position[..2]
        .iter()
        .flat_map(|v| v.to_be_bytes())
        .chain(i16::try_from(ack.position[2])?.to_be_bytes())
        .collect();
    assert_eq!(
        encoded, row.1,
        "ACK carries exactly the committed current-town temple"
    );
    let actor = admitted.runtime_actor.ok_or("Temple actor")?;
    let runtime = owner.runtime.lock().await;
    assert!(
        runtime
            .house_actor_state(actor, admitted.game_session_id)
            .is_err()
    );
    let states = owner.states.spell_states.lock().await;
    let state = states
        .actors
        .iter()
        .find(|(a, s, _)| *a == actor && *s == admitted.game_session_id)
        .ok_or("actual restored native pools")?;
    assert!(state.2.vitals().health > 0);
    assert!(
        states
            .player_death(&runtime, actor, admitted.game_session_id)
            .is_none()
    );
    eprintln!(
        "SRV7_SANTA_HOUSE_LIVE_LETHAL_EXIT_EXECUTED receipt={} temple={:?}",
        row.0.iter().map(|b| format!("{b:02x}")).collect::<String>(),
        ack.position
    );
    Ok(())
}
