// B3-2 PG cases: `combat_pickup::settle_ground_pickup` resolves definition facts from the
// current Content generation (never from the caller) and commits through B3-1's
// `freeze_item_transfer`/`commit_item_transfer`. Reuses the B3-1 harness
// (`item_transfer_postgres_cases`) for PostgreSQL/admission/session bootstrap.

use crate::combat::DurabilitySession;
use crate::combat_pickup::{
    CorpsePickupRequest, GroundPickupError, GroundPickupRequest, PickupContentError,
    settle_corpse_pickup, settle_ground_pickup,
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
use crate::corpse_transfer_postgres_cases::{
    OTHER_TOP, mint_corpse, put_loot_of, set_materialized_ago,
};
use crate::domain::CharacterId;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{
    CurrentCharacterItemFence, ItemTransferDestination, ItemTransferError, ItemTransferOutcome,
    ItemTransferRefusal,
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
/// `oteryn:item.tibia.i2854`). `ABSENT` is deliberately never a key of any definition.
fn pg_content() -> TestResult<CanonicalReferencePlayableContent> {
    let package_key = ProductionKey::new("oteryn:content.pickup-pg")?;
    let package_revision = ProductionAtom::new("pickup PG package revision", "pg-r1")?;
    let package_manifest = PackageManifestBinding::new(
        package_key.clone(),
        package_revision.clone(),
        ProductionAtom::new("pickup PG schema", "schema-v1")?,
        ProductionAtom::new("pickup PG license", "license:project-owned-v1")?,
        Sha256HexDigest::new("cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc")?,
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
        let harness = Harness::create(admin, "pickupunknown").await?;
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

fn corpse_request(
    command: CommandRef,
    corpse: [u8; 16],
    item_instance: [u8; 16],
) -> CorpsePickupRequest {
    CorpsePickupRequest {
        command,
        corpse_item_instance_id: corpse,
        source_item_instance_id: item_instance,
        source_definition: item(COIN),
        destination: ItemTransferDestination::MainBackpack,
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
    }
}

fn is_refusal(
    outcome: &Result<ItemTransferOutcome, GroundPickupError>,
    reason: ItemTransferRefusal,
) -> bool {
    matches!(
        outcome,
        Err(GroundPickupError::Transfer(ItemTransferError::Refused(found))) if *found == reason
    )
}

/// D3-5 (D134): a corpse pickup names its corpse, and each request only takes from the source
/// it names; the D133 window and the corpse's own exclusion come through unchanged from the
/// TRANSFER admission; a replayed command transfers once.
#[test]
fn corpse_pickup_takes_only_from_the_named_corpse_within_the_window_rules() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "corpsepickup").await?;
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
        let ground_coin = harness.mint(&authority, COIN, 1).await?;

        let owned = mint_corpse(&harness, &authority, 2000, id(CHARACTER)).await?;
        let foreign = mint_corpse(&harness, &authority, 2001, id(OTHER_TOP)).await?;
        let owned_loot = put_loot_of(&harness, &item(COIN), owned, 2000, 1, 120).await?;
        let foreign_loot = put_loot_of(&harness, &item(COIN), foreign, 2001, 1, 124).await?;
        set_materialized_ago(&harness, owned, 1_000).await?;
        set_materialized_ago(&harness, foreign, 1_000).await?;

        // A request naming the wrong source family or the wrong corpse is refused before any
        // TRANSFER is frozen, and nothing moves.
        let before = harness.footprint().await?;
        let ground_for_entry = settle_ground_pickup(
            &session,
            &content,
            fence()?,
            pickup_request(
                command(2)?,
                owned_loot,
                item(COIN),
                ItemTransferDestination::MainBackpack,
            ),
        )
        .await;
        assert!(matches!(
            ground_for_entry,
            Err(GroundPickupError::SourceMismatch)
        ));
        for (corpse, entry) in [(foreign, owned_loot), (owned, ground_coin)] {
            let outcome = settle_corpse_pickup(
                &session,
                &content,
                fence()?,
                corpse_request(command(2)?, corpse, entry),
            )
            .await;
            assert!(matches!(outcome, Err(GroundPickupError::SourceMismatch)));
        }
        assert_eq!(harness.footprint().await?, before);

        // The top-damage Character takes its entry inside the window; a replay transfers once.
        let taken = settle_corpse_pickup(
            &session,
            &content,
            fence()?,
            corpse_request(command(3)?, owned, owned_loot),
        )
        .await
        .map_err(debug)?;
        assert!(matches!(taken, ItemTransferOutcome::Committed(_)));
        let replay = settle_corpse_pickup(
            &session,
            &content,
            fence()?,
            corpse_request(command(3)?, owned, owned_loot),
        )
        .await
        .map_err(debug)?;
        assert!(matches!(replay, ItemTransferOutcome::AlreadyCommitted(_)));

        // Someone else's corpse: refused inside the window, allowed at and after its end.
        let early = settle_corpse_pickup(
            &session,
            &content,
            fence()?,
            corpse_request(command(4)?, foreign, foreign_loot),
        )
        .await;
        assert!(is_refusal(
            &early,
            ItemTransferRefusal::CorpseExclusiveWindow
        ));
        set_materialized_ago(&harness, foreign, 11_000).await?;
        let late = settle_corpse_pickup(
            &session,
            &content,
            fence()?,
            corpse_request(command(4)?, foreign, foreign_loot),
        )
        .await
        .map_err(debug)?;
        assert!(matches!(late, ItemTransferOutcome::Committed(_)));

        // The corpse item itself: as a corpse "entry" it is a Ground item (mismatch); named as a
        // Ground item it reaches the TRANSFER, which refuses it even now that it is empty.
        let corpse_as_entry = settle_corpse_pickup(
            &session,
            &content,
            fence()?,
            corpse_request(command(5)?, owned, owned),
        )
        .await;
        assert!(matches!(
            corpse_as_entry,
            Err(GroundPickupError::SourceMismatch)
        ));
        let corpse_as_ground = settle_ground_pickup(
            &session,
            &content,
            fence()?,
            pickup_request(
                command(5)?,
                owned,
                item(COIN),
                ItemTransferDestination::MainBackpack,
            ),
        )
        .await;
        assert!(is_refusal(
            &corpse_as_ground,
            ItemTransferRefusal::CorpseNotPickupable
        ));
        assert!(harness.on_ground(owned).await?);

        let backpack = harness
            .root
            .read_character_backpack(&authority, fence()?.character_id)
            .await
            .map_err(debug)?
            .ok_or("expected an equipped backpack")?;
        let held: u32 = backpack
            .entries
            .iter()
            .map(|entry| entry.item.quantity)
            .sum();
        assert_eq!(held, 2);
        assert!(harness.on_ground(ground_coin).await?);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
