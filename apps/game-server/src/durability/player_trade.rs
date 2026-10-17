//! PLAYER-TRADE0 native inventory observation and atomic two-Character swap.
//!
//! The writer consumes an immutable request frozen from the private native issuer, checks
//! both actual native fences before ordered Character roots, and writes both unchanged
//! item identities, reciprocal receipts and one native type-2 event in one transaction.
//! A matching committed occurrence replays under current recovery without reacquiring
//! either session. Runtime geometry, sole-book/session ingress installation and physical
//! cross-command reservations remain gameplay-owner responsibilities; no DATA row here
//! is issued consent. Migration0100 requires the complete latest0095 union.

#![allow(
    dead_code,
    reason = "native Trade writer awaits its shared owner installation"
)]

use crate::content::{ActiveGeneration, ReferenceItemField};
use crate::domain::{AccountId, CharacterId};
use crate::durability::character_authority::{
    ReconciledCharacterAuthority, SERVER_BUILD_ID, assert_recovery_fence, uuid_text,
};
use crate::durability::character_equipment::{
    EquipmentError, assert_current_equipment_in_transaction,
};
use crate::durability::db::{
    begin_semantic_transaction, begin_type2_transaction, commit_semantic_transaction,
    lock_admission_relations,
};
use crate::durability::item_mint::CORPSE_MATERIALIZATION_PURPOSE_KEY;
use crate::durability::item_mint_audit::{
    self as audit, AuditError, ITEM_LIFECYCLE_LIVE, OneItemStateV1,
    OneItemTypedDefinitionRevisionV1,
};
use crate::durability::item_timed_state::{StoredTimedState, load_timed_state};
use crate::durability::item_transfer::{
    CharacterBackpack, ContainerEntryPosition, CurrentCharacterItemFence,
    GAMEITEM01_STACK_QUANTITY_MAX, character_item_fence_without_root_lock, load_entries, load_slot,
    lock_character_item_root,
};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::{
    ChannelRuntimeAssignmentBinding, ChannelRuntimeV1, CommandRef, RuntimeScopeRefV1,
};
use crate::gameplay_transport::player_trade::{IssuedTradeOccurrence, ReservationIntent};
use prost::Message;
use sha2::{Digest, Sha256};
use sqlx::Row;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Trader {
    pub account_id: AccountId,
    pub fence: CurrentCharacterItemFence,
}

/// Actual loaded native instance state and direct-entry position. The owner must also qualify
/// pickupability, non-corpse/empty-container classification, active definitions and ownership.
/// `timed: None` means a proven absent timed row, never an unavailable read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LeafOffer {
    pub state: OneItemStateV1,
    pub state_revision: u64,
    pub placed_transaction_id: [u8; 16],
    pub timed: Option<StoredTimedState>,
    pub position: ContainerEntryPosition,
}

impl LeafOffer {
    pub(crate) fn matches_backpack(&self, backpack: &CharacterBackpack, world: &[u8; 16]) -> bool {
        self.state.lifecycle == ITEM_LIFECYCLE_LIVE
            && self.state.world_id.as_slice() == world.as_slice()
            && (1..=GAMEITEM01_STACK_QUANTITY_MAX).contains(&self.state.quantity)
            && self.position.parent_item_instance_id == backpack.backpack.item_instance_id
            && self.state.item_instance_id.as_slice()
                != backpack.backpack.item_instance_id.as_slice()
            && backpack.entries.iter().any(|entry| {
                entry.placement_ordinal == self.position.placement_ordinal
                    && entry.item.item_instance_id.as_slice()
                        == self.state.item_instance_id.as_slice()
                    && entry.item.quantity == self.state.quantity
                    && self.state.definition.as_ref().is_some_and(|definition| {
                        definition.family == entry.item.definition.family
                            && definition.production_key == entry.item.definition.production_key
                            && definition.revision_ref == entry.item.definition.revision_ref
                    })
            })
    }
}

/// Current before-data for every direct backpack member, including the backpack itself.
/// The read owner obtains each field from native rows; it grants no future custody.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BackpackItemVersion {
    pub item: [u8; 16],
    pub state_revision: u64,
    pub placed_transaction_id: [u8; 16],
    pub key_number: u32,
    pub timed: Option<StoredTimedState>,
}

/// Independently loaded owner data, not current-authority evidence. Complete item state and
/// backpack changes must also reach the owner's issued/committed-change invalidation hook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TradeData {
    pub traders: [Trader; 2],
    pub backpacks: [CharacterBackpack; 2],
    pub backpack_versions: [Vec<BackpackItemVersion>; 2],
    pub offers: [Option<LeafOffer>; 2],
}

/// Typed read refusals; the gameplay owner maps these to its existing dispositions.
#[derive(Debug)]
pub(crate) enum TradeReadError {
    Authority,
    NoBackpack,
    NotTradeable,
    Stale,
    Storage(DurabilityError),
}
impl std::fmt::Display for TradeReadError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(error) => write!(formatter, "trade storage: {error}"),
            other => write!(formatter, "trade read: {other:?}"),
        }
    }
}
impl std::error::Error for TradeReadError {}
impl From<DurabilityError> for TradeReadError {
    fn from(error: DurabilityError) -> Self {
        Self::Storage(error)
    }
}

impl DurabilityRoot {
    /// Read-only, current native observation of both actual Characters and their direct-entry
    /// leaves. No client handle/current flag supplies custody. This transaction writes no
    /// trade row, cause, reservation, item or event; later boundaries must read again.
    pub(crate) async fn read_player_trade_data(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        runtime: &ChannelRuntimeV1,
        fences: [CurrentCharacterItemFence; 2],
        items: [Option<[u8; 16]>; 2],
        digest: [u8; 32],
    ) -> Result<TradeData, TradeReadError> {
        let record = recovery
            .record_for(self)
            .map_err(|_| TradeReadError::Authority)?;
        runtime
            .owner_fence()
            .map_err(|_| TradeReadError::Authority)?;
        let binding = runtime.binding(); // immutable expected provenance DATA, never current proof
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            // Native equipment admission includes recovery, the real session/node/root and
            // independently current database Content. Lock both Character roots in order
            // before any item reads; the returned EquipmentAuthority never escapes this TX.
            let order = if fences[0].character_id.as_bytes() < fences[1].character_id.as_bytes() { [0,1] } else { [1,0] };
            if fences[0].character_id == fences[1].character_id || fences[0].runtime_scope != fences[1].runtime_scope {
                return Ok(Err(TradeReadError::Authority));
            }
            for side in order {
                match assert_current_equipment_in_transaction(&mut tx, &record, &node, &fences[side], None, digest).await {
                    Ok(_)=>{},
                    Err(EquipmentError::Rejected(_))=>return Ok(Err(TradeReadError::Authority)),
                    Err(EquipmentError::Durability(error))=>return Err(error),
                    Err(EquipmentError::Database(error))=>return Err(error.into()),
                }
            }
            // Read the existing assignment owner under the current native fence's SHARE
            // lock. ALL comparison inputs below come from this independent current row;
            // the carrier's immutable binding supplies only the expected provenance.
            let RuntimeScopeRefV1::Channel {world_id,channel_id}=fences[0].runtime_scope else {return Ok(Err(TradeReadError::Authority));};
            let key=crate::durability::runtime_scope_assignment::scope_key(world_id,channel_id);
            let Some(assignment)=crate::durability::runtime_scope_assignment::load_assignment(&mut tx,&key,false).await? else {return Ok(Err(TradeReadError::Authority));};
            let RuntimeScopeRefV1::Channel {world_id,channel_id}=assignment.scope else {return Ok(Err(TradeReadError::Authority));};
            let Some(holder)=assignment.holder else {return Ok(Err(TradeReadError::Authority));};
            if assignment.state!=crate::durability::runtime_scope_assignment::AssignmentState::Assigned
                || assignment.scope!=fences[0].runtime_scope
                || assignment.ownership_generation!=fences[0].scope_ownership_generation.get()
                || holder!=node.fact()
                || !binding.matches_committed_assignment(world_id,channel_id,holder.node_id(),holder.registration_revision(),assignment.ownership_generation,assignment.source_revision,&assignment.decision_identity)
            {return Ok(Err(TradeReadError::Authority));}
            let mut backpacks = Vec::with_capacity(2);
            let mut traders = Vec::with_capacity(2);
            let mut offers = Vec::with_capacity(2);
            let mut versions = Vec::with_capacity(2);
            for side in [0,1] {
                let account: String = sqlx::query_scalar("SELECT account_id::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                    .bind(fences[side].character_id.as_bytes().as_slice()).fetch_one(&mut *tx).await?;
                traders.push(Trader {account_id:crate::domain::AccountId::from_bytes(uuid_text(&account)?).map_err(|_| DurabilityError::InvalidStoredState)?, fence:fences[side]});
                let Some(backpack) = load_slot(&mut tx, fences[side].character_id).await? else { return Ok(Err(TradeReadError::NoBackpack)); };
                let entries = load_entries(&mut tx, backpack.item_instance_id).await?;
                let backpack = CharacterBackpack {backpack, entries};
                let mut member_versions = Vec::with_capacity(backpack.entries.len()+1);
                for member in std::iter::once(&backpack.backpack).chain(backpack.entries.iter().map(|entry| &entry.item)) {
                    let row = sqlx::query("SELECT i.world_id::text,i.state_revision::text,i.lifecycle,COALESCE(k.key_number,0)::integer AS key_number,COALESCE(s.placed_transaction_id,e.placed_transaction_id)::text AS placed_transaction_id,(s.item_instance_id IS NOT NULL) AS in_slot,(e.item_instance_id IS NOT NULL) AS in_entry FROM game_item_instances i LEFT JOIN game_item_keys k USING(item_instance_id) LEFT JOIN game_item_container_slots s ON s.item_instance_id=i.item_instance_id AND s.character_id=encode($2,'hex')::uuid LEFT JOIN game_item_container_entries e ON e.item_instance_id=i.item_instance_id AND e.character_id=encode($2,'hex')::uuid AND e.parent_item_instance_id=encode($3,'hex')::uuid WHERE i.item_instance_id=encode($1,'hex')::uuid FOR SHARE OF i")
                        .bind(member.item_instance_id.as_slice()).bind(fences[side].character_id.as_bytes().as_slice()).bind(backpack.backpack.item_instance_id.as_slice()).fetch_one(&mut *tx).await?;
                    if row.try_get::<bool,_>("in_slot")? == row.try_get::<bool,_>("in_entry")? || row.try_get::<i16,_>("lifecycle")? != 1 || uuid_text(row.try_get("world_id")?)? != *fences[side].runtime_scope.world_id().as_bytes() {return Ok(Err(TradeReadError::Stale));}
                    let revision = row.try_get::<String,_>("state_revision")?.parse::<u64>().map_err(|_| DurabilityError::InvalidStoredState)?;
                    if revision==0 {return Err(DurabilityError::InvalidStoredState);}
                    member_versions.push(BackpackItemVersion {item:member.item_instance_id,state_revision:revision,placed_transaction_id:uuid_text(row.try_get("placed_transaction_id")?)?,key_number:u32::try_from(row.try_get::<i32,_>("key_number")?).map_err(|_| DurabilityError::InvalidStoredState)?,timed:load_timed_state(&mut tx,&member.item_instance_id).await.map_err(|_| DurabilityError::Unavailable)?});
                }
                versions.push(member_versions);
                let offer = if let Some(item) = items[side] {
                    let Some(entry) = backpack.entries.iter().find(|entry| entry.item.item_instance_id==item) else { return Ok(Err(TradeReadError::NotTradeable)); };
                    let row = sqlx::query("SELECT i.world_id::text,i.definition_family,i.definition_production_key,i.definition_revision_ref,i.quantity,i.lifecycle,i.state_revision::text,COALESCE(k.key_number,0)::integer AS key_number,e.placed_transaction_id::text,EXISTS(SELECT 1 FROM game_item_container_entries c WHERE c.parent_item_instance_id=i.item_instance_id) AS has_contents,EXISTS(SELECT 1 FROM game_item_mint_receipts m WHERE m.item_instance_id=i.item_instance_id AND m.loot_purpose_key=$5) AS is_corpse,EXISTS(SELECT 1 FROM game_character_equipment_slots s WHERE s.item_instance_id=i.item_instance_id) AS is_equipped FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id,world_id) LEFT JOIN game_item_keys k USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid AND e.character_id=encode($2,'hex')::uuid AND e.parent_item_instance_id=encode($3,'hex')::uuid AND e.placement_ordinal=$4::text::numeric FOR SHARE OF i")
                        .bind(item.as_slice()).bind(fences[side].character_id.as_bytes().as_slice()).bind(backpack.backpack.item_instance_id.as_slice()).bind(entry.placement_ordinal.to_string()).bind(CORPSE_MATERIALIZATION_PURPOSE_KEY).fetch_optional(&mut *tx).await?;
                    let Some(row) = row else { return Ok(Err(TradeReadError::Stale)); };
                    if row.try_get::<bool,_>("has_contents")? || row.try_get::<bool,_>("is_corpse")? || row.try_get::<bool,_>("is_equipped")? { return Ok(Err(TradeReadError::NotTradeable)); }
                    let number = |s:String| s.parse::<u64>().map_err(|_| DurabilityError::InvalidStoredState);
                    let state = OneItemStateV1 {
                        item_instance_id:item.to_vec(),world_id:uuid_text(row.try_get("world_id")?)?.to_vec(),
                        definition:Some(OneItemTypedDefinitionRevisionV1 {family:row.try_get("definition_family")?,production_key:row.try_get("definition_production_key")?,revision_ref:row.try_get("definition_revision_ref")?}),
                        quantity:u32::try_from(row.try_get::<i64,_>("quantity")?).map_err(|_| DurabilityError::InvalidStoredState)?,lifecycle:u32::try_from(row.try_get::<i16,_>("lifecycle")?).map_err(|_| DurabilityError::InvalidStoredState)?,key_number:u32::try_from(row.try_get::<i32,_>("key_number")?).map_err(|_| DurabilityError::InvalidStoredState)?,
                    };
                    let leaf = LeafOffer {state,state_revision:number(row.try_get("state_revision")?)?,placed_transaction_id:uuid_text(row.try_get("placed_transaction_id")?)?,timed:load_timed_state(&mut tx,&item).await.map_err(|_| DurabilityError::Unavailable)?,position:crate::durability::item_transfer::ContainerEntryPosition {parent_item_instance_id:backpack.backpack.item_instance_id,placement_ordinal:entry.placement_ordinal}};
                    if leaf.state_revision==0 || !leaf.matches_backpack(&backpack,fences[side].runtime_scope.world_id().as_bytes()) { return Ok(Err(TradeReadError::Stale)); }
                    Some(leaf)
                } else { None };
                backpacks.push(backpack);offers.push(offer);
            }
            let data = TradeData {backpack_versions:versions.try_into().map_err(|_| DurabilityError::InvalidStoredState)?,traders:traders.try_into().map_err(|_| DurabilityError::InvalidStoredState)?,backpacks:backpacks.try_into().map_err(|_| DurabilityError::InvalidStoredState)?,offers:offers.try_into().map_err(|_| DurabilityError::InvalidStoredState)?};
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Ok(data))
        })).await?
    }
}

// PLAYER-TRADE0 §5: two existing item identities, four custody lines, two
// participants, six effect work units and one event. The normal type-2 byte
// ceilings stay unchanged; a maximal valid event is measured by the native tests.
// PLAYER-TRADE0 §6 fixed effect shape, enforced by the exact two-line codec and
// deferred SQL guards. These constants describe the shape, not a charged permit.
// PLAYER-TRADE0 §4 forbids durable reservation rows before the swap.
pub(crate) const PLAYER_TRADE_TOUCHED_ITEMS: u64 = 2;
pub(crate) const PLAYER_TRADE_LOCATION_LINES: u64 = 4;
pub(crate) const PLAYER_TRADE_PARTICIPANTS: u64 = 2;
pub(crate) const PLAYER_TRADE_EFFECT_WORK_UNITS: u64 = 6;
const PLAYER_TRADE_CAUSE: &str = "player_trade";

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemPlayerTradeLocationV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub character_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub parent_item_instance_id: Vec<u8>,
    #[prost(uint64, tag = "3")]
    pub placement_ordinal: u64,
    #[prost(bytes = "vec", tag = "4")]
    pub placed_transaction_id: Vec<u8>,
}

/// One unchanged native timed row, including its independently stored deadline.
/// Absence of this message means the item has no timed row.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemPlayerTradeTimedV1 {
    #[prost(message, optional, tag = "1")]
    pub row: Option<super::item_timed_state_audit::OneItemTimedRowV1>,
    #[prost(int64, optional, tag = "2")]
    pub deadline_at: Option<i64>,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemPlayerTradeParticipantV1 {
    #[prost(bytes = "vec", tag = "1")]
    pub character_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "2")]
    pub account_id: Vec<u8>,
    #[prost(message, optional, tag = "3")]
    pub command: Option<super::item_transfer_audit::OneItemCommandRefV1>,
    #[prost(uint64, tag = "4")]
    pub accepted_connection_generation: u64,
    #[prost(uint64, tag = "5")]
    pub current_connection_generation: u64,
    #[prost(uint64, tag = "6")]
    pub character_lease_generation: u64,
}

#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemPlayerTradeLineV1 {
    #[prost(message, optional, tag = "1")]
    pub before: Option<OneItemStateV1>,
    #[prost(message, optional, tag = "2")]
    pub after: Option<OneItemStateV1>,
    #[prost(uint64, tag = "3")]
    pub state_revision_before: u64,
    #[prost(uint64, tag = "4")]
    pub state_revision_after: u64,
    #[prost(message, optional, tag = "5")]
    pub source: Option<OneItemPlayerTradeLocationV1>,
    #[prost(message, optional, tag = "6")]
    pub destination: Option<OneItemPlayerTradeLocationV1>,
    #[prost(message, optional, tag = "7")]
    pub unchanged_timed_state: Option<OneItemPlayerTradeTimedV1>,
}

/// Native type-2 player_trade operation. This immutable event describes two
/// reciprocal whole-item TRANSFER lines; it does not issue session authority.
#[derive(Clone, PartialEq, Eq, Message)]
pub struct OneItemPlayerTradeV1 {
    #[prost(string, tag = "1")]
    pub typed_cause: String,
    #[prost(bytes = "vec", tag = "2")]
    pub trade_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "3")]
    pub occurrence_id: Vec<u8>,
    #[prost(message, optional, tag = "4")]
    pub first_accept: Option<super::item_transfer_audit::OneItemCommandRefV1>,
    #[prost(message, optional, tag = "5")]
    pub trigger: Option<super::item_transfer_audit::OneItemCommandRefV1>,
    #[prost(bytes = "vec", tag = "6")]
    pub world_id: Vec<u8>,
    #[prost(bytes = "vec", tag = "7")]
    pub channel_id: Vec<u8>,
    #[prost(uint64, tag = "8")]
    pub runtime_scope_ownership_generation: u64,
    #[prost(bytes = "vec", tag = "9")]
    pub content_digest: Vec<u8>,
    #[prost(message, repeated, tag = "10")]
    pub participants: Vec<OneItemPlayerTradeParticipantV1>,
    #[prost(message, repeated, tag = "11")]
    pub lines: Vec<OneItemPlayerTradeLineV1>,
}

fn audit_command(command: CommandRef) -> super::item_transfer_audit::OneItemCommandRefV1 {
    super::item_transfer_audit::OneItemCommandRefV1 {
        game_session_id: command.game_session_id().as_bytes().to_vec(),
        command_id: command.command_id().get(),
    }
}

fn audit_timed(value: Option<StoredTimedState>) -> Option<OneItemPlayerTradeTimedV1> {
    value.map(|value| OneItemPlayerTradeTimedV1 {
        row: Some(super::item_timed_state_audit::OneItemTimedRowV1 {
            charges: value.values.and_then(|values| values.charges()),
            remaining_ms: value.values.and_then(|values| values.remaining_ms()),
            revision: value.revision,
        }),
        deadline_at: value.deadline_at,
    })
}

fn check_trade_command(
    value: &super::item_transfer_audit::OneItemCommandRefV1,
) -> Result<(), AuditError> {
    audit::check_uuid_v7(&value.game_session_id)?;
    if value.command_id == 0 {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

fn check_trade_location(value: &OneItemPlayerTradeLocationV1) -> Result<(), AuditError> {
    audit::check_uuid_v7(&value.character_id)?;
    audit::check_uuid_v7(&value.parent_item_instance_id)?;
    audit::check_uuid_v7(&value.placed_transaction_id)?;
    if value.placement_ordinal == 0 {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

pub(crate) fn check_player_trade(value: &OneItemPlayerTradeV1) -> Result<(), AuditError> {
    use crate::domain::timed_item::{
        TIMEDITEM0_RL_01_CHARGES_MAX, TIMEDITEM0_RL_02_REMAINING_MS_MAX,
    };
    audit::check_uuid_v7(&value.trade_id)?;
    audit::check_uuid_v7(&value.occurrence_id)?;
    audit::check_uuid_v7(&value.world_id)?;
    audit::check_uuid_v7(&value.channel_id)?;
    let first = value
        .first_accept
        .as_ref()
        .ok_or(AuditError::InvalidInput)?;
    let trigger = value.trigger.as_ref().ok_or(AuditError::InvalidInput)?;
    check_trade_command(first)?;
    check_trade_command(trigger)?;
    if value.typed_cause != PLAYER_TRADE_CAUSE
        || value.runtime_scope_ownership_generation == 0
        || value.content_digest.len() != 32
        || value.content_digest == [0; 32]
        || first.game_session_id == trigger.game_session_id
        || value.participants.len() != 2
        || value.lines.len() != 2
    {
        return Err(AuditError::InvalidInput);
    }
    let [a, b] = value.participants.as_slice() else {
        return Err(AuditError::InvalidInput);
    };
    if a.character_id >= b.character_id || a.account_id == b.account_id {
        return Err(AuditError::InvalidInput);
    }
    for participant in &value.participants {
        audit::check_uuid_v7(&participant.character_id)?;
        audit::check_uuid_v7(&participant.account_id)?;
        let command = participant
            .command
            .as_ref()
            .ok_or(AuditError::InvalidInput)?;
        check_trade_command(command)?;
        if participant.accepted_connection_generation == 0
            || participant.character_lease_generation == 0
            || (command != first && command != trigger)
            || (command == first
                && participant.current_connection_generation
                    < participant.accepted_connection_generation)
            || (command == trigger
                && participant.current_connection_generation
                    != participant.accepted_connection_generation)
        {
            return Err(AuditError::InvalidInput);
        }
    }
    if value
        .participants
        .iter()
        .filter(|p| p.command.as_ref() == Some(first))
        .count()
        != 1
        || value
            .participants
            .iter()
            .filter(|p| p.command.as_ref() == Some(trigger))
            .count()
            != 1
    {
        return Err(AuditError::InvalidInput);
    }
    let mut item_ids: Vec<&[u8]> = Vec::with_capacity(2);
    let mut sources: Vec<&[u8]> = Vec::with_capacity(2);
    for line in &value.lines {
        let before = line.before.as_ref().ok_or(AuditError::InvalidInput)?;
        let after = line.after.as_ref().ok_or(AuditError::InvalidInput)?;
        let source = line.source.as_ref().ok_or(AuditError::InvalidInput)?;
        let destination = line.destination.as_ref().ok_or(AuditError::InvalidInput)?;
        audit::check_uuid_v7(&before.item_instance_id)?;
        audit::check_definition(before.definition.as_ref())?;
        audit::check_key_number(before)?;
        check_trade_location(source)?;
        check_trade_location(destination)?;
        if before != after
            || before.world_id != value.world_id
            || before.lifecycle != ITEM_LIFECYCLE_LIVE
            || !(1..=GAMEITEM01_STACK_QUANTITY_MAX).contains(&before.quantity)
            || line.state_revision_before == 0
            || line.state_revision_before.checked_add(1) != Some(line.state_revision_after)
            || before.item_instance_id == source.parent_item_instance_id
            || before.item_instance_id == destination.parent_item_instance_id
            || source.parent_item_instance_id == destination.parent_item_instance_id
            || source.placed_transaction_id == destination.placed_transaction_id
            || source.character_id == destination.character_id
            || !value
                .participants
                .iter()
                .any(|p| p.character_id == source.character_id)
            || !value
                .participants
                .iter()
                .any(|p| p.character_id == destination.character_id)
        {
            return Err(AuditError::InvalidInput);
        }
        if let Some(timed) = &line.unchanged_timed_state {
            let row = timed.row.as_ref().ok_or(AuditError::InvalidInput)?;
            if row.revision == 0
                || row
                    .charges
                    .is_some_and(|n| !(1..=TIMEDITEM0_RL_01_CHARGES_MAX).contains(&n))
                || row
                    .remaining_ms
                    .is_some_and(|n| n > TIMEDITEM0_RL_02_REMAINING_MS_MAX)
                || timed.deadline_at.is_some_and(|deadline| deadline <= 0)
                || (timed.deadline_at.is_some() && row.remaining_ms.is_none())
            {
                return Err(AuditError::InvalidInput);
            }
        }
        item_ids.push(before.item_instance_id.as_slice());
        sources.push(source.character_id.as_slice());
    }
    let [first_line, second_line] = value.lines.as_slice() else {
        return Err(AuditError::InvalidInput);
    };
    if item_ids[0] >= item_ids[1]
        || sources[0] == sources[1]
        || first_line
            .source
            .as_ref()
            .map(|s| &s.parent_item_instance_id)
            != second_line
                .destination
                .as_ref()
                .map(|d| &d.parent_item_instance_id)
        || second_line
            .source
            .as_ref()
            .map(|s| &s.parent_item_instance_id)
            != first_line
                .destination
                .as_ref()
                .map(|d| &d.parent_item_instance_id)
        || first_line
            .destination
            .as_ref()
            .map(|d| &d.placed_transaction_id)
            != second_line
                .destination
                .as_ref()
                .map(|d| &d.placed_transaction_id)
    {
        return Err(AuditError::InvalidInput);
    }
    Ok(())
}

pub(crate) fn decode_player_trade_payload(
    bytes: &[u8],
) -> Result<OneItemPlayerTradeV1, AuditError> {
    if bytes.len() > audit::RL07_PAYLOAD_BYTES_MAX {
        return Err(AuditError::CapacityExceeded);
    }
    let transaction =
        audit::OneItemTransactionV1::decode(bytes).map_err(|_| AuditError::InvalidInput)?;
    if transaction.encode_to_vec() != bytes
        || transaction.interpretation_revision != audit::INTERPRETATION_REVISION
    {
        return Err(AuditError::InvalidInput);
    }
    let Some(audit::OneItemOperationV1::PlayerTrade(value)) = transaction.operation else {
        return Err(AuditError::InvalidInput);
    };
    check_player_trade(&value)?;
    Ok(value)
}

pub(crate) fn decode_player_trade_envelope(
    bytes: &[u8],
) -> Result<(audit::EventEnvelopeV1, OneItemPlayerTradeV1), AuditError> {
    let envelope = audit::decode_common_envelope(bytes)?;
    let value = decode_player_trade_payload(&envelope.payload)?;
    let trigger = value.trigger.as_ref().ok_or(AuditError::InvalidInput)?;
    let participant = value
        .participants
        .iter()
        .find(|p| p.command.as_ref() == Some(trigger))
        .ok_or(AuditError::InvalidInput)?;
    let transaction = envelope
        .transaction_event
        .as_ref()
        .ok_or(AuditError::InvalidInput)?;
    if envelope.privacy_class != audit::RESTRICTED_PLAYER_LINKED
        || envelope.world_id.as_deref() != Some(value.world_id.as_slice())
        || envelope.channel_id.as_deref() != Some(value.channel_id.as_slice())
        || envelope.game_session_id.as_deref() != Some(trigger.game_session_id.as_slice())
        || envelope.command_id != Some(trigger.command_id)
        || envelope.connection_generation != Some(participant.current_connection_generation)
        || envelope.causation
            != Some(audit::CausationRefV1 {
                cause: Some(audit::CausationV1::Command(audit::EnvelopeCommandRefV1 {
                    game_session_id: trigger.game_session_id.clone(),
                    command_id: trigger.command_id,
                })),
            })
        || value.lines.iter().any(|line| {
            line.destination
                .as_ref()
                .is_none_or(|d| d.placed_transaction_id != transaction.transaction_id)
        })
    {
        return Err(AuditError::InvalidInput);
    }
    Ok((envelope, value))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerTradeEventIdentity {
    pub(crate) transaction_id: [u8; 16],
    pub(crate) event_id: [u8; 16],
    pub(crate) occurred_at_unix_ms: i64,
}

fn encode_player_trade_event(
    identity: PlayerTradeEventIdentity,
    value: OneItemPlayerTradeV1,
    tuple: audit::Type2EventTuple,
) -> Result<Vec<u8>, AuditError> {
    check_player_trade(&value)?;
    audit::check_uuid_v7(&identity.transaction_id)?;
    audit::check_uuid_v7(&identity.event_id)?;
    audit::check_technical_text(SERVER_BUILD_ID)?;
    if identity.occurred_at_unix_ms <= 0 {
        return Err(AuditError::InvalidInput);
    }
    let trigger = value.trigger.clone().ok_or(AuditError::InvalidInput)?;
    let generation = value
        .participants
        .iter()
        .find(|p| p.command.as_ref() == Some(&trigger))
        .ok_or(AuditError::InvalidInput)?
        .current_connection_generation;
    let world = value.world_id.clone();
    let channel = value.channel_id.clone();
    let payload = audit::encode_bounded(
        &audit::OneItemTransactionV1 {
            interpretation_revision: audit::INTERPRETATION_REVISION,
            operation: Some(audit::OneItemOperationV1::PlayerTrade(value)),
        },
        audit::RL07_PAYLOAD_BYTES_MAX,
    )?;
    let envelope = audit::EventEnvelopeV1 {
        envelope_revision: audit::ENVELOPE_REVISION,
        event_id: identity.event_id.to_vec(),
        event_type_id: audit::EVENT_TYPE_ID,
        event_schema_revision: tuple.schema_revision(),
        durability_class: audit::DURABLE_AUDIT,
        privacy_class: audit::RESTRICTED_PLAYER_LINKED,
        retention_profile_id: tuple.retention_profile_id().into(),
        occurred_at_unix_ms: identity.occurred_at_unix_ms,
        world_id: Some(world),
        channel_id: Some(channel),
        game_session_id: Some(trigger.game_session_id.clone()),
        connection_generation: Some(generation),
        command_id: Some(trigger.command_id),
        transaction_event: Some(audit::TransactionEventRefV1 {
            transaction_id: identity.transaction_id.to_vec(),
            ordinal: 1,
            count: 1,
        }),
        causation: Some(audit::CausationRefV1 {
            cause: Some(audit::CausationV1::Command(audit::EnvelopeCommandRefV1 {
                game_session_id: trigger.game_session_id,
                command_id: trigger.command_id,
            })),
        }),
        server_build_id: SERVER_BUILD_ID.into(),
        payload_sha256: Sha256::digest(&payload).to_vec(),
        payload,
        ..audit::EventEnvelopeV1::default()
    };
    let bytes = audit::encode_bounded(&envelope, audit::RL07_ENVELOPE_BYTES_MAX)?;
    decode_player_trade_envelope(&bytes)?;
    Ok(bytes)
}

#[derive(Debug)]
pub(crate) enum PlayerTradeError {
    InvalidInput,
    AuthorityRejected,
    Stale,
    NoRoom,
    ConflictingOccurrence,
    Audit(AuditError),
    Storage(DurabilityError),
}
impl std::fmt::Display for PlayerTradeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Storage(error) => write!(f, "player trade storage: {error}"),
            other => write!(f, "player trade: {other:?}"),
        }
    }
}
impl std::error::Error for PlayerTradeError {}
impl From<DurabilityError> for PlayerTradeError {
    fn from(error: DurabilityError) -> Self {
        Self::Storage(error)
    }
}
impl From<AuditError> for PlayerTradeError {
    fn from(error: AuditError) -> Self {
        Self::Audit(error)
    }
}

/// Immutable retry input. Only a real private component issuance can supply the
/// captured before-data; no constructor accepts an arbitrary ReservationIntent.
/// This binds the component intent, not the uninstalled session ingress owner.
#[derive(Debug, Clone)]
pub(crate) struct FrozenPlayerTrade {
    trade_id: [u8; 16],
    occurrence: [u8; 16],
    first_accept: CommandRef,
    first_fence: CurrentCharacterItemFence,
    trigger: CommandRef,
    intent: ReservationIntent,
    identity: PlayerTradeEventIdentity,
    runtime_binding: ChannelRuntimeAssignmentBinding,
    content_digest: [u8; 32],
    capacities: [u16; 2],
    binding: [u8; 32],
}

fn ordered_characters(data: &TradeData) -> [usize; 2] {
    if data.traders[0].fence.character_id.as_bytes() < data.traders[1].fence.character_id.as_bytes()
    {
        [0, 1]
    } else {
        [1, 0]
    }
}

impl FrozenPlayerTrade {
    pub(crate) fn from_issued(
        issued: &IssuedTradeOccurrence,
        runtime: &ChannelRuntimeV1,
        generation: &ActiveGeneration,
        identity: PlayerTradeEventIdentity,
    ) -> Result<Self, PlayerTradeError> {
        runtime
            .owner_fence()
            .map_err(|_| PlayerTradeError::AuthorityRejected)?;
        let digest = runtime.content_pin().server_artifact_digest();
        if digest == [0; 32] || generation.identity().server_artifact_digest() != digest {
            return Err(PlayerTradeError::AuthorityRejected);
        }
        audit::check_uuid_v7(&identity.transaction_id)?;
        audit::check_uuid_v7(&identity.event_id)?;
        if identity.transaction_id == identity.event_id
            || identity.occurred_at_unix_ms <= 0
            || identity
                .occurred_at_unix_ms
                .checked_add(audit::AUDIT_RETENTION_P90D_MS)
                .is_none()
        {
            return Err(PlayerTradeError::InvalidInput);
        }
        let intent = issued.intent().clone();
        let data = &intent.binding.data;
        let [a, b] = data.traders;
        let first_side = data
            .traders
            .iter()
            .position(|p| p.fence == issued.first_fence())
            .ok_or(PlayerTradeError::InvalidInput)?;
        let RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } = a.fence.runtime_scope
        else {
            return Err(PlayerTradeError::InvalidInput);
        };
        if a.fence.character_id == b.fence.character_id
            || a.account_id == b.account_id
            || a.fence.game_session_id == b.fence.game_session_id
            || a.fence.runtime_scope != b.fence.runtime_scope
            || a.fence.scope_ownership_generation != b.fence.scope_ownership_generation
            || runtime.binding().world_id() != world_id
            || runtime.binding().channel_id() != channel_id
            || runtime.binding().scope_generation() != a.fence.scope_ownership_generation
            || intent.binding.accepts[first_side] != issued.first_accept()
            || intent.binding.accepts[1 - first_side] != issued.trigger()
            || intent.binding.trigger != issued.trigger()
            || data.backpacks[0].backpack.item_instance_id
                == data.backpacks[1].backpack.item_instance_id
        {
            return Err(PlayerTradeError::InvalidInput);
        }
        let native = generation
            .native_gameplay()
            .ok_or(PlayerTradeError::InvalidInput)?;
        let mut capacities = [0_u16; 2];
        for side in [0, 1] {
            let trader = data.traders[side];
            let bag = &data.backpacks[side];
            let offer = data.offers[side]
                .as_ref()
                .ok_or(PlayerTradeError::InvalidInput)?;
            if intent.binding.accepts[side].game_session_id() != trader.fence.game_session_id
                || !offer.matches_backpack(bag, world_id.as_bytes())
                || offer.state_revision == 0
                || offer.state_revision == u64::MAX
                || bag.entries.len() > 20
                || data.backpack_versions[side].len() != bag.entries.len() + 1
                || intent.destinations[side].parent_item_instance_id
                    != bag.backpack.item_instance_id
                || bag
                    .entries
                    .iter()
                    .map(|e| e.placement_ordinal)
                    .max()
                    .unwrap_or(0)
                    .checked_add(1)
                    != Some(intent.destinations[side].placement_ordinal)
            {
                return Err(PlayerTradeError::InvalidInput);
            }
            let policy = native
                .item_policy(
                    &bag.backpack.definition.production_key,
                    &bag.backpack.definition.revision_ref,
                )
                .ok_or(PlayerTradeError::InvalidInput)?
                .record();
            if policy.production_definition.family != bag.backpack.definition.family
                || policy.production_definition.production_key
                    != bag.backpack.definition.production_key
                || policy.production_definition.revision_ref != bag.backpack.definition.revision_ref
            {
                return Err(PlayerTradeError::InvalidInput);
            }
            let ReferenceItemField::Known(container) = &policy.semantics.container else {
                return Err(PlayerTradeError::InvalidInput);
            };
            let ReferenceItemField::Known(capacity) = container.capacity else {
                return Err(PlayerTradeError::InvalidInput);
            };
            if !(1..=20).contains(&capacity) || bag.entries.len() >= usize::from(capacity) {
                return Err(PlayerTradeError::NoRoom);
            }
            capacities[side] = capacity;
            let definition = offer
                .state
                .definition
                .as_ref()
                .ok_or(PlayerTradeError::InvalidInput)?;
            let policy = native
                .item_policy(&definition.production_key, &definition.revision_ref)
                .ok_or(PlayerTradeError::InvalidInput)?
                .record();
            if policy.production_definition.family != definition.family
                || policy.production_definition.production_key != definition.production_key
                || policy.production_definition.revision_ref != definition.revision_ref
            {
                return Err(PlayerTradeError::InvalidInput);
            }
            let ReferenceItemField::Known(physical) = &policy.semantics.physical else {
                return Err(PlayerTradeError::InvalidInput);
            };
            if physical.pickupable != ReferenceItemField::Known(true) {
                return Err(PlayerTradeError::InvalidInput);
            }
            match &policy.semantics.trade_restrictions {
                ReferenceItemField::Unknown => {}
                ReferenceItemField::Known(flags)
                    if crate::domain::player_trade::tradeable_flag_allows(flags) => {}
                _ => return Err(PlayerTradeError::InvalidInput),
            }
            for (member, version) in std::iter::once(&bag.backpack)
                .chain(bag.entries.iter().map(|e| &e.item))
                .zip(&data.backpack_versions[side])
            {
                if member.item_instance_id != version.item || version.state_revision == 0 {
                    return Err(PlayerTradeError::InvalidInput);
                }
            }
        }
        let mut frozen = Self {
            trade_id: *issued.trade_id(),
            occurrence: *issued.occurrence(),
            first_accept: issued.first_accept(),
            first_fence: issued.first_fence(),
            trigger: issued.trigger(),
            intent,
            identity,
            runtime_binding: runtime.binding(),
            content_digest: digest,
            capacities,
            binding: [0; 32],
        };
        let message = frozen.message(frozen.first_fence.connection_generation.get())?;
        check_player_trade(&message)?;
        frozen.binding = frozen.request_binding(&message);
        Ok(frozen)
    }

    fn first_side(&self) -> Result<usize, PlayerTradeError> {
        self.intent
            .binding
            .data
            .traders
            .iter()
            .position(|p| p.fence == self.first_fence)
            .ok_or(PlayerTradeError::InvalidInput)
    }

    fn message(
        &self,
        current_first_connection: u64,
    ) -> Result<OneItemPlayerTradeV1, PlayerTradeError> {
        let data = &self.intent.binding.data;
        let first_side = self.first_side()?;
        let mut participants = Vec::with_capacity(2);
        for side in ordered_characters(data) {
            let trader = data.traders[side];
            participants.push(OneItemPlayerTradeParticipantV1 {
                character_id: trader.fence.character_id.as_bytes().to_vec(),
                account_id: trader.account_id.as_bytes().to_vec(),
                command: Some(audit_command(self.intent.binding.accepts[side])),
                accepted_connection_generation: trader.fence.connection_generation.get(),
                current_connection_generation: if side == first_side {
                    current_first_connection
                } else {
                    trader.fence.connection_generation.get()
                },
                character_lease_generation: trader.fence.character_lease_generation,
            });
        }
        let mut lines = Vec::with_capacity(2);
        for side in [0, 1] {
            let offer = data.offers[side]
                .as_ref()
                .ok_or(PlayerTradeError::InvalidInput)?;
            let destination = self.intent.destinations[1 - side];
            lines.push(OneItemPlayerTradeLineV1 {
                before: Some(offer.state.clone()),
                after: Some(offer.state.clone()),
                state_revision_before: offer.state_revision,
                state_revision_after: offer
                    .state_revision
                    .checked_add(1)
                    .ok_or(PlayerTradeError::Stale)?,
                source: Some(OneItemPlayerTradeLocationV1 {
                    character_id: data.traders[side].fence.character_id.as_bytes().to_vec(),
                    parent_item_instance_id: offer.position.parent_item_instance_id.to_vec(),
                    placement_ordinal: offer.position.placement_ordinal,
                    placed_transaction_id: offer.placed_transaction_id.to_vec(),
                }),
                destination: Some(OneItemPlayerTradeLocationV1 {
                    character_id: data.traders[1 - side]
                        .fence
                        .character_id
                        .as_bytes()
                        .to_vec(),
                    parent_item_instance_id: destination.parent_item_instance_id.to_vec(),
                    placement_ordinal: destination.placement_ordinal,
                    placed_transaction_id: self.identity.transaction_id.to_vec(),
                }),
                unchanged_timed_state: audit_timed(offer.timed),
            });
        }
        lines.sort_by(|a, b| {
            a.before
                .as_ref()
                .map(|s| &s.item_instance_id)
                .cmp(&b.before.as_ref().map(|s| &s.item_instance_id))
        });
        Ok(OneItemPlayerTradeV1 {
            typed_cause: PLAYER_TRADE_CAUSE.into(),
            trade_id: self.trade_id.to_vec(),
            occurrence_id: self.occurrence.to_vec(),
            first_accept: Some(audit_command(self.first_accept)),
            trigger: Some(audit_command(self.trigger)),
            world_id: self.runtime_binding.world_id().as_bytes().to_vec(),
            channel_id: self.runtime_binding.channel_id().as_bytes().to_vec(),
            runtime_scope_ownership_generation: self.runtime_binding.scope_generation().get(),
            content_digest: self.content_digest.to_vec(),
            participants,
            lines,
        })
    }

    fn request_binding(&self, baseline: &OneItemPlayerTradeV1) -> [u8; 32] {
        fn part(hash: &mut Sha256, bytes: &[u8]) {
            hash.update((bytes.len() as u64).to_be_bytes());
            hash.update(bytes);
        }
        let mut hash = Sha256::new();
        part(&mut hash, b"OTERYN_PLAYER_TRADE_REQUEST/v1");
        part(&mut hash, &baseline.encode_to_vec());
        part(
            &mut hash,
            self.intent.binding.pair.game_session_id().as_bytes(),
        );
        hash.update(self.intent.binding.pair.command_id().get().to_be_bytes());
        hash.update(self.identity.transaction_id);
        hash.update(self.identity.event_id);
        hash.update(self.identity.occurred_at_unix_ms.to_be_bytes());
        hash.update(self.runtime_binding.source_revision().to_be_bytes());
        // All captured backpack state participates in the retry binding. An
        // unrelated item/checkpoint change invalidates this open trade too.
        for side in [0, 1] {
            hash.update(self.capacities[side].to_be_bytes());
            let bag = &self.intent.binding.data.backpacks[side];
            hash.update((bag.entries.len() as u64).to_be_bytes());
            hash.update(
                (self.intent.binding.data.backpack_versions[side].len() as u64).to_be_bytes(),
            );
            for member in std::iter::once(&bag.backpack).chain(bag.entries.iter().map(|e| &e.item))
            {
                part(&mut hash, &member.item_instance_id);
                part(&mut hash, member.definition.family.as_bytes());
                part(&mut hash, member.definition.production_key.as_bytes());
                part(&mut hash, member.definition.revision_ref.as_bytes());
                hash.update(member.quantity.to_be_bytes());
            }
            for entry in &bag.entries {
                hash.update(entry.placement_ordinal.to_be_bytes());
            }
            for version in &self.intent.binding.data.backpack_versions[side] {
                hash.update(version.item);
                hash.update(version.state_revision.to_be_bytes());
                hash.update(version.placed_transaction_id);
                hash.update(version.key_number.to_be_bytes());
                match audit_timed(version.timed) {
                    Some(timed) => {
                        hash.update([1]);
                        part(&mut hash, &timed.encode_to_vec());
                    }
                    None => hash.update([0]),
                }
            }
        }
        hash.finalize().into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlayerTradeReceipt {
    pub(crate) item_instance_id: [u8; 16],
    pub(crate) source_character_id: CharacterId,
    pub(crate) destination_character_id: CharacterId,
    pub(crate) destination: ContainerEntryPosition,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommittedPlayerTrade {
    pub(crate) occurrence: [u8; 16],
    pub(crate) trade_id: [u8; 16],
    pub(crate) transaction_id: [u8; 16],
    pub(crate) event_id: [u8; 16],
    pub(crate) receipts: [PlayerTradeReceipt; 2],
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PlayerTradeOutcome {
    Committed(CommittedPlayerTrade),
    AlreadyCommitted(CommittedPlayerTrade),
}

/// Actual borrowed owners are required on every new attempt. The first accept
/// may resume its same GameSession at a higher connection generation; all other
/// captured first-side fence components and the pending trigger remain pinned.
pub(crate) struct PlayerTradeCommitAuthority<'a, 'f, 's> {
    pub(crate) recovery: &'a ReconciledCharacterAuthority<'f, 's>,
    pub(crate) node: &'a NodeIncarnationProof,
    pub(crate) runtime: &'a ChannelRuntimeV1,
    /// Borrowed from the still-TRANSFERRING native component for the entire attempt.
    pub(crate) issued: &'a IssuedTradeOccurrence,
    pub(crate) first_current_fence: CurrentCharacterItemFence,
}

fn first_fence_continues(
    expected: CurrentCharacterItemFence,
    current: CurrentCharacterItemFence,
) -> bool {
    expected.character_id == current.character_id
        && expected.game_session_id == current.game_session_id
        && expected.character_lease_generation == current.character_lease_generation
        && expected.runtime_scope == current.runtime_scope
        && expected.scope_ownership_generation == current.scope_ownership_generation
        && current.connection_generation.get() >= expected.connection_generation.get()
}

async fn lock_player_trade_cause(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &FrozenPlayerTrade,
) -> Result<(), DurabilityError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:player-trade:' || encode($1,'hex'),0))")
        .bind(request.occurrence.as_slice()).execute(&mut **tx).await?;
    Ok(())
}

fn number(row: &sqlx::postgres::PgRow, column: &str) -> Result<u64, DurabilityError> {
    row.try_get::<String, _>(column)?
        .parse::<u64>()
        .map_err(|_| DurabilityError::InvalidStoredState)
}

async fn load_player_trade(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &FrozenPlayerTrade,
) -> Result<Option<Result<CommittedPlayerTrade, PlayerTradeError>>, DurabilityError> {
    let Some(row)=sqlx::query("SELECT trade_id::text,transaction_id::text,event_id::text,request_binding FROM game_player_trades WHERE occurrence_id=encode($1,'hex')::uuid")
        .bind(request.occurrence.as_slice()).fetch_optional(&mut **tx).await? else{return Ok(None);};
    if row.try_get::<Vec<u8>, _>("request_binding")? != request.binding {
        return Ok(Some(Err(PlayerTradeError::ConflictingOccurrence)));
    }
    let lines=sqlx::query("SELECT item_instance_id::text,source_character_id::text,destination_character_id::text,destination_parent_item_instance_id::text,destination_ordinal::text FROM game_player_trade_receipts WHERE occurrence_id=encode($1,'hex')::uuid ORDER BY line_ordinal LIMIT 3")
        .bind(request.occurrence.as_slice()).fetch_all(&mut **tx).await?;
    if lines.len() != 2 {
        return Err(DurabilityError::InvalidStoredState);
    }
    let receipts = lines
        .iter()
        .map(|line| {
            Ok(PlayerTradeReceipt {
                item_instance_id: uuid_text(line.try_get("item_instance_id")?)?,
                source_character_id: CharacterId::from_bytes(uuid_text(
                    line.try_get("source_character_id")?,
                )?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
                destination_character_id: CharacterId::from_bytes(uuid_text(
                    line.try_get("destination_character_id")?,
                )?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
                destination: ContainerEntryPosition {
                    parent_item_instance_id: uuid_text(
                        line.try_get("destination_parent_item_instance_id")?,
                    )?,
                    placement_ordinal: number(line, "destination_ordinal")?,
                },
            })
        })
        .collect::<Result<Vec<_>, DurabilityError>>()?;
    let receipts: [PlayerTradeReceipt; 2] = receipts
        .try_into()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if receipts[0].item_instance_id >= receipts[1].item_instance_id
        || receipts.iter().any(|line| {
            line.source_character_id == line.destination_character_id
                || line.destination.placement_ordinal == 0
        })
        || receipts[0].source_character_id != receipts[1].destination_character_id
        || receipts[1].source_character_id != receipts[0].destination_character_id
    {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(Some(Ok(CommittedPlayerTrade {
        occurrence: request.occurrence,
        trade_id: uuid_text(row.try_get("trade_id")?)?,
        transaction_id: uuid_text(row.try_get("transaction_id")?)?,
        event_id: uuid_text(row.try_get("event_id")?)?,
        receipts,
    })))
}

async fn trade_identity_reused(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &FrozenPlayerTrade,
) -> Result<bool, DurabilityError> {
    Ok(sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_player_trades WHERE transaction_id=encode($1,'hex')::uuid OR event_id=encode($2,'hex')::uuid OR trade_id=encode($3,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_player_trade_participants WHERE (game_session_id=encode($4,'hex')::uuid AND command_id=$5::text::numeric) OR (game_session_id=encode($6,'hex')::uuid AND command_id=$7::text::numeric)) OR EXISTS(SELECT 1 FROM game_item_audit_outbox WHERE transaction_id=encode($1,'hex')::uuid OR event_id=encode($2,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_item_instances WHERE minted_transaction_id=encode($1,'hex')::uuid)")
        .bind(request.identity.transaction_id.as_slice()).bind(request.identity.event_id.as_slice()).bind(request.trade_id.as_slice())
        .bind(request.first_accept.game_session_id().as_bytes().as_slice()).bind(request.first_accept.command_id().get().to_string())
        .bind(request.trigger.game_session_id().as_bytes().as_slice()).bind(request.trigger.command_id().get().to_string())
        .fetch_one(&mut **tx).await?)
}

async fn current_trade_content_and_assignment(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    node: &NodeIncarnationProof,
    request: &FrozenPlayerTrade,
) -> Result<bool, DurabilityError> {
    let binding = request.runtime_binding;
    let world = binding.world_id();
    let channel = binding.channel_id();
    let key = super::runtime_scope_assignment::scope_key(world, channel);
    let Some(assignment) =
        super::runtime_scope_assignment::load_assignment(tx, &key, false).await?
    else {
        return Ok(false);
    };
    let Some(holder) = assignment.holder else {
        return Ok(false);
    };
    let RuntimeScopeRefV1::Channel {
        world_id: actual_world,
        channel_id: actual_channel,
    } = assignment.scope
    else {
        return Ok(false);
    };
    if assignment.state != super::runtime_scope_assignment::AssignmentState::Assigned
        || holder != node.fact()
        || !binding.matches_committed_assignment(
            actual_world,
            actual_channel,
            holder.node_id(),
            holder.registration_revision(),
            assignment.ownership_generation,
            assignment.source_revision,
            &assignment.decision_identity,
        )
    {
        return Ok(false);
    }
    let digest:Option<Vec<u8>>=sqlx::query_scalar("SELECT server_artifact_digest FROM game_content_activations WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid ORDER BY activation_sequence DESC LIMIT 1")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    Ok(digest.as_deref() == Some(request.content_digest.as_slice()))
}

/// Under both current Character roots, both offered item locks and both ordered
/// slot locks, re-read every captured member/version. No old read flag or client
/// location is accepted as the current physical source.
async fn trade_before_refusal(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &FrozenPlayerTrade,
) -> Result<Option<PlayerTradeError>, DurabilityError> {
    let data = &request.intent.binding.data;
    let world = request.runtime_binding.world_id();
    for side in [0, 1] {
        let Some(bag) = load_slot(tx, data.traders[side].fence.character_id).await? else {
            return Ok(Some(PlayerTradeError::Stale));
        };
        if bag != data.backpacks[side].backpack {
            return Ok(Some(PlayerTradeError::Stale));
        }
        let entries = load_entries(tx, bag.item_instance_id).await?;
        if entries.len() >= usize::from(request.capacities[side]) {
            return Ok(Some(PlayerTradeError::NoRoom));
        }
        if entries != data.backpacks[side].entries {
            return Ok(Some(PlayerTradeError::Stale));
        }
        for version in &data.backpack_versions[side] {
            let row=sqlx::query("SELECT i.world_id::text,i.lifecycle,i.state_revision::text,COALESCE(k.key_number,0)::integer AS key_number,COALESCE(s.placed_transaction_id,e.placed_transaction_id)::text AS placed_transaction_id,(s.item_instance_id IS NOT NULL) AS in_slot,(e.item_instance_id IS NOT NULL) AS in_entry FROM game_item_instances i LEFT JOIN game_item_keys k USING(item_instance_id) LEFT JOIN game_item_container_slots s ON s.item_instance_id=i.item_instance_id AND s.character_id=encode($2,'hex')::uuid LEFT JOIN game_item_container_entries e ON e.item_instance_id=i.item_instance_id AND e.character_id=encode($2,'hex')::uuid AND e.parent_item_instance_id=encode($3,'hex')::uuid WHERE i.item_instance_id=encode($1,'hex')::uuid")
                .bind(version.item.as_slice()).bind(data.traders[side].fence.character_id.as_bytes().as_slice()).bind(bag.item_instance_id.as_slice()).fetch_optional(&mut **tx).await?;
            let Some(row) = row else {
                return Ok(Some(PlayerTradeError::Stale));
            };
            if row.try_get::<bool, _>("in_slot")? == row.try_get::<bool, _>("in_entry")?
                || row.try_get::<i16, _>("lifecycle")? != 1
                || uuid_text(row.try_get("world_id")?)? != *world.as_bytes()
                || number(&row, "state_revision")? != version.state_revision
                || uuid_text(row.try_get("placed_transaction_id")?)?
                    != version.placed_transaction_id
                || u32::try_from(row.try_get::<i32, _>("key_number")?)
                    .map_err(|_| DurabilityError::InvalidStoredState)?
                    != version.key_number
                || load_timed_state(&mut **tx, &version.item)
                    .await
                    .map_err(|_| DurabilityError::Unavailable)?
                    != version.timed
            {
                return Ok(Some(PlayerTradeError::Stale));
            }
        }
        let offer = data.offers[side]
            .as_ref()
            .ok_or(DurabilityError::InvalidStoredState)?;
        let item: [u8; 16] = offer
            .state
            .item_instance_id
            .as_slice()
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let forbidden:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_item_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_item_corpse_container_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid) OR EXISTS(SELECT 1 FROM game_item_mint_receipts WHERE item_instance_id=encode($1,'hex')::uuid AND loot_purpose_key=$2) OR EXISTS(SELECT 1 FROM game_character_equipment_slots WHERE item_instance_id=encode($1,'hex')::uuid)")
            .bind(item.as_slice()).bind(CORPSE_MATERIALIZATION_PURPOSE_KEY).fetch_one(&mut **tx).await?;
        if forbidden {
            return Ok(Some(PlayerTradeError::Stale));
        }
    }
    Ok(None)
}

impl DurabilityRoot {
    pub(crate) async fn reconcile_player_trade(
        &self,
        recovery: &ReconciledCharacterAuthority<'_, '_>,
        request: &FrozenPlayerTrade,
    ) -> Result<Option<CommittedPlayerTrade>, PlayerTradeError> {
        let recovery = recovery
            .record_for(self)
            .map_err(|_| PlayerTradeError::AuthorityRejected)?;
        let request = request.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    lock_player_trade_cause(&mut tx, &request).await?;
                    let outcome = match load_player_trade(&mut tx, &request).await? {
                        None => Ok(None),
                        Some(Ok(committed)) => Ok(Some(committed)),
                        Some(Err(error)) => Err(error),
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(outcome)
                })
            })
            .await?
    }

    /// One native item-only transaction: both whole source entries become new
    /// reciprocal entries, never a merge and never using a just-freed source
    /// slot to pass capacity. No CharacterRevision is advanced.
    pub(crate) async fn commit_player_trade(
        &self,
        authority: PlayerTradeCommitAuthority<'_, '_, '_>,
        request: &FrozenPlayerTrade,
    ) -> Result<PlayerTradeOutcome, PlayerTradeError> {
        let recovery = authority
            .recovery
            .record_for(self)
            .map_err(|_| PlayerTradeError::AuthorityRejected)?;
        let node = authority.node.clone();
        let first_current = authority.first_current_fence;
        // Evaluated from the actual borrowed runtime, and applied only after
        // historical outcome lookup. A replay requires recovery, not a live actor.
        let runtime_matches = authority.runtime.owner_fence().is_ok()
            && authority.runtime.binding() == request.runtime_binding
            && authority.runtime.content_pin().server_artifact_digest() == request.content_digest;
        let issuance_matches = authority.issued.trade_id() == &request.trade_id
            && authority.issued.occurrence() == &request.occurrence
            && authority.issued.first_accept() == request.first_accept
            && authority.issued.first_fence() == request.first_fence
            && authority.issued.trigger() == request.trigger
            && authority.issued.intent() == &request.intent;
        let request = request.clone();
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_type2_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            lock_player_trade_cause(&mut tx,&request).await?;
            if let Some(committed)=load_player_trade(&mut tx,&request).await? {
                let committed=match committed {Ok(value)=>value,Err(error)=>return Ok(Err(error))};
                tx.commit(deadline).await?;
                return Ok(Ok(PlayerTradeOutcome::AlreadyCommitted(committed)));
            }
            if trade_identity_reused(&mut tx,&request).await? {return Ok(Err(PlayerTradeError::ConflictingOccurrence));}
            if !runtime_matches || !issuance_matches || !first_fence_continues(request.first_fence,first_current) {
                return Ok(Err(PlayerTradeError::AuthorityRejected));
            }
            let first_side=match request.first_side(){Ok(side)=>side,Err(error)=>return Ok(Err(error))};
            let mut fences=request.intent.binding.data.traders.map(|p|p.fence);
            fences[first_side]=first_current;
            let order=ordered_characters(&request.intent.binding.data);
            let world=request.runtime_binding.world_id();let channel=request.runtime_binding.channel_id();
            // Same native Content activation owner lock as equipment. Acquire
            // before any Character root; later independent digest reads stay pinned.
            sqlx::query("SELECT pg_advisory_xact_lock_shared(hashtextextended('oteryn:content-activation:' || encode($1::bytea,'hex')::uuid::text || ':' || encode($2::bytea,'hex')::uuid::text,0))")
                .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&mut **tx).await?;
            let mut accounts=[String::new(),String::new()];
            for side in order {
                let Some(account)=character_item_fence_without_root_lock(&mut tx,&node,&fences[side],request.intent.binding.accepts[side],fences[side].character_id,*world.as_bytes(),*channel.as_bytes()).await?
                    else{return Ok(Err(PlayerTradeError::AuthorityRejected));};
                if uuid_text(&account)?!=*request.intent.binding.data.traders[side].account_id.as_bytes() {
                    return Ok(Err(PlayerTradeError::AuthorityRejected));
                }
                accounts[side]=account;
            }
            if accounts[0]==accounts[1] || !current_trade_content_and_assignment(&mut tx,&node,&request).await? {
                return Ok(Err(PlayerTradeError::AuthorityRejected));
            }
            let mut root_revisions=[0_u64;2];
            for side in order {
                if !lock_character_item_root(&mut tx,&fences[side],&accounts[side]).await? {return Ok(Err(PlayerTradeError::AuthorityRejected));}
                let revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                    .bind(fences[side].character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
                root_revisions[side]=revision.parse().map_err(|_|DurabilityError::InvalidStoredState)?;
            }
            let message=match request.message(first_current.connection_generation.get()){Ok(message)=>message,Err(error)=>return Ok(Err(error))};
            let item_ids=message.lines.iter().map(|line|line.before.as_ref().map(|s|s.item_instance_id.clone()).ok_or(DurabilityError::InvalidStoredState)).collect::<Result<Vec<_>,DurabilityError>>()?;
            // Only the two offered instance rows, ordered by their native IDs.
            let locked=sqlx::query("SELECT item_instance_id FROM game_item_instances WHERE item_instance_id IN(encode($1,'hex')::uuid,encode($2,'hex')::uuid) ORDER BY item_instance_id FOR UPDATE")
                .bind(item_ids[0].as_slice()).bind(item_ids[1].as_slice()).fetch_all(&mut **tx).await?;
            if locked.len()!=2{return Ok(Err(PlayerTradeError::Stale));}
            // Runtime deliberately has no UPDATE grant on container slots.
            // The owned bounded SQL function grants only these ordered locks.
            let locked:i16=sqlx::query_scalar("SELECT game_player_trade_lock_slots(encode($1,'hex')::uuid,encode($2,'hex')::uuid)")
                .bind(fences[order[0]].character_id.as_bytes().as_slice()).bind(fences[order[1]].character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            if locked!=2{return Ok(Err(PlayerTradeError::Stale));}
            if let Some(refusal)=trade_before_refusal(&mut tx,&request).await? {return Ok(Err(refusal));}
            let tuple=tx.tuple().tuple();
            let envelope=match encode_player_trade_event(request.identity,message.clone(),tuple){Ok(bytes)=>bytes,Err(error)=>return Ok(Err(error.into()))};
            apply_player_trade(&mut tx,&request,&message,&root_revisions,&envelope,tuple).await?;
            // Native deferred SQL guards prove the entire two-line shape. Any
            // failure before or during commit leaves no definitive success.
            let Some(committed)=load_player_trade(&mut tx,&request).await? else{return Err(DurabilityError::InvalidStoredState);};
            let committed=match committed {Ok(value)=>value,Err(error)=>return Ok(Err(error))};
            tx.commit(deadline).await?;
            Ok(Ok(PlayerTradeOutcome::Committed(committed)))
        })).await?
    }
}

async fn apply_player_trade(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    request: &FrozenPlayerTrade,
    message: &OneItemPlayerTradeV1,
    root_revisions: &[u64; 2],
    envelope: &[u8],
    tuple: audit::Type2EventTuple,
) -> Result<(), DurabilityError> {
    sqlx::query("INSERT INTO game_player_trades(occurrence_id,trade_id,transaction_id,event_id,first_game_session_id,first_command_id,trigger_game_session_id,trigger_command_id,request_binding,world_id,channel_id,runtime_scope_ownership_generation,content_digest,occurred_at,envelope_sha256,committed_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,encode($7,'hex')::uuid,$8::text::numeric,$9,encode($10,'hex')::uuid,encode($11,'hex')::uuid,$12::text::numeric,$13,$14,sha256($15),floor(extract(epoch FROM clock_timestamp())*1000)::bigint)")
        .bind(request.occurrence.as_slice()).bind(request.trade_id.as_slice()).bind(request.identity.transaction_id.as_slice()).bind(request.identity.event_id.as_slice())
        .bind(request.first_accept.game_session_id().as_bytes().as_slice()).bind(request.first_accept.command_id().get().to_string())
        .bind(request.trigger.game_session_id().as_bytes().as_slice()).bind(request.trigger.command_id().get().to_string()).bind(request.binding.as_slice())
        .bind(message.world_id.as_slice()).bind(message.channel_id.as_slice()).bind(message.runtime_scope_ownership_generation.to_string())
        .bind(request.content_digest.as_slice()).bind(request.identity.occurred_at_unix_ms).bind(envelope).execute(&mut **tx).await?;
    let data = &request.intent.binding.data;
    for (ordinal, side) in ordered_characters(data).into_iter().enumerate() {
        let participant = &message.participants[ordinal];
        let trader = data.traders[side];
        let bag = &data.backpacks[side];
        let bag_version = data.backpack_versions[side]
            .first()
            .ok_or(DurabilityError::InvalidStoredState)?;
        let count =
            i16::try_from(bag.entries.len()).map_err(|_| DurabilityError::InvalidStoredState)?;
        let maximum = bag
            .entries
            .iter()
            .map(|entry| entry.placement_ordinal)
            .max()
            .ok_or(DurabilityError::InvalidStoredState)?;
        sqlx::query("INSERT INTO game_player_trade_participants(occurrence_id,participant_ordinal,character_id,account_id,game_session_id,command_id,connection_generation,current_connection_generation,character_lease_generation,character_revision,backpack_item_instance_id,backpack_placed_transaction_id,backpack_state_revision,backpack_capacity,entry_count_before,max_ordinal_before) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,$7::text::numeric,$8::text::numeric,$9::text::numeric,$10::text::numeric,encode($11,'hex')::uuid,encode($12,'hex')::uuid,$13::text::numeric,$14,$15,$16::text::numeric)")
            .bind(request.occurrence.as_slice()).bind(i16::try_from(ordinal+1).map_err(|_|DurabilityError::InvalidStoredState)?)
            .bind(trader.fence.character_id.as_bytes().as_slice()).bind(trader.account_id.as_bytes().as_slice())
            .bind(trader.fence.game_session_id.as_bytes().as_slice()).bind(request.intent.binding.accepts[side].command_id().get().to_string())
            .bind(participant.accepted_connection_generation.to_string()).bind(participant.current_connection_generation.to_string())
            .bind(trader.fence.character_lease_generation.to_string()).bind(root_revisions[side].to_string())
            .bind(bag.backpack.item_instance_id.as_slice()).bind(bag_version.placed_transaction_id.as_slice()).bind(bag_version.state_revision.to_string())
            .bind(i16::try_from(request.capacities[side]).map_err(|_|DurabilityError::InvalidStoredState)?).bind(count).bind(maximum.to_string())
            .execute(&mut **tx).await?;
    }
    for (ordinal, line) in message.lines.iter().enumerate() {
        let state = line
            .before
            .as_ref()
            .ok_or(DurabilityError::InvalidStoredState)?;
        let definition = state
            .definition
            .as_ref()
            .ok_or(DurabilityError::InvalidStoredState)?;
        let source = line
            .source
            .as_ref()
            .ok_or(DurabilityError::InvalidStoredState)?;
        let destination = line
            .destination
            .as_ref()
            .ok_or(DurabilityError::InvalidStoredState)?;
        let timed = line.unchanged_timed_state.as_ref();
        let row = timed.and_then(|t| t.row.as_ref());
        let remaining = row
            .and_then(|r| r.remaining_ms)
            .map(i64::try_from)
            .transpose()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        sqlx::query("INSERT INTO game_player_trade_receipts(occurrence_id,line_ordinal,item_instance_id,source_character_id,source_parent_item_instance_id,source_placement_ordinal,source_placed_transaction_id,destination_character_id,destination_parent_item_instance_id,destination_ordinal,definition_family,definition_production_key,definition_revision_ref,quantity,key_number,state_revision_before,timed_revision,timed_charges,timed_remaining_ms,timed_deadline_at) VALUES(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,encode($5,'hex')::uuid,$6::text::numeric,encode($7,'hex')::uuid,encode($8,'hex')::uuid,encode($9,'hex')::uuid,$10::text::numeric,$11,$12,$13,$14,$15,$16::text::numeric,$17::text::numeric,$18,$19,$20)")
            .bind(request.occurrence.as_slice()).bind(i16::try_from(ordinal+1).map_err(|_|DurabilityError::InvalidStoredState)?)
            .bind(state.item_instance_id.as_slice()).bind(source.character_id.as_slice()).bind(source.parent_item_instance_id.as_slice()).bind(source.placement_ordinal.to_string())
            .bind(source.placed_transaction_id.as_slice()).bind(destination.character_id.as_slice()).bind(destination.parent_item_instance_id.as_slice()).bind(destination.placement_ordinal.to_string())
            .bind(&definition.family).bind(&definition.production_key).bind(&definition.revision_ref).bind(i64::from(state.quantity)).bind(i32::try_from(state.key_number).map_err(|_|DurabilityError::InvalidStoredState)?)
            .bind(line.state_revision_before.to_string()).bind(row.map(|r|r.revision.to_string())).bind(row.and_then(|r|r.charges).map(i64::from)).bind(remaining).bind(timed.and_then(|t|t.deadline_at))
            .execute(&mut **tx).await?;
    }
    // Every before-row has been locked and independently re-read. Delete BOTH
    // old locations before inserting either new location; never UPDATE an entry.
    for line in &message.lines {
        let state = line
            .before
            .as_ref()
            .ok_or(DurabilityError::InvalidStoredState)?;
        let source = line
            .source
            .as_ref()
            .ok_or(DurabilityError::InvalidStoredState)?;
        let removed=sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid AND parent_item_instance_id=encode($3,'hex')::uuid AND placement_ordinal=$4::text::numeric AND placed_transaction_id=encode($5,'hex')::uuid")
            .bind(state.item_instance_id.as_slice()).bind(source.character_id.as_slice()).bind(source.parent_item_instance_id.as_slice()).bind(source.placement_ordinal.to_string()).bind(source.placed_transaction_id.as_slice()).execute(&mut **tx).await?;
        if removed.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    for line in &message.lines {
        let state = line
            .before
            .as_ref()
            .ok_or(DurabilityError::InvalidStoredState)?;
        let updated=sqlx::query("UPDATE game_item_instances SET last_transaction_id=encode($2,'hex')::uuid WHERE item_instance_id=encode($1,'hex')::uuid AND lifecycle=1 AND quantity=$3 AND state_revision=$4::text::numeric")
            .bind(state.item_instance_id.as_slice()).bind(request.identity.transaction_id.as_slice()).bind(i64::from(state.quantity)).bind(line.state_revision_before.to_string()).execute(&mut **tx).await?;
        if updated.rows_affected() != 1 {
            return Err(DurabilityError::InvalidStoredState);
        }
    }
    for line in &message.lines {
        let state = line
            .before
            .as_ref()
            .ok_or(DurabilityError::InvalidStoredState)?;
        let destination = line
            .destination
            .as_ref()
            .ok_or(DurabilityError::InvalidStoredState)?;
        sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid)")
            .bind(state.item_instance_id.as_slice()).bind(message.world_id.as_slice()).bind(destination.character_id.as_slice()).bind(destination.parent_item_instance_id.as_slice()).bind(destination.placement_ordinal.to_string()).bind(request.identity.transaction_id.as_slice()).execute(&mut **tx).await?;
    }
    let anchor = message
        .lines
        .first()
        .and_then(|l| l.before.as_ref())
        .ok_or(DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_item_audit_outbox(event_id,transaction_id,transaction_ordinal,transaction_count,event_type_id,schema_revision,retention_profile_id,item_instance_id,occurred_at,expires_at,envelope,envelope_sha256,publication_state,published_at) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,1,2,$3,$4,encode($5,'hex')::uuid,$6,$6+$7,$8,sha256($8),1,NULL)")
        .bind(request.identity.event_id.as_slice()).bind(request.identity.transaction_id.as_slice()).bind(i64::from(tuple.schema_revision())).bind(tuple.retention_profile_id()).bind(anchor.item_instance_id.as_slice()).bind(request.identity.occurred_at_unix_ms).bind(audit::AUDIT_RETENTION_P90D_MS).bind(envelope).execute(&mut **tx).await?;
    Ok(())
}

#[cfg(test)]
#[path = "player_trade_tests.rs"]
mod tests;
