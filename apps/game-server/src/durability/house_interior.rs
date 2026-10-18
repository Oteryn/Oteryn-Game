//! HOUSE-RUNTIME-1a house-side transition facts (accepted packet §2.1).
//!
//! Native reservation/retry/replay owner and atomic House shape writer. Current
//! named session, assignment and access are independently re-read under native
//! locks; historical handoff and request metadata are never current authority.
//! Production property/tile admission stays fail-closed until its owning writer.

use super::house_scope_handoff::{HOUSE_TILE_MAX_BYTES, HouseId};
use super::item_mint_audit::check_uuid_v7;
use crate::domain::ItemInstanceId;
use crate::foundation::CharacterId;

/// HOUSEOWN0-RL-14, enforced by the native SQL shapes under the house lock.
pub const HOUSE_INTERIOR_ITEMS_MAX: u64 = 2_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseInteriorError {
    InvalidLocation,
    InvalidProvenance,
    CrossHouseMove,
    NoAccess,
}

/// House endpoint audit facts, not proof of tile membership or current scope authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseInteriorLocation {
    pub house: HouseId,
    pub spatial_position: Vec<u8>,
    pub stack_ordinal: u64,
    pub scope_ownership_generation: u64,
    pub placement_transaction_id: [u8; 16],
}

impl HouseInteriorLocation {
    pub fn validate(&self) -> Result<(), HouseInteriorError> {
        if self.spatial_position.is_empty()
            || self.spatial_position.len() > HOUSE_TILE_MAX_BYTES
            || self.stack_ordinal == 0
            || self.scope_ownership_generation == 0
            || check_uuid_v7(&self.placement_transaction_id).is_err()
        {
            return Err(HouseInteriorError::InvalidLocation);
        }
        Ok(())
    }
}

/// Migration 0025 provenance: it is never current access authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HousingReclaimProvenance {
    pub house: HouseId,
    pub item_instance_id: ItemInstanceId,
    pub reclaim_subject: CharacterId,
    pub placement_transaction_id: [u8; 16],
    pub revision: u64,
}

/// The house portion of each accepted shape. Character-side source/destination,
/// transaction identities and command/session fences still belong to the writer/event.
/// The enum prevents missing or duplicate house endpoints without inventing a wire layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HouseInteriorEffect {
    Place {
        destination: HouseInteriorLocation,
        provenance: HousingReclaimProvenance,
    },
    Take {
        source: HouseInteriorLocation,
        provenance: HousingReclaimProvenance,
    },
    TileMove {
        source: HouseInteriorLocation,
        destination: HouseInteriorLocation,
        provenance_before: HousingReclaimProvenance,
        provenance_after: HousingReclaimProvenance,
    },
}

impl HouseInteriorEffect {
    /// Checks immutable event relationships. It neither authorizes nor mutates anything.
    pub fn validate(
        &self,
        item: ItemInstanceId,
        actor: CharacterId,
    ) -> Result<(), HouseInteriorError> {
        let check = |location: &HouseInteriorLocation, provenance: &HousingReclaimProvenance| {
            location.validate()?;
            if provenance.house != location.house
                || provenance.item_instance_id != item
                || provenance.revision == 0
                || provenance.placement_transaction_id != location.placement_transaction_id
            {
                return Err(HouseInteriorError::InvalidProvenance);
            }
            Ok(())
        };
        match self {
            Self::Place {
                destination,
                provenance,
            } => {
                check(destination, provenance)?;
                if provenance.reclaim_subject != actor {
                    return Err(HouseInteriorError::InvalidProvenance);
                }
            }
            Self::Take { source, provenance } => check(source, provenance)?,
            Self::TileMove {
                source,
                destination,
                provenance_before,
                provenance_after,
            } => {
                check(source, provenance_before)?;
                check(destination, provenance_after)?;
                if source.house != destination.house {
                    return Err(HouseInteriorError::CrossHouseMove);
                }
                if source.scope_ownership_generation != destination.scope_ownership_generation
                    || source.placement_transaction_id == destination.placement_transaction_id
                    || provenance_before.reclaim_subject != provenance_after.reclaim_subject
                    || provenance_before.revision.checked_add(1) != Some(provenance_after.revision)
                {
                    return Err(HouseInteriorError::InvalidProvenance);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseItemRole {
    Owner,
    Subowner,
    Guest,
}

impl HouseItemRole {
    /// Exact roles returned by game_house_access; unknown roles fail closed.
    pub fn from_access_role(role: &str) -> Option<Self> {
        match role {
            "OWNER" => Some(Self::Owner),
            "SUBOWNER" => Some(Self::Subowner),
            "GUEST" => Some(Self::Guest),
            _ => None,
        }
    }

    /// HOUSE-RUNTIME-0 §5.2 policy only. The SQL commit must resolve this role
    /// afresh under locks and compare every authorization revision.
    pub fn check_policy(
        self,
        actor: CharacterId,
        effect: &HouseInteriorEffect,
    ) -> Result<(), HouseInteriorError> {
        if let HouseInteriorEffect::Take { provenance, .. } = effect
            && self == Self::Guest
            && provenance.reclaim_subject != actor
        {
            return Err(HouseInteriorError::NoAccess);
        }
        Ok(())
    }
}

use super::character_authority::{
    ReconciledCharacterAuthority, SERVER_BUILD_ID, assert_recovery_fence,
};
use super::db::{
    begin_semantic_transaction, begin_type2_transaction, commit_semantic_transaction,
    lock_admission_relations,
};
use super::house_interior_audit as event;
use super::item_mint::{TypedDefinitionRef, uuid_text};
use super::item_mint_audit::{self as audit, OneItemStateV1, OneItemTypedDefinitionRevisionV1};
use super::item_transfer::CurrentCharacterItemFence;
use super::item_transfer_audit::{
    OneItemCommandRefV1, OneItemTransferCauseV1, TransferEventIdentity,
};
use super::runtime_scope_assignment::{NodeIncarnationProof, prove_current_incarnation};
use super::{DurabilityError, DurabilityRoot};
use crate::domain::CharacterId as DurableCharacterId;
use crate::foundation::CommandRef;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::{Postgres, Row, Transaction};

type WriterResult<T> = Result<T, HouseWriterError>;
type DbResult<T> = Result<T, DurabilityError>;

#[derive(Debug)]
pub enum HouseWriterError {
    InvalidInput,
    NotCurrent,
    ConflictingOccurrence,
    RetryBudgetExhausted,
    NoAccess,
    HouseClosed,
    AccessChanged,
    ContentChanged,
    SourceChanged,
    ItemHasContents,
    NotHouseTile,
    StorageFull,
    InventoryFull,
    NoChange,
    Audit(audit::AuditError),
    Durability(DurabilityError),
}
impl From<DurabilityError> for HouseWriterError {
    fn from(value: DurabilityError) -> Self {
        Self::Durability(value)
    }
}
impl From<audit::AuditError> for HouseWriterError {
    fn from(value: audit::AuditError) -> Self {
        Self::Audit(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HouseCharacterSource {
    Inventory { parent: [u8; 16], ordinal: u64 },
    ContainerSlot,
    Equipment { slot: u8 },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HouseItemAction {
    Place {
        source: HouseCharacterSource,
        position: Vec<u8>,
    },
    Take {
        placement: [u8; 16],
        provenance_revision: u64,
    },
    TileMove {
        placement: [u8; 16],
        provenance_revision: u64,
        position: Vec<u8>,
    },
}
impl HouseItemAction {
    fn shape(&self) -> i16 {
        match self {
            Self::Place { .. } => 5,
            Self::Take { .. } => 6,
            Self::TileMove { .. } => 7,
        }
    }
    fn source_kind(&self) -> i16 {
        match self {
            Self::Place {
                source: HouseCharacterSource::Inventory { .. },
                ..
            } => 1,
            Self::Place {
                source: HouseCharacterSource::ContainerSlot,
                ..
            } => 2,
            Self::Place {
                source: HouseCharacterSource::Equipment { .. },
                ..
            } => 3,
            _ => 4,
        }
    }
}

/// Current compatible Content data for the equipped backpack, exactly as for native
/// item-transfer. This is not a grant: SQL compares the complete stored definition and
/// Character root content/rules revisions and independently allocates the actual entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseBackpackFacts {
    pub definition: TypedDefinitionRef,
    pub capacity: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseItemRequest {
    pub command: CommandRef,
    pub house: HouseId,
    pub item_instance_id: [u8; 16],
    pub action: HouseItemAction,
    pub expected_acl_revision: u64,
    pub expected_guild_revisions: Option<Value>,
    pub backpack: Option<HouseBackpackFacts>,
    pub content_revision: String,
    pub ruleset_revision: String,
    pub sim_revision: String,
}

/// A durable reservation of one immutable intent; never current authority. Private
/// fields prevent manufacturing a different transaction/event identity or retry budget.
#[derive(Debug)]
pub struct HouseItemCandidate {
    frozen: FrozenHouseItem,
}
#[derive(Debug, Clone)]
struct FrozenHouseItem {
    request: HouseItemRequest,
    character: DurableCharacterId,
    binding: Vec<u8>,
    transaction: [u8; 16],
    event: [u8; 16],
    occurred_at: i64,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HouseItemReceipt {
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub occurred_at_unix_ms: i64,
    pub envelope_sha256: [u8; 32],
    /// Present only while the independently expirable audit outbox retains it.
    pub envelope: Option<Vec<u8>>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HouseItemOutcome {
    Committed(HouseItemReceipt),
    AlreadyCommitted(HouseItemReceipt),
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut text = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        let _ = write!(text, "{byte:02x}");
    }
    text
}
fn context(request: &HouseItemRequest, character: DurableCharacterId) -> WriterResult<Value> {
    audit::check_uuid_v7(&request.item_instance_id)?;
    if request.expected_acl_revision == 0 {
        return Err(HouseWriterError::InvalidInput);
    }
    for text in [
        &request.content_revision,
        &request.ruleset_revision,
        &request.sim_revision,
    ] {
        audit::check_content_key(text)?;
    }
    let mut p = json!({"session":hex(request.command.game_session_id().as_bytes()),
        "command":request.command.command_id().get().to_string(),"character":hex(character.as_bytes()),
        "world":hex(request.house.world_id().as_bytes()),"house":request.house.house_key(),
        "item":hex(&request.item_instance_id),"acl_revision":request.expected_acl_revision.to_string(),
        "guild_revisions":request.expected_guild_revisions,"content_revision":request.content_revision,
        "ruleset_revision":request.ruleset_revision,"sim_revision":request.sim_revision,
        "shape":request.action.shape(),"source_kind":request.action.source_kind()});
    match &request.action {
        HouseItemAction::Place { source, position } => {
            match source {
                HouseCharacterSource::Inventory { parent, ordinal } => {
                    audit::check_uuid_v7(parent)?;
                    if *ordinal == 0 || parent == &request.item_instance_id {
                        return Err(HouseWriterError::InvalidInput);
                    }
                    p["source_parent"] = hex(parent).into();
                    p["source_ordinal"] = ordinal.to_string().into();
                }
                HouseCharacterSource::ContainerSlot => {}
                HouseCharacterSource::Equipment { slot } => {
                    if !(1..=10).contains(slot) || *slot == 9 {
                        return Err(HouseWriterError::InvalidInput);
                    }
                    p["source_slot"] = (*slot).into();
                }
            }
            audit::check_technical_bytes(position)?;
            p["destination_position"] = hex(position).into();
        }
        HouseItemAction::Take {
            placement,
            provenance_revision,
        }
        | HouseItemAction::TileMove {
            placement,
            provenance_revision,
            ..
        } => {
            audit::check_uuid_v7(placement)?;
            if *provenance_revision == 0 {
                return Err(HouseWriterError::InvalidInput);
            }
            p["source_placement"] = hex(placement).into();
            p["source_provenance_revision"] = provenance_revision.to_string().into();
            if let HouseItemAction::TileMove { position, .. } = &request.action {
                audit::check_technical_bytes(position)?;
                p["destination_position"] = hex(position).into();
            }
        }
    }
    match (&request.action, &request.backpack) {
        (HouseItemAction::Take { .. }, Some(backpack)) if (1..=20).contains(&backpack.capacity) => {
            let d = &backpack.definition;
            audit::check_definition(Some(&OneItemTypedDefinitionRevisionV1 {
                family: d.family.clone(),
                production_key: d.production_key.clone(),
                revision_ref: d.revision_ref.clone(),
            }))?;
            p["backpack_family"] = d.family.clone().into();
            p["backpack_key"] = d.production_key.clone().into();
            p["backpack_revision"] = d.revision_ref.clone().into();
            p["backpack_capacity"] = backpack.capacity.into();
        }
        (HouseItemAction::Take { .. }, _) => return Err(HouseWriterError::InvalidInput),
        (_, None) => {}
        _ => return Err(HouseWriterError::InvalidInput),
    }
    if p.to_string().len() > 12_000 {
        return Err(HouseWriterError::InvalidInput);
    }
    Ok(p)
}
fn intent_binding(p: &Value) -> Vec<u8> {
    let mut bytes = vec![1];
    bytes.extend_from_slice(
        &Sha256::new()
            .chain_update(b"HOUSE-RUNTIME-1a/intent/v1\0")
            .chain_update(p.to_string())
            .finalize(),
    );
    bytes
}
async fn cause_lock(tx: &mut Transaction<'_, Postgres>, command: CommandRef) -> DbResult<()> {
    let mut key = command.game_session_id().as_bytes().to_vec();
    key.extend_from_slice(&command.command_id().get().to_be_bytes());
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:item-transfer:'||encode($1,'hex'),0))")
        .bind(key).execute(&mut **tx).await?;
    Ok(())
}
async fn reservation(
    tx: &mut Transaction<'_, Postgres>,
    command: CommandRef,
) -> DbResult<Option<sqlx::postgres::PgRow>> {
    Ok(sqlx::query("SELECT transaction_id::text,event_id::text,character_id::text,occurred_at,intent_binding FROM game_item_transfer_reservations WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric FOR UPDATE")
        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut **tx).await?)
}
async fn receipt(
    tx: &mut Transaction<'_, Postgres>,
    command: CommandRef,
    binding: &[u8],
) -> DbResult<WriterResult<Option<HouseItemReceipt>>> {
    let row=sqlx::query("SELECT r.intent_binding,r.transaction_id::text,r.event_id::text,r.character_id::text,r.source_item_instance_id::text,r.source_quantity_before,r.occurred_at,r.shape,r.envelope_sha256,a.envelope,a.envelope_sha256 AS audit_sha256,a.transaction_id::text AS audit_transaction_id,a.event_id IS NOT NULL AS audit_present FROM game_item_transfer_receipts r LEFT JOIN game_item_audit_outbox a ON a.event_id=r.event_id WHERE r.game_session_id=encode($1,'hex')::uuid AND r.command_id=$2::text::numeric")
        .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(Ok(None));
    };
    if row.try_get::<Vec<u8>, _>("intent_binding")? != binding
        || !(5..=7).contains(&row.try_get::<i16, _>("shape")?)
    {
        return Ok(Err(HouseWriterError::ConflictingOccurrence));
    }
    let transaction_id = uuid_text(row.try_get("transaction_id")?)?;
    let event_id = uuid_text(row.try_get("event_id")?)?;
    let occurred_at = row.try_get("occurred_at")?;
    let envelope_sha256: [u8; 32] = row
        .try_get::<Vec<u8>, _>("envelope_sha256")?
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let retained: Option<Vec<u8>> = row.try_get("envelope")?;
    if row.try_get::<bool, _>("audit_present")? {
        let bytes = retained
            .as_ref()
            .ok_or(DurabilityError::InvalidStoredState)?;
        let computed: [u8; 32] = Sha256::digest(bytes).into();
        if row
            .try_get::<Option<String>, _>("audit_transaction_id")?
            .as_deref()
            .map(uuid_text)
            .transpose()?
            .as_ref()
            != Some(&transaction_id)
            || row
                .try_get::<Option<Vec<u8>>, _>("audit_sha256")?
                .as_deref()
                != Some(envelope_sha256.as_slice())
            || computed != envelope_sha256
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let (envelope, message) = event::decode_house_transfer_envelope(bytes)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let state = message
            .before
            .as_ref()
            .ok_or(DurabilityError::InvalidStoredState)?;
        if envelope.event_id != event_id
            || envelope
                .transaction_event
                .as_ref()
                .map(|t| t.transaction_id.as_slice())
                != Some(transaction_id.as_slice())
            || envelope.occurred_at_unix_ms != occurred_at
            || envelope.game_session_id.as_deref()
                != Some(command.game_session_id().as_bytes().as_slice())
            || envelope.command_id != Some(command.command_id().get())
            || message.character_id != uuid_text(row.try_get("character_id")?)?
            || state.item_instance_id != uuid_text(row.try_get("source_item_instance_id")?)?
            || i64::from(state.quantity) != row.try_get::<i64, _>("source_quantity_before")?
        {
            return Err(DurabilityError::InvalidStoredState);
        }
    } else if retained.is_some() {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(Ok(Some(HouseItemReceipt {
        transaction_id,
        event_id,
        occurred_at_unix_ms: occurred_at,
        envelope_sha256,
        envelope: retained,
    })))
}

async fn named_house_current(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    request: &HouseItemRequest,
) -> DbResult<bool> {
    if fence.game_session_id != request.command.game_session_id()
        || request.house.runtime_scope()? != fence.runtime_scope
        || !prove_current_incarnation(tx, node).await?
    {
        return Ok(false);
    }
    Ok(sqlx::query("SELECT 1 FROM game_durability_reconnect_sessions s JOIN game_runtime_scope_assignments a ON a.scope_key=game_house_scope_key(s.world_id,s.runtime_scope_house_key) JOIN game_character_roots r ON r.character_id=s.character_id WHERE s.game_session_id=encode($1,'hex')::uuid AND s.character_id=encode($2,'hex')::uuid AND s.current_generation=$3::text::numeric AND s.character_lease_generation=$4::text::numeric AND s.session_state IN(1,2) AND s.world_id=encode($5,'hex')::uuid AND s.runtime_scope_kind=2 AND s.runtime_scope_world_id=s.world_id AND s.runtime_scope_channel_id IS NULL AND s.runtime_scope_house_key=$6 AND s.runtime_scope_instance_id=game_house_scope_instance_id(s.world_id,$6) AND s.scope_ownership_generation=$7::text::numeric AND a.scope_kind=2 AND a.world_id=s.world_id AND a.house_key=$6 AND a.state=1 AND a.ownership_generation=s.scope_ownership_generation AND a.holder_node_id=encode($8,'hex')::uuid AND a.holder_registration_revision=$9::text::numeric AND r.account_id=s.account_id AND r.world_id=s.world_id AND r.lifecycle=1 FOR SHARE OF s,a,r")
        .bind(fence.game_session_id.as_bytes().as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(fence.connection_generation.get().to_string()).bind(fence.character_lease_generation.to_string()).bind(request.house.world_id().as_bytes().as_slice()).bind(request.house.house_key()).bind(fence.scope_ownership_generation.get().to_string()).bind(node.fact().node_id().as_bytes().as_slice()).bind(node.fact().registration_revision().to_string()).fetch_optional(&mut **tx).await?.is_some())
}

impl DurabilityRoot {
    /// Reserve immutable IDs and DB time under the real recovery/current named House
    /// fence. This moves no item and does not grant access; each commit rechecks storage.
    pub async fn freeze_house_item(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: HouseItemRequest,
    ) -> WriterResult<HouseItemCandidate> {
        let p = context(&request, fence.character_id)?;
        let binding = intent_binding(&p);
        let recovery = authority
            .record_for(self)
            .map_err(|_| HouseWriterError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;cause_lock(&mut tx,request.command).await?;
            if let Some(row)=reservation(&mut tx,request.command).await? {
                if row.try_get::<Vec<u8>,_>("intent_binding")? != binding {return Ok(Err(HouseWriterError::ConflictingOccurrence));}
                let historical=match receipt(&mut tx,request.command,&binding).await? {Ok(v)=>v.is_some(),Err(e)=>return Ok(Err(e))};
                if !historical && !named_house_current(&mut tx,&node,fence,&request).await? {return Ok(Err(HouseWriterError::NotCurrent));}
                let frozen=FrozenHouseItem{request,character:fence.character_id,binding,transaction:uuid_text(row.try_get("transaction_id")?)?,event:uuid_text(row.try_get("event_id")?)?,occurred_at:row.try_get("occurred_at")?};
                if !historical && let Err(error)=run_shape(&mut tx,&node,fence,&frozen,true).await? {return Ok(Err(error));}
                commit_semantic_transaction(tx,deadline).await?;return Ok(Ok(HouseItemCandidate{frozen}));
            }
            if !named_house_current(&mut tx,&node,fence,&request).await? {return Ok(Err(HouseWriterError::NotCurrent));}
            let row=sqlx::query("SELECT game_character_uuid_v7()::text transaction_id,game_character_uuid_v7()::text event_id,floor(extract(epoch FROM statement_timestamp())*1000)::bigint occurred_at").fetch_one(&mut *tx).await?;
            let frozen=FrozenHouseItem{request,character:fence.character_id,binding,transaction:uuid_text(row.try_get("transaction_id")?)?,event:uuid_text(row.try_get("event_id")?)?,occurred_at:row.try_get("occurred_at")?};
            sqlx::query("INSERT INTO game_item_transfer_reservations(game_session_id,command_id,character_id,world_id,channel_id,source_item_instance_id,destination_kind,intent_binding,transaction_id,event_id,occurred_at,work_units_used,reserved_at,source_kind,house_key,house_scope_generation,house_intent) VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,encode($4,'hex')::uuid,NULL,encode($5,'hex')::uuid,$6,$7,encode($8,'hex')::uuid,encode($9,'hex')::uuid,$10,0,floor(extract(epoch FROM statement_timestamp())*1000)::bigint,$11,$12,$13::text::numeric,$14)")
                .bind(frozen.request.command.game_session_id().as_bytes().as_slice()).bind(frozen.request.command.command_id().get().to_string()).bind(frozen.character.as_bytes().as_slice()).bind(frozen.request.house.world_id().as_bytes().as_slice()).bind(frozen.request.item_instance_id.as_slice()).bind(frozen.request.action.shape()-2).bind(&frozen.binding).bind(frozen.transaction.as_slice()).bind(frozen.event.as_slice()).bind(frozen.occurred_at).bind(frozen.request.action.source_kind()).bind(frozen.request.house.house_key()).bind(fence.scope_ownership_generation.get().to_string()).bind(p).execute(&mut *tx).await?;
            if let Err(error)=run_shape(&mut tx,&node,fence,&frozen,true).await? {return Ok(Err(error));}
            commit_semantic_transaction(tx,deadline).await?;Ok(Ok(HouseItemCandidate{frozen}))
        })).await?
    }

    /// Recovery-first historical read, then one durable retry-work charge, then a fresh
    /// same-physical-transaction shape/receipt/event. No CharacterRevision is advanced.
    pub async fn commit_house_item(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        candidate: &HouseItemCandidate,
    ) -> WriterResult<HouseItemOutcome> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| HouseWriterError::NotCurrent)?;
        let frozen = candidate.frozen.clone();
        let charge = frozen.clone();
        let charge_recovery = recovery.clone();
        let replay = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &charge_recovery).await?;
                    cause_lock(&mut tx, charge.request.command).await?;
                    match receipt(&mut tx, charge.request.command, &charge.binding).await? {
                        Ok(Some(r)) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            return Ok(Ok(Some(r)));
                        }
                        Err(e) => return Ok(Err(e)),
                        _ => {}
                    }
                    if let Err(error) = charge_retry_work(&mut tx, &charge).await? {
                        return Ok(Err(error));
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(None))
                })
            })
            .await??;
        if let Some(receipt) = replay {
            return Ok(HouseItemOutcome::AlreadyCommitted(receipt));
        }
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move {
            let mut tx=begin_type2_transaction(holder,deadline).await?;assert_recovery_fence(&mut tx,&recovery).await?;lock_admission_relations(&mut tx).await?;cause_lock(&mut tx,frozen.request.command).await?;
            match receipt(&mut tx,frozen.request.command,&frozen.binding).await? {
                Ok(Some(r))=>{tx.commit(deadline).await?;return Ok(Ok(HouseItemOutcome::AlreadyCommitted(r)));},Err(e)=>return Ok(Err(e)),_=>{}
            }
            if fence.character_id!=frozen.character || !named_house_current(&mut tx,&node,fence,&frozen.request).await? {return Ok(Err(HouseWriterError::NotCurrent));}
            let facts=match run_shape(&mut tx,&node,fence,&frozen,false).await? {Ok(facts)=>facts,Err(error)=>return Ok(Err(error))};
            let message=message_from_facts(&frozen,&facts)?;
            let tuple=tx.tuple();
            let envelope=match event::encode_house_transfer_event(TransferEventIdentity{event_id:frozen.event,transaction_id:frozen.transaction,occurred_at_unix_ms:frozen.occurred_at,server_build_id:SERVER_BUILD_ID},message,tuple.tuple()){Ok(v)=>v,Err(e)=>return Ok(Err(e.into()))};
            let command=frozen.request.command;
            sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1)")
                .bind(frozen.event.as_slice()).bind(frozen.transaction.as_slice()).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(frozen.request.item_instance_id.as_slice()).bind(frozen.occurred_at).bind(audit::AUDIT_RETENTION_P90D_MS).bind(&envelope).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO game_item_transfer_receipts(game_session_id,command_id,character_id,intent_binding,transaction_id,event_id,shape,source_item_instance_id,source_quantity_before,source_quantity_after,destination_parent_item_instance_id,destination_ordinal,occurred_at,envelope_sha256,committed_at) VALUES(encode($1,'hex')::uuid,$2::text::numeric,encode($3,'hex')::uuid,$4,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7,encode($8,'hex')::uuid,$9,$9,$10::text::uuid,$11::text::numeric,$12,sha256($13),floor(extract(epoch FROM statement_timestamp())*1000)::bigint)")
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string()).bind(frozen.character.as_bytes().as_slice()).bind(&frozen.binding).bind(frozen.transaction.as_slice()).bind(frozen.event.as_slice()).bind(frozen.request.action.shape()).bind(frozen.request.item_instance_id.as_slice()).bind(number(&facts,"quantity_before")? as i64).bind(facts["destination_parent"].as_str()).bind(optional_number(&facts,"destination_ordinal")?.map(|v|v.to_string())).bind(frozen.occurred_at).bind(&envelope).execute(&mut **tx).await?;
            tx.commit(deadline).await?;Ok(Ok(HouseItemOutcome::Committed(HouseItemReceipt{transaction_id:frozen.transaction,event_id:frozen.event,occurred_at_unix_ms:frozen.occurred_at,envelope_sha256:Sha256::digest(&envelope).into(),envelope:Some(envelope)})))
        })).await?
    }

    /// Resolves an unknown commit under the actual recovery fence and the SAME cause
    /// lock. None permits retry of the original IDs; it never authorizes another move.
    pub async fn reconcile_house_item(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        candidate: &HouseItemCandidate,
    ) -> WriterResult<Option<HouseItemReceipt>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| HouseWriterError::NotCurrent)?;
        let frozen = candidate.frozen.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    cause_lock(&mut tx, frozen.request.command).await?;
                    let value = receipt(&mut tx, frozen.request.command, &frozen.binding).await?;
                    if matches!(value, Ok(None))
                        && let Err(error) = charge_retry_work(&mut tx, &frozen).await?
                    {
                        return Ok(Err(error));
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(value)
                })
            })
            .await?
    }
}

fn typed_sql_refusal(error: &sqlx::Error) -> Option<HouseWriterError> {
    let sqlx::Error::Database(db) = error else {
        return None;
    };
    if db.code().as_deref() != Some("P0001") {
        return None;
    }
    Some(match db.message() {
        "INVALID_INPUT" => HouseWriterError::InvalidInput,
        "NOT_CURRENT" => HouseWriterError::NotCurrent,
        "CONFLICTING_OCCURRENCE" => HouseWriterError::ConflictingOccurrence,
        "NO_ACCESS" => HouseWriterError::NoAccess,
        "HOUSE_CLOSED" => HouseWriterError::HouseClosed,
        "ACCESS_CHANGED" => HouseWriterError::AccessChanged,
        "CONTENT_CHANGED" => HouseWriterError::ContentChanged,
        "SOURCE_CHANGED" => HouseWriterError::SourceChanged,
        "ITEM_HAS_CONTENTS" => HouseWriterError::ItemHasContents,
        "NOT_HOUSE_TILE" => HouseWriterError::NotHouseTile,
        "HOUSE_STORAGE_FULL" => HouseWriterError::StorageFull,
        "INVENTORY_FULL" => HouseWriterError::InventoryFull,
        "NO_CHANGE" => HouseWriterError::NoChange,
        _ => return None,
    })
}
fn number(p: &Value, key: &str) -> DbResult<u64> {
    p[key]
        .as_u64()
        .or_else(|| p[key].as_str().and_then(|s| s.parse().ok()))
        .ok_or(DurabilityError::InvalidStoredState)
}
fn optional_number(p: &Value, key: &str) -> DbResult<Option<u64>> {
    if p[key].is_null() {
        Ok(None)
    } else {
        number(p, key).map(Some)
    }
}
fn bytes_uuid(p: &Value, key: &str) -> DbResult<Vec<u8>> {
    Ok(uuid_text(p[key].as_str().ok_or(DurabilityError::InvalidStoredState)?)?.to_vec())
}
fn bytes_position(p: &Value, key: &str) -> DbResult<Vec<u8>> {
    let text = p[key]
        .as_str()
        .and_then(|s| s.strip_prefix("\\x"))
        .ok_or(DurabilityError::InvalidStoredState)?;
    if text.len() % 2 != 0 {
        return Err(DurabilityError::InvalidStoredState);
    }
    (0..text.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&text[i..i + 2], 16).map_err(|_| DurabilityError::InvalidStoredState)
        })
        .collect()
}
fn message_from_facts(
    frozen: &FrozenHouseItem,
    p: &Value,
) -> DbResult<event::OneItemHouseTransferV1> {
    let string = |key: &str| {
        p[key]
            .as_str()
            .map(str::to_owned)
            .ok_or(DurabilityError::InvalidStoredState)
    };
    let state = OneItemStateV1 {
        item_instance_id: bytes_uuid(p, "item_instance_id")?,
        world_id: bytes_uuid(p, "world_id")?,
        definition: Some(OneItemTypedDefinitionRevisionV1 {
            family: string("definition_family")?,
            production_key: string("definition_production_key")?,
            revision_ref: string("definition_revision_ref")?,
        }),
        quantity: u32::try_from(number(p, "quantity_before")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        lifecycle: 1,
        key_number: u32::try_from(number(p, "key_number")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    };
    let house_location = |source: bool| -> DbResult<event::OneItemHouseLocationV1> {
        Ok(event::OneItemHouseLocationV1 {
            world_id: state.world_id.clone(),
            house_key: string("house_key")?,
            spatial_position: bytes_position(
                p,
                if source {
                    "source_position"
                } else {
                    "destination_position"
                },
            )?,
            stack_ordinal: number(
                p,
                if source {
                    "source_stack_ordinal"
                } else {
                    "destination_stack_ordinal"
                },
            )?,
            scope_ownership_generation: number(p, "scope_generation")?,
            placement_transaction_id: if source {
                bytes_uuid(p, "source_placement")?
            } else {
                frozen.transaction.to_vec()
            },
            reclaim_subject: bytes_uuid(p, "reclaim_subject")?,
            provenance_revision: number(p, "provenance_before")?
                .checked_add(u64::from(!source))
                .ok_or(DurabilityError::InvalidStoredState)?,
        })
    };
    let source = if frozen.request.action.shape() == 5 {
        event::HouseTransferSourceV1::Character(event::OneItemHouseCharacterLocationV1 {
            character_id: frozen.character.as_bytes().to_vec(),
            parent_item_instance_id: if p["source_parent"].is_null() {
                vec![]
            } else {
                bytes_uuid(p, "source_parent")?
            },
            placement_ordinal: optional_number(p, "source_ordinal")?.unwrap_or(0),
            equipment_slot: optional_number(p, "source_slot")?.unwrap_or(0) as u32,
        })
    } else {
        event::HouseTransferSourceV1::House(house_location(true)?)
    };
    let destination = if frozen.request.action.shape() == 6 {
        event::HouseTransferDestinationV1::Inventory(event::OneItemHouseCharacterLocationV1 {
            character_id: frozen.character.as_bytes().to_vec(),
            parent_item_instance_id: bytes_uuid(p, "destination_parent")?,
            placement_ordinal: number(p, "destination_ordinal")?,
            equipment_slot: 0,
        })
    } else {
        event::HouseTransferDestinationV1::House(house_location(false)?)
    };
    Ok(event::OneItemHouseTransferV1 {
        before: Some(state.clone()),
        after: Some(state),
        source: Some(source),
        destination: Some(destination),
        cause: Some(OneItemTransferCauseV1 {
            typed_cause: "house_interior_transfer".into(),
            command_ref: Some(OneItemCommandRefV1 {
                game_session_id: frozen.request.command.game_session_id().as_bytes().to_vec(),
                command_id: frozen.request.command.command_id().get(),
            }),
            content_revision: string("content_revision")?,
            ruleset_revision: string("ruleset_revision")?,
            sim_revision: frozen.request.sim_revision.clone(),
        }),
        character_id: frozen.character.as_bytes().to_vec(),
        connection_generation: number(p, "connection_generation")?,
        character_lease_generation: number(p, "lease_generation")?,
    })
}

// The selector is a local execution phase, never an eligibility proof. Both phases
// perform the same native DB/fence/access/source checks; only apply may change custody.
async fn run_shape(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    fence: CurrentCharacterItemFence,
    frozen: &FrozenHouseItem,
    preflight: bool,
) -> DbResult<WriterResult<Value>> {
    let mut p = match context(&frozen.request, frozen.character) {
        Ok(p) => p,
        Err(e) => return Ok(Err(e)),
    };
    p["transaction"] = hex(&frozen.transaction).into();
    p["scope_generation"] = fence.scope_ownership_generation.get().to_string().into();
    p["connection_generation"] = fence.connection_generation.get().to_string().into();
    p["lease_generation"] = fence.character_lease_generation.to_string().into();
    p["phase"] = if preflight { "prepare" } else { "apply" }.into();
    let query = match frozen.request.action {
        HouseItemAction::Place { .. } => {
            "SELECT game_house_item_place($1,encode($2,'hex')::uuid,$3::text::numeric,$4)"
        }
        HouseItemAction::Take { .. } => {
            "SELECT game_house_item_take($1,encode($2,'hex')::uuid,$3::text::numeric,$4)"
        }
        HouseItemAction::TileMove { .. } => {
            "SELECT game_house_item_move($1,encode($2,'hex')::uuid,$3::text::numeric,$4)"
        }
    };
    match sqlx::query_scalar(query)
        .bind(p)
        .bind(node.fact().node_id().as_bytes().as_slice())
        .bind(node.fact().registration_revision().to_string())
        .bind(node.sql_possession_secret())
        .fetch_one(&mut **tx)
        .await
    {
        Ok(v) => Ok(Ok(v)),
        Err(error) => match typed_sql_refusal(&error) {
            Some(e) => Ok(Err(e)),
            None => Err(error.into()),
        },
    }
}

// Both a new apply attempt and a not-yet-committed reconciliation consume the
// existing native durable RL08 counter. Terminal historical replay is a read only.
async fn charge_retry_work(
    tx: &mut Transaction<'_, Postgres>,
    frozen: &FrozenHouseItem,
) -> DbResult<WriterResult<()>> {
    let updated=sqlx::query("UPDATE game_item_transfer_reservations SET work_units_used=work_units_used+1 WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric AND transaction_id=encode($3,'hex')::uuid AND intent_binding=$4 AND work_units_used<$5")
        .bind(frozen.request.command.game_session_id().as_bytes().as_slice()).bind(frozen.request.command.command_id().get().to_string()).bind(frozen.transaction.as_slice()).bind(&frozen.binding).bind(i16::from(audit::RL08_RETRY_WORK_UNITS_MAX)).execute(&mut **tx).await?;
    if updated.rows_affected() == 1 {
        Ok(Ok(()))
    } else {
        Ok(Err(HouseWriterError::RetryBudgetExhausted))
    }
}
