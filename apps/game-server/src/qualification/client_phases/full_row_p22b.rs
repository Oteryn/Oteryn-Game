//! P22b (bosstiary tracker add/remove, kill-gated):
//! 1. Admit the node-boot character and require the tracker capability (197).
//! 2. Add a boss to the bosstiary tracker. The server only accepts a boss the character has a
//!    credited kill of; a refusal is the correct answer for a character without one, so the row
//!    then reports the missing precondition (read-only SQL shows the kill progress it saw).
//! 3. With a credited kill: require the accepted refreshed list, remove it again and require
//!    the list to drop it.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::{CommandStatus, cyclopedia_tracker as tr};
use sqlx::Connection;

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
    if let Some(snapshot) = &snapshot {
        expect(snapshot.boss, "decode", "expected-bosstiary-snapshot")?;
    }
    Ok((reply.status, snapshot))
}

fn change(action: tr::TrackerAction, key: &str) -> tr::TrackerRequest {
    tr::TrackerRequest::Change {
        action,
        creature_key: key.to_owned(),
    }
}

fn contains_boss(snapshot: &tr::TrackerSnapshot) -> bool {
    snapshot
        .entries
        .iter()
        .any(|entry| entry.creature_key == BOSS)
}

fn has_seeded_credit(snapshot: &tr::TrackerSnapshot) -> bool {
    snapshot
        .entries
        .iter()
        .any(|entry| entry.creature_key == BOSS && entry.kill_count == 1)
}

async fn read(client: &mut DevClientSession, command: u32) -> TestResult<tr::TrackerSnapshot> {
    let (status, list) = call(client, command, &tr::TrackerRequest::Read).await?;
    expect(status == CommandStatus::Accepted, "read", status)?;
    list.ok_or_else(|| "step=read;observed=missing-tracker-snapshot".into())
}

const BOSS: &str = "oteryn:creature.ferumbras";

async fn progress(clients: &SeamClients<'_>) -> TestResult<i64> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let rows: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_bosstiary_progress \
         WHERE character_id=encode($1,'hex')::uuid AND boss_key=$2 AND kill_count::bigint > 0",
    )
    .bind(clients.characters[0].as_slice())
    .bind(BOSS)
    .fetch_one(&mut connection)
    .await?;
    Ok(rows)
}

async fn seed_boss_credit(clients: &SeamClients<'_>) -> TestResult {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    // One unboosted Nemesis kill reaches Prowess (10 points).
    let (kills, points): (i64, i64) = sqlx::query_as(
        "INSERT INTO game_character_bosstiary_progress
         (character_id, boss_key, kill_count, boss_points)
         VALUES (encode($1,'hex')::uuid, $2, 1, 10)
         RETURNING kill_count, boss_points",
    )
    .bind(clients.characters[0].as_slice())
    .bind(BOSS)
    .fetch_one(&mut connection)
    .await?;
    expect(
        (kills, points) == (1, 10),
        "fixture-credit",
        (kills, points),
    )?;
    evidence(&format!(
        "FIXTURE row=P22b.full account=q_p22b boss={BOSS} kill_count={kills} boss_points={points}"
    ));
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<Verdict> {
    let dedicated = character_prep::dedicated_named_character(
        clients,
        "NODE_BOOT_P22B_ACCOUNT",
        Some("Prowess Hunter"),
    )
    .await?;
    let scoped = dedicated.clients(clients);
    let clients = &scoped;
    seed_boss_credit(clients).await?;
    let mut client = connect_character(clients, 0xd2, 0).await?;
    selected(&client)?;
    let boss = tr::COMMAND_TYPE_BOSSTIARY_TRACKER;
    let before = read(&mut client, boss).await?;
    expect(
        !contains_boss(&before),
        "before-add",
        "boss-already-tracked",
    )?;
    let credited = progress(clients).await?;
    let (status, list) = call(&mut client, boss, &change(tr::TrackerAction::Add, BOSS)).await?;
    if status == CommandStatus::Rejected {
        let list = read(&mut client, boss).await?;
        expect(
            list == before,
            "refused-add",
            "tracker-changed-despite-refusal",
        )?;
        let (removed, _) =
            call(&mut client, boss, &change(tr::TrackerAction::Remove, BOSS)).await?;
        logout(&mut client).await?;
        return Ok((
            "FAIL",
            format!(
                "step=boss-add;observed=Rejected,remove:{removed:?},credited-rows:{credited};needed=credited-kill-of-{BOSS}"
            ),
        ));
    }
    expect(
        status == CommandStatus::Accepted
            && list.as_ref().is_some_and(has_seeded_credit)
            && credited > 0,
        "boss-add",
        (status, credited),
    )?;
    let list = read(&mut client, boss).await?;
    expect(has_seeded_credit(&list), "read-after-add", "boss-missing")?;
    // Durable: the entry survives logout and a fresh admission.
    logout(&mut client).await?;
    let mut client = connect_character(clients, 0xd3, 0).await?;
    selected(&client)?;
    let list = read(&mut client, boss).await?;
    expect(
        has_seeded_credit(&list),
        "relog-with-entry",
        "boss-lost-after-relog",
    )?;
    let (status, list) = call(&mut client, boss, &change(tr::TrackerAction::Remove, BOSS)).await?;
    expect(
        status == CommandStatus::Accepted && list.as_ref().is_some_and(|list| !contains_boss(list)),
        "boss-remove",
        status,
    )?;
    let list = read(&mut client, boss).await?;
    expect(
        !contains_boss(&list),
        "read-after-remove",
        "boss-still-tracked",
    )?;
    // Durable: the removal also survives logout and a fresh admission.
    logout(&mut client).await?;
    let mut client = connect_character(clients, 0xd4, 0).await?;
    selected(&client)?;
    let list = read(&mut client, boss).await?;
    expect(
        !contains_boss(&list),
        "relog-after-remove",
        "boss-back-after-relog",
    )?;
    logout(&mut client).await?;
    Ok((
        "PASS",
        "boss-add-relog-remove-relog-with-credited-kill-validated".to_owned(),
    ))
}

type Verdict = (&'static str, String);

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(120), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok((status, detail))) => (status, token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=P22b.full status={status} seconds={:.3} evidence={detail}",
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
