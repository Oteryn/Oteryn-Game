use super::party_membership_fixtures::read_live_frame;
use super::party_skull_fixtures::icon_snapshot_revision;
use crate::bestiary_postgres_harness::{Harness, TestResult, debug};
use crate::foundation::{MessageType, decode_wire_envelope};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    capabilities::SelectedCapabilities,
    connection::{
        AdmittedSession, ConnectionEnd, FreshAdmissionAuthority, LivenessPolicy, serve_admitted,
    },
    tcp_tls::{read_frame, write_frame},
};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
    party::*,
    party_skull::CAPABILITY_PARTY_SKULL_V1,
    world_spatial_entities::{
        DeltaSidecar, EntityFields, EntityRef, decode_world_spatial_entities_delta_with_sidecar,
    },
};
use std::time::Duration;
use tokio::io::{AsyncWriteExt, DuplexStream};

const CAPABILITIES: [u32; 3] = [6, CAPABILITY_PARTY_V1, CAPABILITY_PARTY_SKULL_V1];

pub(super) async fn check(
    harness: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut [AdmittedSession],
    targets: &[EntityRef],
    successor: [u8; 16],
) -> TestResult {
    let [old, new, other] = admitted else {
        return Err("three live party members required".into());
    };
    assert_eq!(targets.len(), 3);
    let selected = SelectedCapabilities::select(owner.offered_capabilities(), &CAPABILITIES)
        .filter(|selected| selected.contains(CAPABILITY_PARTY_SKULL_V1))
        .ok_or("party shield offer")?;
    let observing = |mut session: AdmittedSession| {
        session.continuity.selected_capabilities = selected;
        session
    };
    let [old_session, new_session, other_session] =
        [observing(*old), observing(*new), observing(*other)];
    let (old_server, mut old_client) = tokio::io::duplex(64 << 10);
    let (new_server, mut new_client) = tokio::io::duplex(64 << 10);
    let (other_server, mut other_client) = tokio::io::duplex(64 << 10);
    let policy = LivenessPolicy {
        interval: Duration::from_millis(750),
        missed_limit: 10,
    };
    let generation = |session: AdmittedSession| session.continuity.connection_generation;
    let serving = |mut stream, session| async move {
        serve_admitted(&mut stream, session, owner, policy).await
    };
    let driving = async move {
        tokio::try_join!(
            icon_snapshot_revision(&mut old_client, generation(old_session)),
            icon_snapshot_revision(&mut new_client, generation(new_session)),
            icon_snapshot_revision(&mut other_client, generation(other_session)),
        )?;
        let initial = [4, 3, 3];
        tokio::try_join!(
            wait_shields(&mut old_client, generation(old_session), targets, initial),
            wait_shields(&mut new_client, generation(new_session), targets, initial),
            wait_shields(
                &mut other_client,
                generation(other_session),
                targets,
                initial
            ),
        )?;
        // A successful initial background read clears any prior foreground window.
        // Hold the new foreground demand through the assertion so a periodic
        // visibility read cannot be the source of the transferred shields.
        tokio::time::timeout(Duration::from_secs(4), async {
            let _foreground = harness.root.foreground_demand()?;
            let command_id = old_session.continuity.next_command_id;
            let old_generation = generation(old_session);
            let roster = send_party(&mut old_client, old_generation, command_id, None).await?;
            assert_eq!(roster.members.len(), 3);
            let handle = crate::gameplay_transport::party_command::party_member_handle(old_session, successor)
                .ok_or("successor handle")?;
            assert!(roster.members.iter().any(|member| member.handle == handle && !member.leader));
            let result = send_party(
                &mut old_client,
                old_generation,
                command_id + 1,
                Some(PartyIntent::PassLeadership(handle)),
            ).await?;
            assert_eq!(result.disposition, PartyDisposition::Ok);
            let transferred = [3, 4, 3];
            tokio::try_join!(
                wait_shields(&mut old_client, old_generation, targets, transferred),
                wait_shields(&mut new_client, generation(new_session), targets, transferred),
                wait_shields(&mut other_client, generation(other_session), targets, transferred),
            )?;
            Ok::<(), Box<dyn std::error::Error>>(())
        })
        .await
        .map_err(|_| "all three members must receive the committed shield update before a background turn")??;

        // Read a probe before taking the lane, then keep it held across another
        // visibility tick and probe. A stalled read prevents the second probe.
        read_probe(&mut other_client, generation(other_session)).await?;
        let lane = owner
            .spell_lane
            .acquire()
            .await
            .map_err(|_| "fixture spell lane is parked")?;
        tokio::time::timeout(Duration::from_secs(3), async {
            read_probe(&mut other_client, generation(other_session)).await?;
            read_probe(&mut other_client, generation(other_session)).await
        })
        .await
        .map_err(|_| "liveness probe must arrive while the spell lane is held")??;
        drop(lane);
        old_client.shutdown().await?;
        new_client.shutdown().await?;
        other_client.shutdown().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (old_end, new_end, other_end, result) =
        tokio::time::timeout(Duration::from_secs(30), async {
            tokio::join!(
                Box::pin(serving(old_server, old_session)),
                Box::pin(serving(new_server, new_session)),
                Box::pin(serving(other_server, other_session)),
                driving,
            )
        })
        .await?;
    result?;
    for (entry, end) in [(old, old_end), (new, new_end), (other, other_end)] {
        let ConnectionEnd::AdmittedThenDisconnected(mut current) = end else {
            return Err(format!("live party peer ended: {end:?}").into());
        };
        current.continuity.selected_capabilities = entry.continuity.selected_capabilities;
        *entry = current;
    }
    Ok(())
}

async fn send_party(
    client: &mut DuplexStream,
    generation: u64,
    command_id: u64,
    intent: Option<PartyIntent>,
) -> TestResult<PartyResult> {
    let (command_type, payload) = match intent {
        Some(intent) => (
            COMMAND_TYPE_PARTY_INTENT,
            encode_party_intent(intent).map_err(debug)?,
        ),
        None => (COMMAND_TYPE_PARTY_QUERY, Vec::new()),
    };
    let frame = encode_client_command(
        generation,
        &ClientCommandValue {
            command_id,
            command_type,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    write_frame(client, &frame).await?;
    loop {
        let frame = read_live_frame(client, generation).await?;
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(result.command_id, command_id);
            assert_eq!(result.status, CommandStatus::Accepted);
            return decode_party_result(result.payload).map_err(|error| debug(error).into());
        }
    }
}

async fn wait_shields(
    client: &mut DuplexStream,
    generation: u64,
    targets: &[EntityRef],
    expected: [u8; 3],
) -> TestResult {
    let fields = EntityFields::selected(&CAPABILITIES);
    loop {
        let frame = read_live_frame(client, generation).await?;
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() != MessageType::StateDelta {
            continue;
        }
        let delta =
            oteryn_protocol_oteryn::decode_state_delta(envelope.payload()).map_err(debug)?;
        if delta.domain_id
            != oteryn_protocol_oteryn::world_spatial::STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY
        {
            continue;
        }
        let (_, _, sidecar) =
            decode_world_spatial_entities_delta_with_sidecar(delta.payload, fields)
                .map_err(debug)?;
        if let Some(DeltaSidecar::PartyIcons(icons)) = sidecar
            && targets.iter().zip(expected).all(|(target, shield)| {
                icons
                    .iter()
                    .any(|icon| icon.entity == *target && icon.shield == shield)
            })
        {
            return Ok(());
        }
    }
}

async fn read_probe(client: &mut DuplexStream, generation: u64) -> TestResult {
    loop {
        let frame = read_frame(client).await?;
        let envelope = decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == MessageType::LivenessProbe {
            let probe =
                oteryn_protocol_oteryn::decode_liveness_probe(envelope.payload()).map_err(debug)?;
            let ack =
                oteryn_protocol_oteryn::encode_liveness_ack(generation, probe, 0).map_err(debug)?;
            write_frame(client, &ack).await?;
            return Ok(());
        }
    }
}
