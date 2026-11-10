//! G21 chat console sender-level lookup (capability 7, command 6117).
//! 1. Admit the node-boot character; look up its own name, the same name upper-cased, the other
//!    provisioned character and a name no character holds.
//! 2. The levels must equal the durable progression levels (read-only SQL), the name match must
//!    be case-insensitive, and the unknown name must resolve to nothing.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::chat::console as wire;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(90), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "chat-console-levels-match-durable-progression".to_owned(),
        ),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G21.full status={status} seconds={:.3} evidence={detail}",
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

const CAPABILITY: u32 = 7;
const UNKNOWN: &str = "Nobodyherexyz";

async fn name_of(clients: &SeamClients<'_>, index: usize) -> TestResult<String> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    Ok(sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(clients.characters[index].as_slice())
    .fetch_one(&mut connection)
    .await?)
}

async fn level_of(clients: &SeamClients<'_>, index: usize) -> TestResult<u32> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let level: i64 = sqlx::query_scalar(
        "SELECT p.level::bigint FROM game_character_roots r JOIN game_character_progression_state p USING(character_id) WHERE r.character_id=encode($1,'hex')::uuid",
    )
    .bind(clients.characters[index].as_slice())
    .fetch_one(&mut connection)
    .await?;
    Ok(u32::try_from(level)?)
}

fn judged(levels: &[Option<u32>], own: u32, other: u32) -> bool {
    levels.len() == 4
        && levels[0] == Some(own)
        && levels[1] == Some(own)
        && levels[2] == Some(other)
        && levels[3].is_none()
}

async fn body(client: &mut DevClientSession, clients: &SeamClients<'_>) -> TestResult {
    let own = name_of(clients, 1).await?;
    let other = name_of(clients, 0).await?;
    let names = [own.clone(), own.to_uppercase(), other, UNKNOWN.to_owned()];
    let payload = wire::encode_names(&names).ok_or("step=console observed=names-encode")?;
    let reply = accepted(client, CAPABILITY, wire::COMMAND, &payload).await?;
    let levels = wire::decode_levels(&reply).ok_or("step=console observed=decode-failed")?;
    let (own_level, other_level) = (level_of(clients, 1).await?, level_of(clients, 0).await?);
    if !judged(&levels, own_level, other_level) {
        return Err(format!(
            "step=console observed={levels:?} needed={own_level},{own_level},{other_level},none"
        )
        .into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0x84).await?;
    let outcome = body(&mut client, clients).await;
    let closed = finish(&mut client).await;
    outcome?;
    closed
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
    fn judged_requires_case_insensitive_hits_and_unknown_miss() {
        assert!(judged(&[Some(8), Some(8), Some(9), None], 8, 9));
        assert!(!judged(&[Some(8), None, Some(9), None], 8, 9));
        assert!(!judged(&[Some(8), Some(8), Some(9), Some(1)], 8, 9));
        assert!(!judged(&[Some(8)], 8, 9));
    }
}
