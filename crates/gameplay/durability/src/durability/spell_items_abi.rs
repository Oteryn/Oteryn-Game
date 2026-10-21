//! Shared durable spell item/cost/source ABI; no dependency on runtime spell adapters.
//! There is deliberately no second in-memory item store. Reservations are
//! constructed only by locked database reads, not from planner booleans.
#![allow(
    dead_code,
    reason = "spell import candidate; awaits its production owner caller"
)]

use crate::durability::item_mint::{GroundPlacement, TypedDefinitionRef};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellGroundTarget {
    pub spatial_position: Vec<u8>,
    pub map_revision: String,
    pub content_revision: String,
    pub placement_context: Vec<u8>,
}

/// A source-qualified definition from the compatible content generation. The
/// durable corpse cause is checked separately; appearance membership cannot
/// construct or authorize a corpse reservation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedItemDefinition {
    pub definition: TypedDefinitionRef,
    pub content_generation_digest: [u8; 32],
    pub materializable: bool,
    pub movable: bool,
    pub stack_maximum: u32,
    pub container_capacity: Option<u16>,
    pub inventory_destination: bool,
    pub ground_destination: bool,
    pub decay: Option<QualifiedItemDecay>,
    pub decay_chain: Vec<QualifiedItemDecayStage>,
}

/// Complete bounded source field lifecycle, captured in the original mint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedItemDecayStage {
    pub definition: TypedDefinitionRef,
    pub duration_millis: u32,
    pub target: Option<TypedDefinitionRef>,
    pub blocks_movement: bool,
    pub blocks_projectile: bool,
    pub immovable_block_solid: bool,
}

/// Source-qualified absolute ItemType lifetime. The original mint stores this
/// complete immutable policy; timer execution cannot invent a new definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedItemDecay {
    pub duration_millis: u32,
    pub target: Option<TypedDefinitionRef>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableCorpseReservation {
    pub item_instance_id: [u8; 16],
    pub definition: TypedDefinitionRef,
    pub state_revision: u64,
    pub top_down_ordinal: u64,
    pub placement: GroundPlacement,
    pub mint_transaction_id: [u8; 16],
    pub quantity: u32,
    pub contents: Vec<DurableContainedRetirement>,
}

impl DurableCorpseReservation {
    pub fn item_instance_id(&self) -> &[u8; 16] {
        &self.item_instance_id
    }
    pub fn definition(&self) -> &TypedDefinitionRef {
        &self.definition
    }
    pub fn state_revision(&self) -> u64 {
        self.state_revision
    }
    pub fn top_down_ordinal(&self) -> u64 {
        self.top_down_ordinal
    }
    pub fn placement(&self) -> &GroundPlacement {
        &self.placement
    }
}

/// The existing combat death owner uses this exact signed 10-byte codec. Any
/// opaque legacy address is unknown; it is never interpreted as an empty cell.
pub fn decode_ground_cell(bytes: &[u8]) -> Result<SpellItemCell, &'static str> {
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
pub struct CastCostBinding {
    pub vitals_revision_before: u64,
    pub vitals_revision_after: u64,
    pub mana_before: u32,
    pub mana_after: u32,
    pub soul_before: u32,
    pub soul_after: u32,
    pub cooldowns_before: Vec<(String, u64)>,
    pub cooldowns_after: Vec<(String, u64)>,
    pub caster_digest_before: [u8; 32],
    pub caster_digest_after: [u8; 32],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellItemOperation {
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
pub enum InventoryCustody {
    Container { parent: [u8; 16], ordinal: u64 },
    Equipment { slot: u8, equipment_revision: u64 },
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableInventoryConsumption {
    pub item_instance_id: [u8; 16],
    pub definition: QualifiedItemDefinition,
    pub state_revision: u64,
    pub quantity_before: u32,
    pub quantity_after: u32,
    pub custody: InventoryCustody,
    pub caster_ground: GroundPlacement,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableContainedRetirement {
    pub item_instance_id: [u8; 16],
    pub definition: TypedDefinitionRef,
    pub state_revision: u64,
    pub quantity: u32,
    pub parent: [u8; 16],
    pub ordinal: u64,
    pub root: [u8; 16],
    pub depth: u16,
    pub root_placement: GroundPlacement,
    pub corpse_entry: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableInventoryGrantReservation {
    pub item_instance_id: [u8; 16],
    pub definition: QualifiedItemDefinition,
    pub state_revision: u64,
    pub quantity_before: u32,
    pub quantity_after: u32,
    pub parent: [u8; 16],
    pub placement_ordinal: u64,
    pub caster_ground: GroundPlacement,
}
impl DurableInventoryGrantReservation {
    pub fn granted_quantity(&self) -> u32 {
        self.quantity_after - self.quantity_before
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableGroundItemReservation {
    pub item_instance_id: [u8; 16],
    pub definition: TypedDefinitionRef,
    pub state_revision: u64,
    pub top_down_ordinal: u64,
    pub placement: GroundPlacement,
    pub quantity: u32,
    pub contents: Vec<DurableContainedRetirement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableTileItem {
    pub item_instance_id: [u8; 16],
    pub definition: TypedDefinitionRef,
    pub state_revision: u64,
    pub top_down_ordinal: u64,
    pub blocks_movement: bool,
    pub blocks_projectile: bool,
    pub immovable_block_solid: bool,
    pub expires_at_unix_ms: Option<i64>,
    pub field_origin: Option<DurableFieldOrigin>,
    pub creature_field_origin: Option<DurableCreatureFieldOrigin>,
}

/// Historical creation source only. A field tick must independently resolve
/// the current player/lease/placement; these bytes cannot reconstruct an actor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableFieldOrigin {
    /// Actual DB-clock timestamp from the immutable originating cast receipt.
    pub created_at_unix_ms: i64,
    pub character_id: [u8; 16],
    pub game_session_id: [u8; 16],
    pub character_lease_generation: u64,
    pub actor_placement_digest: [u8; 16],
    pub source_scope_generation: u64,
    pub content_digest: [u8; 32],
    pub catalog_digest: [u8; 32],
    pub creation_command_id: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceCasterOrigin {
    pub actor: crate::foundation::ExactActorRef,
    pub character_lease_generation: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DurableTileItems {
    pub items: Vec<DurableTileItem>,
    pub ownership_generation: u64,
    pub target: SpellGroundTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpellItemTransactionRequest {
    pub command: crate::foundation::CommandRef,
    pub spell: TypedDefinitionRef,
    pub catalog_digest: [u8; 32],
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub cost: CastCostBinding,
    pub caster_origin: Option<SourceCasterOrigin>,
    pub operations: Vec<SpellItemOperation>,
    pub companion:
        Option<crate::durability::spell_item_transaction::PreparedCompanionAcquisition>,
    pub direct_companion:
        Option<crate::durability::spell_item_transaction::PreparedDirectCompanionAcquisition>,
}

/// Receipt descriptor. Only the actual common commit owner can attest its
/// terminal state. Its replay does not authorize a second
/// spawn, cost debit, owner assignment, item creation or corpse consumption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedSpellItems {
    pub transaction_id: [u8; 16],
    pub event_id: [u8; 16],
    pub command: crate::foundation::CommandRef,
    pub binding: [u8; 32],
    pub cost_binding: [u8; 32],
    pub item_instance_ids: Vec<[u8; 16]>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellItemTransactionOutcome {
    Applied(CommittedSpellItems),
    AlreadyCommitted(CommittedSpellItems),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpellItemCell {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// Source-qualified RuneSpell ItemCount consumption (not a separate charge map).
/// The native source adapter compares the actual rune carrier's source ID with
/// the active explicit production binding before constructing this policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedRuneDefinition {
    pub item: QualifiedItemDefinition,
    pub source_item_id: u32,
}
impl QualifiedRuneDefinition {
    pub fn item(&self) -> &QualifiedItemDefinition {
        &self.item
    }
    pub fn source_item_id(&self) -> u32 {
        self.source_item_id
    }
}

/// Sealed locked Rune source, bound to the exact physical transaction. The
/// caller must append its consumption to the same cast receipt/SQL transaction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DurableRuneUseReservation {
    pub(super) physical_transaction: String,
    pub(super) command: crate::foundation::CommandRef,
    pub(super) source: DurableInventoryConsumption,
}
impl DurableRuneUseReservation {
    pub fn item_instance_id(&self) -> &[u8; 16] {
        &self.source.item_instance_id
    }
    pub fn state_revision(&self) -> u64 {
        self.source.state_revision
    }
    pub fn consumption(&self) -> &DurableInventoryConsumption {
        &self.source
    }
}

/// Immutable historical Creature cause. It grants no live actor or Item authority.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DurableCreatureFieldOrigin {
    pub world: [u8; 16],
    pub channel: [u8; 16],
    pub created_at_unix_ms: i64,
    pub source_scope_generation: u64,
    pub content_digest: [u8; 32],
    pub catalog_digest: [u8; 32],
    pub map_digest: [u8; 32],
    pub frame_digest: [u8; 32],
    pub source_body_digest: [u8; 32],
    pub source_cast_digest: [u8; 32],
    pub actor_placement: [u8; 16],
    pub actor_generation: u64,
    pub creature_key: String,
    pub creature_revision: String,
    pub ability_key: String,
    pub ability_revision: String,
    pub occurrence: String,
    pub item_instance: [u8; 16],
    pub item_key: String,
    pub item_revision: String,
}
impl DurableCreatureFieldOrigin {
    fn valid(&self) -> bool {
        self.world != [0; 16]
            && self.channel != [0; 16]
            && self.created_at_unix_ms >= 0
            && self.source_scope_generation > 0
            && self.actor_generation > 0
            && self.actor_placement != [0; 16]
            && self.item_instance != [0; 16]
            && [
                self.content_digest,
                self.catalog_digest,
                self.map_digest,
                self.frame_digest,
                self.source_body_digest,
                self.source_cast_digest,
            ]
            .iter()
            .all(|d| *d != [0; 32])
            && self.content_digest == self.catalog_digest
            && self.creature_key.starts_with("oteryn:creature.")
            && self.creature_key.len() <= 512
            && self.ability_key.starts_with("oteryn:ability.")
            && self.ability_key.len() <= 512
            && self.item_key.starts_with("oteryn:item.tibia.i")
            && self.item_key.len() <= 512
            && [
                &self.creature_revision,
                &self.ability_revision,
                &self.item_revision,
            ]
            .iter()
            .all(|v| !v.is_empty() && v.len() <= 512)
            && !self.occurrence.is_empty()
            && self.occurrence.len() <= 1024
    }
    pub fn condition_source_key(&self) -> Option<String> {
        self.valid().then_some(())?;
        Some(format!(
            "creature:field:{}",
            serde_json::to_string(self).ok()?
        ))
    }
    pub fn from_condition_source(source: &str) -> Option<Self> {
        let encoded = source.strip_prefix("creature:field:")?;
        if encoded.len() > 16384 {
            return None;
        }
        let origin: Self = serde_json::from_str(encoded).ok()?;
        (origin.valid() && origin.condition_source_key().as_deref() == Some(source))
            .then_some(origin)
    }
    pub fn matches_condition_source(&self, source: &str) -> bool {
        Self::from_condition_source(source).as_ref() == Some(self)
    }
}

#[cfg(test)]
mod creature_field_origin_tests {
    use super::*;
    #[test]
    fn typed_creature_field_source_roundtrip_refuses_truncation_unknown_fields_and_zero_generation()
    -> Result<(), Box<dyn std::error::Error>> {
        let origin = DurableCreatureFieldOrigin {
            world: [1; 16],
            channel: [2; 16],
            created_at_unix_ms: 10,
            source_scope_generation: 1,
            content_digest: [3; 32],
            catalog_digest: [3; 32],
            map_digest: [4; 32],
            frame_digest: [5; 32],
            source_body_digest: [6; 32],
            source_cast_digest: [7; 32],
            actor_placement: [8; 16],
            actor_generation: 1,
            creature_key: "oteryn:creature.fire_elemental".into(),
            creature_revision: "definition-r1".into(),
            ability_key: "oteryn:ability.creature.fire_elemental.attack-3".into(),
            ability_revision: "definition-r1".into(),
            occurrence: "source-fixture-A".into(),
            item_instance: [9; 16],
            item_key: "oteryn:item.tibia.i2118".into(),
            item_revision: "definition-r1".into(),
        };
        let key = origin.condition_source_key().ok_or("source key")?;
        assert_eq!(
            DurableCreatureFieldOrigin::from_condition_source(&key),
            Some(origin.clone())
        );
        assert!(origin.matches_condition_source(&key));
        assert!(
            DurableCreatureFieldOrigin::from_condition_source("creature:field:truncated").is_none()
        );
        let bad = key.trim_end_matches('}').to_owned() + ",\"player\":true}";
        assert!(DurableCreatureFieldOrigin::from_condition_source(&bad).is_none());
        let mut bad = origin;
        bad.actor_generation = 0;
        assert!(bad.condition_source_key().is_none());
        Ok(())
    }
}
