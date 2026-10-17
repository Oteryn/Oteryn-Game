//! HOUSE-ACL-1 private native owner. UNINSTALLED candidate property/guild dependencies.
//! No production access-function replacement, grants, registration, wire or content authority.
//! Receipts/views are data, not admission, placement or continuing action capabilities.
#![allow(
    dead_code,
    reason = "Owned private component; production composition remains CP"
)]
#[path = "../house/acl.rs"]
pub(crate) mod rules;
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence, uuid_text};
use super::character_progression::assert_item_only_fence;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::house_scope_handoff::HouseId;
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::spell_house_abi::HouseAccess;
use super::{DurabilityError, DurabilityRoot};
use crate::character_recovery_fence::CharacterRecoveryFenceV1;
use crate::domain::CharacterId;
use crate::foundation::{CommandRef, RuntimeScopeRefV1};
use crate::map::overlay::TilePos;
use rules::{Entry, GuildId, GuildState, List, ListMatch, Match, Membership, Rank};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};
use std::collections::{BTreeMap, BTreeSet};

type Result<T> = std::result::Result<T, AclError>;
#[derive(Debug)]
pub(crate) enum AclError {
    InvalidInput,
    AuthorityRejected,
    NotOwner,
    StaleRevision,
    ConflictingOccurrence,
    RetrySourcesChanged,
    GuildDisbanding,
    RevisionExhausted,
    Unavailable(DurabilityError),
}
impl From<DurabilityError> for AclError {
    fn from(e: DurabilityError) -> Self {
        Self::Unavailable(e)
    }
}
impl From<sqlx::Error> for AclError {
    fn from(e: sqlx::Error) -> Self {
        Self::Unavailable(e.into())
    }
}
impl From<rules::RuleError> for AclError {
    fn from(_: rules::RuleError) -> Self {
        Self::InvalidInput
    }
}
fn corrupt() -> AclError {
    DurabilityError::InvalidStoredState.into()
}
fn number(s: String) -> Result<u64> {
    s.parse().map_err(|_| corrupt())
}
fn character(s: String) -> Result<CharacterId> {
    CharacterId::from_bytes(uuid_text(&s)?).map_err(|_| corrupt())
}
fn guild(s: String) -> Result<GuildId> {
    GuildId::from_bytes(uuid_text(&s)?).map_err(|_| corrupt())
}

#[derive(Clone)]
pub(crate) struct AclSet {
    pub(crate) house: HouseId,
    pub(crate) command: CommandRef,
    pub(crate) list: List,
    pub(crate) expected_revision: u64,
    pub(crate) entries: Vec<Entry>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AclReceipt {
    pub(crate) operation: [u8; 16],
    pub(crate) revision: u64,
    pub(crate) replayed: bool,
}
/// Read observation only; native action owners must retain current scope/placement/custody.
pub(crate) struct AclView {
    pub(crate) role: HouseAccess,
    pub(crate) revision: u64,
    /// Selected-list grant only, not full door permission. Absent/empty both yield no grant.
    /// The composed runtime independently determines whether this door is restricted.
    pub(crate) door_list_granted: Option<bool>,
    pub(crate) guild_revisions: BTreeMap<GuildId, u64>,
}
fn slot(list: List) -> (i16, i32, i32, i16) {
    match list {
        List::Subowners => (1, -1, -1, -99),
        List::Guests => (2, -1, -1, -99),
        List::Door(p) => (3, i32::from(p.x), i32::from(p.y), i16::from(p.floor)),
    }
}
fn binding(actor: CharacterId, request: &AclSet, entries: &BTreeSet<Entry>) -> Vec<u8> {
    let mut h = Sha256::new();
    h.update(b"HOUSE-ACL-1/REQUEST/v1\0");
    h.update(actor.as_bytes());
    h.update(request.house.world_id().as_bytes());
    h.update((request.house.house_key().len() as u64).to_be_bytes());
    h.update(request.house.house_key().as_bytes());
    let (kind, x, y, z) = slot(request.list);
    h.update(kind.to_be_bytes());
    h.update(x.to_be_bytes());
    h.update(y.to_be_bytes());
    h.update(z.to_be_bytes());
    h.update(request.expected_revision.to_be_bytes());
    for e in entries {
        let (kind, id, m, l) = entry_data(*e);
        h.update(kind.to_be_bytes());
        h.update(id);
        h.update(m.to_be_bytes());
        h.update(l.to_be_bytes());
    }
    h.finalize().to_vec()
}
fn entry_data(e: Entry) -> (i16, [u8; 16], i16, i16) {
    match e {
        Entry::Character(id) => (1, *id.as_bytes(), 0, 0),
        Entry::Exclude(id) => (2, *id.as_bytes(), 0, 0),
        Entry::Guild {
            guild,
            matching,
            level,
        } => (
            3,
            *guild.as_bytes(),
            if matching == Match::Exact { 1 } else { 2 },
            i16::from(level.level()),
        ),
    }
}

impl DurabilityRoot {
    pub(crate) async fn replace_house_acl(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: AclSet,
    ) -> Result<AclReceipt> {
        request.list.validate()?;
        let entries = rules::checked_entries(&request.entries)?;
        let recovery = authority
            .record_for(self)
            .map_err(|_| AclError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    match replace_in_transaction(
                        &mut tx, &recovery, &node, fence, &request, &entries,
                    )
                    .await
                    {
                        Ok(value) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(AclError::Unavailable(e)) => Err(e),
                        Err(e) => Ok(Err(e)),
                    }
                })
            })
            .await?
    }
    pub(crate) async fn read_current_house_acl(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        house: HouseId,
        door: Option<TilePos>,
    ) -> Result<AclView> {
        if let Some(p) = door {
            List::Door(p).validate()?;
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| AclError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let value = async {
                        assert_recovery_fence(&mut tx, &recovery).await?;
                        lock_admission_relations(&mut tx).await?;
                        current_actor(&mut tx, &node, fence, &house).await?;
                        let source = locked_sources(
                            &mut tx,
                            fence.character_id,
                            &house,
                            door,
                            &BTreeSet::new(),
                            Lock::Read,
                        )
                        .await?;
                        source.view(fence.character_id, door)
                    }
                    .await;
                    match value {
                        Ok(value) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(value))
                        }
                        Err(AclError::Unavailable(e)) => Err(e),
                        Err(e) => Ok(Err(e)),
                    }
                })
            })
            .await?
    }
}
async fn current_actor(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    house: &HouseId,
) -> Result<()> {
    let RuntimeScopeRefV1::Channel { world_id, .. } = fence.runtime_scope else {
        return Err(AclError::AuthorityRejected);
    };
    if world_id != house.world_id() {
        return Err(AclError::AuthorityRejected);
    }
    assert_item_only_fence(tx, &fence, node)
        .await?
        .map_err(|_| AclError::AuthorityRejected)?;
    Ok(())
}

async fn replace_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    recovery: &CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    request: &AclSet,
    entries: &BTreeSet<Entry>,
) -> Result<AclReceipt> {
    assert_recovery_fence(tx, recovery).await?;
    lock_admission_relations(tx).await?;
    if request.command.game_session_id() != fence.game_session_id {
        return Err(AclError::AuthorityRejected);
    }
    let b = binding(fence.character_id, request, entries);
    // Row-backed occurrence serialization before Character/root/property locks. A typed refusal
    // leaves this insertion to roll back; incomplete occurrences cannot commit in the proposal.
    sqlx::query("INSERT INTO game_house_acl_operations(game_session_id,command_id,world_id,house_key,character_id,binding) VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,$4,encode($5,'hex')::uuid,$6) ON CONFLICT(game_session_id,command_id) DO NOTHING")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(request.house.world_id().as_bytes().as_slice()).bind(request.house.house_key()).bind(fence.character_id.as_bytes().as_slice()).bind(&b).execute(&mut **tx).await?;
    let row=sqlx::query("SELECT operation_id::text,binding,revision_after::text FROM game_house_acl_operations WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric FOR UPDATE")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).fetch_one(&mut **tx).await?;
    if row.try_get::<Vec<u8>, _>("binding")? != b {
        return Err(AclError::ConflictingOccurrence);
    }
    let operation = uuid_text(row.try_get("operation_id")?)?;
    if let Some(rev) = row.try_get::<Option<String>, _>("revision_after")? {
        // Historical result only; no session/controller/owner grant is reacquired.
        return Ok(AclReceipt {
            operation,
            revision: number(rev)?,
            replayed: true,
        });
    }
    current_actor(tx, node, fence, &request.house).await?;
    let door = if let List::Door(p) = request.list {
        Some(p)
    } else {
        None
    };
    let source = locked_sources(
        tx,
        fence.character_id,
        &request.house,
        door,
        entries,
        Lock::Write,
    )
    .await?;
    let view = source.view(fence.character_id, door)?;
    if !rules::may_edit(view.role, request.list) {
        return Err(AclError::NotOwner);
    }
    if source.owner_guild.is_some_and(|g| {
        source
            .guilds
            .get(&g)
            .is_some_and(|f| !matches!(f.state, GuildState::Forming | GuildState::Active))
    }) {
        return Err(AclError::GuildDisbanding);
    }
    if view.revision != request.expected_revision {
        return Err(AclError::StaleRevision);
    }
    let next = view
        .revision
        .checked_add(1)
        .ok_or(AclError::RevisionExhausted)?;
    validate_subjects(tx, &request.house, entries, &source.guilds).await?;
    let (kind, x, y, z) = slot(request.list);
    sqlx::query("DELETE FROM game_house_acl_entries WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND list_kind=$3 AND door_x=$4 AND door_y=$5 AND door_floor=$6")
        .bind(request.house.world_id().as_bytes().as_slice()).bind(request.house.house_key()).bind(kind).bind(x).bind(y).bind(z).execute(&mut **tx).await?;
    for e in entries {
        let (entry_kind, subject, m, l) = entry_data(*e);
        sqlx::query("INSERT INTO game_house_acl_entries(world_id,house_key,list_kind,door_x,door_y,door_floor,entry_kind,subject_id,match_kind,rank_level) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5,$6,$7,encode($8,'hex')::uuid,$9,$10)")
            .bind(request.house.world_id().as_bytes().as_slice()).bind(request.house.house_key()).bind(kind).bind(x).bind(y).bind(z).bind(entry_kind).bind(subject.as_slice()).bind(m).bind(l).execute(&mut **tx).await?;
    }
    let changed=sqlx::query("UPDATE game_house_properties SET acl_revision=$3::text::numeric WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND acl_revision=$4::text::numeric")
        .bind(request.house.world_id().as_bytes().as_slice()).bind(request.house.house_key()).bind(next.to_string()).bind(view.revision.to_string()).execute(&mut **tx).await?.rows_affected();
    if changed != 1 {
        return Err(AclError::StaleRevision);
    }
    // Private immutable change evidence. This is NOT a registered HouseEvent envelope.
    let data = serde_json::json!({"property_revision":source.property_revision.to_string(),"acl_before":view.revision.to_string(),"acl_after":next.to_string(),"list":[kind,x,y,z],"entries":entries.iter().map(|e|{let(k,id,m,l)=entry_data(*e);serde_json::json!([k,id,m,l])}).collect::<Vec<_>>(),"connection_generation":fence.connection_generation.get().to_string(),"character_lease_generation":fence.character_lease_generation.to_string(),"scope_ownership_generation":fence.scope_ownership_generation.get().to_string(),"guild_revisions":source.guilds.iter().map(|(g,f)|serde_json::json!([g.as_bytes(),f.revision.to_string()])).collect::<Vec<_>>()});
    sqlx::query("UPDATE game_house_acl_operations SET revision_before=$3::text::numeric,revision_after=$4::text::numeric,change_data=$5 WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(request.command.command_id().get().to_string()).bind(view.revision.to_string()).bind(next.to_string()).bind(&data).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_house_acl_change_outbox(operation_id,change_data) VALUES(encode($1,'hex')::uuid,$2)").bind(operation.as_slice()).bind(data).execute(&mut **tx).await?;
    Ok(AclReceipt {
        operation,
        revision: next,
        replayed: false,
    })
}

struct GuildFact {
    state: GuildState,
    revision: u64,
}
struct Sources {
    property_revision: u64,
    revision: u64,
    owner: Option<CharacterId>,
    owner_guild: Option<GuildId>,
    entries: BTreeMap<(i16, i32, i32, i16), BTreeSet<Entry>>,
    guilds: BTreeMap<GuildId, GuildFact>,
    member: Option<Membership>,
    ranks: BTreeSet<Rank>,
}
impl Sources {
    fn list(&self, list: List) -> BTreeSet<Entry> {
        self.entries.get(&slot(list)).cloned().unwrap_or_default()
    }
    fn view(&self, actor: CharacterId, door: Option<TilePos>) -> Result<AclView> {
        let owner = self.owner == Some(actor)
            || self.owner_guild.is_some_and(|g| {
                self.member.is_some_and(|m| {
                    m.guild == g && m.level.level() == 1 && m.state != GuildState::Disbanded
                })
            });
        let role = if owner {
            HouseAccess::Owner
        } else {
            rules::role(
                rules::match_list(&self.list(List::Subowners), actor, self.member, &self.ranks),
                rules::match_list(&self.list(List::Guests), actor, self.member, &self.ranks),
            )
        };
        let door_list_granted = door.map(|p| {
            owner
                || (role != HouseAccess::NotInvited
                    && rules::match_list(
                        &self.list(List::Door(p)),
                        actor,
                        self.member,
                        &self.ranks,
                    ) == ListMatch::Granted)
        });
        Ok(AclView {
            role,
            revision: self.revision,
            door_list_granted,
            guild_revisions: self
                .guilds
                .iter()
                .map(|(id, f)| (*id, f.revision))
                .collect(),
        })
    }
}
#[derive(Clone, Copy)]
enum Lock {
    Read,
    Write,
}
fn selected(door: Option<TilePos>) -> (bool, i32, i32, i16) {
    door.map_or((false, 0, 0, 0), |p| {
        (true, i32::from(p.x), i32::from(p.y), i16::from(p.floor))
    })
}
async fn preflight_guilds(
    tx: &mut Transaction<'_, Postgres>,
    actor: CharacterId,
    house: &HouseId,
    door: Option<TilePos>,
    requested: &BTreeSet<Entry>,
) -> Result<(Option<GuildId>, Option<GuildId>, BTreeSet<GuildId>)> {
    let owner:Option<Option<String>>=sqlx::query_scalar("SELECT owner_guild_id::text FROM game_house_properties WHERE world_id=encode($1,'hex')::uuid AND house_key=$2")
        .bind(house.world_id().as_bytes().as_slice()).bind(house.house_key()).fetch_optional(&mut **tx).await?;
    let owner = owner.flatten().map(guild).transpose()?;
    let (has, x, y, z) = selected(door);
    let ids:Vec<String>=sqlx::query_scalar("SELECT DISTINCT subject_id::text FROM game_house_acl_entries WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND entry_kind=3 AND (list_kind IN(1,2) OR ($3 AND list_kind=3 AND door_x=$4 AND door_y=$5 AND door_floor=$6)) LIMIT 601")
        .bind(house.world_id().as_bytes().as_slice()).bind(house.house_key()).bind(has).bind(x).bind(y).bind(z).fetch_all(&mut **tx).await?;
    if ids.len() > 600 {
        return Err(corrupt());
    }
    let mut required = ids
        .into_iter()
        .map(guild)
        .collect::<Result<BTreeSet<_>>>()?;
    required.extend(owner);
    required.extend(requested.iter().filter_map(|e| {
        if let Entry::Guild { guild, .. } = e {
            Some(*guild)
        } else {
            None
        }
    }));
    let mut acting_guild = None;
    if !required.is_empty() {
        let actor_guild:Option<String>=sqlx::query_scalar("SELECT guild_id::text FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid").bind(actor.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        if let Some(id) = actor_guild {
            let id = guild(id)?;
            required.insert(id);
            acting_guild = Some(id);
        }
    }
    Ok((owner, acting_guild, required))
}
async fn locked_sources(
    tx: &mut Transaction<'_, Postgres>,
    actor: CharacterId,
    house: &HouseId,
    door: Option<TilePos>,
    requested: &BTreeSet<Entry>,
    lock: Lock,
) -> Result<Sources> {
    let (owner_pre, acting_pre, required) =
        preflight_guilds(tx, actor, house, door, requested).await?;
    let mut guilds = BTreeMap::new();
    for id in required {
        let sql = if matches!(lock, Lock::Write)
            && (owner_pre == Some(id) || acting_pre == Some(id))
        {
            "SELECT world_id::text,state,revision::text FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid FOR UPDATE"
        } else {
            "SELECT world_id::text,state,revision::text FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid FOR SHARE"
        };
        let r = sqlx::query(sql)
            .bind(id.as_bytes().as_slice())
            .fetch_optional(&mut **tx)
            .await?
            .ok_or(AclError::InvalidInput)?;
        if uuid_text(r.try_get("world_id")?)? != *house.world_id().as_bytes() {
            return Err(AclError::InvalidInput);
        }
        let state = match r.try_get::<i16, _>("state")? {
            1 => GuildState::Forming,
            2 => GuildState::Active,
            3 => GuildState::Disbanding,
            4 => GuildState::Disbanded,
            _ => return Err(corrupt()),
        };
        let revision = number(r.try_get("revision")?)?;
        if revision == 0 {
            return Err(corrupt());
        }
        guilds.insert(id, GuildFact { state, revision });
    }
    let mut member = None;
    let mut ranks = BTreeSet::new();
    if !guilds.is_empty() {
        let row=sqlx::query("SELECT guild_id::text,rank_level FROM game_guild_members WHERE character_id=encode($1,'hex')::uuid FOR SHARE").bind(actor.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        if let Some(r) = row {
            let id = guild(r.try_get("guild_id")?)?;
            let facts = guilds.get(&id).ok_or(AclError::RetrySourcesChanged)?;
            let level =
                Rank::new(u8::try_from(r.try_get::<i16, _>("rank_level")?).map_err(|_| corrupt())?)
                    .map_err(|_| corrupt())?;
            member = Some(Membership {
                guild: id,
                level,
                state: facts.state,
            });
            // GUILD0 lifecycle/rank writers lock this already-held guild FOR UPDATE;
            // no extra rank UPDATE privilege/late lock is needed for this observation.
            let levels:Vec<i16>=sqlx::query_scalar("SELECT level FROM game_guild_ranks WHERE guild_id=encode($1,'hex')::uuid ORDER BY level").bind(id.as_bytes().as_slice()).fetch_all(&mut **tx).await?;
            for n in levels {
                ranks.insert(
                    Rank::new(u8::try_from(n).map_err(|_| corrupt())?).map_err(|_| corrupt())?,
                );
            }
            if !ranks.contains(&level) {
                return Err(corrupt());
            }
        }
    }
    let sql = if matches!(lock, Lock::Write) {
        "SELECT owner_kind,owner_character_id::text,owner_account_id::text,owner_guild_id::text,revision::text,acl_revision::text,state FROM game_house_properties WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 FOR UPDATE"
    } else {
        "SELECT owner_kind,owner_character_id::text,owner_account_id::text,owner_guild_id::text,revision::text,acl_revision::text,state FROM game_house_properties WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 FOR SHARE"
    };
    let property = sqlx::query(sql)
        .bind(house.world_id().as_bytes().as_slice())
        .bind(house.house_key())
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(AclError::NotOwner)?;
    let owner_guild = property
        .try_get::<Option<String>, _>("owner_guild_id")?
        .map(guild)
        .transpose()?;
    if owner_guild != owner_pre || owner_guild.is_some_and(|g| !guilds.contains_key(&g)) {
        return Err(AclError::RetrySourcesChanged);
    }
    let owner = property
        .try_get::<Option<String>, _>("owner_character_id")?
        .map(character)
        .transpose()?;
    let account = property.try_get::<Option<String>, _>("owner_account_id")?;
    match property
        .try_get::<Option<String>, _>("owner_kind")?
        .as_deref()
    {
        Some("CHARACTER") if owner.is_some() && account.is_some() && owner_guild.is_none() => {
            let r=sqlx::query("SELECT account_id::text,world_id::text,lifecycle FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(owner.ok_or_else(corrupt)?.as_bytes().as_slice()).fetch_optional(&mut **tx).await?.ok_or_else(corrupt)?;
            if Some(r.try_get::<String, _>("account_id")?) != account
                || uuid_text(r.try_get("world_id")?)? != *house.world_id().as_bytes()
                || r.try_get::<i16, _>("lifecycle")? != 1
            {
                return Err(corrupt());
            }
        }
        Some("GUILD") if owner.is_none() && account.is_none() && owner_guild.is_some() => {}
        _ => return Err(AclError::NotOwner),
    }
    if !matches!(
        property.try_get::<String, _>("state")?.as_str(),
        "OWNED" | "MOVE_OUT_PENDING" | "DISPOSITION"
    ) {
        return Err(AclError::NotOwner);
    }
    let (has, x, y, z) = selected(door);
    let rows=sqlx::query("SELECT list_kind,door_x,door_y,door_floor,entry_kind,subject_id::text,match_kind,rank_level FROM game_house_acl_entries WHERE world_id=encode($1,'hex')::uuid AND house_key=$2 AND (list_kind IN(1,2) OR ($3 AND list_kind=3 AND door_x=$4 AND door_y=$5 AND door_floor=$6)) ORDER BY list_kind,door_x,door_y,door_floor,entry_kind,subject_id,match_kind,rank_level LIMIT 601 FOR SHARE")
        .bind(house.world_id().as_bytes().as_slice()).bind(house.house_key()).bind(has).bind(x).bind(y).bind(z).fetch_all(&mut **tx).await?;
    if rows.len() > 600 {
        return Err(corrupt());
    }
    let mut lists: BTreeMap<_, BTreeSet<_>> = BTreeMap::new();
    for r in rows {
        let kind = r.try_get::<i16, _>("list_kind")?;
        let x = r.try_get::<i32, _>("door_x")?;
        let y = r.try_get::<i32, _>("door_y")?;
        let z = r.try_get::<i16, _>("door_floor")?;
        if !(matches!((kind, x, y, z), (1 | 2, -1, -1, -99))
            || (kind == 3
                && (0..=65535).contains(&x)
                && (0..=65535).contains(&y)
                && (-15..=0).contains(&z)))
        {
            return Err(corrupt());
        }
        let m = r.try_get::<i16, _>("match_kind")?;
        let level = r.try_get::<i16, _>("rank_level")?;
        let e = match r.try_get::<i16, _>("entry_kind")? {
            1 if m == 0 && level == 0 => Entry::Character(character(r.try_get("subject_id")?)?),
            2 if m == 0 && level == 0 => Entry::Exclude(character(r.try_get("subject_id")?)?),
            3 => {
                let id = guild(r.try_get("subject_id")?)?;
                if !guilds.contains_key(&id) {
                    return Err(AclError::RetrySourcesChanged);
                }
                Entry::Guild {
                    guild: id,
                    matching: match m {
                        1 => Match::Exact,
                        2 => Match::AtLeast,
                        _ => return Err(corrupt()),
                    },
                    level: Rank::new(u8::try_from(level).map_err(|_| corrupt())?)
                        .map_err(|_| corrupt())?,
                }
            }
            _ => return Err(corrupt()),
        };
        let list = lists.entry((kind, x, y, z)).or_default();
        if !list.insert(e) || list.len() > rules::LIST_MAX {
            return Err(corrupt());
        }
    }
    let property_revision = number(property.try_get("revision")?)?;
    if property_revision == 0 {
        return Err(corrupt());
    }
    Ok(Sources {
        property_revision,
        revision: number(property.try_get("acl_revision")?)?,
        owner,
        owner_guild,
        entries: lists,
        guilds,
        member,
        ranks,
    })
}
async fn validate_subjects(
    tx: &mut Transaction<'_, Postgres>,
    house: &HouseId,
    entries: &BTreeSet<Entry>,
    guilds: &BTreeMap<GuildId, GuildFact>,
) -> Result<()> {
    for e in entries {
        match e {
            Entry::Character(id) | Entry::Exclude(id) => {
                let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND lifecycle=1)").bind(id.as_bytes().as_slice()).bind(house.world_id().as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                if !valid {
                    return Err(AclError::InvalidInput);
                }
            }
            Entry::Guild { guild, .. } => {
                if !guilds.contains_key(guild) {
                    return Err(AclError::RetrySourcesChanged);
                }
            }
        }
    }
    Ok(())
}

/// Candidate private DDL, never a runtime installer. HOUSE1a/GUILD1 must admit their owner
/// schema and lifecycle first; deployment, grants and immutable owner-definers are CP-only.
/// Occurrence metadata deliberately has no property/root FK: early FK key locks would violate
/// occurrence → actor → guild → property ordering. Current native reads validate new writes.
/// The property prerequisite is intentionally represented separately from this ACL owner's DDL.
#[cfg(test)]
const PROPERTY_PREREQUISITE_SCHEMA_PROPOSAL: &str = r#"
CREATE TABLE game_house_properties(
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),house_key TEXT NOT NULL CHECK(game_house_scope_key_valid(house_key)),
 owner_kind TEXT,owner_character_id UUID REFERENCES game_character_roots,owner_account_id UUID,owner_guild_id UUID,
 revision NUMERIC(20,0) NOT NULL CHECK(revision BETWEEN 1 AND 18446744073709551615),
 acl_revision NUMERIC(20,0) NOT NULL CHECK(acl_revision BETWEEN 0 AND 18446744073709551615),
 state TEXT NOT NULL CHECK(state IN('VACANT','AUCTION','OWNED','MOVE_OUT_PENDING','DISPOSITION','RETIRED')),
 content_fence_operation UUID,PRIMARY KEY(world_id,house_key),
 CHECK(((owner_kind IS NULL AND owner_character_id IS NULL AND owner_account_id IS NULL AND owner_guild_id IS NULL)
 OR (owner_kind='CHARACTER' AND owner_character_id IS NOT NULL AND owner_account_id IS NOT NULL AND owner_guild_id IS NULL)
 OR (owner_kind='GUILD' AND owner_character_id IS NULL AND owner_account_id IS NULL AND owner_guild_id IS NOT NULL)) IS TRUE));
"#;

#[cfg(test)]
const HOUSE_ACL_SCHEMA_PROPOSAL: &str = r#"
CREATE TABLE game_house_acl_entries(
 world_id UUID NOT NULL,house_key TEXT NOT NULL,list_kind SMALLINT NOT NULL,door_x INTEGER NOT NULL,door_y INTEGER NOT NULL,door_floor SMALLINT NOT NULL,
 entry_kind SMALLINT NOT NULL CHECK(entry_kind IN(1,2,3)),subject_id UUID NOT NULL CHECK(game_character_is_uuid_v7(subject_id)),
 match_kind SMALLINT NOT NULL,rank_level SMALLINT NOT NULL,
 FOREIGN KEY(world_id,house_key) REFERENCES game_house_properties,
 CHECK((list_kind IN(1,2) AND door_x=-1 AND door_y=-1 AND door_floor=-99) OR (list_kind=3 AND door_x BETWEEN 0 AND 65535 AND door_y BETWEEN 0 AND 65535 AND door_floor BETWEEN -15 AND 0)),
 CHECK((entry_kind IN(1,2) AND match_kind=0 AND rank_level=0) OR (entry_kind=3 AND match_kind IN(1,2) AND rank_level BETWEEN 1 AND 20)),
 PRIMARY KEY(world_id,house_key,list_kind,door_x,door_y,door_floor,entry_kind,subject_id,match_kind,rank_level));
CREATE TABLE game_house_acl_operations(
 game_session_id UUID NOT NULL CHECK(game_character_is_uuid_v7(game_session_id)),command_id NUMERIC(20,0) NOT NULL CHECK(command_id BETWEEN 1 AND 18446744073709551615),
 operation_id UUID NOT NULL DEFAULT game_character_uuid_v7() UNIQUE CHECK(game_character_is_uuid_v7(operation_id)),
 world_id UUID NOT NULL CHECK(game_character_is_uuid_v7(world_id)),house_key TEXT NOT NULL CHECK(game_house_scope_key_valid(house_key)),character_id UUID NOT NULL CHECK(game_character_is_uuid_v7(character_id)),
 binding BYTEA NOT NULL CHECK(octet_length(binding)=32),revision_before NUMERIC(20,0),revision_after NUMERIC(20,0),change_data JSONB,
 PRIMARY KEY(game_session_id,command_id));
CREATE TABLE game_house_acl_change_outbox(operation_id UUID PRIMARY KEY REFERENCES game_house_acl_operations(operation_id),change_data JSONB NOT NULL);
CREATE FUNCTION game_house_acl_immutable() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
 IF TG_OP='DELETE' OR TG_TABLE_NAME='game_house_acl_change_outbox' OR OLD.revision_after IS NOT NULL THEN RAISE EXCEPTION 'immutable ACL evidence'; END IF;
 IF (NEW.game_session_id,NEW.command_id,NEW.operation_id,NEW.world_id,NEW.house_key,NEW.character_id,NEW.binding)
 IS DISTINCT FROM (OLD.game_session_id,OLD.command_id,OLD.operation_id,OLD.world_id,OLD.house_key,OLD.character_id,OLD.binding) THEN RAISE EXCEPTION 'ACL binding changed'; END IF;RETURN NEW;END $$;
CREATE TRIGGER house_acl_operation_immutable BEFORE UPDATE OR DELETE ON game_house_acl_operations FOR EACH ROW EXECUTE FUNCTION game_house_acl_immutable();
CREATE TRIGGER house_acl_outbox_immutable BEFORE UPDATE OR DELETE ON game_house_acl_change_outbox FOR EACH ROW EXECUTE FUNCTION game_house_acl_immutable();
CREATE FUNCTION game_house_acl_complete() RETURNS trigger LANGUAGE plpgsql AS $$ DECLARE op game_house_acl_operations;BEGIN
 SELECT * INTO op FROM game_house_acl_operations WHERE operation_id=NEW.operation_id;
 IF op.revision_before IS NULL OR op.revision_before<0 OR op.revision_after IS NULL OR op.revision_after<>op.revision_before+1 OR op.revision_after>18446744073709551615 OR op.change_data IS NULL
 OR NOT EXISTS(SELECT 1 FROM game_house_acl_change_outbox o WHERE o.operation_id=op.operation_id AND o.change_data=op.change_data)
 THEN RAISE EXCEPTION 'incomplete ACL occurrence'; END IF;RETURN NULL;END $$;
CREATE CONSTRAINT TRIGGER house_acl_operation_complete AFTER INSERT OR UPDATE ON game_house_acl_operations DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_acl_complete();
CREATE FUNCTION game_house_acl_bound() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN
 IF (SELECT count(*) FROM game_house_acl_entries e WHERE (e.world_id,e.house_key,e.list_kind,e.door_x,e.door_y,e.door_floor)=(NEW.world_id,NEW.house_key,NEW.list_kind,NEW.door_x,NEW.door_y,NEW.door_floor))>200 THEN RAISE EXCEPTION 'ACL list exceeds200';END IF;RETURN NULL;END $$;
CREATE CONSTRAINT TRIGGER house_acl_list_bound AFTER INSERT OR UPDATE ON game_house_acl_entries DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_house_acl_bound();
"#;

#[cfg(test)]
#[path = "../house/acl_tests.rs"]
mod tests;
