//! S14b (Team Finder browse and join request/cancel), two live characters:
//! 1. The leader (character 0) advertises; the member (character 1) browses and finds the
//!    advertisement with no request of its own.
//! 2. The member requests to join: accepted, browse shows Pending, the leader's applicant list
//!    shows a Pending applicant and the durable member row exists.
//! 3. The member cancels: accepted, browse shows no status, the applicant and the row are gone.
//! 4. The leader removes the advertisement: it is no longer browsable. Both log out.
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

fn same(left: &LeaderListing, right: &LeaderListing) -> bool {
    left.min_level == right.min_level
        && left.max_level == right.max_level
        && left.vocation_mask == right.vocation_mask
        && left.slots == right.slots
        && left.target == right.target
}

/// The handle and this member's status of the advertisement equal to `want`, if browsable.
async fn browse(
    client: &mut DevClientSession,
    want: &LeaderListing,
) -> TestResult<Option<(u64, tf::MemberStatus)>> {
    let mut after = None;
    for _ in 0..8 {
        match query(client, tf::TeamFinderQuery::Browse { after }).await? {
            tf::TeamFinderResult::Browse { listings, next } => {
                if let Some(found) = listings.iter().find(|entry| same(&entry.listing, want)) {
                    return Ok(Some((found.leader_handle, found.status)));
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
    Ok(None)
}

async fn applicants(client: &mut DevClientSession) -> TestResult<Vec<tf::Applicant>> {
    match query(client, tf::TeamFinderQuery::Applicants { after: None }).await? {
        tf::TeamFinderResult::Applicants { members, .. } => Ok(members),
        tf::TeamFinderResult::Browse { .. } => Err("step=applicants;observed=browse-result".into()),
    }
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

/// Leader (character 0) and member (character 1) in one world, the leader advertising `want`.
async fn pair(
    clients: &SeamClients<'_>,
    tags: (u8, u8),
    want: LeaderListing,
) -> TestResult<(DevClientSession, DevClientSession, u64)> {
    let mut lead = connect_character(clients, tags.0, 0).await?;
    let mut member = connect_world(clients, tags.1).await?;
    capable(&lead, leader::CAPABILITY_TEAM_FINDER_LEADER_V1)?;
    capable(&lead, tf::CAPABILITY_TEAM_FINDER_MEMBERS_V1)?;
    capable(&member, tf::CAPABILITY_TEAM_FINDER_MEMBERS_V1)?;
    remove(&mut lead).await?;
    let status = upsert(&mut lead, want).await?;
    expect(status == CommandStatus::Accepted, "leader-upsert", status)?;
    let (handle, state) = browse(&mut member, &want)
        .await?
        .ok_or("step=browse;observed=active-listing-not-browsable")?;
    expect(
        state == tf::MemberStatus::None,
        "browse-initial-status",
        state,
    )?;
    Ok((lead, member, handle))
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

async fn pending(clients: &SeamClients<'_>) -> TestResult<i64> {
    let mut connection = sqlx::PgConnection::connect(clients.url).await?;
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_team_finder_members WHERE leader_character_id=encode($1,'hex')::uuid \
         AND member_character_id=encode($2,'hex')::uuid",
    )
    .bind(clients.characters[0].as_slice())
    .bind(clients.characters[1].as_slice())
    .fetch_one(&mut connection)
    .await?;
    Ok(count)
}

async fn journey(clients: &SeamClients<'_>) -> TestResult<Verdict> {
    let want = leader_listing();
    let (mut lead, mut member, handle) = pair(clients, (0xe2, 0xe3), want).await?;
    expect(
        pending(clients).await? == 0,
        "baseline",
        "member-row-present",
    )?;

    let status = member_action(&mut member, tf::MemberAction::Request { leader: handle }).await?;
    expect(status == CommandStatus::Accepted, "request", status)?;
    let (_, state) = browse(&mut member, &want)
        .await?
        .ok_or("step=request;observed=listing-vanished")?;
    expect(state == tf::MemberStatus::Pending, "request-status", state)?;
    let waiting = applicants(&mut lead).await?;
    expect(
        waiting
            .iter()
            .any(|entry| entry.status == tf::MemberStatus::Pending),
        "request-applicant",
        waiting.len(),
    )?;
    expect(
        pending(clients).await? == 1,
        "request-durable",
        "member-row-missing",
    )?;

    let status = member_action(&mut member, tf::MemberAction::Cancel { leader: handle }).await?;
    expect(status == CommandStatus::Accepted, "cancel", status)?;
    let (_, state) = browse(&mut member, &want)
        .await?
        .ok_or("step=cancel;observed=listing-vanished")?;
    expect(state == tf::MemberStatus::None, "cancel-status", state)?;
    let waiting = applicants(&mut lead).await?;
    expect(
        waiting
            .iter()
            .all(|entry| entry.status != tf::MemberStatus::Pending),
        "cancel-applicant",
        waiting.len(),
    )?;
    expect(
        pending(clients).await? == 0,
        "cancel-durable",
        "member-row-remains",
    )?;

    let status = remove(&mut lead).await?;
    expect(status == CommandStatus::Accepted, "leader-remove", status)?;
    expect(
        browse(&mut member, &want).await?.is_none(),
        "leader-remove",
        "listing-still-browsable",
    )?;
    logout(&mut member).await?;
    logout(&mut lead).await?;
    Ok((
        "PASS",
        "browse-request-cancel-and-listing-removal-validated".to_owned(),
    ))
}

type Verdict = (&'static str, String);

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(180), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok((status, detail))) => (status, token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=S14B.full status={status} seconds={:.3} evidence={detail}",
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
