//! B3-2: Combat ground pickup wiring (`B3-INVENTORY-DESTINATION-CAPACITY-STACKS-V1`, D80-D83
//! and D114). B3-1 (`durability::item_transfer`) admits a TRANSFER whose caller supplies
//! `ItemDefinitionFacts` directly and only checks the moved item's *identity* against the
//! stored source; it deliberately left "binding [definition facts] to the current Content" to
//! this component (`docs/agents/tasks/active/OTV2-20260928-b3-1-transfer.md`, "Open
//! follow-ups").
//!
//! [`settle_ground_pickup`] never accepts caller-supplied `ItemDefinitionFacts`: its caller
//! names which item it means to pick up ([`GroundPickupRequest::source_definition`], exactly as
//! `source_item_instance_id` is already caller-known) and where to; the stack class, container
//! capacity and `container`-slot equip pattern of that item, and (for `MainBackpack`) of the
//! character's currently equipped backpack, are always read from the current
//! [`CanonicalReferencePlayableContent`] generation through [`resolve_item_definition_facts`].
//! `freeze_item_transfer`'s own admission still cross-checks the resolved identity against the
//! authoritative stored item, so a caller that names the wrong item is refused
//! (`DefinitionMismatch`), never trusted.
//!
//! Like D2a/D2b (`combat::death_reward`, `combat::loot_plan`), this has no production caller
//! yet: GAME-INTERACTION's network dispatch (resolving `source_definition` from a live Ground
//! listing, proving the CommandRef is still pending before commit) is a later admission stage.
//!
//! D3-5 ([`settle_corpse_pickup`], decision D134) is the same pickup with a corpse-container
//! source: its caller names the corpse as well as the entry. Neither request trusts the family it
//! names. Both read where the item currently is and refuse a mismatch (`SourceMismatch`), so a
//! Ground request never takes a corpse entry and a corpse request never takes a Ground item or
//! an entry of a different corpse. The D133 window and the corpse's own exclusion stay entirely
//! in `durability::item_transfer`'s admission and its database guards.
//!
//! This is `crate::combat_pickup` (a top-level module, not `combat::pickup`): `combat.rs` is
//! also recompiled standalone by `foundation/mod.rs`'s `#[cfg(test)] mod exact_actor_test_combat`
//! and by PG test binaries that need `combat::death_reward`/`combat::loot_plan` but not Content,
//! so a `crate::content` dependency inside `combat.rs`'s own module tree would force every one
//! of those to also carry a local `content` module. This module reaches
//! `combat::death_reward::DurabilitySession` through `combat`'s own `pub(crate)` re-export.

use crate::combat::DurabilitySession;
use crate::content::{
    CanonicalReferencePlayableContent, DefinitionFamily, ReferenceDefinitionKind,
    ReferenceItemField, ReferenceItemStackClass,
};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{
    CurrentCharacterItemFence, ItemDefinitionFacts, ItemSourceLocation, ItemStackClass,
    ItemTransferDestination, ItemTransferError, ItemTransferOutcome, ItemTransferRequest,
};
use crate::foundation::CommandRef;

/// The Content-binding half of B3-2: no admission, no identity check, no I/O.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PickupContentError {
    /// No definition in the current Content generation matches the claimed identity.
    DefinitionNotFound,
    /// The identity resolves to a definition that is not an Item.
    NotAnItem,
}

impl std::fmt::Display for PickupContentError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DefinitionNotFound => {
                formatter.write_str("pickup: no Content definition matches the claimed identity")
            }
            Self::NotAnItem => formatter.write_str("pickup: claimed identity is not an Item"),
        }
    }
}

impl std::error::Error for PickupContentError {}

const fn family_wire_name(family: DefinitionFamily) -> &'static str {
    match family {
        DefinitionFamily::Terrain => "Terrain",
        DefinitionFamily::Presentation => "Presentation",
        DefinitionFamily::LocalObject => "LocalObject",
        DefinitionFamily::Creature => "Creature",
        DefinitionFamily::Item => "Item",
        DefinitionFamily::Loot => "Loot",
        DefinitionFamily::Ability => "Ability",
        DefinitionFamily::Effect => "Effect",
        DefinitionFamily::Formula => "Formula",
        DefinitionFamily::Behavior => "Behavior",
    }
}

/// D82/D83/GAME-ITEM-01 §6.2 facts of `definition`, read from `content`. Never trusts a caller:
/// a definition absent from `content`, or present under a non-Item family/kind, fails closed
/// the same way D82 fails closed on an unknown stack class.
pub(crate) fn resolve_item_definition_facts(
    content: &CanonicalReferencePlayableContent,
    definition: &TypedDefinitionRef,
) -> Result<ItemDefinitionFacts, PickupContentError> {
    let found = content.definitions.iter().find(|candidate| {
        family_wire_name(candidate.definition.family()) == definition.family
            && candidate.definition.key().as_str() == definition.production_key
            && candidate.definition.revision().as_str() == definition.revision_ref
    });
    let Some(found) = found else {
        return Err(PickupContentError::DefinitionNotFound);
    };
    let ReferenceDefinitionKind::Item(item) = &found.kind else {
        return Err(PickupContentError::NotAnItem);
    };

    let stack = match item.stack_class {
        ReferenceItemStackClass::Unknown => ItemStackClass::Unknown,
        ReferenceItemStackClass::NonStackable => ItemStackClass::NonStackable,
        ReferenceItemStackClass::StackCapable => {
            let proven_maximum = match &item.semantics.stack {
                ReferenceItemField::Known(stack) => match stack.stack_max {
                    ReferenceItemField::Known(maximum) => Some(u32::from(maximum)),
                    _ => None,
                },
                _ => None,
            };
            ItemStackClass::Stackable { proven_maximum }
        }
    };

    // Only a definitively KNOWN capacity is surfaced; a container whose capacity is not
    // KNOWN fails closed as a non-container, the same D82 posture as an unknown stack class.
    let container_capacity = match &item.semantics.container {
        ReferenceItemField::Known(container) => match container.capacity {
            ReferenceItemField::Known(capacity) => Some(u32::from(capacity)),
            _ => None,
        },
        _ => None,
    };

    // GAME-ITEM-01 §6.2: a *complete* equip pattern whose primary slot is `container`. Being a
    // container is not enough (checked separately via `container_capacity`, above). Content's
    // own link/compile-time validation already rejects an incomplete or malformed KNOWN
    // pattern set before it can reach a live generation, so this only asks whether one KNOWN
    // pattern's primary slot is `container`.
    let container_slot_equip_pattern = matches!(
        &item.semantics.equipment,
        ReferenceItemField::Known(equipment)
            if matches!(
                &equipment.patterns,
                ReferenceItemField::Known(patterns)
                    if patterns.iter().any(|pattern| matches!(
                        pattern.primary_slot,
                        ReferenceItemField::Known(crate::content::ReferenceEquipmentSlot::Container)
                    ))
            )
    );

    Ok(ItemDefinitionFacts {
        definition: definition.clone(),
        stack,
        container_capacity,
        container_slot_equip_pattern,
    })
}

/// Ground-pickup intent as GAME-INTERACTION resolves it: which item, where to. Deliberately
/// carries no `ItemDefinitionFacts`; only [`settle_ground_pickup`] resolves those, from Content.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GroundPickupRequest {
    pub(crate) command: CommandRef,
    pub(crate) source_item_instance_id: [u8; 16],
    /// The claimed identity of the moved item. `freeze_item_transfer`'s own admission still
    /// cross-checks it against the authoritative stored source (`DefinitionMismatch` on a lie);
    /// this component only trusts Content for the identity's *facts*, never for its truth.
    pub(crate) source_definition: TypedDefinitionRef,
    pub(crate) destination: ItemTransferDestination,
    pub(crate) content_revision: String,
    pub(crate) ruleset_revision: String,
    pub(crate) sim_revision: String,
}

/// D3-5 corpse-container pickup intent: the [`GroundPickupRequest`] shape, with the source
/// named as an entry of one corpse instead of a bare Ground item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CorpsePickupRequest {
    pub(crate) command: CommandRef,
    pub(crate) corpse_item_instance_id: [u8; 16],
    /// The loot entry to take out of that corpse.
    pub(crate) source_item_instance_id: [u8; 16],
    /// As [`GroundPickupRequest::source_definition`].
    pub(crate) source_definition: TypedDefinitionRef,
    pub(crate) destination: ItemTransferDestination,
    pub(crate) content_revision: String,
    pub(crate) ruleset_revision: String,
    pub(crate) sim_revision: String,
}

#[derive(Debug)]
pub(crate) enum GroundPickupError {
    /// The claimed item, or (for `MainBackpack`) the currently equipped backpack, has no
    /// admissible Content definition.
    Content(PickupContentError),
    /// The item is a live source, but not the one the request named: a Ground request for a
    /// corpse entry, or a corpse request for a Ground item or another corpse's entry.
    SourceMismatch,
    Transfer(ItemTransferError),
}

impl From<ItemTransferError> for GroundPickupError {
    fn from(error: ItemTransferError) -> Self {
        Self::Transfer(error)
    }
}

impl std::fmt::Display for GroundPickupError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Content(error) => write!(formatter, "pickup content resolution: {error}"),
            Self::SourceMismatch => {
                formatter.write_str("pickup: the item is not at the source the request named")
            }
            Self::Transfer(error) => write!(formatter, "pickup transfer: {error}"),
        }
    }
}

impl std::error::Error for GroundPickupError {}

/// B3-2 entry point: resolve `request`'s definition facts from `content` (never from the
/// caller), then commit the pickup through B3-1's `freeze_item_transfer`/`commit_item_transfer`.
/// For `MainBackpack`, the equipped backpack's identity is read via the already-proven
/// `read_character_backpack`, and its facts are resolved from `content` the same way; no
/// backpack equipped is passed through as `backpack: None`, so the existing
/// `ItemTransferRefusal::NoMainBackpack` (D80) still fires, unchanged.
pub(crate) async fn settle_ground_pickup(
    session: &DurabilitySession<'_, '_, '_>,
    content: &CanonicalReferencePlayableContent,
    fence: CurrentCharacterItemFence,
    request: GroundPickupRequest,
) -> Result<ItemTransferOutcome, GroundPickupError> {
    settle_pickup(session, content, fence, ItemSourceLocation::Ground, request).await
}

/// D3-5 entry point: [`settle_ground_pickup`] for an entry of the named corpse. Whether the
/// requester may take it yet (D133) is decided by the TRANSFER admission, not here.
pub(crate) async fn settle_corpse_pickup(
    session: &DurabilitySession<'_, '_, '_>,
    content: &CanonicalReferencePlayableContent,
    fence: CurrentCharacterItemFence,
    request: CorpsePickupRequest,
) -> Result<ItemTransferOutcome, GroundPickupError> {
    let source = ItemSourceLocation::CorpseEntry {
        corpse_item_instance_id: request.corpse_item_instance_id,
    };
    let request = GroundPickupRequest {
        command: request.command,
        source_item_instance_id: request.source_item_instance_id,
        source_definition: request.source_definition,
        destination: request.destination,
        content_revision: request.content_revision,
        ruleset_revision: request.ruleset_revision,
        sim_revision: request.sim_revision,
    };
    settle_pickup(session, content, fence, source, request).await
}

async fn settle_pickup(
    session: &DurabilitySession<'_, '_, '_>,
    content: &CanonicalReferencePlayableContent,
    fence: CurrentCharacterItemFence,
    named_source: ItemSourceLocation,
    request: GroundPickupRequest,
) -> Result<ItemTransferOutcome, GroundPickupError> {
    let item = resolve_item_definition_facts(content, &request.source_definition)
        .map_err(GroundPickupError::Content)?;
    // A live item at another source than the named one is refused. An item that is no longer
    // a live source at all falls through, so a replayed command still finds its receipt and a
    // stale one gets the TRANSFER's own `SourceNotOnGround`.
    if let Some(current) = session
        .root
        .read_item_source_location(session.authority, request.source_item_instance_id)
        .await?
        && current != named_source
    {
        return Err(GroundPickupError::SourceMismatch);
    }
    let backpack = match request.destination {
        ItemTransferDestination::ContainerSlot => None,
        ItemTransferDestination::MainBackpack => {
            match session
                .root
                .read_character_backpack(session.authority, fence.character_id)
                .await?
            {
                Some(current) => Some(
                    resolve_item_definition_facts(content, &current.backpack.definition)
                        .map_err(GroundPickupError::Content)?,
                ),
                None => None,
            }
        }
    };
    let transfer_request = ItemTransferRequest {
        command: request.command,
        source_item_instance_id: request.source_item_instance_id,
        destination: request.destination,
        item,
        backpack,
        content_revision: request.content_revision,
        ruleset_revision: request.ruleset_revision,
        sim_revision: request.sim_revision,
    };
    let mut candidate = session
        .root
        .freeze_item_transfer(session.authority, session.node, fence, transfer_request)
        .await?;
    Ok(session
        .root
        .commit_item_transfer(session.authority, session.node, fence, &mut candidate)
        .await?)
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::content::{
        ClientProjectionClass, ContentError, ContentLockBinding, ContentLockEntry,
        CoordinateFrameRef, DefinitionRevisionRef, PackageManifestBinding, ProductionAtom,
        ProductionKey, REFERENCE_PLAYABLE_CAPABILITY_PROFILE,
        REFERENCE_PLAYABLE_CONTENT_PROFILE_ID, ReferenceDefinition, ReferenceEquipmentPattern,
        ReferenceEquipmentSlot, ReferenceItemContainer, ReferenceItemDefinition,
        ReferenceItemDestination, ReferenceItemEquipment, ReferenceItemPhysicalClass,
        ReferenceItemSemantics, ReferenceItemStack, ReferencePlayableContentSource,
        Sha256HexDigest, TypedDefinitionRef as ContentTypedDefinitionRef, link_reference_playable,
    };
    use crate::foundation::WorldId;
    use ReferenceItemField::{Known, Unknown};

    const COIN: &str = "oteryn:pickup.demo.coin";
    const BACKPACK: &str = "oteryn:pickup.demo.backpack";
    const UNKNOWN_STACK: &str = "oteryn:pickup.demo.unknown-stack";

    /// This fixture module only ever builds Item definitions.
    fn typed(definition: TypedDefinitionRef) -> Result<ContentTypedDefinitionRef, ContentError> {
        assert_eq!(definition.family, "Item");
        Ok(ContentTypedDefinitionRef::new(
            DefinitionFamily::Item,
            ProductionKey::new(&definition.production_key)?,
            DefinitionRevisionRef::new(&definition.revision_ref)?,
        ))
    }

    fn item(key: &str) -> TypedDefinitionRef {
        TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: "definition-r1".into(),
        }
    }

    fn identity_only_item() -> ReferenceItemDefinition {
        ReferenceItemDefinition {
            physical_class: ReferenceItemPhysicalClass::Unknown,
            materializable: false,
            stack_class: ReferenceItemStackClass::Unknown,
            legal_destinations: Vec::new(),
            semantics: ReferenceItemSemantics::default(),
        }
    }

    /// A minimal `CanonicalReferencePlayableContent` with three Item definitions:
    /// - `COIN`: stackable, proven maximum 30.
    /// - `BACKPACK`: container capacity 20, a complete `container`-slot equip pattern (mirrors
    ///   the D114 content record for `oteryn:item.tibia.i2854`).
    /// - `UNKNOWN_STACK`: identity-only, D82 fail-closed.
    fn fixture_content() -> Result<CanonicalReferencePlayableContent, Box<dyn std::error::Error>> {
        let package_key = ProductionKey::new("oteryn:content.pickup-demo")?;
        let package_revision = ProductionAtom::new("pickup test package revision", "demo-r1")?;
        let package_manifest = PackageManifestBinding::new(
            package_key.clone(),
            package_revision.clone(),
            ProductionAtom::new("pickup test schema", "schema-v1")?,
            ProductionAtom::new("pickup test license", "license:project-owned-v1")?,
            Sha256HexDigest::new(
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            )?,
        );
        let provenance = package_manifest.package_provenance_digest()?;
        let content_lock = ContentLockBinding {
            revision_digest_token: ProductionAtom::new("pickup test content lock", "lock:demo-r1")?,
            entries: vec![ContentLockEntry::exact(
                package_key,
                package_revision,
                provenance,
            )],
        };

        let coin = ReferenceItemDefinition {
            physical_class: ReferenceItemPhysicalClass::Physical,
            materializable: true,
            stack_class: ReferenceItemStackClass::StackCapable,
            legal_destinations: vec![ReferenceItemDestination::CharacterInventory],
            semantics: ReferenceItemSemantics {
                stack: Known(ReferenceItemStack {
                    stackable: Known(true),
                    stack_max: Known(30),
                }),
                ..ReferenceItemSemantics::default()
            },
        };
        let backpack = ReferenceItemDefinition {
            physical_class: ReferenceItemPhysicalClass::Physical,
            materializable: true,
            stack_class: ReferenceItemStackClass::NonStackable,
            legal_destinations: vec![ReferenceItemDestination::CharacterInventory],
            semantics: ReferenceItemSemantics {
                container: Known(ReferenceItemContainer {
                    capacity: Known(20),
                }),
                equipment: Known(ReferenceItemEquipment {
                    patterns: Known(vec![ReferenceEquipmentPattern {
                        pattern_id: 1,
                        primary_slot: Known(ReferenceEquipmentSlot::Container),
                        additional_reserved_slots: Unknown,
                        mutually_exclusive_groups: Unknown,
                        vocations: Unknown,
                        level: Unknown,
                        compatibility_rule: Unknown,
                    }]),
                }),
                ..ReferenceItemSemantics::default()
            },
        };

        Ok(link_reference_playable(ReferencePlayableContentSource {
            profile_revision: ProductionAtom::new(
                "pickup test profile",
                REFERENCE_PLAYABLE_CONTENT_PROFILE_ID,
            )?,
            capability_profile: ProductionAtom::new(
                "pickup test capability profile",
                REFERENCE_PLAYABLE_CAPABILITY_PROFILE,
            )?,
            package_manifest,
            content_lock,
            world_id: WorldId::decode(&[
                7, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 7,
            ])?,
            coordinate_frame: CoordinateFrameRef::new("global-target-2026-09-27")?,
            definitions: vec![
                ReferenceDefinition {
                    definition: typed(item(COIN))?,
                    kind: ReferenceDefinitionKind::Item(coin),
                    client_projection: ClientProjectionClass::ClientSafe,
                },
                ReferenceDefinition {
                    definition: typed(item(BACKPACK))?,
                    kind: ReferenceDefinitionKind::Item(backpack),
                    client_projection: ClientProjectionClass::ClientSafe,
                },
                ReferenceDefinition {
                    definition: typed(item(UNKNOWN_STACK))?,
                    kind: ReferenceDefinitionKind::Item(identity_only_item()),
                    client_projection: ClientProjectionClass::ClientSafe,
                },
            ],
            placements: vec![],
            ordered_placements: vec![],
            transitions: vec![],
        })?)
    }

    #[test]
    fn resolves_stackable_facts_with_proven_maximum() -> Result<(), Box<dyn std::error::Error>> {
        let content = fixture_content()?;
        let facts = resolve_item_definition_facts(&content, &item(COIN))?;
        assert_eq!(
            facts.stack,
            ItemStackClass::Stackable {
                proven_maximum: Some(30)
            }
        );
        assert_eq!(facts.container_capacity, None);
        assert!(!facts.container_slot_equip_pattern);
        Ok(())
    }

    #[test]
    fn resolves_backpack_container_and_equip_pattern_facts()
    -> Result<(), Box<dyn std::error::Error>> {
        let content = fixture_content()?;
        let facts = resolve_item_definition_facts(&content, &item(BACKPACK))?;
        assert_eq!(facts.stack, ItemStackClass::NonStackable);
        assert_eq!(facts.container_capacity, Some(20));
        assert!(facts.container_slot_equip_pattern);
        Ok(())
    }

    #[test]
    fn unknown_stack_class_fails_closed() -> Result<(), Box<dyn std::error::Error>> {
        let content = fixture_content()?;
        let facts = resolve_item_definition_facts(&content, &item(UNKNOWN_STACK))?;
        assert_eq!(facts.stack, ItemStackClass::Unknown);
        assert_eq!(facts.container_capacity, None);
        assert!(!facts.container_slot_equip_pattern);
        Ok(())
    }

    #[test]
    fn missing_definition_is_refused() -> Result<(), Box<dyn std::error::Error>> {
        let content = fixture_content()?;
        assert_eq!(
            resolve_item_definition_facts(&content, &item("oteryn:pickup.test.absent")),
            Err(PickupContentError::DefinitionNotFound)
        );
        Ok(())
    }

    #[test]
    fn wrong_family_is_refused() -> Result<(), Box<dyn std::error::Error>> {
        let content = fixture_content()?;
        let wrong_family = TypedDefinitionRef {
            family: "Creature".into(),
            ..item(COIN)
        };
        assert_eq!(
            resolve_item_definition_facts(&content, &wrong_family),
            Err(PickupContentError::DefinitionNotFound)
        );
        Ok(())
    }
}
