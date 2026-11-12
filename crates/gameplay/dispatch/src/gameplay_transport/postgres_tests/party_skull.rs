//! PARTY-SKULL-1 through the actual admitted command router and visibility cadence.
use crate::bestiary_postgres_harness::{Harness, TestResult, debug};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::SelectedCapabilities,
    connection::{
        AdmittedSession, ConnectionEnd, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
    },
    tcp_tls::write_frame,
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

const ICON_CAPABILITIES: &[u32] = &[6, 42, 134];

pub(super) async fn icon_snapshot_revision(
    client: &mut tokio::io::DuplexStream,
    generation: u64,
) -> TestResult<u64> {
    let mut revision = None;
    loop {
        let frame = super::party_membership::read_live_frame(client, generation).await?;
        let env = decode_wire_envelope(&frame).map_err(debug)?;
        if env.connection_generation() != generation {
            return Err("party snapshot has wrong connection generation".into());
        }
        if env.message_type() == MessageType::SnapshotChunk {
            // This fixed-room fixture's three actors fit its existing single chunk.
            let (_, domains) =
                oteryn_protocol_oteryn::decode_snapshot_chunk(env.payload()).map_err(debug)?;
            for domain in domains {
                if domain.domain_id == 1 {
                    oteryn_protocol_oteryn::world_spatial_entities::decode_world_spatial_snapshot_view(
                        ICON_CAPABILITIES, domain.snapshot_type, domain.payload,
                    ).map_err(debug)?;
                    revision = Some(domain.revision);
                }
            }
        }
        if env.message_type() == MessageType::SnapshotCommit {
            return revision.ok_or_else(|| "party snapshot lacks visibility baseline".into());
        }
    }
}

fn assert_icons(
    icons: &[PartyIcon],
    targets: &[EntityRef],
    shields: &[u8],
    skulls: &[u8],
) -> TestResult {
    assert_eq!(icons.len(), targets.len());
    assert_eq!(targets.len(), shields.len());
    assert_eq!(targets.len(), skulls.len());
    for ((target, shield), skull) in targets.iter().zip(shields).zip(skulls) {
        let icon = icons
            .iter()
            .find(|icon| icon.entity == *target)
            .ok_or("visible icon")?;
        assert_eq!(
            (icon.shield, icon.skull),
            (*shield, *skull),
            "party icon values"
        );
    }
    Ok(())
}

pub(super) async fn read_icons(
    client: &mut tokio::io::DuplexStream,
    generation: u64,
    command_id: u64,
    baseline: u64,
    targets: &[EntityRef],
    shields: &[u8],
    skulls: &[u8],
) -> TestResult {
    // Retain only one fully checked observation of each kind, never a frame history.
    let (mut received, mut pushed) = (false, false);
    while !received || !pushed {
        let frame = super::party_membership::read_live_frame(client, generation).await?;
        let env = decode_wire_envelope(&frame).map_err(debug)?;
        if env.connection_generation() != generation {
            return Err("party observation has wrong connection generation".into());
        }
        match env.message_type() {
            MessageType::CommandResult => {
                let result = decode_command_result(env.payload()).map_err(debug)?;
                if result.command_id != command_id {
                    return Err("party observation has wrong command id".into());
                }
                assert_eq!(result.status, CommandStatus::Accepted);
                let icons = decode_party_icons(result.payload).map_err(debug)?;
                assert_icons(&icons, targets, shields, skulls)?;
                received = true;
            }
            MessageType::StateDelta => {
                let delta =
                    oteryn_protocol_oteryn::decode_state_delta(env.payload()).map_err(debug)?;
                if delta.domain_id != 1 || delta.delta_type != 2 {
                    continue;
                }
                if delta.base_revision < baseline || delta.new_revision <= baseline {
                    return Err("party icon delta predates the snapshot baseline".into());
                }
                use oteryn_protocol_oteryn::world_spatial_entities as spatial;
                // Retain the selected decoder and old-client counterfactual checks.
                spatial::decode_world_spatial_delta_view(ICON_CAPABILITIES, 2, delta.payload)
                    .map_err(debug)?;
                let (_, _, icons) = spatial::decode_world_spatial_entities_delta_with_icons(
                    delta.payload,
                    spatial::EntityFields::selected(ICON_CAPABILITIES),
                )
                .map_err(debug)?;
                let Some(icons) = icons else {
                    continue;
                };
                assert!(
                    spatial::decode_world_spatial_delta_view(&[6, 42], 2, delta.payload).is_err(),
                    "counterfactual: icons cannot reach an old client"
                );
                assert_icons(&icons, targets, shields, skulls)?;
                pushed = true;
            }
            MessageType::ProtocolError => {
                return Err("party observation received a protocol error".into());
            }
            _ => {}
        }
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
    let serving = async move {
        let ended = serve_admitted(&mut server, current, owner, IDLE_LIVENESS).await;
        drop(server);
        ended
    };
    let driving = async move {
        let baseline =
            icon_snapshot_revision(&mut client, current.continuity.connection_generation).await?;
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
        read_icons(
            &mut client,
            current.continuity.connection_generation,
            current.continuity.next_command_id,
            baseline,
            targets,
            shields,
            skulls,
        )
        .await?;
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
    super::party_effects::check(owner, admitted, targets, shields).await?;
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
    let serving = async move {
        let ended = serve_admitted(&mut server, current, owner, IDLE_LIVENESS).await;
        drop(server);
        ended
    };
    let driving = async move {
        icon_snapshot_revision(&mut client, current.continuity.connection_generation).await?;
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
            let frame = super::party_membership::read_live_frame(
                &mut client,
                current.continuity.connection_generation,
            )
            .await?;
            let env = decode_wire_envelope(&frame).map_err(debug)?;
            if env.message_type() == MessageType::CommandResult {
                let result = decode_command_result(env.payload()).map_err(debug)?;
                assert_eq!(
                    env.connection_generation(),
                    current.continuity.connection_generation
                );
                assert_eq!(result.command_id, current.continuity.next_command_id);
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
