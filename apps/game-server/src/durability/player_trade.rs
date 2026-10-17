//! PLAYER-TRADE0 native inventory observation and atomic two-Character swap.
//!
//! This leaf provides native inventory observation, typed before/after data and the bounded
//! type-2 codec. The real writer and its private runtime-issued request live in the owning
//! gameplay_transport::player_trade module. Keeping these storage/codec dependencies separate
//! lets the existing standalone durability test roots mount their real modules without a
//! synthetic Content or gameplay owner. Migration0100 preserves the full latest0095 union.

#![allow(
    dead_code,
    reason = "native Trade writer awaits its shared owner installation"
)]

use crate::domain::{AccountId, CharacterId};
use crate::durability::character_authority::{
    ReconciledCharacterAuthority, SERVER_BUILD_ID, uuid_text,
};
use crate::durability::character_equipment::{
    EquipmentError, assert_current_equipment_in_transaction,
};
use crate::durability::db::begin_semantic_transaction;
use crate::durability::item_mint::CORPSE_MATERIALIZATION_PURPOSE_KEY;
use crate::durability::item_mint_audit::{
    self as audit, AuditError, ITEM_LIFECYCLE_LIVE, OneItemStateV1,
    OneItemTypedDefinitionRevisionV1,
};
use crate::durability::item_timed_state::{StoredTimedState, load_timed_state};
use crate::durability::item_transfer::{
    CharacterBackpack, ContainerEntryPosition, CurrentCharacterItemFence,
    GAMEITEM01_STACK_QUANTITY_MAX, load_entries, load_slot,
};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::{ChannelRuntimeV1, CommandRef, RuntimeScopeRefV1};
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
pub(crate) const PLAYER_TRADE_CAUSE: &str = "player_trade";

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

pub(crate) fn audit_command(
    command: CommandRef,
) -> super::item_transfer_audit::OneItemCommandRefV1 {
    super::item_transfer_audit::OneItemCommandRefV1 {
        game_session_id: command.game_session_id().as_bytes().to_vec(),
        command_id: command.command_id().get(),
    }
}

pub(crate) fn audit_timed(value: Option<StoredTimedState>) -> Option<OneItemPlayerTradeTimedV1> {
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

pub(crate) fn encode_player_trade_event(
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

pub(crate) fn first_fence_continues(
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

#[cfg(test)]
#[path = "player_trade_tests.rs"]
mod tests;
