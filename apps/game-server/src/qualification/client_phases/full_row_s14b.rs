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

#[path = "full_row_s14b_replay.rs"]
mod replay;
use replay::{Action, Client};

async fn query(
    client: &mut Client,
    request: tf::TeamFinderQuery,
) -> TestResult<tf::TeamFinderResult> {
    let payload = tf::encode_query(request)
        .map_err(|error| format!("step=encode-query;observed={error:?}"))?;
    let reply = client
        .record(
            Action::Query(request),
            tf::CAPABILITY_TEAM_FINDER_MEMBERS_V1,
            tf::COMMAND_TYPE_TEAM_FINDER_QUERY,
            payload,
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
async fn browse(client: &mut Client, leader_name: &str) -> TestResult<Option<tf::BrowseListing>> {
    let result = query(client, tf::TeamFinderQuery::Browse { after: None }).await?;
    unique_listing(result, leader_name)
}

// An action handle is only valid on the current page. An incomplete browse
// cannot prove uniqueness (or absence) on this isolated fixture.
fn unique_listing(
    result: tf::TeamFinderResult,
    leader_name: &str,
) -> TestResult<Option<tf::BrowseListing>> {
    let tf::TeamFinderResult::Browse { listings, next } = result else {
        return Err("step=browse;observed=applicants-result".into());
    };
    expect(next.is_none(), "browse", "incomplete-page")?;
    let mut matches = listings
        .into_iter()
        .filter(|entry| entry.name == leader_name);
    let found = matches.next();
    expect(matches.next().is_none(), "browse", "duplicate-leader")?;
    Ok(found)
}

async fn applicants(
    client: &mut Client,
    leader_name: &str,
    want: Option<LeaderListing>,
    pending_member: Option<&str>,
) -> TestResult {
    let observed = query(client, tf::TeamFinderQuery::Applicants { after: None }).await?;
    expect(
        exact_applicants(&observed, leader_name, want, pending_member),
        "applicants",
        observed,
    )
}

fn exact_applicants(
    observed: &tf::TeamFinderResult,
    leader_name: &str,
    want: Option<LeaderListing>,
    pending_member: Option<&str>,
) -> bool {
    let tf::TeamFinderResult::Applicants {
        listing,
        occupied_slots,
        members,
        next,
    } = observed
    else {
        return false;
    };
    if *listing != want || *occupied_slots != 0 || next.is_some() {
        return false;
    }
    match (want, pending_member, members.as_slice()) {
        (None, None, []) => true,
        (Some(_), None, [leader]) => {
            leader.name == leader_name && leader.status == tf::MemberStatus::Accepted
        }
        (Some(_), Some(member_name), [leader, member]) => {
            leader.name == leader_name
                && leader.status == tf::MemberStatus::Accepted
                && member.name == member_name
                && member.status == tf::MemberStatus::Pending
        }
        _ => false,
    }
}

async fn member_action(client: &mut Client, action: tf::MemberAction) -> TestResult<CommandStatus> {
    let payload = tf::encode_member_action(action)
        .map_err(|error| format!("step=encode-member;observed={error:?}"))?;
    let reply = client
        .record(
            Action::Member(action),
            tf::CAPABILITY_TEAM_FINDER_MEMBERS_V1,
            tf::COMMAND_TYPE_TEAM_FINDER_MEMBER_ACTION,
            payload,
        )
        .await?;
    Ok(reply.status)
}

async fn upsert(client: &mut Client, listing: LeaderListing) -> TestResult<CommandStatus> {
    let payload = leader::encode_upsert(listing)
        .map_err(|error| format!("step=encode;observed={error:?}"))?;
    let reply = client
        .record(
            Action::Leader(Some(listing)),
            leader::CAPABILITY_TEAM_FINDER_LEADER_V1,
            leader::COMMAND_TYPE_TEAM_FINDER_LEADER_UPSERT,
            payload,
        )
        .await?;
    Ok(reply.status)
}

async fn remove(client: &mut Client) -> TestResult<CommandStatus> {
    let reply = client
        .record(
            Action::Leader(None),
            leader::CAPABILITY_TEAM_FINDER_LEADER_V1,
            leader::COMMAND_TYPE_TEAM_FINDER_LEADER_REMOVE,
            Vec::new(),
        )
        .await?;
    Ok(reply.status)
}

fn capable(client: &Client, capability: u32) -> TestResult {
    expect(
        client.wire.selected_capabilities().contains(&capability),
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
    member: &mut Client,
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
    let mut lead = Client::new(connect_character(clients, 0xe2, 0).await?);
    let mut member = Client::new(connect_world(clients, 0xe3).await?);
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
    applicants(&mut lead, &leader_name, None, None).await?;
    pending(&mut connection, clients, None, "baseline-member").await?;

    let status = upsert(&mut lead, want).await?;
    expect(status == CommandStatus::Accepted, "leader-upsert", status)?;
    expect(
        template_exists(&mut connection, leader_id).await?,
        "leader-upsert",
        "template-missing",
    )?;
    applicants(&mut lead, &leader_name, Some(want), None).await?;
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
    applicants(&mut lead, &leader_name, Some(want), Some(&member_name)).await?;
    pending(&mut connection, clients, PENDING, "request-durable").await?;

    // Keep the leader online so its advertisement and the member's pending row
    // must survive the member's new session. Browse issues that session's handle.
    logout(&mut member, "request-member-logout").await?;
    let requested = member;
    let mut member = Client::new(connect_world(clients, 0xe4).await?);
    let handle = listing_status(
        &mut member,
        &leader_name,
        want,
        tf::MemberStatus::Pending,
        "request-relog-status",
    )
    .await?;
    applicants(&mut lead, &leader_name, Some(want), Some(&member_name)).await?;
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
    applicants(&mut lead, &leader_name, Some(want), None).await?;
    pending(&mut connection, clients, None, "cancel-durable").await?;

    logout(&mut member, "cancel-member-logout").await?;
    let canceled = member;
    let mut member = Client::new(connect_world(clients, 0xe5).await?);
    listing_status(
        &mut member,
        &leader_name,
        want,
        tf::MemberStatus::None,
        "cancel-relog-status",
    )
    .await?;
    applicants(&mut lead, &leader_name, Some(want), None).await?;
    pending(&mut connection, clients, None, "cancel-relog-durable").await?;

    let status = remove(&mut lead).await?;
    expect(status == CommandStatus::Accepted, "leader-remove", status)?;
    expect(
        browse(&mut member, &leader_name).await?.is_none(),
        "leader-remove",
        "listing-still-browsable",
    )?;
    applicants(&mut lead, &leader_name, None, None).await?;
    expect(
        !template_exists(&mut connection, leader_id).await?,
        "leader-remove",
        "template-remains",
    )?;

    logout(&mut lead, "removal-leader-logout").await?;
    let removed = lead;
    let mut lead = Client::new(connect_character(clients, 0xe6, 0).await?);
    expect(
        browse(&mut member, &leader_name).await?.is_none(),
        "removal-relog",
        "listing-still-browsable",
    )?;
    applicants(&mut lead, &leader_name, None, None).await?;
    expect(
        !template_exists(&mut connection, leader_id).await?,
        "removal-relog",
        "template-remains",
    )?;
    logout(&mut member, "member-logout").await?;
    logout(&mut lead, "leader-logout").await?;
    for client in [requested, canceled, member, removed, lead] {
        client.replay(&leader_name)?;
    }
    Ok((
        "PASS",
        "browse-request-cancel-removal-relogs-and-native-replay-validated".to_owned(),
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

async fn logout(client: &mut Client, step: &str) -> TestResult {
    let outcome = client
        .wire
        .session_mut()
        .logout()
        .await
        .map_err(|error| format!("step={step};observed={error}"))?;
    expect(
        outcome.result == Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted),
        step,
        &outcome,
    )?;
    client.seal(outcome.command_id, outcome.result_server_sequence);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browse_rejects_duplicates_conflicts_and_incomplete_pages() -> TestResult {
        let correct = tf::BrowseListing {
            leader_handle: 1,
            name: "Leader".to_owned(),
            listing: leader_listing(),
            occupied_slots: 0,
            status: tf::MemberStatus::None,
        };
        let conflicting = tf::BrowseListing {
            leader_handle: 2,
            status: tf::MemberStatus::Pending,
            ..correct.clone()
        };
        let page = |listings, next| tf::TeamFinderResult::Browse { listings, next };

        assert_eq!(
            unique_listing(page(vec![correct.clone()], None), "Leader")?,
            Some(correct.clone()),
        );
        assert!(
            unique_listing(page(vec![correct.clone(), correct.clone()], None), "Leader").is_err()
        );
        assert!(unique_listing(page(vec![correct.clone(), conflicting], None), "Leader").is_err());
        assert!(unique_listing(page(vec![correct], Some(1)), "Leader").is_err());
        assert!(unique_listing(page(Vec::new(), Some(1)), "Leader").is_err());
        Ok(())
    }

    #[test]
    fn applicants_rejects_stale_member_and_wrong_leader_status() {
        let want = Some(leader_listing());
        let leader = tf::Applicant {
            handle: 1,
            name: "Leader".to_owned(),
            level: 8,
            vocation: 1,
            status: tf::MemberStatus::Accepted,
        };
        let stale = tf::Applicant {
            handle: 2,
            name: "Member".to_owned(),
            status: tf::MemberStatus::Denied,
            ..leader.clone()
        };
        let pending = tf::Applicant {
            status: tf::MemberStatus::Pending,
            ..stale.clone()
        };
        let wrong_status = tf::Applicant {
            status: tf::MemberStatus::Denied,
            ..leader.clone()
        };
        let page = |members| tf::TeamFinderResult::Applicants {
            listing: want,
            occupied_slots: 0,
            members,
            next: None,
        };

        assert!(exact_applicants(
            &page(vec![leader.clone()]),
            "Leader",
            want,
            None
        ));
        assert!(exact_applicants(
            &page(vec![leader.clone(), pending.clone()]),
            "Leader",
            want,
            Some("Member")
        ));
        assert!(!exact_applicants(
            &page(vec![stale.clone()]),
            "Leader",
            want,
            None
        ));
        assert!(!exact_applicants(
            &page(vec![wrong_status.clone()]),
            "Leader",
            want,
            None
        ));
        assert!(!exact_applicants(
            &page(vec![stale, pending.clone()]),
            "Leader",
            want,
            Some("Member")
        ));
        assert!(!exact_applicants(
            &page(vec![wrong_status, pending]),
            "Leader",
            want,
            Some("Member")
        ));
    }

    #[test]
    fn token_has_no_whitespace_and_is_bounded() {
        let value = token(&format!("step=a b\tc\n{}", "x".repeat(400)));
        assert!(!value.contains(char::is_whitespace));
        assert!(value.len() <= 240);
        assert!(value.starts_with("step=a-b-c-"));
    }
}
