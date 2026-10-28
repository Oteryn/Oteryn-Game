//! PARTY-SKULL-1 through the actual admitted command router and visibility cadence.
use crate::bestiary_postgres_harness::{Harness, TestResult, debug};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::SelectedCapabilities,
    connection::{
        AdmittedSession, ConnectionEnd, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
    },
    tcp_tls::{read_frame, write_frame},
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
    party_skull::*, world_spatial_entities::EntityRef,
};
use tokio::io::AsyncWriteExt;

#[test]
fn party_skull_real_dispatch_postgres_end_to_end() -> TestResult {
    super::party_membership::run(true)
}

#[test]
fn party_skull_pvp_real_dispatch_attribution_counterfactual() -> TestResult {
    super::item_look::large_stack(|| super::pvp_live::scenario(14))
}

pub(super) async fn aggression(
    h: &Harness,
    targets: &[EntityRef],
    characters: &[[u8; 16]; 2],
    active: bool,
) -> TestResult {
    if active {
        sqlx::query("INSERT INTO game_character_pvp_state(character_id,world_id) SELECT character_id,world_id FROM game_character_roots WHERE character_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid) ON CONFLICT DO NOTHING")
            .bind(characters[0].as_slice()).bind(characters[1].as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_character_pvp_aggression(attacker,target,until_at,scope_generation,attacker_placement,target_placement) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,clock_timestamp()+interval '70 seconds',1,$3,$4)")
            .bind(characters[0].as_slice()).bind(characters[1].as_slice()).bind(targets[0].identity.as_slice()).bind(targets[1].identity.as_slice()).execute(&h.pool).await?;
    } else {
        sqlx::query("DELETE FROM game_character_pvp_aggression")
            .execute(&h.pool)
            .await?;
    }
    Ok(())
}

pub(super) async fn check(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    targets: &[EntityRef],
    shields: &[u8],
    skulls: &[u8],
) -> TestResult {
    let original = admitted.continuity.selected_capabilities;
    // Negotiate the production offer; prerequisites keep the extension closed to older clients.
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[6, 42, 134])
            .ok_or("selection")?;
    let current = *admitted;
    let (mut server, mut client) = tokio::io::duplex(32_768);
    let serving = async { serve_admitted(&mut server, current, owner, IDLE_LIVENESS).await };
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        let request = encode_client_command(
            current.continuity.connection_generation,
            &ClientCommandValue {
                command_id: current.continuity.next_command_id,
                command_type: COMMAND_TYPE_PARTY_SKULL_QUERY,
                payload: &[],
            },
        )
        .map_err(debug)?;
        write_frame(&mut client, &request).await?;
        let mut received = false;
        loop {
            let frame = read_frame(&mut client).await?;
            let env = decode_wire_envelope(&frame).map_err(debug)?;
            if env.message_type() == MessageType::CommandResult {
                let result = decode_command_result(env.payload()).map_err(debug)?;
                assert_eq!(result.status, CommandStatus::Accepted);
                let icons = decode_party_icons(result.payload).map_err(debug)?;
                assert_eq!(icons.len(), targets.len());
                for ((target, shield), skull) in targets.iter().zip(shields).zip(skulls) {
                    let icon = icons
                        .iter()
                        .find(|i| i.entity == *target)
                        .ok_or("visible icon")?;
                    assert_eq!((icon.shield, icon.skull), (*shield, *skull));
                }
                received = true;
            }
            if received && env.message_type() == MessageType::StateDelta {
                let delta =
                    oteryn_protocol_oteryn::decode_state_delta(env.payload()).map_err(debug)?;
                if delta.domain_id == 1 && delta.delta_type == 2 {
                    // The base visibility decoder validates optional field 6 only with capability 134.
                    oteryn_protocol_oteryn::world_spatial_entities::decode_world_spatial_delta_view(&[6,42,134],2,delta.payload).map_err(debug)?;
                    assert!(oteryn_protocol_oteryn::world_spatial_entities::decode_world_spatial_delta_view(&[6,42],2,delta.payload).is_err(),"counterfactual: icons cannot reach an old client");
                    break;
                }
            }
        }
        client.shutdown().await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, result) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::fixture::join_two(serving, driving),
    )
    .await?;
    result?;
    let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
        return Err(format!("icon connection: {end:?}").into());
    };
    admitted.continuity = ended.continuity;
    admitted.continuity.selected_capabilities = original;
    Ok(())
}

pub(super) async fn logout(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    owner.admit_progression_session(admitted).await;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("logout fence")?;
    assert!(owner.track_restored(
        admitted.game_session_id,
        admitted.runtime_actor.ok_or("logout actor")?,
        &crate::gameplay_transport::last_position::AdmissionPosition::FirstLogin,
        fence
    ));
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[6, 21, 42])
            .ok_or("logout selection")?;
    let current = *admitted;
    let (mut server, mut client) = tokio::io::duplex(32_768);
    let serving = async { serve_admitted(&mut server, current, owner, IDLE_LIVENESS).await };
    let driving = async {
        loop {
            let frame = read_frame(&mut client).await?;
            if decode_wire_envelope(&frame).map_err(debug)?.message_type()
                == MessageType::SnapshotCommit
            {
                break;
            }
        }
        let request = encode_client_command(
            current.continuity.connection_generation,
            &ClientCommandValue {
                command_id: current.continuity.next_command_id,
                command_type: oteryn_protocol_oteryn::logout::COMMAND_TYPE_LOGOUT_INTENT,
                payload: &[],
            },
        )
        .map_err(debug)?;
        write_frame(&mut client, &request).await?;
        loop {
            let frame = read_frame(&mut client).await?;
            let env = decode_wire_envelope(&frame).map_err(debug)?;
            if env.message_type() == MessageType::CommandResult {
                let result = decode_command_result(env.payload()).map_err(debug)?;
                assert_eq!(result.status, CommandStatus::Accepted);
                assert_eq!(
                    oteryn_protocol_oteryn::logout::decode_logout_result(result.payload)
                        .map_err(debug)?,
                    oteryn_protocol_oteryn::logout::LogoutResult::Accepted
                );
                break;
            }
        }
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, result) = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        super::fixture::join_two(serving, driving),
    )
    .await?;
    result?;
    assert!(
        matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)),
        "logout end: {end:?}"
    );
    Ok(())
}
