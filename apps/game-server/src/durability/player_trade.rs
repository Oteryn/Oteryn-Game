//! Read-only native Trade inventory observation. Runtime geometry and lifecycle stay
//! with the gameplay owner; these DTOs are immutable data, never reservation authority.

use crate::domain::AccountId;
use crate::durability::character_authority::{ReconciledCharacterAuthority, uuid_text};
use crate::durability::character_equipment::{
    EquipmentError, assert_current_equipment_in_transaction,
};
use crate::durability::db::{begin_semantic_transaction, commit_semantic_transaction};
use crate::durability::item_mint::CORPSE_MATERIALIZATION_PURPOSE_KEY;
use crate::durability::item_mint_audit::{
    ITEM_LIFECYCLE_LIVE, OneItemStateV1, OneItemTypedDefinitionRevisionV1,
};
use crate::durability::item_timed_state::{StoredTimedState, load_timed_state};
use crate::durability::item_transfer::{
    CharacterBackpack, ContainerEntryPosition, CurrentCharacterItemFence,
    GAMEITEM01_STACK_QUANTITY_MAX, load_entries, load_slot,
};
use crate::durability::runtime_scope_assignment::NodeIncarnationProof;
use crate::durability::{DurabilityError, DurabilityRoot};
use crate::foundation::{ChannelRuntimeV1, RuntimeScopeRefV1};
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
