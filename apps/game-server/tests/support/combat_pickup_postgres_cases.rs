// B3-2 PG cases: `combat_pickup::settle_ground_pickup` resolves definition facts from the
// current Content generation (never from the caller) and commits through B3-1's
// `freeze_item_transfer`/`commit_item_transfer`. Reuses the B3-1 harness
// (`item_transfer_postgres_cases`) for PostgreSQL/admission/session bootstrap.

use crate::combat::DurabilitySession;
use crate::combat_pickup::{
    GroundPickupError, GroundPickupRequest, PickupContentError, settle_ground_pickup,
};
use crate::content::{
    CanonicalReferencePlayableContent, ClientProjectionClass, ContentLockBinding, ContentLockEntry,
    CoordinateFrameRef, DefinitionFamily, DefinitionRevisionRef, PackageManifestBinding,
    ProductionAtom, ProductionKey, REFERENCE_PLAYABLE_CAPABILITY_PROFILE,
    REFERENCE_PLAYABLE_CONTENT_PROFILE_ID, ReferenceDefinition, ReferenceDefinitionKind,
    ReferenceEquipmentPattern, ReferenceEquipmentSlot, ReferenceItemContainer,
    ReferenceItemDefinition, ReferenceItemDestination, ReferenceItemEquipment, ReferenceItemField,
    ReferenceItemPhysicalClass, ReferenceItemSemantics, ReferenceItemStack,
    ReferenceItemStackClass, ReferencePlayableContentSource, Sha256HexDigest,
    TypedDefinitionRef as ContentTypedDefinitionRef, link_reference_playable,
};
use crate::domain::CharacterId;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{
    CurrentCharacterItemFence, ItemTransferDestination, ItemTransferOutcome,
};
use crate::foundation::{
    CommandId, CommandRef, ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration, WorldId,
};
use crate::item_transfer_postgres_cases::{
    CHARACTER, Harness, SESSION, TestResult, configured_admin, debug, id, runtime, scope,
};
use ReferenceItemField::{Known, Unknown};

const COIN: &str = "oteryn:pickup.pg.coin";
const BACKPACK: &str = "oteryn:pickup.pg.backpack";
const ABSENT: &str = "oteryn:pickup.pg.absent-from-content";

fn item(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    }
}

fn content_typed(definition: &TypedDefinitionRef) -> TestResult<ContentTypedDefinitionRef> {
    Ok(ContentTypedDefinitionRef::new(
        DefinitionFamily::Item,
        ProductionKey::new(&definition.production_key)?,
        DefinitionRevisionRef::new(&definition.revision_ref)?,
    ))
}

/// A `CanonicalReferencePlayableContent` with the same two admissible Item definitions as the
/// D114 content record and the D82 case they exercise: `COIN` (stackable, proven maximum 30) and
/// `BACKPACK` (container capacity 20, a complete `container`-slot equip pattern, mirroring
/// `oteryn:item.registry.i00002752`). `ABSENT` is deliberately never a key of any definition.
fn pg_content() -> TestResult<CanonicalReferencePlayableContent> {
    let package_key = ProductionKey::new("oteryn:content.pickup-pg")?;
    let package_revision = ProductionAtom::new("pickup PG package revision", "pg-r1")?;
    let package_manifest = PackageManifestBinding::new(
        package_key.clone(),
        package_revision.clone(),
        ProductionAtom::new("pickup PG schema", "schema-v1")?,
        ProductionAtom::new("pickup PG license", "license:project-owned-v1")?,
        Sha256HexDigest::new("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc")?,
    );
    let provenance = package_manifest.package_provenance_digest()?;
    let content_lock = ContentLockBinding {
        revision_digest_token: ProductionAtom::new("pickup PG content lock", "lock:pg-r1")?,
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
            "pickup PG profile",
            REFERENCE_PLAYABLE_CONTENT_PROFILE_ID,
        )?,
        capability_profile: ProductionAtom::new(
            "pickup PG capability profile",
            REFERENCE_PLAYABLE_CAPABILITY_PROFILE,
        )?,
        package_manifest,
        content_lock,
        world_id: WorldId::decode(&id(9))?,
        coordinate_frame: CoordinateFrameRef::new("global-target-2026-09-27")?,
        definitions: vec![
            ReferenceDefinition {
                definition: content_typed(&item(COIN))?,
                kind: ReferenceDefinitionKind::Item(coin),
                client_projection: ClientProjectionClass::ClientSafe,
            },
            ReferenceDefinition {
                definition: content_typed(&item(BACKPACK))?,
                kind: ReferenceDefinitionKind::Item(backpack),
                client_projection: ClientProjectionClass::ClientSafe,
            },
        ],
        placements: vec![],
        ordered_placements: vec![],
        transitions: vec![],
    })?)
}

fn fence() -> TestResult<CurrentCharacterItemFence> {
    Ok(CurrentCharacterItemFence {
        character_id: CharacterId::from_bytes(id(CHARACTER)).map_err(debug)?,
        game_session_id: GameSessionId::decode(&id(SESSION)).map_err(debug)?,
        connection_generation: ConnectionGeneration::new(1).map_err(debug)?,
        character_lease_generation: 1,
        runtime_scope: scope()?,
        scope_ownership_generation: ScopeOwnershipGeneration::new(1).map_err(debug)?,
    })
}

fn command(value: u64) -> TestResult<CommandRef> {
    Ok(CommandRef::new(
        GameSessionId::decode(&id(SESSION)).map_err(debug)?,
        CommandId::new(value).map_err(debug)?,
    ))
}

fn pickup_request(
    command: CommandRef,
    item_instance: [u8; 16],
    claimed: TypedDefinitionRef,
    destination: ItemTransferDestination,
) -> GroundPickupRequest {
    GroundPickupRequest {
        command,
        source_item_instance_id: item_instance,
        source_definition: claimed,
        destination,
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
    }
}

#[test]
fn pickup_resolves_content_facts_for_container_slot_and_backpack_entry() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "pickup").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let content = pg_content()?;
        let session = DurabilitySession {
            root: &harness.root,
            authority: &authority,
            node: &harness.node,
        };

        let backpack_item = harness.mint(&authority, BACKPACK, 1).await?;
        let coin_item = harness.mint(&authority, COIN, 12).await?;

        // Content resolves the backpack's real facts (capacity 20, a complete container-slot
        // equip pattern) with no caller-supplied `ItemDefinitionFacts` anywhere in this call.
        let equipped = settle_ground_pickup(
            &session,
            &content,
            fence()?,
            pickup_request(
                command(1)?,
                backpack_item,
                item(BACKPACK),
                ItemTransferDestination::ContainerSlot,
            ),
        )
        .await
        .map_err(debug)?;
        assert!(matches!(equipped, ItemTransferOutcome::Committed(_)));
        assert!(!harness.on_ground(backpack_item).await?);

        let slot = harness
            .root
            .read_character_backpack(&authority, fence()?.character_id)
            .await
            .map_err(debug)?
            .ok_or("expected an equipped backpack")?;
        assert_eq!(slot.backpack.item_instance_id, backpack_item);
        assert!(slot.entries.is_empty());

        // Content resolves the coin's stack facts (stackable, proven maximum 30) and the
        // equipped backpack's facts (read via `read_character_backpack`, then Content), so a
        // fresh entry commits with no caller-supplied facts either.
        let placed = settle_ground_pickup(
            &session,
            &content,
            fence()?,
            pickup_request(
                command(2)?,
                coin_item,
                item(COIN),
                ItemTransferDestination::MainBackpack,
            ),
        )
        .await
        .map_err(debug)?;
        assert!(matches!(placed, ItemTransferOutcome::Committed(_)));
        assert!(!harness.on_ground(coin_item).await?);

        let backpack = harness
            .root
            .read_character_backpack(&authority, fence()?.character_id)
            .await
            .map_err(debug)?
            .ok_or("expected an equipped backpack")?;
        assert_eq!(backpack.entries.len(), 1);
        assert_eq!(backpack.entries[0].item.item_instance_id, coin_item);
        assert_eq!(backpack.entries[0].item.quantity, 12);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn pickup_is_refused_before_any_write_when_content_has_no_definition_for_the_claimed_item()
-> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "pickup-unknown").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let content = pg_content()?;
        let session = DurabilitySession {
            root: &harness.root,
            authority: &authority,
            node: &harness.node,
        };

        let stray = harness.mint(&authority, ABSENT, 1).await?;
        let before = harness.item_state(stray).await?;

        let outcome = settle_ground_pickup(
            &session,
            &content,
            fence()?,
            pickup_request(
                command(1)?,
                stray,
                item(ABSENT),
                ItemTransferDestination::ContainerSlot,
            ),
        )
        .await;
        assert!(matches!(
            outcome,
            Err(GroundPickupError::Content(
                PickupContentError::DefinitionNotFound
            ))
        ));
        // Content resolution failed before `freeze_item_transfer` ever ran: nothing moved.
        assert!(harness.on_ground(stray).await?);
        assert_eq!(harness.item_state(stray).await?, before);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
