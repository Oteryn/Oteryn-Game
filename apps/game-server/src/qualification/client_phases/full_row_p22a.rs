//! P22a (Crystal Cyclopedia bestiary tracker add/remove), driven through the native session:
//! 1. Admit the node-boot character and require the tracker capability (197).
//! 2. Read the bestiary tracker, add a known race, and require the accepted refreshed list to
//!    contain it; an unknown race key must be refused.
//! 3. Read it again, require the bosstiary list not to contain the race, remove it and require
//!    the accepted refreshed list and a later read to drop it.
//! 4. Log out; the logout must be accepted.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{CommandStatus, cyclopedia_tracker as tr};

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

const UNKNOWN: &str = "oteryn:creature.no-such-race-p22a";

async fn journey(clients: &SeamClients<'_>) -> TestResult<Verdict> {
    let mut client = connect_world(clients, 0xd1).await?;
    selected(&client)?;
    let best = tr::COMMAND_TYPE_BESTIARY_TRACKER;
    let boss = tr::COMMAND_TYPE_BOSSTIARY_TRACKER;
    // A known start; the status of this remove is not judged.
    call(&mut client, best, &change(tr::TrackerAction::Remove, RAT)).await?;
    let list = read(&mut client, best).await?;
    expect(!has(&list, RAT), "baseline", "race-still-tracked")?;

    let (status, list) = call(&mut client, best, &change(tr::TrackerAction::Add, RAT)).await?;
    expect(
        status == CommandStatus::Accepted && has(&list, RAT),
        "add",
        status,
    )?;
    let list = read(&mut client, best).await?;
    expect(has(&list, RAT), "read-after-add", "race-missing")?;
    let bosses = read(&mut client, boss).await?;
    expect(
        !has(&bosses, RAT),
        "bosstiary-isolated",
        "race-in-boss-list",
    )?;

    let (status, _) = call(&mut client, best, &change(tr::TrackerAction::Add, UNKNOWN)).await?;
    expect(status == CommandStatus::Rejected, "unknown-race", status)?;
    let list = read(&mut client, best).await?;
    expect(!has(&list, UNKNOWN), "unknown-race", "unknown-key-tracked")?;

    // Durable: the entry survives logout and a fresh admission, and stays out of the bosses.
    logout(&mut client).await?;
    let mut client = connect_world(clients, 0xd3).await?;
    selected(&client)?;
    let list = read(&mut client, best).await?;
    expect(has(&list, RAT), "relog-with-entry", "race-lost-after-relog")?;
    let bosses = read(&mut client, boss).await?;
    expect(
        !has(&bosses, RAT),
        "relog-bosstiary-isolated",
        "race-in-boss-list",
    )?;

    let (status, list) = call(&mut client, best, &change(tr::TrackerAction::Remove, RAT)).await?;
    expect(
        status == CommandStatus::Accepted && !has(&list, RAT),
        "remove",
        status,
    )?;
    let list = read(&mut client, best).await?;
    expect(!has(&list, RAT), "read-after-remove", "race-still-tracked")?;
    // Durable: the removal also survives logout and a fresh admission.
    logout(&mut client).await?;
    let mut client = connect_world(clients, 0xd4).await?;
    selected(&client)?;
    let list = read(&mut client, best).await?;
    expect(
        !has(&list, RAT),
        "relog-after-remove",
        "race-back-after-relog",
    )?;
    logout(&mut client).await?;
    Ok((
        "PASS",
        "add-read-relog-remove-relog-and-unknown-refusal-validated".to_owned(),
    ))
}

type Verdict = (&'static str, String);

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok((status, detail))) => (status, token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=P22a.full status={status} seconds={:.3} evidence={detail}",
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
