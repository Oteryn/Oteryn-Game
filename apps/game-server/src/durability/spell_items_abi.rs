//! Shared durable spell item/cost/source ABI; no dependency on runtime spell adapters.
//! There is deliberately no second in-memory item store. Reservations are
//! constructed only by locked database reads, not from planner booleans.

use crate::durability::item_mint::{GroundPlacement, TypedDefinitionRef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpellGroundTarget {
    pub(crate) spatial_position: Vec<u8>,
    pub(crate) map_revision: String,
    pub(crate) content_revision: String,
    pub(crate) placement_context: Vec<u8>,
}

/// A source-qualified definition from the compatible content generation. The
/// durable corpse cause is checked separately; appearance membership cannot
/// construct or authorize a corpse reservation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QualifiedItemDefinition {
    pub(crate) definition: TypedDefinitionRef,
    pub(crate) content_generation_digest: [u8; 32],
    pub(crate) materializable: bool,
    pub(crate) movable: bool,
    pub(crate) stack_maximum: u32,
    pub(crate) container_capacity: Option<u16>,
    pub(crate) inventory_destination: bool,
    pub(crate) ground_destination: bool,
    pub(crate) decay: Option<QualifiedItemDecay>,
    pub(crate) decay_chain: Vec<QualifiedItemDecayStage>,
}

/// Complete bounded source field lifecycle, captured in the original mint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QualifiedItemDecayStage {
    pub(crate) definition: TypedDefinitionRef,
    pub(crate) duration_millis: u32,
    pub(crate) target: Option<TypedDefinitionRef>,
    pub(crate) blocks_movement: bool,
    pub(crate) blocks_projectile: bool,
    pub(crate) immovable_block_solid: bool,
}

/// Source-qualified absolute ItemType lifetime. The original mint stores this
/// complete immutable policy; timer execution cannot invent a new definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QualifiedItemDecay {
    pub(crate) duration_millis: u32,
    pub(crate) target: Option<TypedDefinitionRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DurableCorpseReservation {
    pub(crate) item_instance_id: [u8; 16],
    pub(crate) definition: TypedDefinitionRef,
    pub(crate) state_revision: u64,
    pub(crate) top_down_ordinal: u64,
    pub(crate) placement: GroundPlacement,
    pub(crate) mint_transaction_id: [u8; 16],
    pub(crate) quantity: u32,
    pub(crate) contents: Vec<DurableContainedRetirement>,
}

impl DurableCorpseReservation {
    pub(crate) fn item_instance_id(&self) -> &[u8; 16] {
        &self.item_instance_id
    }
    pub(crate) fn definition(&self) -> &TypedDefinitionRef {
        &self.definition
    }
    pub(crate) fn state_revision(&self) -> u64 {
        self.state_revision
    }
    pub(crate) fn top_down_ordinal(&self) -> u64 {
        self.top_down_ordinal
    }
    pub(crate) fn placement(&self) -> &GroundPlacement {
        &self.placement
    }
}

/// The existing combat death owner uses this exact signed 10-byte codec. Any
/// opaque legacy address is unknown; it is never interpreted as an empty cell.
pub(crate) fn decode_ground_cell(bytes: &[u8]) -> Result<SpellItemCell, &'static str> {
    if bytes.len() != 10 {
        return Err("unknown Ground address codec");
    }
    let x = i32::from_be_bytes(bytes[0..4].try_into().map_err(|_| "Ground x")?);
    let y = i32::from_be_bytes(bytes[4..8].try_into().map_err(|_| "Ground y")?);
    let z = i32::from(i16::from_be_bytes(
        bytes[8..10].try_into().map_err(|_| "Ground floor")?,
    ));
    Ok(SpellItemCell { x, y, z })
}

/// Complete caster cost successor, bound into the durable item receipt. The
/// runtime compositor independently compare-commits the same player state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CastCostBinding {
    pub(crate) vitals_revision_before: u64,
    pub(crate) vitals_revision_after: u64,
    pub(crate) mana_before: u32,
    pub(crate) mana_after: u32,
    pub(crate) soul_before: u32,
    pub(crate) soul_after: u32,
    pub(crate) cooldowns_before: Vec<(String, u64)>,
    pub(crate) cooldowns_after: Vec<(String, u64)>,
    pub(crate) caster_digest_before: [u8; 32],
    pub(crate) caster_digest_after: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SpellItemOperation {
    ConsumeCorpse(DurableCorpseReservation),
    RemoveGround(DurableGroundItemReservation),
    MergeInventory(DurableInventoryGrantReservation),
    ConsumeInventory(DurableInventoryConsumption),
    RetireContained(DurableContainedRetirement),
    MintInventory {
        item_instance_id: [u8; 16],
        definition: QualifiedItemDefinition,
        quantity: u32,
        backpack: QualifiedItemDefinition,
        overflow_placement: GroundPlacement,
        allow_ground_overflow: bool,
    },
    MintGround {
        item_instance_id: [u8; 16],
        definition: QualifiedItemDefinition,
        quantity: u32,
        placement: GroundPlacement,
        /// None for permanent conjured items; fixed absolute lifetime for
        /// fields/barriers. The writer timestamps it with the database clock.
        lifetime_millis: Option<u32>,
        /// Qualified source field behavior. Ordinary food/runes have no
        /// movement blocking; barrier payloads carry their explicit flags.
        blocks_movement: bool,
        blocks_projectile: bool,
        /// Source caster attribution, retained with the immutable MINT cause.
        description: Option<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InventoryCustody {
    Container { parent: [u8; 16], ordinal: u64 },
    Equipment { slot: u8, equipment_revision: u64 },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DurableInventoryConsumption {
    pub(crate) item_instance_id: [u8; 16],
    pub(crate) definition: QualifiedItemDefinition,
    pub(crate) state_revision: u64,
    pub(crate) quantity_before: u32,
    pub(crate) quantity_after: u32,
    pub(crate) custody: InventoryCustody,
    pub(crate) caster_ground: GroundPlacement,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DurableContainedRetirement {
    pub(crate) item_instance_id: [u8; 16],
    pub(crate) definition: TypedDefinitionRef,
    pub(crate) state_revision: u64,
    pub(crate) quantity: u32,
    pub(crate) parent: [u8; 16],
    pub(crate) ordinal: u64,
    pub(crate) root: [u8; 16],
    pub(crate) depth: u16,
    pub(crate) root_placement: GroundPlacement,
    pub(crate) corpse_entry: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DurableInventoryGrantReservation {
    pub(crate) item_instance_id: [u8; 16],
    pub(crate) definition: QualifiedItemDefinition,
    pub(crate) state_revision: u64,
    pub(crate) quantity_before: u32,
    pub(crate) quantity_after: u32,
    pub(crate) parent: [u8; 16],
    pub(crate) placement_ordinal: u64,
    pub(crate) caster_ground: GroundPlacement,
}
impl DurableInventoryGrantReservation {
    pub(crate) fn granted_quantity(&self) -> u32 {
        self.quantity_after - self.quantity_before
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DurableGroundItemReservation {
    pub(crate) item_instance_id: [u8; 16],
    pub(crate) definition: TypedDefinitionRef,
    pub(crate) state_revision: u64,
    pub(crate) top_down_ordinal: u64,
    pub(crate) placement: GroundPlacement,
    pub(crate) quantity: u32,
    pub(crate) contents: Vec<DurableContainedRetirement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DurableTileItem {
    pub(crate) item_instance_id: [u8; 16],
    pub(crate) definition: TypedDefinitionRef,
    pub(crate) state_revision: u64,
    pub(crate) top_down_ordinal: u64,
    pub(crate) blocks_movement: bool,
    pub(crate) blocks_projectile: bool,
    pub(crate) immovable_block_solid: bool,
    pub(crate) expires_at_unix_ms: Option<i64>,
    pub(crate) field_origin: Option<DurableFieldOrigin>,
}

/// Historical creation source only. A field tick must independently resolve
/// the current player/lease/placement; these bytes cannot reconstruct an actor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DurableFieldOrigin {
    /// Actual DB-clock timestamp from the immutable originating cast receipt.
    pub(crate) created_at_unix_ms: i64,
    pub(crate) character_id: [u8; 16],
    pub(crate) game_session_id: [u8; 16],
    pub(crate) character_lease_generation: u64,
    pub(crate) actor_placement_digest: [u8; 16],
    pub(crate) source_scope_generation: u64,
    pub(crate) content_digest: [u8; 32],
    pub(crate) catalog_digest: [u8; 32],
    pub(crate) creation_command_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SourceCasterOrigin {
    pub(crate) actor: crate::foundation::ExactActorRef,
    pub(crate) character_lease_generation: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct DurableTileItems {
    pub(crate) items: Vec<DurableTileItem>,
    pub(crate) ownership_generation: u64,
    pub(crate) target: SpellGroundTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SpellItemTransactionRequest {
    pub(crate) command: crate::foundation::CommandRef,
    pub(crate) spell: TypedDefinitionRef,
    pub(crate) catalog_digest: [u8; 32],
    pub(crate) transaction_id: [u8; 16],
    pub(crate) event_id: [u8; 16],
    pub(crate) cost: CastCostBinding,
    pub(crate) caster_origin: Option<SourceCasterOrigin>,
    pub(crate) operations: Vec<SpellItemOperation>,
    pub(crate) companion:
        Option<crate::durability::spell_item_transaction::PreparedCompanionAcquisition>,
    pub(crate) direct_companion:
        Option<crate::durability::spell_item_transaction::PreparedDirectCompanionAcquisition>,
}

/// Receipt descriptor. Only the actual common commit owner can attest its
/// terminal state. Its replay does not authorize a second
/// spawn, cost debit, owner assignment, item creation or corpse consumption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommittedSpellItems {
    pub(crate) transaction_id: [u8; 16],
    pub(crate) event_id: [u8; 16],
    pub(crate) command: crate::foundation::CommandRef,
    pub(crate) binding: [u8; 32],
    pub(crate) cost_binding: [u8; 32],
    pub(crate) item_instance_ids: Vec<[u8; 16]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SpellItemTransactionOutcome {
    Applied(CommittedSpellItems),
    AlreadyCommitted(CommittedSpellItems),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SpellItemCell {
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) z: i32,
}

/// Source-qualified RuneSpell ItemCount consumption (not a separate charge map).
/// The native source adapter compares the actual rune carrier's source ID with
/// the active explicit production binding before constructing this policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QualifiedRuneDefinition {
    pub(crate) item: QualifiedItemDefinition,
    pub(crate) source_item_id: u32,
}
impl QualifiedRuneDefinition {
    pub(crate) fn item(&self) -> &QualifiedItemDefinition {
        &self.item
    }
    pub(crate) fn source_item_id(&self) -> u32 {
        self.source_item_id
    }
}

/// Sealed locked Rune source, bound to the exact physical transaction. The
/// caller must append its consumption to the same cast receipt/SQL transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DurableRuneUseReservation {
    pub(super) physical_transaction: String,
    pub(super) command: crate::foundation::CommandRef,
    pub(super) source: DurableInventoryConsumption,
}
impl DurableRuneUseReservation {
    pub(crate) fn item_instance_id(&self) -> &[u8; 16] {
        &self.source.item_instance_id
    }
    pub(crate) fn state_revision(&self) -> u64 {
        self.source.state_revision
    }
    pub(crate) fn consumption(&self) -> &DurableInventoryConsumption {
        &self.source
    }
}
