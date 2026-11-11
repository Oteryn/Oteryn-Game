//! S14c (Team Finder leader accept/deny/invite), two live characters:
//! 1. The leader (character 0) advertises; the member (character 1) requests to join.
//! 2. Deny: accepted, applicant and browse show Denied. The member cancels back to no status.
//! 3. The member requests again; Invite: accepted, applicant and browse show Invited.
//! 4. Accept: the server forms the party; the accepted reply and an Accepted status are
//!    required. The party effect is then proved: the member must hold the party invitation the
//!    Accept issued, accept it, and the leader's party state must list both members. A refusal
//!    (for example no visible party target) is reported as the observed precondition after
//!    cleanup, never as a pass.
//! 5. Cleanup: both leave the party, the member cancels, the leader removes the advertisement,
//!    both log out.
use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_dev_client::client_runtime::party::party::{PartyDisposition, PartyIntent};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::team_finder::{self as leader, LeaderListing, TeamTarget};
use oteryn_protocol_oteryn::team_finder_members as tf;

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

async fn party(
    client: &mut DevClientSession,
    intent: Option<PartyIntent>,
) -> TestResult<(
    PartyDisposition,
    oteryn_dev_client::client_runtime::party::party::PartyResult,
)> {
    use oteryn_dev_client::client_runtime::party as runtime;
    let command = match intent {
        Some(intent) => {
            runtime::PartyCommand::intent(intent).map_err(|e| format!("step=party encode={e:?}"))?
        }
        None => runtime::PartyCommand::query(),
    };
    let reply = runtime::send(client.session_mut(), &command).await?;
    let operation = runtime::party::decode_party_result(&reply.operation)
        .map_err(|e| format!("step=party operation={e:?}"))?;
    let view =
        runtime::party::decode_party_result(reply.view.as_deref().ok_or("step=party view-absent")?)
            .map_err(|e| format!("step=party view={e:?}"))?;
    Ok((operation.disposition, view))
}

/// The member holds the invitation the Accept issued, accepts it, and the leader's party lists
/// both members. Returns the observation when the party transition did not happen.
async fn prove_party(
    lead: &mut DevClientSession,
    member: &mut DevClientSession,
) -> TestResult<String> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    let handle = loop {
        let (_, view) = party(member, None).await?;
        if let Some(invitation) = view.invitations.first() {
            break invitation.handle;
        }
        if tokio::time::Instant::now() >= deadline {
            return Ok("member-has-no-party-invitation".to_owned());
        }
        member.service_liveness(Duration::from_millis(100)).await?;
    };
    let (reply, _) = party(member, Some(PartyIntent::Accept(handle))).await?;
    if reply != PartyDisposition::Ok {
        return Ok(format!("invitation-accept-reply:{reply:?}"));
    }
    let (_, view) = party(lead, None).await?;
    if view.members.len() < 2 {
        return Ok(format!("leader-party-members:{}", view.members.len()));
    }
    Ok(String::new())
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

async fn leader_action(
    client: &mut DevClientSession,
    action: tf::LeaderAction,
) -> TestResult<CommandStatus> {
    let payload = tf::encode_leader_action(action)
        .map_err(|error| format!("step=encode-leader;observed={error:?}"))?;
    let reply = client
        .session_mut()
        .gated_command(
            tf::CAPABILITY_TEAM_FINDER_MEMBERS_V1,
            tf::COMMAND_TYPE_TEAM_FINDER_LEADER_ACTION,
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
) -> TestResult<(RowClient, RowClient, u64)> {
    let mut lead = row_connect_character(clients, tags.0, 0).await?;
    let mut member = row_connect_world(clients, tags.1).await?;
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

async fn journey(clients: &SeamClients<'_>) -> TestResult<Verdict> {
    let want = leader_listing();
    let (mut lead, mut member, handle) = pair(clients, (0xe4, 0xe5), want).await?;

    let status = member_action(&mut member, tf::MemberAction::Request { leader: handle }).await?;
    expect(status == CommandStatus::Accepted, "request", status)?;
    let waiting = applicants(&mut lead).await?;
    let applicant = waiting
        .iter()
        .find(|entry| entry.status == tf::MemberStatus::Pending)
        .map(|entry| entry.handle)
        .ok_or("step=request;observed=no-pending-applicant")?;

    let status = leader_action(&mut lead, tf::LeaderAction::Deny { member: applicant }).await?;
    expect(status == CommandStatus::Accepted, "deny", status)?;
    let waiting = applicants(&mut lead).await?;
    expect(
        waiting
            .iter()
            .any(|entry| entry.status == tf::MemberStatus::Denied),
        "deny-applicant",
        waiting.len(),
    )?;
    let (_, state) = browse(&mut member, &want)
        .await?
        .ok_or("step=deny;observed=listing-vanished")?;
    expect(state == tf::MemberStatus::Denied, "deny-status", state)?;

    let status = member_action(&mut member, tf::MemberAction::Cancel { leader: handle }).await?;
    expect(
        status == CommandStatus::Accepted,
        "cancel-after-deny",
        status,
    )?;
    let (_, state) = browse(&mut member, &want)
        .await?
        .ok_or("step=cancel;observed=listing-vanished")?;
    expect(
        state == tf::MemberStatus::None,
        "cancel-after-deny-status",
        state,
    )?;

    let status = member_action(&mut member, tf::MemberAction::Request { leader: handle }).await?;
    expect(status == CommandStatus::Accepted, "re-request", status)?;
    let waiting = applicants(&mut lead).await?;
    let applicant = waiting
        .iter()
        .find(|entry| entry.status == tf::MemberStatus::Pending)
        .map(|entry| entry.handle)
        .ok_or("step=re-request;observed=no-pending-applicant")?;
    let status = leader_action(&mut lead, tf::LeaderAction::Invite { member: applicant }).await?;
    expect(status == CommandStatus::Accepted, "invite", status)?;
    let waiting = applicants(&mut lead).await?;
    expect(
        waiting
            .iter()
            .any(|entry| entry.status == tf::MemberStatus::Invited),
        "invite-applicant",
        waiting.len(),
    )?;
    let (_, state) = browse(&mut member, &want)
        .await?
        .ok_or("step=invite;observed=listing-vanished")?;
    expect(state == tf::MemberStatus::Invited, "invite-status", state)?;

    let accepted = leader_action(&mut lead, tf::LeaderAction::Accept { member: applicant }).await?;
    let waiting = applicants(&mut lead).await?;
    let reached = waiting
        .iter()
        .any(|entry| entry.status == tf::MemberStatus::Accepted);

    let party_gap = if accepted == CommandStatus::Accepted && reached {
        prove_party(&mut lead, &mut member).await?
    } else {
        "accept-not-reached".to_owned()
    };

    // Cleanup runs whatever Accept answered.
    party(&mut member, Some(PartyIntent::Leave)).await.ok();
    party(&mut lead, Some(PartyIntent::Leave)).await.ok();
    member_action(&mut member, tf::MemberAction::Cancel { leader: handle }).await?;
    let status = remove(&mut lead).await?;
    expect(status == CommandStatus::Accepted, "leader-remove", status)?;
    logout(&mut member).await?;
    logout(&mut lead).await?;
    if accepted == CommandStatus::Accepted && reached && party_gap.is_empty() {
        Ok((
            "PASS",
            "deny-invite-accept-and-party-formed-validated".to_owned(),
        ))
    } else {
        Ok((
            "FAIL",
            format!(
                "step=accept;observed=reply:{accepted:?},applicant-accepted:{reached},party:{party_gap};needed=party-invitation-target-visible-with-free-slots"
            ),
        ))
    }
}

type Verdict = (&'static str, String);

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = row_complete_with_timeout(Duration::from_secs(240), journey(clients)).await;
    let (status, detail) = match result {
        Ok(Ok((status, detail))) => (status, token(&detail)),
        Ok(Err(error)) => ("FAIL", token(&error.to_string())),
        Err(_) => ("FAIL", "step=journey;observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=S14c.full status={status} seconds={:.3} evidence={detail}",
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

async fn logout(client: &mut RowClient) -> TestResult {
    let reply = client.row_logout().await?;
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
