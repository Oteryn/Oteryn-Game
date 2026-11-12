//! P22c (tracker persistence across login):
//! 1. Admit the node-boot character, track a race and require the durable tracker row
//!    (read-only SQL) to exist.
//! 2. Log out, freshly admit the character and require the tracker read and the durable row to
//!    still list the race.
//! 3. Remove it, log out, admit again and require it gone from the read and from the table.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{CommandStatus, cyclopedia_tracker as tr};
use sqlx::Connection;

const RAT: &str = "oteryn:creature.rat";

fn selected(client: &DevClientSession) -> TestResult {
    expect(
        client
            .selected_capabilities()
            .contains(&tr::CAPABILITY_CYCLOPEDIA_TRACKER_V1),
        "capability",
        "needed=capability-197-selected-at-join",
    )
}

/// One tracker command; the reply snapshot is `None` for an empty refusal payload.
async fn call(
    client: &mut DevClientSession,
    command: u32,
    request: &tr::TrackerRequest,
) -> TestResult<(CommandStatus, Option<tr::TrackerSnapshot>)> {
    let payload =
        tr::encode_request(request).ok_or("step=encode;observed=request-not-encodable")?;
    let reply = client
        .session_mut()
        .gated_command(tr::CAPABILITY_CYCLOPEDIA_TRACKER_V1, command, &payload)
        .await?;
    let snapshot = if reply.payload.is_empty() {
        None
    } else {
        Some(
            tr::decode_snapshot(&reply.payload)
                .ok_or("step=decode;observed=undecodable-tracker-snapshot")?,
        )
    };
    Ok((reply.status, snapshot))
}

fn change(action: tr::TrackerAction, key: &str) -> tr::TrackerRequest {
    tr::TrackerRequest::Change {
        action,
        creature_key: key.to_owned(),
    }
}

fn has(snapshot: &Option<tr::TrackerSnapshot>, key: &str) -> bool {
    snapshot
        .as_ref()
        .is_some_and(|list| list.entries.iter().any(|entry| entry.creature_key == key))
}

async fn read(
    client: &mut DevClientSession,
    command: u32,
) -> TestResult<Option<tr::TrackerSnapshot>> {
    let (status, list) = call(client, command, &tr::TrackerRequest::Read).await?;
    expect(status == CommandStatus::Accepted, "read", status)?;
    Ok(list)
}

async fn durable(clients: &SeamClients<'_>) -> TestResult<i64> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_cyclopedia_tracker \
         WHERE character_id=encode($1,'hex')::uuid AND tracker_kind=1 AND creature_key=$2",
    )
    .bind(clients.characters[1].as_slice())
    .bind(RAT)
    .fetch_one(&mut connection)
    .await?;
    Ok(rows)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<Verdict> {
    let best = tr::COMMAND_TYPE_BESTIARY_TRACKER;
    let mut client = connect_world(clients, 0xd3).await?;
    selected(&client)?;
    let (status, list) = call(&mut client, best, &change(tr::TrackerAction::Add, RAT)).await?;
    expect(
        status == CommandStatus::Accepted && has(&list, RAT),
        "add",
        status,
    )?;
    expect(
        durable(clients).await? == 1,
        "durable-after-add",
        "row-count-not-1",
    )?;
    logout(&mut client).await?;

    let mut client = connect_world(clients, 0xd4).await?;
    selected(&client)?;
    let list = read(&mut client, best).await?;
    expect(has(&list, RAT), "read-after-relog", "race-lost")?;
    expect(
        durable(clients).await? == 1,
        "durable-after-relog",
        "row-count-not-1",
    )?;
    let (status, list) = call(&mut client, best, &change(tr::TrackerAction::Remove, RAT)).await?;
    expect(
        status == CommandStatus::Accepted && !has(&list, RAT),
        "remove",
        status,
    )?;
    logout(&mut client).await?;

    let mut client = connect_world(clients, 0xd5).await?;
    let list = read(&mut client, best).await?;
    expect(
        !has(&list, RAT),
        "read-after-remove-relog",
        "race-still-tracked",
    )?;
    expect(
        durable(clients).await? == 0,
        "durable-after-remove",
        "row-remains",
    )?;
    logout(&mut client).await?;
    Ok((
        "PASS",
        "tracker-persists-across-login-and-removal-persists".to_owned(),
    ))
}

type Verdict = (&'static str, String);

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(150), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok((status, detail))) => (status, token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=P22c.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

/// One evidence token: no whitespace, bounded length.
fn token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || "-_.:/=<>,;".contains(character) {
                character
            } else {
                '-'
            }
        })
        .take(240)
        .collect()
}

fn expect(ok: bool, step: &str, observed: impl std::fmt::Debug) -> TestResult {
    if ok {
        Ok(())
    } else {
        Err(format!("step={step};observed={observed:?}").into())
    }
}

async fn logout(client: &mut DevClientSession) -> TestResult {
    let reply = client.session_mut().logout().await?;
    expect(
        reply.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted),
        "logout",
        reply.result,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_has_no_whitespace_and_is_bounded() {
        let value = token(&format!("step=a b\tc\n{}", "x".repeat(400)));
        assert!(!value.contains(char::is_whitespace));
        assert!(value.len() <= 240);
        assert!(value.starts_with("step=a-b-c-"));
    }
}
