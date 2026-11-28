use super::*;
use oteryn_protocol_oteryn::presentation::*;

#[test]
fn cooldown_delta_type_and_replacement_semantics_follow_negotiation() -> Result<(), BoxError> {
    block_on(async {
        let value = ActorCooldowns {
            server_now_ms: 0,
            rtt_ms: None,
            entries: vec![],
        };
        let payload = encode_actor_cooldowns(&value).map_err(|error| format!("{error:?}"))?;
        const WORLD: u32 =
            oteryn_protocol_oteryn::world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES;
        const LEGACY: &[u32] = &[WORLD, CAPABILITY_PRESENTATION_V1];
        const SPARSE: &[u32] = &[
            WORLD,
            CAPABILITY_PRESENTATION_V1,
            CAPABILITY_ACTOR_COOLDOWN_DELTAS_V1,
        ];
        for (capabilities, sparse) in [(LEGACY, false), (SPARSE, true)] {
            let raw = vec![
                (
                    STATE_DOMAIN_WORLD_PRESENTATION,
                    0,
                    SNAPSHOT_TYPE_WORLD_PRESENTATION_V1,
                    encode_presentation_snapshot(),
                ),
                (
                    STATE_DOMAIN_ACTOR_COOLDOWNS,
                    0,
                    SNAPSHOT_TYPE_ACTOR_COOLDOWNS_V1,
                    payload.clone(),
                ),
            ];
            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let admission = admission_with(capabilities)?;
            let peer = tokio::spawn(async move {
                join_peer_with(
                    &mut server,
                    capabilities,
                    capabilities,
                    &[],
                    entity_snapshot(vec![]),
                    &raw,
                )
                .await
            });
            let mut session = Session::admit(client, admission).await?;
            peer.await??;

            let (expected, unnegotiated) = if sparse {
                (
                    DELTA_TYPE_ACTOR_COOLDOWNS_SPARSE_V1,
                    DELTA_TYPE_ACTOR_COOLDOWNS_V1,
                )
            } else {
                (
                    DELTA_TYPE_ACTOR_COOLDOWNS_V1,
                    DELTA_TYPE_ACTOR_COOLDOWNS_SPARSE_V1,
                )
            };
            let frame = encode_state_delta(
                1,
                41,
                STATE_DOMAIN_ACTOR_COOLDOWNS,
                0,
                1,
                unnegotiated,
                &payload,
            )?;
            assert!(
                session
                    .apply_state_delta(41, decode_wire_envelope(&frame)?.payload())
                    .is_err()
            );
            let frame = encode_state_delta(
                1,
                41,
                STATE_DOMAIN_ACTOR_COOLDOWNS,
                0,
                1,
                expected,
                &payload,
            )?;
            let event = session.apply_state_delta(41, decode_wire_envelope(&frame)?.payload())?;
            assert!(matches!(event, SessionEvent::ActorCooldowns(update)
                if update.value.snapshot != sparse && update.value.value == value));
        }
        Ok(())
    })?
}
