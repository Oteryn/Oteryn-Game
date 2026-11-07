//! Team Finder source owner: durable S14a template, session advertisement and member map.
//! Crystal 201189f protocolgame.cpp:2823-3030,3060-3157; game.cpp:1422-1426.
//! PR #2767 answers Q1B/Q2C/Q3B/Q4A/Q5A define the explicit donor deviations.
use super::admission_journal::party_target_binding::QualifiedPartyVisibleTarget;
use super::spell_item_transaction::{SpellItemAuthority, SpellItemError, check_transaction};
use super::world_party;
use crate::foundation::{GameSessionId, PartyAction, PartySnapshot, RuntimeScopeRefV1};
use oteryn_protocol_oteryn::team_finder::{LeaderListing, TeamTarget, encode_upsert};
use oteryn_protocol_oteryn::team_finder_members::MemberStatus;
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction, postgres::PgRow};

type Result<T> = std::result::Result<T, SpellItemError>;
type Scope = ([u8; 16], [u8; 16], u64);

/// A session-scoped handle resolves to these DATA, never to current authority.
/// Every mutation independently compares them with the locked canonical session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Target {
    pub character: [u8; 16],
    pub session: GameSessionId,
    pub connection_generation: u64,
}

#[derive(Debug)]
pub struct ListingRow {
    pub leader_target: Target,
    pub name: String,
    pub listing: LeaderListing,
    pub occupied_slots: u16,
    pub status: MemberStatus,
}

#[derive(Debug)]
pub struct ApplicantRow {
    pub target: Target,
    pub name: String,
    pub level: u16,
    pub vocation: u8,
    pub status: MemberStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MutationOutcome {
    pub accepted: bool,
    pub replayed: bool,
    /// Only a newly committed command may emit its donor notification.
    pub notify_session: Option<GameSessionId>,
}

fn reject(reason: &'static str) -> SpellItemError {
    SpellItemError::Rejected(reason)
}

fn uuid(value: Vec<u8>) -> Result<[u8; 16]> {
    value.try_into().map_err(|_| reject("team finder UUID"))
}

fn session(value: Vec<u8>) -> Result<GameSessionId> {
    GameSessionId::decode(&uuid(value)?).map_err(|_| reject("team finder session UUID"))
}

fn status(value: i16) -> Result<MemberStatus> {
    u8::try_from(value)
        .ok()
        .and_then(|value| MemberStatus::try_from(value).ok())
        .ok_or(reject("team finder member status"))
}

fn target(row: &PgRow) -> Result<Target> {
    Ok(Target {
        character: uuid(row.try_get("character")?)?,
        session: session(row.try_get("session")?)?,
        connection_generation: row
            .try_get::<String, _>("generation")?
            .parse()
            .map_err(|_| reject("team finder connection generation"))?,
    })
}

fn stored_listing(row: &PgRow) -> Result<LeaderListing> {
    let number = |key| -> Result<u16> {
        u16::try_from(row.try_get::<i32, _>(key)?).map_err(|_| reject("team finder listing number"))
    };
    let first = number("target_id")?;
    let target = match row.try_get::<i16, _>("team_type")? {
        1 => TeamTarget::Boss(first),
        2 => TeamTarget::Hunt {
            kind: first,
            area: number("hunt_area")?,
        },
        3 => TeamTarget::Quest(first),
        _ => return Err(reject("team finder target kind")),
    };
    Ok(LeaderListing {
        min_level: number("min_level")?,
        max_level: number("max_level")?,
        vocation_mask: u8::try_from(row.try_get::<i16, _>("vocation_mask")?)
            .map_err(|_| reject("team finder vocation mask"))?,
        slots: number("team_slots")?,
        free_slots: number("free_slots")?,
        party_members_join: row.try_get("party_members_join")?,
        timestamp: u32::try_from(row.try_get::<i64, _>("client_timestamp")?)
            .map_err(|_| reject("team finder timestamp"))?,
        target,
    })
}

/// Donor occupancy, including the leader only through its authored baseline.
/// Q5A adds saturating free capacity and capacity refusal on a new acceptance.
fn occupied_slots(listing: LeaderListing, accepted: u64) -> u16 {
    u16::try_from(
        u64::from(listing.slots - listing.free_slots)
            .max(accepted)
            .min(u64::from(u16::MAX)),
    )
    .unwrap_or(u16::MAX)
}

fn effective_listing(mut listing: LeaderListing, accepted: u64) -> (LeaderListing, u16) {
    let occupied = occupied_slots(listing, accepted);
    listing.free_slots = listing.slots.saturating_sub(occupied);
    (listing, occupied)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LeaderDecision {
    Refused,
    StatusOnly,
    PartyInvitation,
}

fn leader_decision(
    previous: Option<MemberStatus>,
    desired: MemberStatus,
    free_slots: u16,
    already_in_leaders_party: bool,
) -> LeaderDecision {
    if matches!(previous, None | Some(MemberStatus::None)) {
        return LeaderDecision::Refused;
    }
    match desired {
        MemberStatus::Invited | MemberStatus::Denied => LeaderDecision::StatusOnly,
        MemberStatus::Accepted if previous == Some(MemberStatus::Accepted) => {
            LeaderDecision::StatusOnly
        }
        MemberStatus::Accepted if free_slots == 0 => LeaderDecision::Refused,
        MemberStatus::Accepted if already_in_leaders_party => LeaderDecision::StatusOnly,
        MemberStatus::Accepted => LeaderDecision::PartyInvitation,
        MemberStatus::None | MemberStatus::Pending => LeaderDecision::Refused,
    }
}

/// Client IDs, including promotions: Crystal data/XML/vocations.xml.
fn client_vocation(vocation: &str) -> Result<u8> {
    match vocation {
        "none" => Ok(0),
        "knight" => Ok(1),
        "paladin" => Ok(2),
        "sorcerer" => Ok(3),
        "druid" => Ok(4),
        "monk" => Ok(5),
        "elite_knight" => Ok(11),
        "royal_paladin" => Ok(12),
        "master_sorcerer" => Ok(13),
        "elder_druid" => Ok(14),
        "exalted_monk" => Ok(15),
        _ => Err(reject("team finder unknown vocation")),
    }
}

fn page_limit(limit: i64) -> Result<i64> {
    // Wire pages contain 100 entries; the extra row proves continuation.
    if (1..=101).contains(&limit) {
        Ok(limit)
    } else {
        Err(reject("team finder page bound"))
    }
}

async fn owner(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
) -> Result<(Scope, PartySnapshot)> {
    check_transaction(tx, authority).await?;
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = authority.runtime_scope()
    else {
        return Err(reject("team finder Channel authority"));
    };
    // The established party owner is acquired before the Team Finder lock on
    // every path. Importing a party or accepting an applicant keeps that order.
    let party = world_party::read_world_party_in_transaction(tx, authority).await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:team-finder:' || encode($1,'hex') || ':' || encode($2,'hex'),0))")
        .bind(world_id.as_bytes().as_slice())
        .bind(channel_id.as_bytes().as_slice())
        .execute(&mut **tx)
        .await?;
    Ok((
        (
            *world_id.as_bytes(),
            *channel_id.as_bytes(),
            authority.scope_generation(),
        ),
        party,
    ))
}

fn binding(authority: &SpellItemAuthority, action: u8, target: Target) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"oteryn.team-finder.member.v1");
    hash.update(authority.character_id_bytes());
    hash.update([action]);
    hash.update(target.character);
    hash.update(target.session.as_bytes());
    hash.update(target.connection_generation.to_be_bytes());
    hash.finalize().into()
}

async fn replay(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    binding: &[u8; 32],
) -> Result<Option<MutationOutcome>> {
    let command = authority.command();
    let row = sqlx::query("SELECT binding,wire_binding,accepted,uuid_send(character_id) AS character FROM game_team_finder_member_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric(20,0)")
        .bind(command.game_session_id().as_bytes().as_slice())
        .bind(command.command_id().get().to_string())
        .fetch_optional(&mut **tx)
        .await?;
    row.map(|row| {
        let matches = row.try_get::<Vec<u8>, _>("binding")? == binding
            && row.try_get::<Vec<u8>, _>("wire_binding")? == authority.compatible_content_digest()
            && uuid(row.try_get("character")?)? == authority.character_id_bytes();
        Ok(MutationOutcome {
            accepted: matches && row.try_get("accepted")?,
            replayed: true,
            notify_session: None,
        })
    })
    .transpose()
}

/// Receipt recovery precedes connection-local handle resolution. The compositor
/// binds command type plus the exact raw payload as compatible_content_digest;
/// an old handle need not remain cached to recover its committed result.
pub async fn replay_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
) -> Result<Option<MutationOutcome>> {
    check_transaction(tx, authority).await?;
    let command = authority.command();
    let row = sqlx::query("SELECT wire_binding,accepted,uuid_send(character_id) AS character FROM game_team_finder_member_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric(20,0)")
        .bind(command.game_session_id().as_bytes().as_slice())
        .bind(command.command_id().get().to_string())
        .fetch_optional(&mut **tx)
        .await?;
    row.map(|row| {
        let matches = row.try_get::<Vec<u8>, _>("wire_binding")?
            == authority.compatible_content_digest()
            && uuid(row.try_get("character")?)? == authority.character_id_bytes();
        Ok(MutationOutcome {
            accepted: matches && row.try_get("accepted")?,
            replayed: true,
            notify_session: None,
        })
    })
    .transpose()
}

async fn record(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    binding: &[u8; 32],
    accepted: bool,
    notify_session: Option<GameSessionId>,
) -> Result<MutationOutcome> {
    let command = authority.command();
    sqlx::query("INSERT INTO game_team_finder_member_receipts(game_session_id,command_id,character_id,binding,wire_binding,accepted) VALUES(encode($1,'hex')::uuid,$2::text::numeric(20,0),encode($3,'hex')::uuid,$4,$5,$6)")
        .bind(command.game_session_id().as_bytes().as_slice())
        .bind(command.command_id().get().to_string())
        .bind(authority.character_id_bytes().as_slice())
        .bind(binding.as_slice())
        .bind(authority.compatible_content_digest().as_slice())
        .bind(accepted)
        .execute(&mut **tx)
        .await?;
    Ok(MutationOutcome {
        accepted,
        replayed: false,
        notify_session,
    })
}

async fn active_listing(
    tx: &mut Transaction<'_, Postgres>,
    scope: Scope,
    leader: [u8; 16],
) -> Result<Option<PgRow>> {
    // As in admission_journal/party_target_binding.rs:245-248, a retained
    // session is not current authority without its live Character/account
    // claims. The caller's independently fenced Channel generation must agree.
    Ok(sqlx::query("SELECT l.*,uuid_send(a.leader_character_id) AS character,uuid_send(a.leader_game_session_id) AS session,a.connection_generation::text AS generation,(SELECT count(*) FROM game_team_finder_members m WHERE m.leader_character_id=a.leader_character_id AND m.status=3) AS accepted_count FROM game_team_finder_active_listings a JOIN game_team_finder_leader_listings l USING(leader_character_id) JOIN game_durability_reconnect_sessions s ON s.game_session_id=a.leader_game_session_id JOIN game_character_roots r ON r.character_id=s.character_id JOIN game_durability_admission_character_guards cg ON cg.character_id=s.character_id JOIN game_durability_admission_account_guards ag ON ag.account_id=cg.account_id WHERE a.leader_character_id=encode($1,'hex')::uuid AND a.world_id=encode($2,'hex')::uuid AND a.channel_id=encode($3,'hex')::uuid AND s.character_id=a.leader_character_id AND s.world_id=a.world_id AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=a.world_id AND s.runtime_scope_channel_id=a.channel_id AND s.session_state=2 AND s.current_generation=a.connection_generation AND s.character_lease_generation=a.character_lease_generation AND s.scope_ownership_generation=a.scope_ownership_generation AND a.scope_ownership_generation=$4::text::numeric(20,0) AND cg.account_id=r.account_id AND s.account_id=r.account_id AND cg.world_id=r.world_id AND cg.world_id=a.world_id AND cg.eligible AND cg.lease_generation=s.character_lease_generation AND cg.holder_game_session_id=s.game_session_id AND ag.presence_character_id=s.character_id AND ag.holder_game_session_id=s.game_session_id FOR SHARE OF a,s,l,r,cg,ag")
        .bind(leader.as_slice())
        .bind(scope.0.as_slice())
        .bind(scope.1.as_slice())
        .bind(scope.2.to_string())
        .fetch_optional(&mut **tx)
        .await?)
}

async fn current_target(
    tx: &mut Transaction<'_, Postgres>,
    scope: Scope,
    target: Target,
) -> Result<bool> {
    Ok(sqlx::query_scalar::<_, i16>("SELECT 1::smallint FROM game_durability_reconnect_sessions s JOIN game_character_roots r ON r.character_id=s.character_id JOIN game_durability_admission_character_guards cg ON cg.character_id=s.character_id JOIN game_durability_admission_account_guards ag ON ag.account_id=cg.account_id WHERE s.character_id=encode($1,'hex')::uuid AND s.game_session_id=encode($2,'hex')::uuid AND s.current_generation=$3::text::numeric(20,0) AND s.world_id=encode($4,'hex')::uuid AND s.runtime_scope_kind=1 AND s.runtime_scope_world_id=encode($4,'hex')::uuid AND s.runtime_scope_channel_id=encode($5,'hex')::uuid AND s.session_state=2 AND s.scope_ownership_generation=$6::text::numeric(20,0) AND cg.account_id=r.account_id AND s.account_id=r.account_id AND cg.world_id=r.world_id AND cg.world_id=s.world_id AND cg.eligible AND cg.lease_generation=s.character_lease_generation AND cg.holder_game_session_id=s.game_session_id AND ag.presence_character_id=s.character_id AND ag.holder_game_session_id=s.game_session_id FOR SHARE OF s,r,cg,ag")
        .bind(target.character.as_slice())
        .bind(target.session.as_bytes().as_slice())
        .bind(target.connection_generation.to_string())
        .bind(scope.0.as_slice())
        .bind(scope.1.as_slice())
        .bind(scope.2.to_string())
        .fetch_optional(&mut **tx)
        .await?
        .is_some())
}

/// Request/cancel the selected live leader's map (Q1B fixes the donor lookup).
pub async fn apply_member_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    target: Target,
    request: bool,
) -> Result<MutationOutcome> {
    let (scope, _) = owner(tx, authority).await?;
    let binding = binding(authority, if request { 1 } else { 2 }, target);
    if let Some(replayed) = replay(tx, authority, &binding).await? {
        return Ok(replayed);
    }
    let actor = authority.character_id_bytes();
    let listing = active_listing(tx, scope, target.character).await?;
    if actor == target.character
        || !listing
            .as_ref()
            .map(target_of_matches(target))
            .transpose()?
            .unwrap_or(false)
    {
        return record(tx, authority, &binding, false, None).await;
    }
    if request {
        let existing: Option<i16> = sqlx::query_scalar("SELECT status FROM game_team_finder_members WHERE leader_character_id=encode($1,'hex')::uuid AND member_character_id=encode($2,'hex')::uuid")
            .bind(target.character.as_slice()).bind(actor.as_slice()).fetch_optional(&mut **tx).await?;
        if existing.is_none() {
            let pending: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_team_finder_members WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND member_character_id=encode($3,'hex')::uuid AND status=1)")
                .bind(scope.0.as_slice()).bind(scope.1.as_slice()).bind(actor.as_slice()).fetch_one(&mut **tx).await?;
            if pending {
                return record(tx, authority, &binding, false, None).await;
            }
            sqlx::query("INSERT INTO game_team_finder_members(leader_character_id,world_id,channel_id,member_character_id,status) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1)")
                .bind(target.character.as_slice()).bind(scope.0.as_slice()).bind(scope.1.as_slice()).bind(actor.as_slice()).execute(&mut **tx).await?;
        }
        record(tx, authority, &binding, true, Some(target.session)).await
    } else {
        sqlx::query("DELETE FROM game_team_finder_members WHERE leader_character_id=encode($1,'hex')::uuid AND member_character_id=encode($2,'hex')::uuid")
            .bind(target.character.as_slice()).bind(actor.as_slice()).execute(&mut **tx).await?;
        record(tx, authority, &binding, true, None).await
    }
}

fn target_of_matches(expected: Target) -> impl FnOnce(&PgRow) -> Result<bool> {
    move |row| Ok(target(row)? == expected)
}

/// A leader decision changes an existing applicant. Acceptance delegates its
/// ordinary invitation to the established party owner in this same transaction.
pub async fn apply_leader_member_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    target: Target,
    desired: MemberStatus,
    visible: Option<&QualifiedPartyVisibleTarget<'_>>,
) -> Result<MutationOutcome> {
    let (scope, party) = owner(tx, authority).await?;
    let action = match desired {
        MemberStatus::Invited => 3,
        MemberStatus::Accepted => 4,
        MemberStatus::Denied => 5,
        _ => return Err(reject("team finder leader decision")),
    };
    let binding = binding(authority, action, target);
    if let Some(replayed) = replay(tx, authority, &binding).await? {
        return Ok(replayed);
    }
    let actor = authority.character_id_bytes();
    let Some(row) = active_listing(tx, scope, actor).await? else {
        return record(tx, authority, &binding, false, None).await;
    };
    if target.character == actor || !current_target(tx, scope, target).await? {
        return record(tx, authority, &binding, false, None).await;
    }
    let previous: Option<i16> = sqlx::query_scalar("SELECT status FROM game_team_finder_members WHERE leader_character_id=encode($1,'hex')::uuid AND member_character_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(actor.as_slice()).bind(target.character.as_slice()).fetch_optional(&mut **tx).await?;
    let accepted = u64::try_from(row.try_get::<i64, _>("accepted_count")?)
        .map_err(|_| reject("team finder accepted count"))?;
    let (listing, _) = effective_listing(stored_listing(&row)?, accepted);
    let already_in_leaders_party = party.leader == Some(actor)
        && party
            .members
            .iter()
            .any(|member| member.character == target.character);
    let decision = leader_decision(
        previous.map(status).transpose()?,
        desired,
        listing.free_slots,
        already_in_leaders_party,
    );
    if decision == LeaderDecision::Refused {
        return record(tx, authority, &binding, false, None).await;
    }
    if decision == LeaderDecision::PartyInvitation {
        let party = world_party::apply_world_party_command_in_transaction(
            tx,
            authority,
            PartyAction::Invite {
                target: target.character,
            },
            visible,
        )
        .await?;
        if party.disposition != "APPLIED" {
            return record(tx, authority, &binding, false, None).await;
        }
    }
    sqlx::query("UPDATE game_team_finder_members SET status=$3 WHERE leader_character_id=encode($1,'hex')::uuid AND member_character_id=encode($2,'hex')::uuid")
        .bind(actor.as_slice()).bind(target.character.as_slice()).bind(i16::from(desired as u8)).execute(&mut **tx).await?;
    record(tx, authority, &binding, true, Some(target.session)).await
}

#[path = "team_finder/listing.rs"]
mod listing;
#[path = "team_finder/query.rs"]
mod query;
pub use listing::apply_leader_listing_in_transaction;
pub use query::{applicants_in_transaction, browse_in_transaction};

#[cfg(test)]
#[path = "team_finder/tests.rs"]
mod tests;
