//! S14b (Team Finder browse and join request/cancel), two live characters:
//! 1. The leader (character 0) advertises; the member (character 1) browses and finds the
//!    advertisement with no request of its own.
//! 2. The member requests to join: accepted, browse shows Pending, the leader's applicant list
//!    shows a Pending applicant and the durable member row exists.
//! 3. The member cancels: accepted, browse shows no status, the applicant and the row are gone.
//! 4. The leader removes the advertisement and its durable template.
//! Member relogs verify request and cancel. A leader relog verifies removal.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::team_finder::{self as leader, LeaderListing, TeamTarget};
use oteryn_protocol_oteryn::team_finder_members as tf;
use sqlx::Connection;

async fn query(
    client: &mut DevClientSession,
    request: tf::TeamFinderQuery,
) -> TestResult<tf::TeamFinderResult> {
    let payload = tf::encode_query(request)
        .map_err(|error| format!("step=encode-query;observed={error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            tf::CAPABILITY_TEAM_FINDER_MEMBERS_V1,
            tf::COMMAND_TYPE_TEAM_FINDER_QUERY,
            &payload,
        )
        .await?;
    expect(
        reply.status == CommandStatus::Accepted,
        "query",
        reply.status,
    )?;
    Ok(tf::decode_result(&reply.payload)
        .map_err(|error| format!("step=query-decode;observed={error:?}"))?)
}

/// Find the advertisement belonging to the exact leader.
async fn browse(
    client: &mut DevClientSession,
    leader_name: &str,
) -> TestResult<Option<tf::BrowseListing>> {
    let mut after = None;
    for _ in 0..8 {
        match query(client, tf::TeamFinderQuery::Browse { after }).await? {
            tf::TeamFinderResult::Browse { listings, next } => {
                if let Some(found) = listings.into_iter().find(|entry| entry.name == leader_name) {
                    return Ok(Some(found));
                }
                match next {
                    Some(cursor) => after = Some(cursor),
                    None => return Ok(None),
                }
            }
            tf::TeamFinderResult::Applicants { .. } => {
                return Err("step=browse;observed=applicants-result".into());
            }
        }
    }
    Err("step=browse;observed=pagination-limit".into())
}

async fn applicants(
    client: &mut DevClientSession,
    want: Option<LeaderListing>,
    pending_member: Option<&str>,
) -> TestResult {
    let observed = query(client, tf::TeamFinderQuery::Applicants { after: None }).await?;
    let tf::TeamFinderResult::Applicants {
        listing,
        occupied_slots,
        members,
        next,
    } = &observed
    else {
        return Err("step=applicants;observed=browse-result".into());
    };
    let expected_count = usize::from(want.is_some()) + usize::from(pending_member.is_some());
    expect(
        *listing == want
            && *occupied_slots == 0
            && next.is_none()
            && members.len() == expected_count
            && match pending_member {
                Some(name) => members
                    .iter()
                    .any(|entry| entry.name == name && entry.status == tf::MemberStatus::Pending),
                None => members
                    .iter()
                    .all(|entry| entry.status != tf::MemberStatus::Pending),
            },
        "applicants",
        observed,
    )
}

async fn member_action(
    client: &mut DevClientSession,
    action: tf::MemberAction,
) -> TestResult<CommandStatus> {
    let payload = tf::encode_member_action(action)
        .map_err(|error| format!("step=encode-member;observed={error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            tf::CAPABILITY_TEAM_FINDER_MEMBERS_V1,
            tf::COMMAND_TYPE_TEAM_FINDER_MEMBER_ACTION,
            &payload,
        )
        .await?;
    Ok(reply.status)
}

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

fn capable(client: &DevClientSession, capability: u32) -> TestResult {
    expect(
        client.selected_capabilities().contains(&capability),
        "capability",
        format!("needed=capability-{capability}-selected-at-join"),
    )
}

fn leader_listing() -> LeaderListing {
    LeaderListing {
        min_level: 3,
        max_level: 4_321,
        vocation_mask: 0x3f,
        slots: 7,
        free_slots: 7,
        party_members_join: false,
        timestamp: 1,
        target: TeamTarget::Boss(1),
    }
}

async fn character_name(
    connection: &mut sqlx::PgConnection,
    character: &[u8; 16],
) -> TestResult<String> {
    Ok(sqlx::query_scalar(
        "SELECT name FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_slice())
    .fetch_one(connection)
    .await?)
}

async fn pending(
    connection: &mut sqlx::PgConnection,
    clients: &SeamClients<'_>,
    expected: Option<i16>,
    step: &str,
) -> TestResult {
    let status: Option<i16> = sqlx::query_scalar(
        "SELECT status FROM game_team_finder_members WHERE leader_character_id=encode($1,'hex')::uuid \
         AND member_character_id=encode($2,'hex')::uuid",
    )
    .bind(clients.characters[0].as_slice())
    .bind(clients.characters[1].as_slice())
    .fetch_optional(connection)
    .await?;
    expect(status == expected, step, status)
}

async fn template_exists(
    connection: &mut sqlx::PgConnection,
    character: &[u8; 16],
) -> TestResult<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM game_team_finder_leader_listings \
         WHERE leader_character_id=encode($1,'hex')::uuid)",
    )
    .bind(character.as_slice())
    .fetch_one(connection)
    .await?)
}

async fn listing_status(
    member: &mut DevClientSession,
    leader_name: &str,
    want: LeaderListing,
    status: tf::MemberStatus,
    step: &str,
) -> TestResult<u64> {
    let listing = browse(member, leader_name)
        .await?
        .ok_or_else(|| format!("step={step};observed=listing-missing"))?;
    expect(
        listing.listing == want && listing.occupied_slots == 0 && listing.status == status,
        step,
        &listing,
    )?;
    Ok(listing.leader_handle)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<Verdict> {
    const PENDING: Option<i16> = Some(1);
    let want = leader_listing();
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let leader_id = &clients.characters[0];
    let leader_name = character_name(&mut connection, leader_id).await?;
    let member_name = character_name(&mut connection, &clients.characters[1]).await?;
    let mut lead = connect_character(clients, 0xe2, 0).await?;
    let mut member = connect_world(clients, 0xe3).await?;
    capable(&lead, leader::CAPABILITY_TEAM_FINDER_LEADER_V1)?;
    capable(&lead, tf::CAPABILITY_TEAM_FINDER_MEMBERS_V1)?;
    capable(&member, tf::CAPABILITY_TEAM_FINDER_MEMBERS_V1)?;
    expect(
        !template_exists(&mut connection, leader_id).await?,
        "baseline",
        "template-present",
    )?;
    expect(
        browse(&mut member, &leader_name).await?.is_none(),
        "baseline",
        "listing-present",
    )?;
    applicants(&mut lead, None, None).await?;
    pending(&mut connection, clients, None, "baseline-member").await?;

    let status = upsert(&mut lead, want).await?;
    expect(status == CommandStatus::Accepted, "leader-upsert", status)?;
    expect(
        template_exists(&mut connection, leader_id).await?,
        "leader-upsert",
        "template-missing",
    )?;
    applicants(&mut lead, Some(want), None).await?;
    let handle = listing_status(
        &mut member,
        &leader_name,
        want,
        tf::MemberStatus::None,
        "browse-initial-status",
    )
    .await?;

    let status = member_action(&mut member, tf::MemberAction::Request { leader: handle }).await?;
    expect(status == CommandStatus::Accepted, "request", status)?;
    listing_status(
        &mut member,
        &leader_name,
        want,
        tf::MemberStatus::Pending,
        "request-status",
    )
    .await?;
    applicants(&mut lead, Some(want), Some(&member_name)).await?;
    pending(&mut connection, clients, PENDING, "request-durable").await?;

    // Keep the leader online so its advertisement and the member's pending row
    // must survive the member's new session. Browse issues that session's handle.
    logout(&mut member, "request-member-logout").await?;
    let mut member = connect_world(clients, 0xe4).await?;
    let handle = listing_status(
        &mut member,
        &leader_name,
        want,
        tf::MemberStatus::Pending,
        "request-relog-status",
    )
    .await?;
    applicants(&mut lead, Some(want), Some(&member_name)).await?;
    pending(&mut connection, clients, PENDING, "request-relog-durable").await?;

    let status = member_action(&mut member, tf::MemberAction::Cancel { leader: handle }).await?;
    expect(status == CommandStatus::Accepted, "cancel", status)?;
    listing_status(
        &mut member,
        &leader_name,
        want,
        tf::MemberStatus::None,
        "cancel-status",
    )
    .await?;
    applicants(&mut lead, Some(want), None).await?;
    pending(&mut connection, clients, None, "cancel-durable").await?;

    logout(&mut member, "cancel-member-logout").await?;
    let mut member = connect_world(clients, 0xe5).await?;
    listing_status(
        &mut member,
        &leader_name,
        want,
        tf::MemberStatus::None,
        "cancel-relog-status",
    )
    .await?;
    applicants(&mut lead, Some(want), None).await?;
    pending(&mut connection, clients, None, "cancel-relog-durable").await?;

    let status = remove(&mut lead).await?;
    expect(status == CommandStatus::Accepted, "leader-remove", status)?;
    expect(
        browse(&mut member, &leader_name).await?.is_none(),
        "leader-remove",
        "listing-still-browsable",
    )?;
    applicants(&mut lead, None, None).await?;
    expect(
        !template_exists(&mut connection, leader_id).await?,
        "leader-remove",
        "template-remains",
    )?;

    logout(&mut lead, "removal-leader-logout").await?;
    let mut lead = connect_character(clients, 0xe6, 0).await?;
    expect(
        browse(&mut member, &leader_name).await?.is_none(),
        "removal-relog",
        "listing-still-browsable",
    )?;
    applicants(&mut lead, None, None).await?;
    expect(
        !template_exists(&mut connection, leader_id).await?,
        "removal-relog",
        "template-remains",
    )?;
    logout(&mut member, "member-logout").await?;
    logout(&mut lead, "leader-logout").await?;
    Ok((
        "PASS",
        "browse-request-cancel-removal-and-relogs-validated".to_owned(),
    ))
}

type Verdict = (&'static str, String);

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result =
        verdict::indexed_timeout(Duration::from_secs(180), verdict::observe(journey(clients)))
            .await;
    let (status, detail) = match result {
        Ok(Ok((status, detail))) => (status, token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=S14b.full status={status} seconds={:.3} evidence={detail}",
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

async fn logout(client: &mut DevClientSession, step: &str) -> TestResult {
    let outcome = client
        .session_mut()
        .logout()
        .await
        .map_err(|error| format!("step={step};observed={error}"))?;
    expect(
        outcome.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted),
        step,
        outcome,
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
