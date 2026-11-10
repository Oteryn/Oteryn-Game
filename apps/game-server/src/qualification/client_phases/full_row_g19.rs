//! G19 battle list metadata query (capability 17, command 740).
//! 1. Admit both provisioned characters and wait until each sees the other.
//! 2. Each queries the metadata of the other player entity plus an unknown entity: the server
//!    must fill in the other player's vocation and answer nothing for the unknown reference.
//! 3. Log both out.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::EntityRef;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::battle_metadata as wire;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(120), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "battle-metadata-vocation-for-visible-players".to_owned(),
        ),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G19.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn compact(value: &str) -> String {
    let joined = value.split_whitespace().collect::<Vec<_>>().join("-");
    joined.chars().take(400).collect()
}

fn gate(client: &DevClientSession, capability: u32) -> TestResult {
    if client.selected_capabilities().contains(&capability) {
        return Ok(());
    }
    Err(format!(
        "step=admit observed=capability-{capability}-not-selected needed=capability-{capability}-selected"
    )
    .into())
}

async fn send(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    gate(client, capability)?;
    let reply = client
        .session_mut()
        .gated_command(capability, command, payload)
        .await?;
    client.service_liveness(Duration::from_millis(50)).await?;
    Ok((reply.status, reply.payload))
}

async fn accepted(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    let (status, payload) = send(client, capability, command, payload).await?;
    if status != CommandStatus::Accepted {
        return Err(format!(
            "step=command-{command} observed=status:{status:?} needed=accepted-reply"
        )
        .into());
    }
    Ok(payload)
}

async fn finish(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

const CAPABILITY: u32 = 17;

fn own(client: &DevClientSession) -> TestResult<EntityRef> {
    let entities = client
        .world_entities()
        .ok_or("step=admit observed=no-entity-view")?;
    entities
        .iter()
        .find(|entry| entry.entity.identity == *entities.own_identity())
        .map(|entry| entry.entity)
        .ok_or_else(|| "step=admit observed=own-entity-absent".into())
}

fn seen(client: &DevClientSession, identity: [u8; 16]) -> Option<EntityRef> {
    client
        .world_entities()?
        .iter()
        .find(|entry| entry.entity.identity == identity)
        .map(|entry| entry.entity)
}

async fn pump(a: &mut DevClientSession, b: &mut DevClientSession) -> TestResult {
    a.service_liveness(Duration::from_millis(25)).await?;
    b.service_liveness(Duration::from_millis(25)).await?;
    a.take_events();
    b.take_events();
    Ok(())
}

/// Waits until `a` sees `b`; returns `b` as `a` sees it.
async fn see_each_other(
    a: &mut DevClientSession,
    b: &mut DevClientSession,
) -> TestResult<EntityRef> {
    let target = own(b)?.identity;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(12);
    loop {
        if let Some(found) = seen(a, target) {
            return Ok(found);
        }
        if tokio::time::Instant::now() >= deadline {
            return Err(
                "step=admit observed=other-player-not-in-view needed=two-players-in-view".into(),
            );
        }
        pump(a, b).await?;
    }
}

fn unknown_entity() -> EntityRef {
    EntityRef {
        identity: [0x5a; 16],
        generation: 1,
    }
}

fn judge(rows: &[wire::PlayerMetadata], player: EntityRef) -> TestResult {
    let row = rows
        .iter()
        .find(|row| row.entity == player)
        .ok_or("step=metadata observed=no-row-for-visible-player")?;
    if row.vocation.is_none() {
        return Err("step=metadata observed=vocation-missing-for-visible-player".into());
    }
    if rows.iter().any(|row| {
        row.entity == unknown_entity() && (row.vocation.is_some() || row.same_guild.is_some())
    }) {
        return Err("step=metadata observed=data-for-unknown-entity".into());
    }
    Ok(())
}

async fn ask(
    client: &mut DevClientSession,
    player: EntityRef,
) -> TestResult<Vec<wire::PlayerMetadata>> {
    let payload = wire::encode_query(&[player, unknown_entity()])
        .ok_or("step=metadata observed=query-encode")?;
    let reply = accepted(
        client,
        CAPABILITY,
        wire::COMMAND_TYPE_BATTLE_METADATA_QUERY,
        &payload,
    )
    .await?;
    wire::decode_reply(&reply).ok_or_else(|| "step=metadata observed=decode-failed".into())
}

async fn body(
    a: &mut DevClientSession,
    b: &mut DevClientSession,
    _clients: &SeamClients<'_>,
) -> TestResult {
    let b_seen = see_each_other(a, b).await?;
    let a_own = own(a)?;
    let a_seen =
        seen(b, a_own.identity).ok_or("step=admit observed=first-player-not-in-view-of-second")?;
    judge(&ask(a, b_seen).await?, b_seen)?;
    judge(&ask(b, a_seen).await?, a_seen)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut a = connect_character(clients, 0x80, 0).await?;
    let mut b = match connect_character(clients, 0x81, 1).await {
        Ok(b) => b,
        Err(error) => {
            finish(&mut a).await.ok();
            return Err(error);
        }
    };
    let outcome = body(&mut a, &mut b, clients).await;

    let closed_b = finish(&mut b).await;
    let closed_a = finish(&mut a).await;
    outcome?;
    closed_b?;
    closed_a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_is_one_bounded_token() {
        assert_eq!(
            compact("step=a  observed=b\nneeded=c"),
            "step=a-observed=b-needed=c"
        );
        assert!(!compact(&"x ".repeat(500)).contains(' '));
        assert!(compact(&"y".repeat(900)).len() <= 400);
    }

    #[test]
    fn judge_requires_vocation_and_silence_for_unknowns() {
        let player = EntityRef {
            identity: [1; 16],
            generation: 3,
        };
        let row = |entity, vocation| wire::PlayerMetadata {
            entity,
            vocation,
            same_guild: None,
        };
        assert!(judge(&[row(player, Some(2))], player).is_ok());
        assert!(judge(&[row(player, None)], player).is_err());
        assert!(judge(&[], player).is_err());
        assert!(
            judge(
                &[row(player, Some(2)), row(unknown_entity(), Some(1))],
                player
            )
            .is_err()
        );
    }
}
