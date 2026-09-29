// D39 PG cases: `interaction_chest_use::settle_chest_use` resolves the placed chest and the
// reward and backpack facts from the current Content generation, builds the GAME-INTERACTION
// child occurrence and claims through CHEST-1's `freeze_reward_claim_mint`/
// `commit_reward_claim_mint`. Reuses the B3-1 harness (`item_transfer_postgres_cases`).

use crate::combat::DurabilitySession;
use crate::combat_pickup::{GroundPickupRequest, PickupContentError, settle_ground_pickup};
use crate::content::{
    CanonicalReferencePlayableContent, ClientProjectionClass, ContentLockBinding, ContentLockEntry,
    CoordinateFrameRef, DefinitionFamily, DefinitionRevisionRef, EvidenceBindingRef,
    EvidenceDisposition, FootprintCell, FootprintRelation, LogicalCell, MapRevisionRef,
    PackageManifestBinding, PlacementKey, PlacementRef, ProductionAtom, ProductionKey,
    REFERENCE_PLAYABLE_CAPABILITY_PROFILE, REFERENCE_PLAYABLE_CONTENT_PROFILE_ID,
    ReferenceDefinition, ReferenceDefinitionKind, ReferenceEquipmentPattern,
    ReferenceEquipmentSlot, ReferenceItemContainer, ReferenceItemDefinition,
    ReferenceItemDestination, ReferenceItemEquipment, ReferenceItemField,
    ReferenceItemPhysicalClass, ReferenceItemSemantics, ReferenceItemStack,
    ReferenceItemStackClass, ReferencePlayableContentSource, Sha256HexDigest, SpatialAddress,
    TypedDefinitionRef as ContentTypedDefinitionRef, link_reference_playable,
};
use crate::domain::CharacterId;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{
    CurrentCharacterItemFence, ItemTransferDestination, ItemTransferOutcome,
};
use crate::durability::reward_claim_mint::{
    RewardClaimMintError, RewardClaimMintOutcome, RewardClaimRefusal,
};
use crate::foundation::{
    CommandId, CommandRef, ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration, WorldId,
};
use crate::interaction::InteractionError;
use crate::interaction_chest_use::{
    ChestUseError, ChestUseOutcome, ChestUseRequest, chest_use_occurrence, settle_chest_use,
};
use crate::item_transfer_postgres_cases::{
    CHARACTER, Harness, SESSION, TestResult, configured_admin, debug, id, runtime, scope,
};
use ReferenceItemField::{Known, Unknown};
use std::collections::BTreeMap;

const COIN: &str = "oteryn:chestuse.pg.coin";
const BACKPACK: &str = "oteryn:chestuse.pg.backpack";
const CHEST: &str = "oteryn:chestuse.pg.chest";
const ABSENT: &str = "oteryn:chestuse.pg.absent-from-content";
const CHEST_PLACEMENT: &str = "oteryn:chestuse.pg.placement.chest";
const DOOR_PLACEMENT: &str = "oteryn:chestuse.pg.placement.door";
const CLAIM: &str = "oteryn:chestuse.pg.claim.first";
const OTHER_CLAIM: &str = "oteryn:chestuse.pg.claim.second";

fn item(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
    }
}

fn claim(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "RewardClaim".into(),
        production_key: key.into(),
        revision_ref: "claim-r1".into(),
    }
}

fn content_typed(family: DefinitionFamily, key: &str) -> TestResult<ContentTypedDefinitionRef> {
    Ok(ContentTypedDefinitionRef::new(
        family,
        ProductionKey::new(key)?,
        DefinitionRevisionRef::new("definition-r1")?,
    ))
}

fn placement(
    key: &str,
    definition: ContentTypedDefinitionRef,
    content: &CanonicalReferencePlayableContent,
) -> TestResult<PlacementRef> {
    let evidence = EvidenceBindingRef::new(
        ProductionAtom::new("chest use PG manifest revision", "manifest-r0")?,
        ProductionKey::new("oteryn:chestuse.pg.placement-source")?,
        EvidenceDisposition::Unknown,
    );
    let members = vec![FootprintCell {
        dx: 0,
        dy: 0,
        dz: 0,
    }];
    Ok(PlacementRef {
        key: PlacementKey::new(key)?,
        map_revision: MapRevisionRef::new("map-r1")?,
        definition,
        address: SpatialAddress {
            world_id: content.world_id,
            coordinate_frame: content.coordinate_frame.clone(),
            cell: LogicalCell {
                x: 100,
                y: 200,
                z: 7,
            },
            evidence: evidence.clone(),
        },
        presentation_footprint: FootprintRelation::Qualified {
            members: members.clone(),
            evidence: evidence.clone(),
        },
        collision_footprint: FootprintRelation::Qualified { members, evidence },
        local_object_initial_state: None,
        local_object_state_attributes: BTreeMap::new(),
        local_object_revert_after_ms: BTreeMap::new(),
        local_object_event_transitions: BTreeMap::new(),
    })
}

/// Content with a stackable `COIN` (maximum 30), the D114-shaped `BACKPACK` (capacity 20,
/// container-slot pattern) and a `CHEST` item, plus two placements: the chest (an Item
/// placement) and a `LocalObject` door. `ABSENT` is never a definition key.
fn pg_content() -> TestResult<CanonicalReferencePlayableContent> {
    let package_key = ProductionKey::new("oteryn:content.chest-use-pg")?;
    let package_revision = ProductionAtom::new("chest use PG package revision", "pg-r1")?;
    let package_manifest = PackageManifestBinding::new(
        package_key.clone(),
        package_revision.clone(),
        ProductionAtom::new("chest use PG schema", "schema-v1")?,
        ProductionAtom::new("chest use PG license", "license:project-owned-v1")?,
        Sha256HexDigest::new("dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd")?,
    );
    let provenance = package_manifest.package_provenance_digest()?;
    let content_lock = ContentLockBinding {
        revision_digest_token: ProductionAtom::new("chest use PG content lock", "lock:pg-r1")?,
        entries: vec![ContentLockEntry::exact(
            package_key,
            package_revision,
            provenance,
        )],
    };
    let plain = |stack_class, semantics| ReferenceItemDefinition {
        physical_class: ReferenceItemPhysicalClass::Physical,
        materializable: true,
        stack_class,
        legal_destinations: vec![ReferenceItemDestination::CharacterInventory],
        semantics,
    };
    let coin = plain(
        ReferenceItemStackClass::StackCapable,
        ReferenceItemSemantics {
            stack: Known(ReferenceItemStack {
                stackable: Known(true),
                stack_max: Known(30),
            }),
            ..ReferenceItemSemantics::default()
        },
    );
    let backpack = plain(
        ReferenceItemStackClass::NonStackable,
        ReferenceItemSemantics {
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
    );
    let chest = plain(
        ReferenceItemStackClass::NonStackable,
        ReferenceItemSemantics::default(),
    );
    let definition = |key: &str, item: ReferenceItemDefinition| -> TestResult<_> {
        Ok(ReferenceDefinition {
            definition: content_typed(DefinitionFamily::Item, key)?,
            kind: ReferenceDefinitionKind::Item(item),
            client_projection: ClientProjectionClass::ClientSafe,
        })
    };
    let mut content = link_reference_playable(ReferencePlayableContentSource {
        profile_revision: ProductionAtom::new(
            "chest use PG profile",
            REFERENCE_PLAYABLE_CONTENT_PROFILE_ID,
        )?,
        capability_profile: ProductionAtom::new(
            "chest use PG capability profile",
            REFERENCE_PLAYABLE_CAPABILITY_PROFILE,
        )?,
        package_manifest,
        content_lock,
        world_id: WorldId::decode(&id(9))?,
        coordinate_frame: CoordinateFrameRef::new("global-target-2026-09-27")?,
        definitions: vec![
            definition(COIN, coin)?,
            definition(BACKPACK, backpack)?,
            definition(CHEST, chest)?,
        ],
        placements: vec![],
        ordered_placements: vec![],
        transitions: vec![],
    })?;
    // The world runtime binds placements after link (as `world_runtime` does); the chest and
    // the door are added the same way.
    content.placements = vec![
        placement(
            CHEST_PLACEMENT,
            content_typed(DefinitionFamily::Item, CHEST)?,
            &content,
        )?,
        placement(
            DOOR_PLACEMENT,
            content_typed(DefinitionFamily::LocalObject, "oteryn:chestuse.pg.door")?,
            &content,
        )?,
    ];
    Ok(content)
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

fn use_request(
    command: CommandRef,
    chest: &str,
    claimed: &str,
    reward: &str,
    quantity: u32,
) -> TestResult<ChestUseRequest> {
    Ok(ChestUseRequest {
        command,
        chest: PlacementKey::new(chest)?,
        claim: claim(claimed),
        reward_item: item(reward),
        quantity,
        content_revision: "content-1".into(),
        ruleset_revision: "ruleset-1".into(),
        sim_revision: "sim-1".into(),
    })
}

async fn equip_backpack(
    harness: &Harness,
    session: &DurabilitySession<'_, '_, '_>,
    content: &CanonicalReferencePlayableContent,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    let backpack = harness.mint(authority, BACKPACK, 1).await?;
    let equipped = settle_ground_pickup(
        session,
        content,
        fence()?,
        GroundPickupRequest {
            command: command(900)?,
            source_item_instance_id: backpack,
            source_definition: item(BACKPACK),
            destination: ItemTransferDestination::ContainerSlot,
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
        },
    )
    .await
    .map_err(debug)?;
    assert!(matches!(equipped, ItemTransferOutcome::Committed(_)));
    Ok(())
}

async fn backpack_entries(
    harness: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
) -> TestResult<usize> {
    Ok(harness
        .root
        .read_character_backpack(authority, fence()?.character_id)
        .await
        .map_err(debug)?
        .map_or(0, |backpack| backpack.entries.len()))
}

async fn claim_rows(harness: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_reward_claims")
            .fetch_one(&harness.pool)
            .await?,
    )
}

fn committed(
    outcome: &ChestUseOutcome,
) -> TestResult<&crate::durability::reward_claim_mint::CommittedRewardClaimMint> {
    match &outcome.mint {
        RewardClaimMintOutcome::Committed(committed)
        | RewardClaimMintOutcome::AlreadyCommitted(committed) => Ok(committed),
    }
}

#[test]
fn chest_use_mints_the_reward_once_and_replays_the_first_outcome() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "chestuse").await?;
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
        equip_backpack(&harness, &session, &content, &authority).await?;

        let request = use_request(command(1)?, CHEST_PLACEMENT, CLAIM, COIN, 7)?;
        let first = settle_chest_use(&session, &content, fence()?, request.clone())
            .await
            .map_err(debug)?;
        assert!(matches!(first.mint, RewardClaimMintOutcome::Committed(_)));
        assert_eq!(first.child, chest_use_occurrence(&request)?);
        assert_eq!(first.child.ancestry_depth(), 1);
        assert_eq!(committed(&first)?.quantity, 7);
        assert_eq!(backpack_entries(&harness, &authority).await?, 1);
        assert_eq!(claim_rows(&harness).await?, 1);

        // §17.1: the same CommandRef returns its first outcome and the same child.
        let replay = settle_chest_use(&session, &content, fence()?, request.clone())
            .await
            .map_err(debug)?;
        assert!(matches!(
            replay.mint,
            RewardClaimMintOutcome::AlreadyCommitted(_)
        ));
        assert_eq!(committed(&replay)?, committed(&first)?);
        assert_eq!(replay.child, first.child);

        // §17: a changed intent under the same CommandRef conflicts.
        let changed = use_request(command(1)?, CHEST_PLACEMENT, CLAIM, COIN, 8)?;
        let conflict = settle_chest_use(&session, &content, fence()?, changed).await;
        assert!(matches!(
            conflict,
            Err(ChestUseError::Mint(RewardClaimMintError::ConflictingCause))
        ));

        // D42: a new command on the claimed `once` claim is refused with nothing written.
        let again = settle_chest_use(
            &session,
            &content,
            fence()?,
            use_request(command(2)?, CHEST_PLACEMENT, CLAIM, COIN, 7)?,
        )
        .await;
        assert!(matches!(
            again,
            Err(ChestUseError::Mint(RewardClaimMintError::Refused(
                RewardClaimRefusal::AlreadyClaimed
            )))
        ));

        // Another claim on the same chest is another occurrence and takes the next entry.
        let other = settle_chest_use(
            &session,
            &content,
            fence()?,
            use_request(command(3)?, CHEST_PLACEMENT, OTHER_CLAIM, COIN, 1)?,
        )
        .await
        .map_err(debug)?;
        assert!(matches!(other.mint, RewardClaimMintOutcome::Committed(_)));
        assert_ne!(other.child, first.child);
        assert_eq!(backpack_entries(&harness, &authority).await?, 2);
        assert_eq!(claim_rows(&harness).await?, 2);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn chest_use_is_refused_before_any_write() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "chestuserefused").await?;
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

        // No backpack equipped yet: refused as D80 NoMainBackpack, before freeze.
        let no_backpack = settle_chest_use(
            &session,
            &content,
            fence()?,
            use_request(command(1)?, CHEST_PLACEMENT, CLAIM, COIN, 1)?,
        )
        .await;
        assert!(matches!(
            no_backpack,
            Err(ChestUseError::Mint(RewardClaimMintError::Refused(
                RewardClaimRefusal::NoMainBackpack
            )))
        ));

        equip_backpack(&harness, &session, &content, &authority).await?;
        let refusals = [
            // §5.4: the server resolves the target; an unknown placement is refused.
            (
                use_request(
                    command(2)?,
                    "oteryn:chestuse.pg.placement.none",
                    CLAIM,
                    COIN,
                    1,
                )?,
                "not placed",
            ),
            // A placement that is not an Item placement is not a chest.
            (
                use_request(command(3)?, DOOR_PLACEMENT, CLAIM, COIN, 1)?,
                "not an item",
            ),
            // The reward item's facts come from Content; an absent definition fails closed.
            (
                use_request(command(4)?, CHEST_PLACEMENT, CLAIM, ABSENT, 1)?,
                "reward absent",
            ),
            // The chest itself is an Item but not a reward: a claim of a non-item family fails.
            (
                use_request(command(5)?, CHEST_PLACEMENT, CLAIM, COIN, 1).map(|mut request| {
                    request.reward_item.family = "Loot".into();
                    request
                })?,
                "reward not an item",
            ),
            // §4.3/§5.7: the revisions are part of the child identity and must be present.
            (
                use_request(command(6)?, CHEST_PLACEMENT, CLAIM, COIN, 1).map(|mut request| {
                    request.content_revision = " ".into();
                    request
                })?,
                "no revision",
            ),
        ];
        for (request, label) in refusals {
            let outcome = settle_chest_use(&session, &content, fence()?, request).await;
            let expected = match label {
                "not placed" => matches!(outcome, Err(ChestUseError::ChestNotPlaced)),
                "not an item" => matches!(outcome, Err(ChestUseError::ChestNotAnItem)),
                "reward absent" | "reward not an item" => matches!(
                    outcome,
                    Err(ChestUseError::Content(
                        PickupContentError::DefinitionNotFound
                    ))
                ),
                _ => matches!(
                    outcome,
                    Err(ChestUseError::Identity(InteractionError::EmptySemanticKey))
                ),
            };
            assert!(expected, "{label}: {outcome:?}");
        }
        assert_eq!(backpack_entries(&harness, &authority).await?, 0);
        assert_eq!(claim_rows(&harness).await?, 0);
        let reservations: i64 =
            sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_reservations")
                .fetch_one(&harness.pool)
                .await?;
        assert_eq!(reservations, 0);

        // Nothing was written, so a refused command can still claim with its first intent.
        let later = settle_chest_use(
            &session,
            &content,
            fence()?,
            use_request(command(1)?, CHEST_PLACEMENT, CLAIM, COIN, 1)?,
        )
        .await
        .map_err(debug)?;
        assert!(matches!(later.mint, RewardClaimMintOutcome::Committed(_)));

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
