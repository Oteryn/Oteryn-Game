//! S14a (Team Finder leader listing create/edit/remove):
//! 1. Admit the node-boot character with the leader capability (200); a malformed upsert must be
//!    refused and leave no template.
//! 2. Upsert a listing: accepted, the durable template row holds exactly its values and the
//!    advertisement is active. Upsert an edited listing: the row now holds the edit only.
//! 3. Remove: accepted, template and advertisement rows are gone.
//! 4. Upsert again and log out: the template is retained, the advertisement ends with the
//!    session (Crystal game.cpp:1422); a fresh admission removes the template for good.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::team_finder::{self as leader, LeaderListing, TeamTarget};
use sqlx::Connection;

async fn rows(clients: &SeamClients<'_>, index: usize, sql: &str) -> TestResult<i64> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    // Read-only check; the template values are this row's own listing constants.
    let count: i64 = sqlx::query_scalar(sqlx::AssertSqlSafe(sql.to_owned()))
        .bind(clients.characters[index].as_slice())
        .fetch_one(&mut connection)
        .await?;
    Ok(count)
}

fn template_sql(listing: &LeaderListing) -> String {
    format!(
        "SELECT count(*) FROM game_team_finder_leader_listings WHERE leader_character_id=encode($1,'hex')::uuid \
         AND min_level::bigint={} AND max_level::bigint={} AND team_slots::bigint={} AND free_slots::bigint={}",
        listing.min_level, listing.max_level, listing.slots, listing.free_slots
    )
}

const TEMPLATE_ANY: &str = "SELECT count(*) FROM game_team_finder_leader_listings WHERE leader_character_id=encode($1,'hex')::uuid";
const ACTIVE_ANY: &str = "SELECT count(*) FROM game_team_finder_active_listings WHERE leader_character_id=encode($1,'hex')::uuid";

async fn upsert(
    client: &mut DevClientSession,
    listing: LeaderListing,
) -> TestResult<CommandStatus> {
    let payload = leader::encode_upsert(listing)
        .map_err(|error| format!("step=encode;observed={error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            leader::CAPABILITY_TEAM_FINDER_LEADER_V1,
            leader::COMMAND_TYPE_TEAM_FINDER_LEADER_UPSERT,
            &payload,
        )
        .await?;
    Ok(reply.status)
}

async fn remove(client: &mut DevClientSession) -> TestResult<CommandStatus> {
    let reply = client
        .session_mut()
        .gated_command(
            leader::CAPABILITY_TEAM_FINDER_LEADER_V1,
            leader::COMMAND_TYPE_TEAM_FINDER_LEADER_REMOVE,
            &[],
        )
        .await?;
    Ok(reply.status)
}

fn listing(min_level: u16, max_level: u16, slots: u16, free_slots: u16) -> LeaderListing {
    LeaderListing {
        min_level,
        max_level,
        vocation_mask: 0x3f,
        slots,
        free_slots,
        party_members_join: false,
        timestamp: 1,
        target: TeamTarget::Boss(1),
    }
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<Verdict> {
    let first = listing(11, 1_234, 5, 4);
    let edited = listing(22, 1_500, 4, 2);
    let mut client = connect_world(clients, 0xe0).await?;
    expect(
        client
            .selected_capabilities()
            .contains(&leader::CAPABILITY_TEAM_FINDER_LEADER_V1),
        "capability",
        "needed=capability-200-selected-at-join",
    )?;
    remove(&mut client).await?;
    expect(
        rows(clients, 1, TEMPLATE_ANY).await? == 0,
        "baseline",
        "template-present",
    )?;

    let malformed = client
        .session_mut()
        .gated_command(
            leader::CAPABILITY_TEAM_FINDER_LEADER_V1,
            leader::COMMAND_TYPE_TEAM_FINDER_LEADER_UPSERT,
            &[0xff],
        )
        .await?;
    expect(
        malformed.status == CommandStatus::Rejected,
        "malformed-upsert",
        malformed.status,
    )?;
    expect(
        rows(clients, 1, TEMPLATE_ANY).await? == 0,
        "malformed-upsert",
        "template-written",
    )?;

    let status = upsert(&mut client, first).await?;
    expect(status == CommandStatus::Accepted, "create", status)?;
    expect(
        rows(clients, 1, &template_sql(&first)).await? == 1,
        "create-durable",
        "template-values-differ",
    )?;
    expect(
        rows(clients, 1, ACTIVE_ANY).await? == 1,
        "create-active",
        "no-active-listing",
    )?;

    let status = upsert(&mut client, edited).await?;
    expect(status == CommandStatus::Accepted, "edit", status)?;
    expect(
        rows(clients, 1, &template_sql(&edited)).await? == 1,
        "edit-durable",
        "edit-missing",
    )?;
    expect(
        rows(clients, 1, &template_sql(&first)).await? == 0,
        "edit-durable",
        "old-values-remain",
    )?;
    expect(
        rows(clients, 1, TEMPLATE_ANY).await? == 1,
        "edit-durable",
        "template-duplicated",
    )?;

    let status = remove(&mut client).await?;
    expect(status == CommandStatus::Accepted, "remove", status)?;
    expect(
        rows(clients, 1, TEMPLATE_ANY).await? == 0,
        "remove-durable",
        "template-remains",
    )?;
    expect(
        rows(clients, 1, ACTIVE_ANY).await? == 0,
        "remove-active",
        "active-listing-remains",
    )?;

    let status = upsert(&mut client, first).await?;
    expect(status == CommandStatus::Accepted, "recreate", status)?;
    logout(&mut client).await?;

    let mut fresh = connect_world(clients, 0xe1).await?;
    expect(
        rows(clients, 1, &template_sql(&first)).await? == 1,
        "template-after-relog",
        "template-lost",
    )?;
    expect(
        rows(clients, 1, ACTIVE_ANY).await? == 0,
        "active-after-relog",
        "advertisement-survived-logout",
    )?;
    let status = remove(&mut fresh).await?;
    expect(
        status == CommandStatus::Accepted,
        "remove-after-relog",
        status,
    )?;
    expect(
        rows(clients, 1, TEMPLATE_ANY).await? == 0,
        "remove-after-relog",
        "template-remains",
    )?;
    logout(&mut fresh).await?;
    Ok((
        "PASS",
        "create-edit-remove-malformed-and-logout-lifecycle-validated".to_owned(),
    ))
}

type Verdict = (&'static str, String);

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(120), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok((status, detail))) => (status, token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=S14A.full status={status} seconds={:.3} evidence={detail}",
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

    #[test]
    fn template_sql_names_every_listing_value() {
        let sql = template_sql(&listing(11, 1_234, 5, 4));
        assert!(sql.contains("min_level::bigint=11") && sql.contains("max_level::bigint=1234"));
        assert!(sql.contains("team_slots::bigint=5") && sql.contains("free_slots::bigint=4"));
    }
}
