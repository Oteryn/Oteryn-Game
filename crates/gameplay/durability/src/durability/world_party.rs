//! World party writer/read port. PostgreSQL is the sole membership authority; every operation
//! uses the already independently current session/lease/node Item fence in the SAME transaction.
//! These are private candidate source ports, not allocated wire commands or production admission.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]
use super::admission_journal::party_target_binding::QualifiedPartyVisibleTarget;
use super::spell_item_transaction::{SpellItemAuthority, SpellItemError, check_transaction};
use crate::foundation::{
    PartyAction, PartyMemberSnapshot, PartyPresence, PartySnapshot, RuntimeScopeRefV1,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};
type Result<T> = std::result::Result<T, SpellItemError>;
fn reject(s: &'static str) -> SpellItemError {
    SpellItemError::Rejected(s)
}
fn uuid(text: String) -> Result<[u8; 16]> {
    let compact = text.replace('-', "");
    if compact.len() != 32 || !compact.is_ascii() {
        return Err(reject("stored party UUID"));
    }
    let mut out = [0; 16];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&compact[i * 2..i * 2 + 2], 16)
            .map_err(|_| reject("stored party UUID"))?;
    }
    crate::domain::CharacterId::from_bytes(out).map_err(|_| reject("typed party UUIDv7"))?;
    Ok(out)
}
fn number(s: String) -> Result<u64> {
    s.parse().map_err(|_| reject("party unsigned revision"))
}
async fn owner(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
) -> Result<([u8; 16], [u8; 16])> {
    check_transaction(tx, authority).await?;
    let RuntimeScopeRefV1::Channel {
        world_id,
        channel_id,
    } = authority.runtime_scope()
    else {
        return Err(reject("party channel authority"));
    };
    // One strongly ordered World owner across every Channel. The independently current acting
    // Channel/session fence is already held; no actor-supplied World or prepared evidence grants it.
    sqlx::query("SELECT pg_advisory_xact_lock(45,hashtext($1))")
        .bind(
            world_id
                .as_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
        )
        .execute(&mut **tx)
        .await?;
    Ok((*world_id.as_bytes(), *channel_id.as_bytes()))
}
/// Strong read under the actual World owner. Explicit SQL absence means solo; a missing
/// Character/World/source authority is an error. Cache absence never supplies this observation.
pub async fn read_world_party_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
) -> Result<PartySnapshot> {
    let (world, _) = owner(tx, authority).await?;
    let character = authority.character_id_bytes();
    observe_world_party(tx, world, character).await
}
/// Same-transaction setting plus its strong membership predecessor. This is data, not a
/// current-authority token; the kill owner must hold its existing runtime/SQL fences through use.
#[derive(Debug)]
pub struct WorldPartyXp {
    pub source: PartySnapshot,
    pub enabled: bool,
}
/// Read the existing PARTY-1 toggle without changing the shared PartySnapshot contract.
/// The native World lock and party/member row locks remain held by this transaction.
pub async fn read_world_party_xp_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
) -> Result<WorldPartyXp> {
    let source = read_world_party_in_transaction(tx, authority).await?;
    observe_world_party_xp(tx, source).await
}
/// Autonomous kill owners use the existing current scope+actor resolution; no fake cast command.
pub async fn read_world_party_xp_for_scope_actor_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &super::DurabilityRoot,
    scope: &super::spell_item_transaction::SpellItemScopeAuthority,
    runtime: &crate::foundation::ChannelRuntimeV1,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
) -> Result<WorldPartyXp> {
    let source =
        read_world_party_for_scope_actor_in_transaction(tx, root, scope, runtime, actor, session)
            .await?;
    observe_world_party_xp(tx, source).await
}
async fn observe_world_party_xp(
    tx: &mut Transaction<'_, Postgres>,
    source: PartySnapshot,
) -> Result<WorldPartyXp> {
    let enabled = if let Some(party) = source.party_id {
        sqlx::query_scalar("SELECT shared_xp_enabled FROM game_parties WHERE party_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND revision=$3::text::numeric FOR SHARE")
            .bind(party.as_slice())
            .bind(source.world.as_slice())
            .bind(source.revision.ok_or(reject("party XP revision unavailable"))?.to_string())
            .fetch_optional(&mut **tx).await?
            .ok_or(reject("party XP predecessor changed"))?
    } else {
        false
    };
    Ok(WorldPartyXp { source, enabled })
}
/// A real commandless source owner read for periodic Serene; no synthetic cast command.
pub async fn read_world_party_for_scope_actor_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &super::DurabilityRoot,
    scope: &super::spell_item_transaction::SpellItemScopeAuthority,
    runtime: &crate::foundation::ChannelRuntimeV1,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
) -> Result<PartySnapshot> {
    super::spell_item_transaction::check_scope_transaction(tx, scope).await?;
    let character=super::admission_journal::party_target_binding::current_colocated_party_character_for_scope_in_transaction(
        tx,root,scope,runtime,actor,session).await?.ok_or(reject("party periodic current actor source"))?;
    let world = scope.world_bytes();
    sqlx::query("SELECT pg_advisory_xact_lock(45,hashtext($1))")
        .bind(world.iter().map(|b| format!("{b:02x}")).collect::<String>())
        .execute(&mut **tx)
        .await?;
    observe_world_party(tx, world, character).await
}
/// Full-refresh/revision-check producer for the actual complete current scope census.
/// Canonical admission bindings are resolved independently before a single indexed SQL
/// statement reads all local memberships, party revisions and member presence records.
/// The caller retains the real runtime borrow and must COMMIT before publishing these DATA.
pub async fn read_world_party_views_for_scope_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &super::DurabilityRoot,
    scope: &super::spell_item_transaction::SpellItemScopeAuthority,
    runtime: &crate::foundation::ChannelRuntimeV1,
) -> Result<Vec<PartySnapshot>> {
    super::spell_item_transaction::check_scope_transaction(tx, scope).await?;
    if scope.world_bytes() != *runtime.binding().world_id().as_bytes()
        || scope.channel_bytes() != *runtime.binding().channel_id().as_bytes()
        || scope.generation() != runtime.binding().scope_generation().get()
    {
        return Err(reject("party refresh scope binding"));
    }
    let census = runtime
        .positioned_actor_census()
        .map_err(|_| reject("party refresh census"))?;
    let mut characters = std::collections::BTreeSet::new();
    for (actor, _, session) in census {
        let Some(session) = session else { continue };
        let character = super::admission_journal::party_target_binding::current_colocated_party_character_for_scope_in_transaction(
            tx, root, scope, runtime, actor, session).await?
            .ok_or(reject("party refresh current actor"))?;
        if !characters.insert(character) {
            return Err(reject("party refresh duplicate Character"));
        }
    }
    let world = scope.world_bytes();
    sqlx::query("SELECT pg_advisory_xact_lock(45,hashtext($1))")
        .bind(world.iter().map(|b| format!("{b:02x}")).collect::<String>())
        .execute(&mut **tx)
        .await?;
    observe_world_party_batch(tx, world, characters.into_iter().collect()).await
}

async fn observe_world_party_batch(
    tx: &mut Transaction<'_, Postgres>,
    world: [u8; 16],
    characters: Vec<[u8; 16]>,
) -> Result<Vec<PartySnapshot>> {
    if characters.is_empty() {
        return Ok(Vec::new());
    }
    let requested: Vec<String> = characters
        .iter()
        .map(|id| id.iter().map(|byte| format!("{byte:02x}")).collect())
        .collect();
    // One statement snapshot prevents torn membership/revision/presence reads. The World
    // advisory lock also serializes it with this owner's membership and presence writers.
    let rows = sqlx::query(
        "WITH requested AS (SELECT unnest($1::text[]) AS character), observed AS MATERIALIZED (SELECT floor(extract(epoch FROM clock_timestamp())*1000000)::bigint AS micros) \
         SELECT requested.character, r.world_id::text AS character_world, p.world_id::text AS party_world, p.party_id::text, p.revision::text, p.leader_character_id::text, p.shared_xp_enabled, \
         m.character_id::text AS member, m.seq::text, m.presence_revision::text, m.presence_state, m.presence_channel_id::text, observed.micros \
         FROM requested CROSS JOIN observed LEFT JOIN game_character_roots r ON r.character_id=requested.character::uuid \
         LEFT JOIN game_party_members own ON own.character_id=r.character_id LEFT JOIN game_parties p ON p.party_id=own.party_id \
         LEFT JOIN game_party_members m ON m.party_id=p.party_id ORDER BY requested.character,m.seq"
    ).bind(requested).fetch_all(&mut **tx).await?;
    let mut snapshots = std::collections::BTreeMap::new();
    for row in rows {
        let character = uuid(row.try_get("character")?)?;
        if row
            .try_get::<Option<String>, _>("character_world")?
            .map(uuid)
            .transpose()?
            != Some(world)
        {
            return Err(reject("party refresh Character world"));
        }
        let party_id = row
            .try_get::<Option<String>, _>("party_id")?
            .map(uuid)
            .transpose()?;
        if party_id.is_some()
            && row
                .try_get::<Option<String>, _>("party_world")?
                .map(uuid)
                .transpose()?
                != Some(world)
        {
            return Err(reject("party refresh party world"));
        }
        if let std::collections::btree_map::Entry::Vacant(entry) = snapshots.entry(character) {
            entry.insert(PartySnapshot {
                world,
                character,
                party_id,
                revision: row
                    .try_get::<Option<String>, _>("revision")?
                    .map(number)
                    .transpose()?,
                leader: row
                    .try_get::<Option<String>, _>("leader_character_id")?
                    .map(uuid)
                    .transpose()?,
                shared_xp_enabled: row
                    .try_get::<Option<bool>, _>("shared_xp_enabled")?
                    .unwrap_or(false),
                members: Vec::new(),
                observed_database_micros: row.try_get("micros")?,
            });
        }
        let snapshot = snapshots
            .get_mut(&character)
            .ok_or(reject("party refresh Character"))?;
        if let Some(member) = row.try_get::<Option<String>, _>("member")? {
            if snapshot.members.len() >= 50 {
                return Err(reject("party size50"));
            }
            snapshot.members.push(PartyMemberSnapshot {
                character: uuid(member)?,
                invitation_order: number(row.try_get("seq")?)?,
                presence_revision: number(row.try_get("presence_revision")?)?,
                presence: match row.try_get::<i16, _>("presence_state")? {
                    1 => PartyPresence::Channel,
                    2 => PartyPresence::Hidden,
                    3 => PartyPresence::Offline,
                    _ => return Err(reject("stored party presence")),
                },
                channel: row
                    .try_get::<Option<String>, _>("presence_channel_id")?
                    .map(uuid)
                    .transpose()?,
            });
        }
    }
    if snapshots.len() != characters.len()
        || snapshots
            .values()
            .any(|snapshot| !snapshot.is_well_formed())
    {
        return Err(reject("party refresh incomplete snapshot"));
    }
    Ok(snapshots.into_values().collect())
}

async fn observe_world_party(
    tx: &mut Transaction<'_, Postgres>,
    world: [u8; 16],
    character: [u8; 16],
) -> Result<PartySnapshot> {
    let actual:Option<Vec<u8>>=sqlx::query_scalar("SELECT uuid_send(world_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR SHARE").bind(character.as_slice()).fetch_optional(&mut **tx).await?;
    if actual.as_deref() != Some(world.as_slice()) {
        return Err(reject("party Character world"));
    }
    let observed: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000000)::bigint")
            .fetch_one(&mut **tx)
            .await?;
    let row=sqlx::query("SELECT p.party_id::text,p.revision::text,p.leader_character_id::text,p.shared_xp_enabled FROM game_party_members m JOIN game_parties p USING(party_id) WHERE m.character_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid FOR SHARE OF p,m")
  .bind(character.as_slice()).bind(world.as_slice()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(PartySnapshot {
            world,
            character,
            party_id: None,
            revision: None,
            leader: None,
            shared_xp_enabled: false,
            members: vec![],
            observed_database_micros: observed,
        });
    };
    let party = uuid(row.try_get("party_id")?)?;
    let rows=sqlx::query("SELECT m.character_id::text,m.seq::text,m.presence_revision::text,m.presence_state,m.presence_channel_id::text FROM game_party_members m WHERE party_id=encode($1,'hex')::uuid ORDER BY seq FOR SHARE")
  .bind(party.as_slice()).fetch_all(&mut **tx).await?;
    if rows.len() > 50 {
        return Err(reject("party size50"));
    }
    let mut members = Vec::new();
    members
        .try_reserve_exact(rows.len())
        .map_err(|_| reject("party projection budget"))?;
    for r in rows {
        members.push(PartyMemberSnapshot {
            character: uuid(r.try_get("character_id")?)?,
            invitation_order: number(r.try_get("seq")?)?,
            presence_revision: number(r.try_get("presence_revision")?)?,
            presence: match r.try_get::<i16, _>("presence_state")? {
                1 => PartyPresence::Channel,
                2 => PartyPresence::Hidden,
                3 => PartyPresence::Offline,
                _ => return Err(reject("stored party presence")),
            },
            channel: r
                .try_get::<Option<String>, _>("presence_channel_id")?
                .map(uuid)
                .transpose()?,
        });
    }
    Ok(PartySnapshot {
        world,
        character,
        party_id: Some(party),
        revision: Some(number(row.try_get("revision")?)?),
        leader: Some(uuid(row.try_get("leader_character_id")?)?),
        shared_xp_enabled: row.try_get("shared_xp_enabled")?,
        members,
        observed_database_micros: observed,
    })
}
#[derive(Debug)]
pub struct PreparedWorldPartyReceipt {
    pub receipt_id: [u8; 16],
    pub party_id: Option<[u8; 16]>,
    pub revision: Option<u64>,
    pub replayed: bool,
    pub disposition: String,
}
fn result(row: &Value, replayed: bool, receipt_id: [u8; 16]) -> Result<PreparedWorldPartyReceipt> {
    let party_id = row
        .get("party_id")
        .and_then(Value::as_str)
        .map(|s| uuid(s.into()))
        .transpose()?;
    let revision = row.get("revision").and_then(Value::as_u64);
    let disposition = row
        .get("disposition")
        .and_then(Value::as_str)
        .ok_or(reject("stored party result"))?
        .into();
    Ok(PreparedWorldPartyReceipt {
        receipt_id,
        party_id,
        revision,
        replayed,
        disposition,
    })
}
/// Only a trusted real combat/logout owner may provide this proof. The source port
/// is deliberately sealed; the absent PvP owner never turns into a zero-deadline default.
pub mod logout_owner_seal {
    pub trait Sealed {}
}
pub trait PartyLogoutEligibility: logout_owner_seal::Sealed + Send + Sync {
    /// Bind the complete independently current caster occurrence and the actual SQL transaction.
    fn matches(&self, authority: &SpellItemAuthority, transaction_id: &str) -> bool;
    fn locks_clear(&self) -> bool;
}
/// Ordinary source command port; Leave fails closed without the independently current owner proof.
pub async fn apply_world_party_command_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    action: PartyAction,
    visible: Option<&QualifiedPartyVisibleTarget<'_>>,
) -> Result<PreparedWorldPartyReceipt> {
    apply_world_party_command_inner(tx, authority, action, visible, None).await
}
/// The actual source combat owner supplies its sealed same-transaction all-three-deadline proof.
pub async fn apply_world_party_command_with_logout_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    action: PartyAction,
    visible: Option<&QualifiedPartyVisibleTarget<'_>>,
    eligibility: &impl PartyLogoutEligibility,
) -> Result<PreparedWorldPartyReceipt> {
    apply_world_party_command_inner(tx, authority, action, visible, Some(eligibility)).await
}
async fn apply_world_party_command_inner(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    action: PartyAction,
    visible: Option<&QualifiedPartyVisibleTarget<'_>>,
    eligibility: Option<&dyn PartyLogoutEligibility>,
) -> Result<PreparedWorldPartyReceipt> {
    let (world, channel) = owner(tx, authority).await?;
    let character = authority.character_id_bytes();
    let command = authority.command();
    let intent=serde_json::to_vec(&json!({"schema":"WORLD_PARTY_COMMAND/v1","world":world,"character":character,"session":command.game_session_id().as_bytes(),"command":command.command_id().get(),"action":action})).map_err(|_|reject("party intent"))?;
    let binding = Sha256::digest(&intent).to_vec();
    let old=sqlx::query("SELECT receipt_id::text,intent,binding,result FROM game_world_party_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
  .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
    if let Some(row) = old {
        if row.try_get::<Vec<u8>, _>("intent")? != intent
            || row.try_get::<Vec<u8>, _>("binding")? != binding
        {
            return Err(reject("party command conflict"));
        }
        return result(
            &row.try_get::<Value, _>("result")?,
            true,
            uuid(row.try_get("receipt_id")?)?,
        );
    }
    let leave_clear = if matches!(action, PartyAction::Leave) {
        let proof = eligibility.ok_or(reject("current logout-block owner unavailable"))?;
        let xid: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
            .fetch_one(&mut **tx)
            .await?;
        if !proof.matches(authority, &xid) {
            return Err(reject("party logout proof is not current"));
        }
        proof.locks_clear()
    } else {
        false
    };
    // Consent writes serialize on the stable acting root even if no consent row exists.
    // This precedes every party/member/consent-row lock taken by these operations.
    if matches!(
        action,
        PartyAction::Block { .. }
            | PartyAction::Unblock { .. }
            | PartyAction::SetPartyInvites { .. }
            | PartyAction::SetChannelVisibility { .. }
    ) {
        sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
            .bind(character.as_slice()).bind(world.as_slice()).fetch_one(&mut **tx).await?;
    }
    let mut snapshot = read_world_party_in_transaction(tx, authority).await?;
    let mut disposition = "APPLIED";
    let mut party = snapshot.party_id;
    match action {
        PartyAction::Invite { target } => {
            crate::domain::CharacterId::from_bytes(target)
                .map_err(|_| reject("invite target UUIDv7"))?;
            let rate:i64=sqlx::query_scalar("SELECT count(*) FROM game_world_party_receipts WHERE character_id=encode($1,'hex')::uuid AND created_at>clock_timestamp()-interval '1 minute' AND convert_from(intent,'UTF8')::jsonb->'action' ? 'Invite'")
    .bind(character.as_slice()).fetch_one(&mut **tx).await?;
            let qualified =
                visible.is_some_and(|v| v.character() == target && v.matches(authority));
            if !qualified || target == character || rate >= 10 {
                disposition = "NOT_VISIBLE"
            } else {
                // Stable target root lock serializes consent changes and the global per-invitee bound.
                let target_world:Option<Vec<u8>>=sqlx::query_scalar("SELECT uuid_send(world_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE").bind(target.as_slice()).fetch_optional(&mut **tx).await?;
                let settings=sqlx::query("SELECT party_invites,source_policy FROM game_character_social_settings WHERE character_id=encode($1,'hex')::uuid FOR SHARE").bind(target.as_slice()).fetch_optional(&mut **tx).await?;
                let blocked:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_social_blocks WHERE character_id=encode($1,'hex')::uuid AND blocked_character_id=encode($2,'hex')::uuid)").bind(target.as_slice()).bind(character.as_slice()).fetch_one(&mut **tx).await?;
                let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_party_invitations WHERE invitee=encode($1,'hex')::uuid AND expires_at>clock_timestamp()").bind(target.as_slice()).fetch_one(&mut **tx).await?;
                let target_member:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_party_members WHERE character_id=encode($1,'hex')::uuid)").bind(target.as_slice()).fetch_one(&mut **tx).await?;
                let allowed = match settings {
                    Some(s) => {
                        s.try_get::<i16, _>("party_invites")? == 1
                            && s.try_get::<String, _>("source_policy")?
                                == "PARTYPVP0-PARTIES-AND-PVP-V1"
                    }
                    // PARTY_PVP0 §3: an absent settings row has ANYONE/PARTY defaults.
                    None => true,
                };
                if target_world.as_deref() != Some(world.as_slice())
                    || !allowed
                    || blocked
                    || target_member
                    || count >= 20
                {
                    disposition = "NOT_VISIBLE"
                } else if party.is_some() && snapshot.leader != Some(character) {
                    disposition = "NOT_LEADER"
                } else {
                    if party.is_none() {
                        let new: String =
                            sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
                                .fetch_one(&mut **tx)
                                .await?;
                        let id = uuid(new.clone())?;
                        sqlx::query("INSERT INTO game_parties(party_id,world_id,leader_character_id,revision,next_seq) VALUES($1::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,2)").bind(&new).bind(world.as_slice()).bind(character.as_slice()).execute(&mut **tx).await?;
                        let (presence, presence_channel) =
                            initial_member_presence(tx, character, channel).await?;
                        sqlx::query("INSERT INTO game_party_members(character_id,party_id,seq,presence_state,presence_channel_id) VALUES(encode($1,'hex')::uuid,$2::uuid,1,$3,CASE WHEN $4::bytea IS NULL THEN NULL ELSE encode($4,'hex')::uuid END)").bind(character.as_slice()).bind(&new).bind(presence).bind(presence_channel).execute(&mut **tx).await?;
                        party = Some(id);
                    }
                    let id = party.ok_or(reject("created party identity"))?;
                    let invitations:i64=sqlx::query_scalar("SELECT count(*) FROM game_party_invitations WHERE party_id=encode($1,'hex')::uuid AND expires_at>clock_timestamp()").bind(id.as_slice()).fetch_one(&mut **tx).await?;
                    if invitations >= 50 {
                        disposition = "NOT_VISIBLE"
                    } else {
                        let seq:String=sqlx::query_scalar("UPDATE game_parties SET next_seq=next_seq+1,revision=revision+1 WHERE party_id=encode($1,'hex')::uuid RETURNING (next_seq-1)::text").bind(id.as_slice()).fetch_one(&mut **tx).await?;
                        sqlx::query("INSERT INTO game_party_invitations(party_id,invitee,seq,expires_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,clock_timestamp()+interval '300 seconds') ON CONFLICT(party_id,invitee) DO UPDATE SET expires_at=clock_timestamp()+interval '300 seconds'")
       .bind(id.as_slice()).bind(target.as_slice()).bind(seq).execute(&mut **tx).await?;
                    }
                }
            }
        }
        PartyAction::Accept { party: requested } => {
            if party.is_some() {
                disposition = "ALREADY_MEMBER"
            } else {
                let row=sqlx::query("SELECT p.world_id::text,i.seq::text,(SELECT count(*) FROM game_party_members m WHERE m.party_id=p.party_id) AS member_count FROM game_party_invitations i JOIN game_parties p USING(party_id) WHERE i.party_id=encode($1,'hex')::uuid AND i.invitee=encode($2,'hex')::uuid AND i.expires_at>clock_timestamp() FOR UPDATE OF p,i")
     .bind(requested.as_slice()).bind(character.as_slice()).fetch_optional(&mut **tx).await?;
                if let Some(r) = row {
                    if uuid(r.try_get("world_id")?)? != world
                        || r.try_get::<i64, _>("member_count")? >= 50
                    {
                        disposition = "NOT_INVITED"
                    } else {
                        let (presence, presence_channel) =
                            initial_member_presence(tx, character, channel).await?;
                        sqlx::query("INSERT INTO game_party_members(character_id,party_id,seq,presence_state,presence_channel_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,CASE WHEN $5::bytea IS NULL THEN NULL ELSE encode($5,'hex')::uuid END)").bind(character.as_slice()).bind(requested.as_slice()).bind(r.try_get::<String,_>("seq")?).bind(presence).bind(presence_channel).execute(&mut **tx).await?;
                        sqlx::query("DELETE FROM game_party_invitations WHERE party_id=encode($2,'hex')::uuid AND invitee=encode($1,'hex')::uuid").bind(character.as_slice()).bind(requested.as_slice()).execute(&mut **tx).await?;
                        sqlx::query("UPDATE game_parties SET revision=revision+1 WHERE party_id=encode($1,'hex')::uuid").bind(requested.as_slice()).execute(&mut **tx).await?;
                    }
                } else {
                    disposition = "NOT_INVITED"
                }
            }
        }
        PartyAction::Decline { party: requested } => {
            // Lock the inviting party first: closing it requires the caller to hold its row.
            let locked:Option<String>=sqlx::query_scalar("SELECT party_id::text FROM game_parties WHERE party_id=encode($1,'hex')::uuid FOR UPDATE").bind(requested.as_slice()).fetch_optional(&mut **tx).await?;
            let removed=sqlx::query("DELETE FROM game_party_invitations WHERE party_id=encode($1,'hex')::uuid AND invitee=encode($2,'hex')::uuid").bind(requested.as_slice()).bind(character.as_slice()).execute(&mut **tx).await?.rows_affected();
            if locked.is_some() && removed != 0 {
                close_leader_only_party_without_invitations(tx, requested).await?;
            }
        }
        PartyAction::Revoke { target } => {
            if snapshot.leader != Some(character) {
                disposition = "NOT_LEADER"
            } else {
                sqlx::query("DELETE FROM game_party_invitations WHERE party_id=encode($1,'hex')::uuid AND invitee=encode($2,'hex')::uuid").bind(party.ok_or(reject("leader party"))?.as_slice()).bind(target.as_slice()).execute(&mut **tx).await?;
            }
        }
        PartyAction::TransferLeader { target } => {
            if snapshot.leader != Some(character)
                || !snapshot.members.iter().any(|m| m.character == target)
            {
                disposition = "NOT_MEMBER"
            } else {
                sqlx::query("UPDATE game_parties SET leader_character_id=encode($1,'hex')::uuid,revision=revision+1 WHERE party_id=encode($2,'hex')::uuid").bind(target.as_slice()).bind(party.ok_or(reject("leader party"))?.as_slice()).execute(&mut **tx).await?;
            }
        }
        PartyAction::SetSharedExperience { enabled } => {
            // The same-TX strong read holds the current World party, not a PartyView
            // or a caller-supplied leader claim. The occurrence and audit path below
            // is shared with every existing membership operation.
            if snapshot.leader != Some(character) {
                disposition = "NOT_LEADER";
            } else {
                sqlx::query("UPDATE game_parties SET shared_xp_enabled=$1,revision=revision+1 WHERE party_id=encode($2,'hex')::uuid AND shared_xp_enabled IS DISTINCT FROM $1")
                    .bind(enabled)
                    .bind(party.ok_or(reject("leader party"))?.as_slice())
                    .execute(&mut **tx)
                    .await?;
            }
        }
        PartyAction::Block { target } => {
            crate::domain::CharacterId::from_bytes(target)
                .map_err(|_| reject("block target UUIDv7"))?;
            if target == character
                || !visible.is_some_and(|target_proof| {
                    target_proof.character() == target && target_proof.matches(authority)
                })
            {
                disposition = "NOT_VISIBLE";
            } else {
                let existing: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_social_blocks WHERE character_id=encode($1,'hex')::uuid AND blocked_character_id=encode($2,'hex')::uuid)")
                    .bind(character.as_slice()).bind(target.as_slice()).fetch_one(&mut **tx).await?;
                let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_social_blocks WHERE character_id=encode($1,'hex')::uuid")
                    .bind(character.as_slice()).fetch_one(&mut **tx).await?;
                if !existing && count >= 100 {
                    disposition = "RESOURCE_LIMIT";
                } else {
                    // Lock the invitation's owning party before either its row or consent rows.
                    // The target leads at most one party. A block cancels only that party's invite.
                    let inviting_party: Option<String> = sqlx::query_scalar("SELECT p.party_id::text FROM game_parties p WHERE p.world_id=encode($1,'hex')::uuid AND p.leader_character_id=encode($2,'hex')::uuid AND EXISTS(SELECT 1 FROM game_party_invitations i WHERE i.party_id=p.party_id AND i.invitee=encode($3,'hex')::uuid) FOR UPDATE OF p")
                        .bind(world.as_slice()).bind(target.as_slice()).bind(character.as_slice()).fetch_optional(&mut **tx).await?;
                    if let Some(inviting_party) = inviting_party {
                        let inviting_party = uuid(inviting_party)?;
                        let deleted = sqlx::query("DELETE FROM game_party_invitations WHERE party_id=encode($1,'hex')::uuid AND invitee=encode($2,'hex')::uuid")
                            .bind(inviting_party.as_slice()).bind(character.as_slice()).execute(&mut **tx).await?.rows_affected();
                        if deleted != 0 {
                            sqlx::query("UPDATE game_parties SET revision=revision+1 WHERE party_id=encode($1,'hex')::uuid")
                                .bind(inviting_party.as_slice()).execute(&mut **tx).await?;
                            close_leader_only_party_without_invitations(tx, inviting_party).await?;
                        }
                    }
                    sqlx::query("INSERT INTO game_character_social_blocks(character_id,blocked_character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid) ON CONFLICT DO NOTHING")
                        .bind(character.as_slice()).bind(target.as_slice()).execute(&mut **tx).await?;
                }
            }
        }
        PartyAction::Unblock { target } => {
            crate::domain::CharacterId::from_bytes(target)
                .map_err(|_| reject("unblock target UUIDv7"))?;
            // The server resolves its retained block handle to this existing own-list key.
            // No target existence/presence query or target DATA is returned.
            sqlx::query("DELETE FROM game_character_social_blocks WHERE character_id=encode($1,'hex')::uuid AND blocked_character_id=encode($2,'hex')::uuid")
                .bind(character.as_slice()).bind(target.as_slice()).execute(&mut **tx).await?;
        }
        PartyAction::SetPartyInvites { anyone } => {
            sqlx::query("INSERT INTO game_character_social_settings(character_id,party_invites,channel_visibility,source_policy) VALUES(encode($1,'hex')::uuid,$2,1,'PARTYPVP0-PARTIES-AND-PVP-V1') ON CONFLICT(character_id) DO UPDATE SET party_invites=EXCLUDED.party_invites")
                .bind(character.as_slice()).bind(if anyone {1_i16} else {2_i16}).execute(&mut **tx).await?;
        }
        PartyAction::SetChannelVisibility { hidden } => {
            if let Some(id) = party {
                sqlx::query("SELECT party_id FROM game_parties WHERE party_id=encode($1,'hex')::uuid FOR UPDATE")
                    .bind(id.as_slice()).fetch_one(&mut **tx).await?;
            }
            let old: Option<i16> = sqlx::query_scalar("SELECT channel_visibility FROM game_character_social_settings WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(character.as_slice()).fetch_optional(&mut **tx).await?;
            let setting = if hidden { 2_i16 } else { 1_i16 };
            sqlx::query("INSERT INTO game_character_social_settings(character_id,party_invites,channel_visibility,source_policy) VALUES(encode($1,'hex')::uuid,1,$2,'PARTYPVP0-PARTIES-AND-PVP-V1') ON CONFLICT(character_id) DO UPDATE SET channel_visibility=EXCLUDED.channel_visibility")
                .bind(character.as_slice()).bind(setting).execute(&mut **tx).await?;
            if let Some(id) = party {
                let presence_changed = snapshot
                    .members
                    .iter()
                    .find(|member| member.character == character)
                    .is_some_and(|member| match member.presence {
                        PartyPresence::Channel => hidden,
                        PartyPresence::Hidden => !hidden,
                        PartyPresence::Offline => true,
                    });
                // The effective default is PARTY. Any actual authorization change, including
                // repairing an inconsistent old visible record, invalidates the party epoch.
                if old.unwrap_or(1) != setting || presence_changed {
                    sqlx::query("UPDATE game_parties SET revision=revision+1 WHERE party_id=encode($1,'hex')::uuid")
                        .bind(id.as_slice()).execute(&mut **tx).await?;
                }
                refresh_current_member_presence(tx, world, channel, character).await?;
            }
        }
        PartyAction::Leave => {
            if !leave_clear {
                disposition = "LOGOUT_BLOCKED";
            } else if let Some(id) = party {
                remove_member_with_succession(tx, id, character).await?;
            }
        }
    }
    if let Some(id) = party {
        close_leader_only_party_without_invitations(tx, id).await?;
    }
    snapshot = read_world_party_in_transaction(tx, authority).await?;
    let value = json!({"disposition":disposition,"party_id":snapshot.party_id.map(|id|id.iter().map(|b|format!("{b:02x}")).collect::<String>()),"revision":snapshot.revision});
    let id: String = sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut **tx)
        .await?;
    sqlx::query("INSERT INTO game_world_party_receipts(game_session_id,command_id,receipt_id,world_id,character_id,connection_generation,lease_generation,scope_generation,intent,binding,result) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,$7::text::numeric,$8::text::numeric,$9,$10,$11)")
 .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(&id).bind(world.as_slice()).bind(character.as_slice()).bind(authority.connection_generation().get().to_string()).bind(authority.character_lease_generation().to_string()).bind(authority.scope_generation().to_string()).bind(&intent).bind(&binding).bind(&value).execute(&mut **tx).await?;
    sqlx::query(
        "INSERT INTO game_world_party_audit_outbox(receipt_id,envelope) VALUES($1::uuid,$2)",
    )
    .bind(&id)
    .bind(&intent)
    .execute(&mut **tx)
    .await?;
    result(&value, false, uuid(id)?)
}

/// The caller already holds the serialized World owner and current party row.
async fn close_leader_only_party_without_invitations(
    tx: &mut Transaction<'_, Postgres>,
    party: [u8; 16],
) -> Result<bool> {
    let changed=sqlx::query("DELETE FROM game_parties p WHERE p.party_id=encode($1,'hex')::uuid AND (SELECT count(*) FROM game_party_members m WHERE m.party_id=p.party_id)<=1 AND NOT EXISTS(SELECT 1 FROM game_party_invitations i WHERE i.party_id=p.party_id)")
        .bind(party.as_slice()).execute(&mut **tx).await?.rows_affected();
    Ok(changed != 0)
}
async fn remove_member_with_succession(
    tx: &mut Transaction<'_, Postgres>,
    party: [u8; 16],
    character: [u8; 16],
) -> Result<()> {
    let row=sqlx::query("SELECT leader_character_id::text FROM game_parties WHERE party_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(party.as_slice()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else { return Ok(()) };
    let leader = uuid(row.try_get("leader_character_id")?)?;
    // The exact accepted succession rule prefers current state1/2 sessions, then invitation seq.
    if leader == character {
        let next:Option<String>=sqlx::query_scalar("SELECT m.character_id::text FROM game_party_members m WHERE m.party_id=encode($1,'hex')::uuid AND m.character_id<>encode($2,'hex')::uuid ORDER BY EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.character_id=m.character_id AND s.session_state IN(1,2)) DESC,m.seq LIMIT 1 FOR UPDATE OF m")
            .bind(party.as_slice()).bind(character.as_slice()).fetch_optional(&mut **tx).await?;
        if let Some(next) = next {
            sqlx::query("UPDATE game_parties SET leader_character_id=$1::uuid,revision=revision+1 WHERE party_id=encode($2,'hex')::uuid").bind(next).bind(party.as_slice()).execute(&mut **tx).await?;
        } else {
            sqlx::query("DELETE FROM game_parties WHERE party_id=encode($1,'hex')::uuid")
                .bind(party.as_slice())
                .execute(&mut **tx)
                .await?;
            return Ok(());
        }
    } else {
        sqlx::query(
            "UPDATE game_parties SET revision=revision+1 WHERE party_id=encode($1,'hex')::uuid",
        )
        .bind(party.as_slice())
        .execute(&mut **tx)
        .await?;
    }
    sqlx::query("DELETE FROM game_party_members WHERE party_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid").bind(party.as_slice()).bind(character.as_slice()).execute(&mut **tx).await?;
    close_leader_only_party_without_invitations(tx, party).await?;
    Ok(())
}
/// At most100 oldest expired invitations, exactly one party per transaction. The current
/// scoped owner is independently fenced; World serialization prevents cross-Channel races.
pub async fn cleanup_expired_world_party_invitations_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &super::spell_item_transaction::SpellItemScopeAuthority,
) -> Result<usize> {
    super::spell_item_transaction::check_scope_transaction(tx, authority).await?;
    let world = authority.world_bytes();
    sqlx::query("SELECT pg_advisory_xact_lock(45,hashtext($1))")
        .bind(world.iter().map(|b| format!("{b:02x}")).collect::<String>())
        .execute(&mut **tx)
        .await?;
    let party:Option<String>=sqlx::query_scalar("SELECT p.party_id::text FROM game_parties p JOIN game_party_invitations i USING(party_id) WHERE p.world_id=encode($1,'hex')::uuid AND i.expires_at<=clock_timestamp() ORDER BY i.expires_at,i.party_id,i.invitee LIMIT 1 FOR UPDATE OF p")
        .bind(world.as_slice()).fetch_optional(&mut **tx).await?;
    let Some(party) = party else { return Ok(0) };
    let party = uuid(party)?;
    let removed=sqlx::query("DELETE FROM game_party_invitations WHERE (party_id,invitee) IN(SELECT party_id,invitee FROM game_party_invitations WHERE party_id=encode($1,'hex')::uuid AND expires_at<=clock_timestamp() ORDER BY expires_at,invitee LIMIT 100 FOR UPDATE)")
        .bind(party.as_slice()).execute(&mut **tx).await?.rows_affected();
    if removed > 0 {
        sqlx::query(
            "UPDATE game_parties SET revision=revision+1 WHERE party_id=encode($1,'hex')::uuid",
        )
        .bind(party.as_slice())
        .execute(&mut **tx)
        .await?;
    }
    close_leader_only_party_without_invitations(tx, party).await?;
    usize::try_from(removed).map_err(|_| reject("party cleanup bound"))
}
/// Explicit social-baseline initialization at genuine admission. The accepted default for
/// an absent row is ANYONE/PARTY; this source producer may materialize those same defaults.
pub async fn initialize_party_social_baseline_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
) -> Result<()> {
    owner(tx, authority).await?;
    sqlx::query("SELECT character_id FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE").bind(authority.character_id_bytes().as_slice()).fetch_one(&mut **tx).await?;
    sqlx::query("INSERT INTO game_character_social_settings(character_id,party_invites,channel_visibility,source_policy) VALUES(encode($1,'hex')::uuid,1,1,'PARTYPVP0-PARTIES-AND-PVP-V1') ON CONFLICT(character_id) DO NOTHING")
        .bind(authority.character_id_bytes().as_slice()).execute(&mut **tx).await?;
    Ok(())
}
/// Joining/creating a party consumes current durable privacy. Absence has the accepted
/// PARTY default (§3), while a present HIDDEN setting never writes a Channel identifier.
async fn initial_member_presence(
    tx: &mut Transaction<'_, Postgres>,
    character: [u8; 16],
    channel: [u8; 16],
) -> Result<(i16, Option<Vec<u8>>)> {
    let setting: Option<i16> = sqlx::query_scalar("SELECT channel_visibility FROM game_character_social_settings WHERE character_id=encode($1,'hex')::uuid FOR SHARE")
        .bind(character.as_slice()).fetch_optional(&mut **tx).await?;
    match setting {
        None | Some(1) => Ok((1, Some(channel.to_vec()))),
        Some(2) => Ok((2, None)),
        _ => Err(reject("stored party channel visibility")),
    }
}

async fn refresh_current_member_presence(
    tx: &mut Transaction<'_, Postgres>,
    world: [u8; 16],
    channel: [u8; 16],
    character: [u8; 16],
) -> Result<()> {
    let setting:Option<i16>=sqlx::query_scalar("SELECT channel_visibility FROM game_character_social_settings WHERE character_id=encode($1,'hex')::uuid FOR SHARE")
        .bind(character.as_slice()).fetch_optional(&mut **tx).await?;
    let state = match setting {
        None | Some(1) => 1_i16,
        Some(2) => 2,
        _ => return Err(reject("current party presence consent unavailable")),
    };
    let target = if state == 1 {
        Some(channel.to_vec())
    } else {
        None
    };
    sqlx::query("SELECT p.party_id FROM game_parties p JOIN game_party_members m USING(party_id) WHERE m.character_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid FOR SHARE OF p")
        .bind(character.as_slice()).bind(world.as_slice()).fetch_optional(&mut **tx).await?;
    sqlx::query("UPDATE game_party_members m SET absence_observed_at=NULL,presence_state=$1,presence_channel_id=CASE WHEN $2::bytea IS NULL THEN NULL ELSE encode($2,'hex')::uuid END,presence_revision=presence_revision+1 FROM game_parties p WHERE m.party_id=p.party_id AND p.world_id=encode($3,'hex')::uuid AND m.character_id=encode($4,'hex')::uuid AND (m.presence_state<>$1 OR uuid_send(m.presence_channel_id) IS DISTINCT FROM $2::bytea OR m.absence_observed_at IS NOT NULL)")
        .bind(state).bind(target).bind(world.as_slice()).bind(character.as_slice()).execute(&mut **tx).await?;
    Ok(())
}
/// Admission/channel-entry source seam: actual independently current scope + canonical
/// actual actor/session yields the World Character. Unknown source never writes a presence.
pub async fn refresh_world_party_presence_for_scope_actor_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &super::DurabilityRoot,
    scope: &super::spell_item_transaction::SpellItemScopeAuthority,
    runtime: &crate::foundation::ChannelRuntimeV1,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
) -> Result<PartySnapshot> {
    let current =
        read_world_party_for_scope_actor_in_transaction(tx, root, scope, runtime, actor, session)
            .await?;
    if current.party_id.is_some() {
        refresh_current_member_presence(
            tx,
            current.world,
            scope.channel_bytes(),
            current.character,
        )
        .await?;
    }
    observe_world_party(tx, current.world, current.character).await
}

impl super::DurabilityRoot {
    /// Genuine bounded World cleanup source pass. Unknown COMMIT is retried by the same
    /// expired-row predicate; no command occurrence, artificial caster or user payment exists.
    pub async fn drain_world_party_expiry(
        &self,
        permit: &super::spell_owner_commit::SpellLanePermit,
        recovery: &super::character_authority::ReconciledCharacterAuthority<'_, '_>,
        node: &super::runtime_scope_assignment::NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: u64,
    ) -> Result<usize> {
        let record = recovery
            .record_for(self)
            .map_err(|_| reject("party cleanup recovery authority"))?;
        let node = node.clone();
        let mut context = permit;
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, permit| {
                Box::pin(async move {
                    let mut tx = super::spell_item_transaction::begin_spell_owner_transaction(
                        holder, deadline,
                    )
                    .await?;
                    let authority =
                        super::spell_item_transaction::assert_spell_item_scope_with_recovery(
                            &mut tx, permit, &record, &node, scope, generation,
                        )
                        .await
                        .map_err(|_| super::DurabilityError::Unavailable)?;
                    let count =
                        cleanup_expired_world_party_invitations_in_transaction(&mut tx, &authority)
                            .await
                            .map_err(|_| super::DurabilityError::Unavailable)?;
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(count)
                })
            })
            .await
            .map_err(SpellItemError::from)
    }
    /// Complete current Channel-census source pass for node refresh/revision checks.
    /// The membership batch and its source authority stay inside one native transaction.
    pub async fn read_current_world_party_views(
        &self,
        permit: &super::spell_owner_commit::SpellLanePermit,
        recovery: &super::character_authority::ReconciledCharacterAuthority<'_, '_>,
        node: &super::runtime_scope_assignment::NodeIncarnationProof,
        runtime: &crate::foundation::ChannelRuntimeV1,
    ) -> Result<Vec<PartySnapshot>> {
        let record = recovery
            .record_for(self)
            .map_err(|_| reject("party refresh recovery authority"))?;
        let node = node.clone();
        let scope = RuntimeScopeRefV1::channel(
            runtime.binding().world_id(),
            runtime.binding().channel_id(),
        );
        let generation = runtime.binding().scope_generation().get();
        let mut context = (self, runtime, permit);
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let (root, runtime, permit) = context;
                    let mut tx = super::spell_item_transaction::begin_spell_owner_transaction(
                        holder, deadline,
                    )
                    .await?;
                    let authority =
                        super::spell_item_transaction::assert_spell_item_scope_with_recovery(
                            &mut tx, permit, &record, &node, scope, generation,
                        )
                        .await
                        .map_err(|_| super::DurabilityError::Unavailable)?;
                    let views = read_world_party_views_for_scope_in_transaction(
                        &mut tx, root, &authority, runtime,
                    )
                    .await
                    .map_err(|_| super::DurabilityError::Unavailable)?;
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(views)
                })
            })
            .await
            .map_err(SpellItemError::from)
    }
    /// Admission/reentry source presence update is committed before any cache/output use.
    /// Runtime remains borrowed from the real owner throughout the bounded SQL pass.
    pub async fn refresh_current_world_party_presence(
        &self,
        permit: &super::spell_owner_commit::SpellLanePermit,
        recovery: &super::character_authority::ReconciledCharacterAuthority<'_, '_>,
        node: &super::runtime_scope_assignment::NodeIncarnationProof,
        runtime: &crate::foundation::ChannelRuntimeV1,
        actor: crate::foundation::ExactActorRef,
        session: crate::foundation::GameSessionId,
    ) -> Result<PartySnapshot> {
        let record = recovery
            .record_for(self)
            .map_err(|_| reject("party presence recovery authority"))?;
        let node = node.clone();
        let scope = RuntimeScopeRefV1::channel(
            runtime.binding().world_id(),
            runtime.binding().channel_id(),
        );
        let generation = runtime.binding().scope_generation().get();
        let mut context = (self, runtime, permit);
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, context| {
                Box::pin(async move {
                    let (root, runtime, permit) = context;
                    let mut tx = super::spell_item_transaction::begin_spell_owner_transaction(
                        holder, deadline,
                    )
                    .await?;
                    let authority =
                        super::spell_item_transaction::assert_spell_item_scope_with_recovery(
                            &mut tx, permit, &record, &node, scope, generation,
                        )
                        .await
                        .map_err(|_| super::DurabilityError::Unavailable)?;
                    let view = refresh_world_party_presence_for_scope_actor_in_transaction(
                        &mut tx, root, &authority, runtime, actor, session,
                    )
                    .await
                    .map_err(|_| super::DurabilityError::Unavailable)?;
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(view)
                })
            })
            .await
            .map_err(SpellItemError::from)
    }
}

/// Current session absence is sufficient only to hide remote presence. It is NOT proof
/// of legal physical absence or cleared combat deadlines and never authorizes removal.
pub async fn cleanup_world_party_offline_presence_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    scope: &super::spell_item_transaction::SpellItemScopeAuthority,
) -> Result<usize> {
    super::spell_item_transaction::check_scope_transaction(tx, scope).await?;
    let world = scope.world_bytes();
    sqlx::query("SELECT pg_advisory_xact_lock(45,hashtext($1))")
        .bind(world.iter().map(|b| format!("{b:02x}")).collect::<String>())
        .execute(&mut **tx)
        .await?;
    let party:Option<String>=sqlx::query_scalar("SELECT p.party_id::text FROM game_parties p WHERE p.world_id=encode($1,'hex')::uuid AND EXISTS(SELECT 1 FROM game_party_members m WHERE m.party_id=p.party_id AND (m.presence_state<>3 OR m.absence_observed_at IS NULL) AND NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.character_id=m.character_id AND s.session_state IN(1,2))) ORDER BY p.created_at,p.party_id LIMIT 1 FOR UPDATE OF p")
        .bind(world.as_slice()).fetch_optional(&mut **tx).await?;
    let Some(party) = party else { return Ok(0) };
    let party = uuid(party)?;
    let changed=sqlx::query("UPDATE game_party_members m SET presence_state=3,presence_channel_id=NULL,presence_revision=presence_revision+1,absence_observed_at=COALESCE(absence_observed_at,clock_timestamp()) WHERE m.party_id=encode($1,'hex')::uuid AND (m.presence_state<>3 OR m.absence_observed_at IS NULL) AND NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.character_id=m.character_id AND s.session_state IN(1,2))")
        .bind(party.as_slice()).execute(&mut **tx).await?.rows_affected();
    if changed > 0 {
        sqlx::query(
            "UPDATE game_parties SET revision=revision+1 WHERE party_id=encode($1,'hex')::uuid",
        )
        .bind(party.as_slice())
        .execute(&mut **tx)
        .await?;
    }
    usize::try_from(changed).map_err(|_| reject("party offline presence bound"))
}
impl super::DurabilityRoot {
    pub async fn drain_world_party_offline_presence(
        &self,
        permit: &super::spell_owner_commit::SpellLanePermit,
        recovery: &super::character_authority::ReconciledCharacterAuthority<'_, '_>,
        node: &super::runtime_scope_assignment::NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: u64,
    ) -> Result<usize> {
        let record = recovery
            .record_for(self)
            .map_err(|_| reject("party offline recovery authority"))?;
        let node = node.clone();
        let mut context = permit;
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, permit| {
                Box::pin(async move {
                    let mut tx = super::spell_item_transaction::begin_spell_owner_transaction(
                        holder, deadline,
                    )
                    .await?;
                    let authority =
                        super::spell_item_transaction::assert_spell_item_scope_with_recovery(
                            &mut tx, permit, &record, &node, scope, generation,
                        )
                        .await
                        .map_err(|_| super::DurabilityError::Unavailable)?;
                    let count =
                        cleanup_world_party_offline_presence_in_transaction(&mut tx, &authority)
                            .await
                            .map_err(|_| super::DurabilityError::Unavailable)?;
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(count)
                })
            })
            .await
            .map_err(SpellItemError::from)
    }
}
