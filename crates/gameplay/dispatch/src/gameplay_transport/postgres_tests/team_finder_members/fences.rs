//! Foreign-target fencing through retained wire handles and independent current claims.
use super::support::Peer;
use crate::bestiary_postgres_harness::{Harness, TestResult, debug};
use crate::foundation::GameSessionId;
use oteryn_protocol_oteryn::{
    CommandStatus,
    team_finder_members::{self as wire, MemberAction, TeamFinderQuery, TeamFinderResult},
};

async fn members(h: &Harness) -> TestResult<Vec<(Vec<u8>, Vec<u8>, i16)>> {
    Ok(sqlx::query_as("SELECT uuid_send(leader_character_id),uuid_send(member_character_id),status FROM game_team_finder_members ORDER BY leader_character_id,member_character_id")
        .fetch_all(&h.pool).await?)
}

async fn party_counts(h: &Harness) -> TestResult<(i64, i64)> {
    Ok((
        h.count("game_party_members").await?,
        h.count("game_party_invitations").await?,
    ))
}

async fn rejected_request(h: &Harness, applicant: &mut Peer, handle: u64) -> TestResult {
    let payload =
        wire::encode_member_action(MemberAction::Request { leader: handle }).map_err(debug)?;
    let command = applicant
        .action(
            wire::COMMAND_TYPE_TEAM_FINDER_MEMBER_ACTION,
            &payload,
            CommandStatus::Rejected,
        )
        .await?;
    // A durable refusal proves the retained handle reached the SQL owner. An
    // unknown connection-local handle would have no member receipt at all.
    let accepted: bool = sqlx::query_scalar("SELECT accepted FROM game_team_finder_member_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric(20,0)")
        .bind(applicant.admitted.game_session_id.as_bytes().as_slice())
        .bind(command.to_string()).fetch_one(&h.pool).await?;
    assert!(!accepted);
    Ok(())
}

async fn hidden_from_browse(applicant: &mut Peer) -> TestResult {
    assert_eq!(
        applicant
            .query(TeamFinderQuery::Browse { after: None })
            .await?,
        TeamFinderResult::Browse {
            listings: Vec::new(),
            next: None
        },
    );
    Ok(())
}

pub(super) async fn foreign_leader_fence(
    h: &Harness,
    applicant: &mut Peer,
    leader_session: GameSessionId,
    leader_character: [u8; 16],
    leader_handle: u64,
) -> TestResult {
    let before_members = members(h).await?;
    assert_eq!(before_members.len(), 1);
    assert_eq!(before_members[0].2, 1);
    let before_party = party_counts(h).await?;
    assert_eq!(h.count("game_team_finder_active_listings").await?, 1);
    let (claim, lease, generation): (String, String, String) = sqlx::query_as("SELECT c.lease_generation::text,s.character_lease_generation::text,s.current_generation::text FROM game_durability_admission_character_guards c JOIN game_durability_reconnect_sessions s ON s.game_session_id=c.holder_game_session_id WHERE c.character_id=encode($1,'hex')::uuid AND s.game_session_id=encode($2,'hex')::uuid AND s.session_state=2")
        .bind(leader_character.as_slice())
        .bind(leader_session.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(claim, lease);
    let displaced = claim
        .parse::<u64>()?
        .checked_add(1)
        .ok_or("fixture lease exhausted")?;

    // The existing party_xp fixture uses this exact independent claim mutation.
    // The retained session, advertisement, actor and opaque handle stay unchanged.
    let changed = sqlx::query("UPDATE game_durability_admission_character_guards SET lease_generation=$3::text::numeric(20,0) WHERE character_id=encode($1,'hex')::uuid AND holder_game_session_id=encode($2,'hex')::uuid AND lease_generation=$4::text::numeric(20,0)")
        .bind(leader_character.as_slice())
        .bind(leader_session.as_bytes().as_slice())
        .bind(displaced.to_string()).bind(&claim).execute(&h.pool).await?;
    assert_eq!(changed.rows_affected(), 1);
    rejected_request(h, applicant, leader_handle).await?;
    assert_eq!(members(h).await?, before_members);
    assert_eq!(party_counts(h).await?, before_party);
    assert_eq!(h.count("game_team_finder_active_listings").await?, 1);
    hidden_from_browse(applicant).await?;

    // Restore only the claim this helper displaced before exercising another
    // fence. Neither publication history nor any Team Finder row is fabricated.
    let restored = sqlx::query("UPDATE game_durability_admission_character_guards SET lease_generation=$3::text::numeric(20,0) WHERE character_id=encode($1,'hex')::uuid AND holder_game_session_id=encode($2,'hex')::uuid AND lease_generation=$4::text::numeric(20,0)")
        .bind(leader_character.as_slice())
        .bind(leader_session.as_bytes().as_slice())
        .bind(&claim).bind(displaced.to_string()).execute(&h.pool).await?;
    assert_eq!(restored.rows_affected(), 1);
    let TeamFinderResult::Browse { listings, next } = applicant
        .query(TeamFinderQuery::Browse { after: None })
        .await?
    else {
        return Err("expected restored leader browse".into());
    };
    assert_eq!(next, None);
    assert_eq!(listings.len(), 1);
    let handle = listings[0].leader_handle;

    // A successor connection generation must retire the advertisement in the
    // same session-table transaction. Keep the old handle cached until its
    // request reaches the owner; querying first would only test handle eviction.
    let successor = generation
        .parse::<u64>()?
        .checked_add(1)
        .ok_or("fixture generation exhausted")?;
    let changed = sqlx::query("UPDATE game_durability_reconnect_sessions SET current_generation=$3::text::numeric(20,0) WHERE game_session_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND current_generation=$4::text::numeric(20,0) AND session_state=2")
        .bind(leader_session.as_bytes().as_slice())
        .bind(leader_character.as_slice())
        .bind(successor.to_string()).bind(&generation).execute(&h.pool).await?;
    assert_eq!(changed.rows_affected(), 1);
    assert_eq!(h.count("game_team_finder_leader_listings").await?, 1);
    assert_eq!(h.count("game_team_finder_active_listings").await?, 0);
    assert!(members(h).await?.is_empty());
    rejected_request(h, applicant, handle).await?;
    hidden_from_browse(applicant).await?;
    assert_eq!(h.count("game_team_finder_active_listings").await?, 0);
    assert!(members(h).await?.is_empty());
    assert_eq!(party_counts(h).await?, before_party);
    Ok(())
}
