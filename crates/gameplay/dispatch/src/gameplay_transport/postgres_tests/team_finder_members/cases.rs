use super::{Case, support::Peer};
use crate::bestiary_postgres_harness::{Harness, TestResult, debug};
use crate::gameplay_transport::{
    ComposedFreshAdmission, connection::FreshAdmissionAuthority,
    team_finder_members::SessionTeamFinderView,
};
use oteryn_protocol_oteryn::{
    CommandStatus,
    logout::{COMMAND_TYPE_LOGOUT_INTENT, LogoutResult, decode_logout_result},
    party::{self, PartyDisposition, PartyIntent},
    team_finder::{self as leader, LeaderListing, TeamTarget},
    team_finder_members::{
        self as wire, Applicant, BrowseListing, LeaderAction, MemberAction, MemberStatus,
        TeamFinderQuery, TeamFinderResult,
    },
};

const LISTING: LeaderListing = LeaderListing {
    min_level: 8,
    max_level: 120,
    vocation_mask: 15,
    slots: 4,
    free_slots: 4,
    party_members_join: false,
    timestamp: 1_700_000_000,
    target: TeamTarget::Boss(1234),
};

async fn browse(peer: &mut Peer) -> TestResult<Vec<BrowseListing>> {
    let TeamFinderResult::Browse { listings, next } =
        peer.query(TeamFinderQuery::Browse { after: None }).await?
    else {
        return Err("expected Browse result".into());
    };
    assert_eq!(next, None);
    Ok(listings)
}

async fn applicants(peer: &mut Peer) -> TestResult<(LeaderListing, u16, Vec<Applicant>)> {
    let TeamFinderResult::Applicants {
        listing,
        occupied_slots,
        members,
        next,
    } = peer
        .query(TeamFinderQuery::Applicants { after: None })
        .await?
    else {
        return Err("expected Applicants result".into());
    };
    assert_eq!(next, None);
    Ok((
        listing.ok_or("active leader listing")?,
        occupied_slots,
        members,
    ))
}

async fn member_status(
    h: &Harness,
    leader_id: &[u8; 16],
    member_id: &[u8; 16],
) -> TestResult<Option<i16>> {
    Ok(sqlx::query_scalar("SELECT status FROM game_team_finder_members WHERE leader_character_id=encode($1,'hex')::uuid AND member_character_id=encode($2,'hex')::uuid")
        .bind(leader_id.as_slice()).bind(member_id.as_slice()).fetch_optional(&h.pool).await?)
}

async fn member_action(
    peer: &mut Peer,
    action: MemberAction,
    status: CommandStatus,
) -> TestResult<u64> {
    peer.action(
        wire::COMMAND_TYPE_TEAM_FINDER_MEMBER_ACTION,
        &wire::encode_member_action(action).map_err(debug)?,
        status,
    )
    .await
}

async fn leader_action(
    peer: &mut Peer,
    action: LeaderAction,
    status: CommandStatus,
) -> TestResult<u64> {
    peer.action(
        wire::COMMAND_TYPE_TEAM_FINDER_LEADER_ACTION,
        &wire::encode_leader_action(action).map_err(debug)?,
        status,
    )
    .await
}

async fn advertise(peer: &mut Peer, listing: LeaderListing) -> TestResult {
    peer.action(
        leader::COMMAND_TYPE_TEAM_FINDER_LEADER_UPSERT,
        &leader::encode_upsert(listing).map_err(debug)?,
        CommandStatus::Accepted,
    )
    .await?;
    Ok(())
}

async fn another_request(
    other_leader: &mut Peer,
    applicant: &mut Peer,
    expected: CommandStatus,
) -> TestResult {
    advertise(
        other_leader,
        LeaderListing {
            target: TeamTarget::Quest(7),
            ..LISTING
        },
    )
    .await?;
    let second = browse(applicant)
        .await?
        .into_iter()
        .find(|row| row.listing.target == TeamTarget::Quest(7))
        .ok_or("second listing")?;
    member_action(
        applicant,
        MemberAction::Request {
            leader: second.leader_handle,
        },
        expected,
    )
    .await?;
    Ok(())
}

async fn durable_replay(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    applicant: &Peer,
    request_id: u64,
    request_payload: &[u8],
    handle: u64,
) -> TestResult {
    let changed =
        wire::encode_member_action(MemberAction::Cancel { leader: handle }).map_err(debug)?;
    replay_result(
        h,
        owner,
        applicant,
        request_id,
        wire::COMMAND_TYPE_TEAM_FINDER_MEMBER_ACTION,
        request_payload,
        &changed,
    )
    .await
}

async fn replay_result(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    peer: &Peer,
    command_id: u64,
    command_type: u32,
    original: &[u8],
    changed: &[u8],
) -> TestResult {
    let before = h.count("game_team_finder_member_receipts").await?;
    // Fresh connection-local view has no handle mapping. Durable replay must be
    // answered before resolving the original now-unknown opaque handle.
    let mut view = SessionTeamFinderView::default();
    assert_eq!(
        owner
            .team_finder_members_command(
                peer.admitted,
                command_id,
                command_type,
                original,
                &mut view,
            )
            .await,
        Some((true, Vec::new()))
    );
    assert_eq!(
        owner
            .team_finder_members_command(
                peer.admitted,
                command_id,
                command_type,
                changed,
                &mut view,
            )
            .await,
        Some((false, Vec::new()))
    );
    assert_eq!(h.count("game_team_finder_member_receipts").await?, before);
    Ok(())
}

pub(super) async fn drive(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    characters: &[[u8; 16]],
    case: Case,
    [mut leader_peer, mut member_peer, mut other_peer]: [Peer; 3],
) -> TestResult {
    leader_peer.snapshot().await?;
    member_peer.snapshot().await?;
    other_peer.snapshot().await?;
    let listing = if case == Case::Accept {
        // A one-slot advertisement makes the first accepted member exhaust it.
        LeaderListing {
            slots: 1,
            free_slots: 1,
            ..LISTING
        }
    } else {
        LISTING
    };
    advertise(&mut leader_peer, listing).await?;
    assert_eq!(h.count("game_team_finder_active_listings").await?, 1);
    let listings = browse(&mut member_peer).await?;
    assert_eq!(listings.len(), 1);
    assert_eq!(listings[0].listing, listing);
    assert_eq!(listings[0].occupied_slots, 0);
    assert_eq!(listings[0].status, MemberStatus::None);
    let handle = listings[0].leader_handle;
    let request_payload =
        wire::encode_member_action(MemberAction::Request { leader: handle }).map_err(debug)?;
    let request_id = member_peer
        .action(
            wire::COMMAND_TYPE_TEAM_FINDER_MEMBER_ACTION,
            &request_payload,
            CommandStatus::Accepted,
        )
        .await?;
    assert_eq!(
        member_status(h, &characters[0], &characters[1]).await?,
        Some(1)
    );
    let (stored_listing, occupied, members) = applicants(&mut leader_peer).await?;
    assert_eq!(stored_listing, listing);
    assert_eq!(occupied, 0);
    assert_eq!(members.len(), 2);
    assert_eq!(
        members[0].status,
        MemberStatus::Accepted,
        "leader is the first applicant row"
    );
    let member = members
        .iter()
        .find(|member| member.status == MemberStatus::Pending)
        .ok_or("pending applicant")?;
    assert!(!member.name.is_empty());
    assert!(member.level > 0);
    let member_handle = member.handle;

    match case {
        Case::Request => {
            let count = h.count("game_team_finder_member_receipts").await?;
            let (status, bytes) = member_peer
                .replay(
                    request_id,
                    wire::COMMAND_TYPE_TEAM_FINDER_MEMBER_ACTION,
                    &request_payload,
                )
                .await?;
            assert_eq!(status, CommandStatus::DuplicateReplay);
            assert!(bytes.is_empty());
            assert_eq!(h.count("game_team_finder_member_receipts").await?, count);
            assert_eq!(
                browse(&mut member_peer).await?[0].status,
                MemberStatus::Pending
            );
            durable_replay(h, owner, &member_peer, request_id, &request_payload, handle).await?;
            another_request(&mut other_peer, &mut member_peer, CommandStatus::Rejected).await?;
            assert_eq!(
                member_status(h, &characters[2], &characters[1]).await?,
                None
            );
            assert_eq!(h.count("game_team_finder_members").await?, 1);
        }
        Case::Cancel => {
            let payload = wire::encode_member_action(MemberAction::Cancel { leader: handle })
                .map_err(debug)?;
            let cancel_id = member_peer
                .action(
                    wire::COMMAND_TYPE_TEAM_FINDER_MEMBER_ACTION,
                    &payload,
                    CommandStatus::Accepted,
                )
                .await?;
            assert_eq!(
                member_status(h, &characters[0], &characters[1]).await?,
                None
            );
            let (status, _) = member_peer
                .replay(
                    cancel_id,
                    wire::COMMAND_TYPE_TEAM_FINDER_MEMBER_ACTION,
                    &payload,
                )
                .await?;
            assert_eq!(status, CommandStatus::DuplicateReplay);
            replay_result(
                h,
                owner,
                &member_peer,
                cancel_id,
                wire::COMMAND_TYPE_TEAM_FINDER_MEMBER_ACTION,
                &payload,
                &request_payload,
            )
            .await?;
            assert_eq!(
                browse(&mut member_peer).await?[0].status,
                MemberStatus::None
            );
            durable_replay(h, owner, &member_peer, request_id, &request_payload, handle).await?;
            assert_eq!(
                h.count("game_team_finder_members").await?,
                0,
                "replayed old request never resurrects a cancelled entry"
            );
            let refreshed = browse(&mut member_peer).await?[0].leader_handle;
            member_action(
                &mut member_peer,
                MemberAction::Cancel { leader: refreshed },
                CommandStatus::Accepted,
            )
            .await?;
            member_action(
                &mut member_peer,
                MemberAction::Request { leader: refreshed },
                CommandStatus::Accepted,
            )
            .await?;
            assert_eq!(
                member_status(h, &characters[0], &characters[1]).await?,
                Some(1)
            );
        }
        Case::Invite | Case::Deny => {
            let (action, expected) = if case == Case::Invite {
                (
                    LeaderAction::Invite {
                        member: member_handle,
                    },
                    2,
                )
            } else {
                (
                    LeaderAction::Deny {
                        member: member_handle,
                    },
                    4,
                )
            };
            let decision_id =
                leader_action(&mut leader_peer, action, CommandStatus::Accepted).await?;
            assert_eq!(
                member_status(h, &characters[0], &characters[1]).await?,
                Some(expected)
            );
            let original = wire::encode_leader_action(action).map_err(debug)?;
            let changed = wire::encode_leader_action(if case == Case::Invite {
                LeaderAction::Deny {
                    member: member_handle,
                }
            } else {
                LeaderAction::Invite {
                    member: member_handle,
                }
            })
            .map_err(debug)?;
            replay_result(
                h,
                owner,
                &leader_peer,
                decision_id,
                wire::COMMAND_TYPE_TEAM_FINDER_LEADER_ACTION,
                &original,
                &changed,
            )
            .await?;
            assert_eq!(h.count("game_party_invitations").await?, 0);
            assert_eq!(h.count("game_party_members").await?, 0);
            let row = browse(&mut member_peer).await?.remove(0);
            assert_eq!(row.status as i16, expected);
            // C++ map::insert preserves an existing invited/denied status.
            member_action(
                &mut member_peer,
                MemberAction::Request {
                    leader: row.leader_handle,
                },
                CommandStatus::Accepted,
            )
            .await?;
            assert_eq!(
                member_status(h, &characters[0], &characters[1]).await?,
                Some(expected)
            );
            another_request(&mut other_peer, &mut member_peer, CommandStatus::Accepted).await?;
            assert_eq!(
                member_status(h, &characters[2], &characters[1]).await?,
                Some(1),
                "only status 1 holds the pending slot"
            );
        }
        Case::Accept => {
            let payload = wire::encode_leader_action(LeaderAction::Accept {
                member: member_handle,
            })
            .map_err(debug)?;
            let accepted_id = leader_peer
                .action(
                    wire::COMMAND_TYPE_TEAM_FINDER_LEADER_ACTION,
                    &payload,
                    CommandStatus::Accepted,
                )
                .await?;
            assert_eq!(
                member_status(h, &characters[0], &characters[1]).await?,
                Some(3)
            );
            assert_eq!(h.count("game_party_invitations").await?, 1);
            assert_eq!(
                h.count("game_party_members").await?,
                1,
                "leader exists, applicant has not accepted the ordinary invite"
            );
            let (status, _) = leader_peer
                .replay(
                    accepted_id,
                    wire::COMMAND_TYPE_TEAM_FINDER_LEADER_ACTION,
                    &payload,
                )
                .await?;
            assert_eq!(status, CommandStatus::DuplicateReplay);
            let changed = wire::encode_leader_action(LeaderAction::Deny {
                member: member_handle,
            })
            .map_err(debug)?;
            replay_result(
                h,
                owner,
                &leader_peer,
                accepted_id,
                wire::COMMAND_TYPE_TEAM_FINDER_LEADER_ACTION,
                &payload,
                &changed,
            )
            .await?;
            assert_eq!(h.count("game_party_invitations").await?, 1);
            assert_eq!(h.count("game_party_members").await?, 1);
            let (baseline, occupied, _) = applicants(&mut leader_peer).await?;
            assert_eq!(
                baseline.free_slots, 1,
                "do not overwrite the leader's authoring baseline"
            );
            assert_eq!(occupied, 1);
            let (_, status, bytes) = member_peer
                .command(party::COMMAND_TYPE_PARTY_QUERY, &[])
                .await?;
            assert_eq!(status, CommandStatus::Accepted);
            let party = party::decode_party_result(&bytes).map_err(debug)?;
            assert!(party.members.is_empty());
            assert_eq!(party.invitations.len(), 1);
            let accept =
                party::encode_party_intent(PartyIntent::Accept(party.invitations[0].handle))
                    .map_err(debug)?;
            let (_, status, bytes) = member_peer
                .command(party::COMMAND_TYPE_PARTY_INTENT, &accept)
                .await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(
                party::decode_party_result(&bytes)
                    .map_err(debug)?
                    .disposition,
                PartyDisposition::Ok
            );
            assert_eq!(h.count("game_party_members").await?, 2);
            assert_eq!(h.count("game_party_invitations").await?, 0);
            let other_handle = browse(&mut other_peer).await?[0].leader_handle;
            member_action(
                &mut other_peer,
                MemberAction::Request {
                    leader: other_handle,
                },
                CommandStatus::Accepted,
            )
            .await?;
            let (_, _, members) = applicants(&mut leader_peer).await?;
            let pending = members
                .iter()
                .find(|member| member.status == MemberStatus::Pending)
                .ok_or("second pending applicant")?
                .handle;
            let refused_payload =
                wire::encode_leader_action(LeaderAction::Accept { member: pending })
                    .map_err(debug)?;
            let refused_id = leader_peer
                .action(
                    wire::COMMAND_TYPE_TEAM_FINDER_LEADER_ACTION,
                    &refused_payload,
                    CommandStatus::Rejected,
                )
                .await?;
            let recorded: bool = sqlx::query_scalar("SELECT accepted FROM game_team_finder_member_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric(20,0)")
                .bind(leader_peer.admitted.game_session_id.as_bytes().as_slice()).bind(refused_id.to_string()).fetch_one(&h.pool).await?;
            assert!(!recorded, "capacity refusal is a durable outcome too");
            assert_eq!(
                member_status(h, &characters[0], &characters[2]).await?,
                Some(1)
            );
            assert_eq!(h.count("game_party_members").await?, 2);
            assert_eq!(
                h.count("game_party_invitations").await?,
                0,
                "full Team Finder cannot issue a Party invitation"
            );
            // Team Finder cancellation does not replace ordinary Party Leave.
            let handle = browse(&mut member_peer).await?[0].leader_handle;
            member_action(
                &mut member_peer,
                MemberAction::Cancel { leader: handle },
                CommandStatus::Accepted,
            )
            .await?;
            assert_eq!(
                member_status(h, &characters[0], &characters[1]).await?,
                None
            );
            assert_eq!(h.count("game_party_members").await?, 2);
            // Crystal3019-3029: the authoring party flag imports actual Party members.
            let importing = LeaderListing {
                party_members_join: true,
                ..listing
            };
            advertise(&mut leader_peer, importing).await?;
            assert_eq!(
                member_status(h, &characters[0], &characters[1]).await?,
                Some(3)
            );
            assert_eq!(
                member_status(h, &characters[0], &characters[2]).await?,
                Some(1)
            );
            let (baseline, occupied, members) = applicants(&mut leader_peer).await?;
            assert_eq!(baseline, importing);
            assert_eq!(occupied, 1);
            assert_eq!(members.len(), 3);
            let imported = members
                .iter()
                .skip(1)
                .find(|member| member.status == MemberStatus::Accepted)
                .ok_or("imported Party member")?
                .handle;
            advertise(&mut leader_peer, importing).await?;
            assert_eq!(
                h.count("game_team_finder_members").await?,
                2,
                "repeated Party import cannot duplicate entries"
            );
            leader_action(
                &mut leader_peer,
                LeaderAction::Deny { member: imported },
                CommandStatus::Accepted,
            )
            .await?;
            advertise(&mut leader_peer, importing).await?;
            assert_eq!(
                member_status(h, &characters[0], &characters[1]).await?,
                Some(4),
                "C++ map::insert does not overwrite an existing denied member"
            );
            assert_eq!(h.count("game_party_members").await?, 2);
            assert_eq!(h.count("game_party_invitations").await?, 0);
            let mut reset_view = SessionTeamFinderView::default();
            assert_eq!(
                owner
                    .team_finder_members_command(
                        leader_peer.admitted,
                        refused_id,
                        wire::COMMAND_TYPE_TEAM_FINDER_LEADER_ACTION,
                        &refused_payload,
                        &mut reset_view,
                    )
                    .await,
                Some((false, Vec::new())),
                "a retained capacity refusal stays refused after capacity becomes free"
            );
            assert_eq!(
                member_status(h, &characters[0], &characters[2]).await?,
                Some(1)
            );
            assert_eq!(h.count("game_party_invitations").await?, 0);
        }
        Case::Remove => {
            leader_peer
                .action(
                    leader::COMMAND_TYPE_TEAM_FINDER_LEADER_REMOVE,
                    &[],
                    CommandStatus::Accepted,
                )
                .await?;
            assert_eq!(h.count("game_team_finder_leader_listings").await?, 0);
            assert_eq!(h.count("game_team_finder_active_listings").await?, 0);
            assert_eq!(h.count("game_team_finder_members").await?, 0);
            member_action(
                &mut member_peer,
                MemberAction::Cancel { leader: handle },
                CommandStatus::Rejected,
            )
            .await?;
            durable_replay(h, owner, &member_peer, request_id, &request_payload, handle).await?;
            assert_eq!(h.count("game_team_finder_members").await?, 0);
            assert!(browse(&mut member_peer).await?.is_empty());
        }
        Case::Logout => {
            let fence = owner
                .current_quest_fence(leader_peer.admitted.game_session_id)
                .await
                .map_err(|error| format!("logout preflight current gameplay fence: {error:?}"))?
                .ok_or("logout preflight session unexpectedly terminal")?;
            owner
                .root
                .read_pvp_state(owner.character, owner.holder, fence)
                .await
                .map_err(|error| format!("logout preflight current PvP owner read: {error:?}"))?;
            eprintln!("S14BC tf_logout stage=wire_logout");
            let (_, status, bytes) = leader_peer.command(COMMAND_TYPE_LOGOUT_INTENT, &[]).await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(
                decode_logout_result(&bytes).map_err(debug)?,
                LogoutResult::Accepted
            );
            let terminal: i16 = sqlx::query_scalar("SELECT session_state FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid")
                .bind(leader_peer.admitted.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(terminal, 3);
            assert_eq!(
                h.count("game_team_finder_leader_listings").await?,
                1,
                "Q3B preserves only the template"
            );
            assert_eq!(h.count("game_team_finder_active_listings").await?, 0);
            assert_eq!(h.count("game_team_finder_members").await?, 0);
            assert!(browse(&mut member_peer).await?.is_empty());
            durable_replay(h, owner, &member_peer, request_id, &request_payload, handle).await?;
            assert_eq!(h.count("game_team_finder_active_listings").await?, 0);
            assert_eq!(h.count("game_team_finder_members").await?, 0);
        }
        Case::Refusals => {
            member_peer
                .action(
                    wire::COMMAND_TYPE_TEAM_FINDER_MEMBER_ACTION,
                    &[8, 0],
                    CommandStatus::Rejected,
                )
                .await?;
            member_action(
                &mut member_peer,
                MemberAction::Request { leader: u64::MAX },
                CommandStatus::Rejected,
            )
            .await?;
            leader_action(
                &mut other_peer,
                LeaderAction::Deny {
                    member: member_handle,
                },
                CommandStatus::Rejected,
            )
            .await?;
            member_action(
                &mut leader_peer,
                MemberAction::Request {
                    leader: member_handle,
                },
                CommandStatus::Rejected,
            )
            .await?;
            let unknown_cursor = wire::encode_query(TeamFinderQuery::Browse {
                after: Some(u64::MAX),
            })
            .map_err(debug)?;
            member_peer
                .action(
                    wire::COMMAND_TYPE_TEAM_FINDER_QUERY,
                    &unknown_cursor,
                    CommandStatus::Rejected,
                )
                .await?;
            let mut stale = member_peer.admitted;
            let fence = stale.item_fence.as_mut().ok_or("item fence")?;
            fence.character_lease_generation = fence
                .character_lease_generation
                .checked_add(1)
                .ok_or("lease generation")?;
            let mut view = SessionTeamFinderView::default();
            let query =
                wire::encode_query(TeamFinderQuery::Browse { after: None }).map_err(debug)?;
            assert_eq!(
                owner
                    .team_finder_members_command(
                        stale,
                        900,
                        wire::COMMAND_TYPE_TEAM_FINDER_QUERY,
                        &query,
                        &mut view
                    )
                    .await,
                Some((false, Vec::new()))
            );
            assert_eq!(
                member_status(h, &characters[0], &characters[1]).await?,
                Some(1)
            );
            assert_eq!(h.count("game_team_finder_members").await?, 1);
            let privileges: (bool, bool, bool, bool, bool) = sqlx::query_as("SELECT has_table_privilege('public','game_team_finder_members','SELECT'),has_table_privilege('oteryn_game_runtime','game_team_finder_members','DELETE'),has_table_privilege('oteryn_game_control','game_team_finder_members','INSERT'),has_table_privilege('oteryn_game_runtime','game_team_finder_member_receipts','UPDATE'),has_function_privilege('public','game_team_finder_session_ended()','EXECUTE')")
                .fetch_one(&h.pool).await?;
            assert_eq!(privileges, (false, true, false, false, false));
            super::fences::foreign_leader_fence(
                h,
                &mut member_peer,
                leader_peer.admitted.game_session_id,
                characters[0],
                handle,
            )
            .await?;
        }
    }
    if case != Case::Logout {
        leader_peer.shutdown().await?;
    }
    member_peer.shutdown().await?;
    other_peer.shutdown().await?;
    Ok(())
}
