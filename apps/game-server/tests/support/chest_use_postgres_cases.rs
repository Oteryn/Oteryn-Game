// D39 PG cases: `interaction_chest_use::settle_chest_use` resolves the placed chest, the
// RewardClaim that lists it (CHEST-CONTENT-1), its reward and the backpack facts from the
// current Content generation, builds the GAME-INTERACTION
// child occurrence and claims through CHEST-1's `freeze_reward_claim_mint`/
// `commit_reward_claim_mint`. Reuses the B3-1 harness (`item_transfer_postgres_cases`).
// Also covers the §17 intent binding of the chest, the §17.2 pending refusal and the
// reconciliation of an ambiguous (frozen, uncommitted) claim.

use crate::achievement_catalogue::AchievementCatalogue;
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
    ReferenceItemStackClass, ReferencePlayableContentSource, ReferenceRewardClaimDefinition,
    RewardClaimItem, RewardClaimPlacement, Sha256HexDigest, SpatialAddress,
    TypedDefinitionRef as ContentTypedDefinitionRef, link_reference_playable,
};
use crate::domain::CharacterId;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::item_transfer::{
    CurrentCharacterItemFence, ItemTransferDestination, ItemTransferOutcome,
};
use crate::durability::reward_claim_mint::{
    ACHIEVEMENT_SOURCE_KIND, RewardClaimMintError, RewardClaimMintOutcome, RewardClaimRefusal,
};
use crate::foundation::{
    CommandId, CommandRef, ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration, WorldId,
};
use crate::interaction::InteractionError;
use crate::interaction_chest_use::{
    ChestUseError, ChestUseOutcome, ChestUseRequest, chest_use_occurrence, entry_chest,
    prepare_chest_use, resolve_chest, settle_chest_use, use_chest, with_entry_chest,
};
use crate::item_transfer_postgres_cases::{
    CHARACTER, Harness, SESSION, TestResult, configured_admin, debug, id, runtime, scope, uuid_text,
};
use ReferenceItemField::{Known, Unknown};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::collections::BTreeMap;

const COIN: &str = "oteryn:chestuse.pg.coin";
const BACKPACK: &str = "oteryn:chestuse.pg.backpack";
const CHEST: &str = "oteryn:chestuse.pg.chest";
const ABSENT: &str = "oteryn:chestuse.pg.absent-from-content";
const CHEST_PLACEMENT: &str = "oteryn:chestuse.pg.placement.chest";
/// A second chest sharing `CLAIM` with `CHEST_PLACEMENT`, with the same reward.
const SECOND_CHEST_PLACEMENT: &str = "oteryn:chestuse.pg.placement.chest.second";
const OTHER_CHEST_PLACEMENT: &str = "oteryn:chestuse.pg.placement.chest.other";
const THIRD_CHEST_PLACEMENT: &str = "oteryn:chestuse.pg.placement.chest.third";
/// An Item placement that no RewardClaim lists.
const UNCLAIMED_CHEST_PLACEMENT: &str = "oteryn:chestuse.pg.placement.chest.unclaimed";
const DOOR_PLACEMENT: &str = "oteryn:chestuse.pg.placement.door";
const CLAIM: &str = "oteryn:chestuse.pg.claim.first";
const OTHER_CLAIM: &str = "oteryn:chestuse.pg.claim.second";
const THIRD_CLAIM: &str = "oteryn:chestuse.pg.claim.third";
const FOURTH_CLAIM: &str = "oteryn:chestuse.pg.claim.fourth";
/// An earnable record of the embedded catalogue (`content/achievements/`), at revision "1".
const ANNIHILATOR: &str = "oteryn:achievement/annihilator";
/// A key of the achievement grammar that the embedded catalogue lacks.
const ABSENT_ACHIEVEMENT: &str = "oteryn:achievement/not_in_the_catalogue";
/// The harness Character's account (`item_transfer_postgres_cases::seed_character`).
const ACCOUNT: u8 = 40;

fn item(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: "definition-r1".into(),
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
/// container-slot pattern), a `CHEST` item and three RewardClaims:
/// - `CLAIM`: `CHEST_PLACEMENT` and `SECOND_CHEST_PLACEMENT`, 7 coins each;
/// - `OTHER_CLAIM`: `OTHER_CHEST_PLACEMENT`, 1 coin;
/// - `THIRD_CLAIM`: `THIRD_CHEST_PLACEMENT`, 1 coin.
///
/// The placements are those five chests (one unclaimed) and a `LocalObject` door. `ABSENT` is
/// never a definition key.
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
    let reward_claim = |key: &str, chests: &[(&str, u32)]| -> TestResult<_> {
        let placements = chests
            .iter()
            .map(|(chest, count)| -> TestResult<_> {
                Ok(RewardClaimPlacement {
                    placement: PlacementKey::new(chest)?,
                    items: vec![RewardClaimItem {
                        item: content_typed(DefinitionFamily::Item, COIN)?,
                        count: *count,
                    }],
                    achievement: None,
                })
            })
            .collect::<TestResult<Vec<_>>>()?;
        Ok(ReferenceDefinition {
            definition: content_typed(DefinitionFamily::RewardClaim, key)?,
            kind: ReferenceDefinitionKind::RewardClaim(ReferenceRewardClaimDefinition {
                placements,
            }),
            client_projection: ClientProjectionClass::ServerOnly,
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
            reward_claim(CLAIM, &[(CHEST_PLACEMENT, 7), (SECOND_CHEST_PLACEMENT, 7)])?,
            reward_claim(OTHER_CLAIM, &[(OTHER_CHEST_PLACEMENT, 1)])?,
            reward_claim(THIRD_CLAIM, &[(THIRD_CHEST_PLACEMENT, 1)])?,
        ],
        placements: vec![],
        ordered_placements: vec![],
        transitions: vec![],
    })?;
    // The world runtime binds placements after link (as `world_runtime` does); the chests and
    // the door are added the same way.
    let mut placements = Vec::new();
    for chest in [
        CHEST_PLACEMENT,
        SECOND_CHEST_PLACEMENT,
        OTHER_CHEST_PLACEMENT,
        THIRD_CHEST_PLACEMENT,
        UNCLAIMED_CHEST_PLACEMENT,
    ] {
        placements.push(placement(
            chest,
            content_typed(DefinitionFamily::Item, CHEST)?,
            &content,
        )?);
    }
    placements.push(placement(
        DOOR_PLACEMENT,
        content_typed(DefinitionFamily::LocalObject, "oteryn:chestuse.pg.door")?,
        &content,
    )?);
    content.placements = placements;
    Ok(content)
}

/// The runtime catalogue embedded from `content/achievements/`.
fn catalogue() -> TestResult<AchievementCatalogue> {
    Ok(AchievementCatalogue::embedded().map_err(debug)?)
}

/// `content` with the reward of `chest` under its claim replaced, as a later Content revision
/// (or a corrupt one) would carry it.
fn with_reward(
    content: &CanonicalReferencePlayableContent,
    chest: &str,
    item: ContentTypedDefinitionRef,
    count: u32,
) -> TestResult<CanonicalReferencePlayableContent> {
    let mut changed = content.clone();
    let chest = PlacementKey::new(chest)?;
    let entry = changed
        .definitions
        .iter_mut()
        .find_map(|definition| match &mut definition.kind {
            ReferenceDefinitionKind::RewardClaim(claim) => claim
                .placements
                .iter_mut()
                .find(|entry| entry.placement == chest),
            _ => None,
        })
        .ok_or("chest is not listed under a claim")?;
    entry.items = vec![RewardClaimItem { item, count }];
    Ok(changed)
}

/// `content` with the achievement of `chest` under its claim set to `achievement`, as a later
/// Content revision would carry it.
fn with_achievement(
    content: &CanonicalReferencePlayableContent,
    chest: &str,
    achievement: Option<&str>,
) -> TestResult<CanonicalReferencePlayableContent> {
    let mut changed = content.clone();
    let chest = PlacementKey::new(chest)?;
    let entry = changed
        .definitions
        .iter_mut()
        .find_map(|definition| match &mut definition.kind {
            ReferenceDefinitionKind::RewardClaim(claim) => claim
                .placements
                .iter_mut()
                .find(|entry| entry.placement == chest),
            _ => None,
        })
        .ok_or("chest is not listed under a claim")?;
    entry.achievement = achievement.map(str::to_owned);
    Ok(changed)
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

fn use_request(command: CommandRef, chest: &str) -> TestResult<ChestUseRequest> {
    Ok(ChestUseRequest {
        command,
        chest: PlacementKey::new(chest)?,
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

async fn reservation_rows(harness: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_reward_claim_mint_reservations")
            .fetch_one(&harness.pool)
            .await?,
    )
}

fn refused_with(
    outcome: &Result<ChestUseOutcome, ChestUseError>,
    expected: RewardClaimRefusal,
) -> bool {
    matches!(
        outcome,
        Err(ChestUseError::Mint(RewardClaimMintError::Refused(reason))) if *reason == expected
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
        let achievements = catalogue()?;
        let session = DurabilitySession {
            root: &harness.root,
            authority: &authority,
            node: &harness.node,
        };
        equip_backpack(&harness, &session, &content, &authority).await?;

        let request = use_request(command(1)?, CHEST_PLACEMENT)?;
        let first = settle_chest_use(&session, &content, &achievements, fence()?, request.clone())
            .await
            .map_err(debug)?;
        assert!(matches!(first.mint, RewardClaimMintOutcome::Committed(_)));
        let resolved = resolve_chest(&content, &request.chest).map_err(debug)?;
        assert_eq!(resolved.claim.production_key, CLAIM);
        assert_eq!(resolved.quantity, 7);
        assert_eq!(first.child, chest_use_occurrence(&request, &resolved)?);
        assert_eq!(first.child.ancestry_depth(), 1);
        assert_eq!(committed(&first)?.quantity, 7);
        assert_eq!(backpack_entries(&harness, &authority).await?, 1);
        assert_eq!(claim_rows(&harness).await?, 1);

        // §17.1: the same CommandRef returns its first outcome and the same child.
        let replay = settle_chest_use(&session, &content, &achievements, fence()?, request.clone())
            .await
            .map_err(debug)?;
        assert!(matches!(
            replay.mint,
            RewardClaimMintOutcome::AlreadyCommitted(_)
        ));
        assert_eq!(committed(&replay)?, committed(&first)?);
        assert_eq!(replay.child, first.child);

        // §17: a changed intent under the same CommandRef conflicts: here a later Content
        // revision gives the chest 8 coins instead of 7.
        let eight = with_reward(
            &content,
            CHEST_PLACEMENT,
            content_typed(DefinitionFamily::Item, COIN)?,
            8,
        )?;
        let conflict =
            settle_chest_use(&session, &eight, &achievements, fence()?, request.clone()).await;
        assert!(matches!(
            conflict,
            Err(ChestUseError::Mint(RewardClaimMintError::ConflictingCause))
        ));
        // §17 / D40: the chest is part of the intent. The same CommandRef on another chest of
        // the same claim, with the same reward, conflicts; it never replays the first chest.
        let other_chest = use_request(command(1)?, SECOND_CHEST_PLACEMENT)?;
        let conflict =
            settle_chest_use(&session, &content, &achievements, fence()?, other_chest).await;
        assert!(
            matches!(
                conflict,
                Err(ChestUseError::Mint(RewardClaimMintError::ConflictingCause))
            ),
            "{conflict:?}"
        );

        // Replay precondition: a replay recomputes the intent from the current Content. With
        // the chest gone from Content it is refused before DUR-03 and writes nothing; the
        // committed first outcome stays, and replays again once Content is restored.
        let mut without_chest = content.clone();
        without_chest
            .placements
            .retain(|placement| placement.key.as_str() != CHEST_PLACEMENT);
        let gone = settle_chest_use(
            &session,
            &without_chest,
            &achievements,
            fence()?,
            request.clone(),
        )
        .await;
        assert!(
            matches!(gone, Err(ChestUseError::ChestNotPlaced)),
            "{gone:?}"
        );
        let restored =
            settle_chest_use(&session, &content, &achievements, fence()?, request.clone())
                .await
                .map_err(debug)?;
        assert_eq!(committed(&restored)?, committed(&first)?);
        assert_eq!(claim_rows(&harness).await?, 1);
        assert_eq!(backpack_entries(&harness, &authority).await?, 1);

        // D42: a new command on the claimed `once` claim is refused with nothing written, on
        // the same chest and on the other chest that shares the claim.
        for chest in [CHEST_PLACEMENT, SECOND_CHEST_PLACEMENT] {
            let again = settle_chest_use(
                &session,
                &content,
                &achievements,
                fence()?,
                use_request(command(2)?, chest)?,
            )
            .await;
            assert!(
                refused_with(&again, RewardClaimRefusal::AlreadyClaimed),
                "{chest}: {again:?}"
            );
        }

        // A chest of another claim is another occurrence and takes the next entry.
        let other = settle_chest_use(
            &session,
            &content,
            &achievements,
            fence()?,
            use_request(command(3)?, OTHER_CHEST_PLACEMENT)?,
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
        let achievements = catalogue()?;
        let session = DurabilitySession {
            root: &harness.root,
            authority: &authority,
            node: &harness.node,
        };

        // No backpack equipped yet: refused as D80 NoMainBackpack, before freeze.
        let no_backpack = settle_chest_use(
            &session,
            &content,
            &achievements,
            fence()?,
            use_request(command(1)?, CHEST_PLACEMENT)?,
        )
        .await;
        assert!(matches!(
            no_backpack,
            Err(ChestUseError::Mint(RewardClaimMintError::Refused(
                RewardClaimRefusal::NoMainBackpack
            )))
        ));

        equip_backpack(&harness, &session, &content, &authority).await?;
        let absent_reward = with_reward(
            &content,
            CHEST_PLACEMENT,
            content_typed(DefinitionFamily::Item, ABSENT)?,
            1,
        )?;
        let loot_reward = with_reward(
            &content,
            CHEST_PLACEMENT,
            content_typed(DefinitionFamily::Loot, COIN)?,
            1,
        )?;
        let refusals = [
            // §5.4: the server resolves the target; an unknown placement is refused.
            (
                use_request(command(2)?, "oteryn:chestuse.pg.placement.none")?,
                &content,
                "not placed",
            ),
            // A placement that is not an Item placement is not a chest.
            (
                use_request(command(3)?, DOOR_PLACEMENT)?,
                &content,
                "not an item",
            ),
            // An Item placement that no RewardClaim lists rewards nothing.
            (
                use_request(command(7)?, UNCLAIMED_CHEST_PLACEMENT)?,
                &content,
                "no claim",
            ),
            // The reward item's facts come from Content; an absent definition fails closed.
            (
                use_request(command(4)?, CHEST_PLACEMENT)?,
                &absent_reward,
                "reward absent",
            ),
            // A reward that names a non-Item definition fails closed.
            (
                use_request(command(5)?, CHEST_PLACEMENT)?,
                &loot_reward,
                "reward not an item",
            ),
            // §4.3/§5.7: the revisions are part of the child identity and must be present.
            (
                use_request(command(6)?, CHEST_PLACEMENT).map(|mut request| {
                    request.content_revision = " ".into();
                    request
                })?,
                &content,
                "no revision",
            ),
        ];
        for (request, content, label) in refusals {
            let outcome =
                settle_chest_use(&session, content, &achievements, fence()?, request).await;
            let expected = match label {
                "not placed" => matches!(outcome, Err(ChestUseError::ChestNotPlaced)),
                "not an item" => matches!(outcome, Err(ChestUseError::ChestNotAnItem)),
                "no claim" => matches!(outcome, Err(ChestUseError::ChestHasNoClaim)),
                "reward absent" => matches!(
                    outcome,
                    Err(ChestUseError::Content(
                        PickupContentError::DefinitionNotFound
                    ))
                ),
                "reward not an item" => matches!(
                    outcome,
                    Err(ChestUseError::Content(PickupContentError::NotAnItem))
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
            &achievements,
            fence()?,
            use_request(command(1)?, CHEST_PLACEMENT)?,
        )
        .await
        .map_err(debug)?;
        assert!(matches!(later.mint, RewardClaimMintOutcome::Committed(_)));

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

#[test]
fn a_pending_claim_blocks_a_new_command_and_the_same_command_reconciles_it() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "chestusepending").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let content = pg_content()?;
        let achievements = catalogue()?;
        let session = DurabilitySession {
            root: &harness.root,
            authority: &authority,
            node: &harness.node,
        };
        equip_backpack(&harness, &session, &content, &authority).await?;

        // An ambiguous claim: CommandRef 1 is frozen (reserved) and its commit outcome is
        // unknown to the caller.
        let request = use_request(command(1)?, CHEST_PLACEMENT)?;
        let (child, mint_request) =
            prepare_chest_use(&session, &content, &achievements, fence()?, request.clone())
                .await
                .map_err(debug)?;
        let frozen = harness
            .root
            .freeze_reward_claim_mint(&authority, &harness.node, fence()?, mint_request)
            .await
            .map_err(debug)?;
        let frozen_transaction = *frozen.transaction_id();
        let frozen_item = *frozen.item_instance_id();
        drop(frozen);
        assert_eq!(reservation_rows(&harness).await?, 1);

        // §17.2: while CommandRef 1 is pending, a new CommandRef for the same claim is
        // refused with nothing written, on the same chest and on another one.
        for chest in [CHEST_PLACEMENT, SECOND_CHEST_PLACEMENT] {
            let blocked = settle_chest_use(
                &session,
                &content,
                &achievements,
                fence()?,
                use_request(command(2)?, chest)?,
            )
            .await;
            assert!(
                refused_with(&blocked, RewardClaimRefusal::ClaimPending),
                "{chest}: {blocked:?}"
            );
        }
        assert_eq!(reservation_rows(&harness).await?, 1);
        assert_eq!(claim_rows(&harness).await?, 0);

        // The same request reconciles the ambiguous claim: it commits with the frozen
        // identities, then replays as AlreadyCommitted. Exactly one claim row.
        let settled =
            settle_chest_use(&session, &content, &achievements, fence()?, request.clone())
                .await
                .map_err(debug)?;
        let RewardClaimMintOutcome::Committed(ref first) = settled.mint else {
            return Err(format!("expected Committed, got {:?}", settled.mint).into());
        };
        assert_eq!(first.transaction_id, frozen_transaction);
        assert_eq!(first.item_instance_id, frozen_item);
        assert_eq!(settled.child, child);
        let replay = settle_chest_use(&session, &content, &achievements, fence()?, request)
            .await
            .map_err(debug)?;
        assert_eq!(
            replay.mint,
            RewardClaimMintOutcome::AlreadyCommitted(first.clone())
        );
        assert_eq!(claim_rows(&harness).await?, 1);

        // Once CommandRef 1 is terminal, the new CommandRef is refused as AlreadyClaimed.
        let claimed = settle_chest_use(
            &session,
            &content,
            &achievements,
            fence()?,
            use_request(command(2)?, CHEST_PLACEMENT)?,
        )
        .await;
        assert!(
            refused_with(&claimed, RewardClaimRefusal::AlreadyClaimed),
            "{claimed:?}"
        );

        // A reservation that can no longer commit is not pending. (a) Its RL-08 budget is
        // spent: three commits under a stale fence each charge one unit and are rejected.
        let (_, other_request) = prepare_chest_use(
            &session,
            &content,
            &achievements,
            fence()?,
            use_request(command(3)?, OTHER_CHEST_PLACEMENT)?,
        )
        .await
        .map_err(debug)?;
        let mut spent = harness
            .root
            .freeze_reward_claim_mint(&authority, &harness.node, fence()?, other_request)
            .await
            .map_err(debug)?;
        let mut stale = fence()?;
        stale.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        for _ in 0..3 {
            let rejected = harness
                .root
                .commit_reward_claim_mint(&authority, &harness.node, stale, &mut spent)
                .await;
            assert!(
                matches!(rejected, Err(RewardClaimMintError::AuthorityRejected)),
                "{rejected:?}"
            );
        }
        assert_eq!(spent.work_units_used(), 3);
        let after_spent = settle_chest_use(
            &session,
            &content,
            &achievements,
            fence()?,
            use_request(command(4)?, OTHER_CHEST_PLACEMENT)?,
        )
        .await
        .map_err(debug)?;
        assert!(matches!(
            after_spent.mint,
            RewardClaimMintOutcome::Committed(_)
        ));

        // (b) Its GameSession is not live: a reservation of this claim under a GameSession
        // with no live session row does not block the current one.
        sqlx::query(
            "INSERT INTO game_reward_claim_mint_reservations(game_session_id, command_id, \
               character_id, world_id, channel_id, claim_family, claim_production_key, \
               claim_revision_ref, intent_binding, transaction_id, event_id, item_instance_id, \
               occurred_at, work_units_used, reserved_at) \
             VALUES (encode($1,'hex')::uuid, 1, encode($2,'hex')::uuid, encode($3,'hex')::uuid, \
               encode($3,'hex')::uuid, 'RewardClaim', $4, 'claim-r1', $5, \
               encode($6,'hex')::uuid, encode($7,'hex')::uuid, encode($8,'hex')::uuid, 1, 0, 0)",
        )
        .bind(id(77).as_slice())
        .bind(id(CHARACTER).as_slice())
        .bind(id(78).as_slice())
        .bind(THIRD_CLAIM)
        .bind(vec![2_u8; 33])
        .bind(id(79).as_slice())
        .bind(id(80).as_slice())
        .bind(id(81).as_slice())
        .execute(&harness.pool)
        .await?;
        let after_dead_session = settle_chest_use(
            &session,
            &content,
            &achievements,
            fence()?,
            use_request(command(5)?, THIRD_CHEST_PLACEMENT)?,
        )
        .await
        .map_err(debug)?;
        assert!(matches!(
            after_dead_session.mint,
            RewardClaimMintOutcome::Committed(_)
        ));
        assert_eq!(claim_rows(&harness).await?, 3);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

/// Every achievement grant request `(key, revision, source kind)` and every account fact
/// `(account, key)`.
async fn achievement_rows(harness: &Harness) -> TestResult<AchievementRows> {
    let requests = sqlx::query_as(
        "SELECT achievement_key, achievement_revision, source_kind \
           FROM game_account_achievement_grant_requests ORDER BY achievement_key",
    )
    .fetch_all(&harness.pool)
    .await?;
    let facts = sqlx::query_as(
        "SELECT account_id::text, achievement_key FROM game_account_achievements \
          ORDER BY achievement_key",
    )
    .fetch_all(&harness.pool)
    .await?;
    Ok((requests, facts))
}

type AchievementRows = (Vec<(String, String, String)>, Vec<(String, String)>);

#[test]
fn a_chest_with_an_achievement_grants_it_and_a_chest_without_one_grants_none() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "chestuseachievement").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let content = pg_content()?;
        let achievements = catalogue()?;
        let session = DurabilitySession {
            root: &harness.root,
            authority: &authority,
            node: &harness.node,
        };
        equip_backpack(&harness, &session, &content, &authority).await?;

        // A chest whose claim placement names no achievement: the claim commits and no
        // achievement row is written.
        let plain = settle_chest_use(
            &session,
            &content,
            &achievements,
            fence()?,
            use_request(command(1)?, CHEST_PLACEMENT)?,
        )
        .await
        .map_err(debug)?;
        assert!(matches!(plain.mint, RewardClaimMintOutcome::Committed(_)));
        assert_eq!(achievement_rows(&harness).await?, (vec![], vec![]));

        // A chest whose claim placement names Annihilator in Content (the authoring ref
        // `canary:achievement/annihilator`, bound by slug, contract §2.2). The caller names only
        // the chest; the server takes the key from Content and its entry from the embedded
        // catalogue (revision "1"), and the MINT grants it to the account with the claim.
        let annihilator = with_achievement(&content, OTHER_CHEST_PLACEMENT, Some(ANNIHILATOR))?;
        assert!(
            achievements
                .unbound_reward_claim_achievements(&annihilator)
                .is_empty()
        );
        let request = use_request(command(2)?, OTHER_CHEST_PLACEMENT)?;
        let granted = settle_chest_use(
            &session,
            &annihilator,
            &achievements,
            fence()?,
            request.clone(),
        )
        .await
        .map_err(debug)?;
        assert!(matches!(granted.mint, RewardClaimMintOutcome::Committed(_)));
        let granted_rows: AchievementRows = (
            vec![(
                ANNIHILATOR.to_owned(),
                "1".to_owned(),
                ACHIEVEMENT_SOURCE_KIND.to_owned(),
            )],
            vec![(uuid_text(id(ACCOUNT)), ANNIHILATOR.to_owned())],
        );
        assert_eq!(achievement_rows(&harness).await?, granted_rows);

        // A replay returns the first outcome and grants nothing again. A later Content revision
        // without the achievement is another intent for the same command and conflicts.
        let replay = settle_chest_use(
            &session,
            &annihilator,
            &achievements,
            fence()?,
            request.clone(),
        )
        .await
        .map_err(debug)?;
        assert_eq!(committed(&replay)?, committed(&granted)?);
        let conflict = settle_chest_use(&session, &content, &achievements, fence()?, request).await;
        assert!(
            matches!(
                conflict,
                Err(ChestUseError::Mint(RewardClaimMintError::ConflictingCause))
            ),
            "{conflict:?}"
        );
        assert_eq!(achievement_rows(&harness).await?, granted_rows);

        // A retired achievement: the claim commits and nothing is recorded for it.
        let retired_content = with_achievement(
            &content,
            THIRD_CHEST_PLACEMENT,
            Some("oteryn:achievement/the_more_the_merrier"),
        )?;
        assert!(
            achievements
                .unbound_reward_claim_achievements(&retired_content)
                .is_empty()
        );
        let retired = settle_chest_use(
            &session,
            &retired_content,
            &achievements,
            fence()?,
            use_request(command(3)?, THIRD_CHEST_PLACEMENT)?,
        )
        .await
        .map_err(debug)?;
        assert!(matches!(retired.mint, RewardClaimMintOutcome::Committed(_)));
        assert_eq!(achievement_rows(&harness).await?, granted_rows);

        // A key the catalogue lacks: Content validation reports it unbound (the server refuses
        // such an activation), and the MINT still refuses the claim before any write.
        let mut absent_content = content.clone();
        absent_content.definitions.push(ReferenceDefinition {
            definition: content_typed(DefinitionFamily::RewardClaim, FOURTH_CLAIM)?,
            kind: ReferenceDefinitionKind::RewardClaim(ReferenceRewardClaimDefinition {
                placements: vec![RewardClaimPlacement {
                    placement: PlacementKey::new(UNCLAIMED_CHEST_PLACEMENT)?,
                    items: vec![RewardClaimItem {
                        item: content_typed(DefinitionFamily::Item, COIN)?,
                        count: 1,
                    }],
                    achievement: Some(ABSENT_ACHIEVEMENT.into()),
                }],
            }),
            client_projection: ClientProjectionClass::ServerOnly,
        });
        assert_eq!(
            achievements.unbound_reward_claim_achievements(&absent_content),
            [ABSENT_ACHIEVEMENT]
        );
        let absent = settle_chest_use(
            &session,
            &absent_content,
            &achievements,
            fence()?,
            use_request(command(4)?, UNCLAIMED_CHEST_PLACEMENT)?,
        )
        .await;
        assert!(
            matches!(
                absent,
                Err(ChestUseError::Mint(
                    RewardClaimMintError::UnknownAchievement
                ))
            ),
            "{absent:?}"
        );
        assert_eq!(achievement_rows(&harness).await?, granted_rows);
        assert_eq!(claim_rows(&harness).await?, 3);
        assert_eq!(reservation_rows(&harness).await?, 3);
        assert_eq!(backpack_entries(&harness, &authority).await?, 3);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn nothing_written(harness: &Harness) -> TestResult {
    assert_eq!(claim_rows(harness).await?, 0);
    assert_eq!(reservation_rows(harness).await?, 0);
    Ok(())
}

fn entry_item(key: &str) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: key.into(),
        revision_ref: entry_chest::DEFINITION_REVISION.into(),
    }
}

fn entry_request(command: CommandRef) -> TestResult<ChestUseRequest> {
    Ok(ChestUseRequest {
        command,
        chest: PlacementKey::new(entry_chest::PLACEMENT)?,
        content_revision: entry_chest::CONTENT_REVISION.into(),
        ruleset_revision: entry_chest::RULESET_REVISION.into(),
        sim_revision: entry_chest::SIM_REVISION.into(),
    })
}

/// C2: the `USE_INTENT` dispatch of the injected entry chest (`with_entry_chest` +
/// `use_chest`). Without a main backpack it refuses `NoMainBackpack` (production, until
/// STARTER-BACKPACK) and without a fence it refuses; neither writes. With a harness-seeded
/// backpack, a stale fence writes nothing, the first `USE` mints the reward once and a second
/// `USE` is `NOTHING_TO_USE`.
#[test]
fn the_entry_chest_use_mints_once_and_refuses_cleanly_otherwise() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "chestentry").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let content = with_entry_chest(&pg_content()?).map_err(debug)?;
        let achievements = catalogue()?;
        let session = DurabilitySession {
            root: &harness.root,
            authority: &authority,
            node: &harness.node,
        };
        // A Character with no main backpack (every production Character until
        // STARTER-BACKPACK): refused cleanly, nothing written.
        let (disposition, error, _) = use_chest(
            &session,
            &content,
            &achievements,
            Some(fence()?),
            entry_request(command(1)?)?,
        )
        .await;
        assert_eq!(disposition, UseDisposition::Rejected);
        assert!(
            matches!(
                error,
                Some(ChestUseError::Mint(RewardClaimMintError::Refused(
                    RewardClaimRefusal::NoMainBackpack
                )))
            ),
            "{error:?}"
        );
        nothing_written(&harness).await?;

        // A session without an item fence never reaches DUR-03.
        let (disposition, error, _) = use_chest(
            &session,
            &content,
            &achievements,
            None,
            entry_request(command(2)?)?,
        )
        .await;
        assert_eq!(disposition, UseDisposition::Rejected);
        assert!(error.is_none());
        nothing_written(&harness).await?;

        // Harness-seeded main backpack of the injected entry backpack definition.
        let backpack = harness
            .mint_definition(&authority, entry_item(entry_chest::BACKPACK_ITEM), 1)
            .await?;
        let equipped = settle_ground_pickup(
            &session,
            &content,
            fence()?,
            GroundPickupRequest {
                command: command(900)?,
                source_item_instance_id: backpack,
                source_definition: entry_item(entry_chest::BACKPACK_ITEM),
                destination: ItemTransferDestination::ContainerSlot,
                content_revision: "content-1".into(),
                ruleset_revision: "ruleset-1".into(),
                sim_revision: "sim-1".into(),
            },
        )
        .await
        .map_err(debug)?;
        assert!(matches!(equipped, ItemTransferOutcome::Committed(_)));

        // A stale fence (a superseded connection generation) writes nothing.
        let mut stale = fence()?;
        stale.connection_generation = ConnectionGeneration::new(2).map_err(debug)?;
        let (disposition, error, _) = use_chest(
            &session,
            &content,
            &achievements,
            Some(stale),
            entry_request(command(3)?)?,
        )
        .await;
        assert_eq!(disposition, UseDisposition::Rejected);
        assert!(
            matches!(
                error,
                Some(ChestUseError::Mint(RewardClaimMintError::AuthorityRejected))
            ),
            "{error:?}"
        );
        nothing_written(&harness).await?;
        assert_eq!(backpack_entries(&harness, &authority).await?, 0);

        // The first USE mints the chest's reward once, into the backpack.
        let (disposition, error, _) = use_chest(
            &session,
            &content,
            &achievements,
            Some(fence()?),
            entry_request(command(4)?)?,
        )
        .await;
        assert_eq!(disposition, UseDisposition::Committed, "{error:?}");
        assert_eq!(claim_rows(&harness).await?, 1);
        assert_eq!(backpack_entries(&harness, &authority).await?, 1);
        let entry = harness
            .root
            .read_character_backpack(&authority, fence()?.character_id)
            .await
            .map_err(debug)?
            .ok_or("no backpack")?
            .entries
            .into_iter()
            .next()
            .ok_or("no entry")?;
        assert_eq!(entry.item.definition, entry_item(entry_chest::REWARD_ITEM));
        assert_eq!(entry.item.quantity, entry_chest::REWARD_COUNT);

        // A second USE (a new command) finds the `once` claim taken: NOTHING_TO_USE, nothing
        // written.
        let (disposition, error, _) = use_chest(
            &session,
            &content,
            &achievements,
            Some(fence()?),
            entry_request(command(5)?)?,
        )
        .await;
        assert_eq!(disposition, UseDisposition::NothingToUse);
        assert!(refused_with(
            &Err(error.ok_or("no refusal")?),
            RewardClaimRefusal::AlreadyClaimed
        ));
        assert_eq!(claim_rows(&harness).await?, 1);
        assert_eq!(backpack_entries(&harness, &authority).await?, 1);
        Ok(())
    })
}

/// ACH-NOTIFY-1 (ACHIEVEMENT-0 §5): two chest grants committed concurrently each return the
/// notice of their own `Granted` grant with an exact, distinct watermark. The admission locks
/// serialize the commits, so the first reads one fact and the second reads both. A replay and a
/// chest without an achievement return none.
#[test]
fn concurrent_granted_chests_return_distinct_exact_notices() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    runtime()?.block_on(async move {
        let harness = Harness::create(admin, "chestusenotice").await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let content = pg_content()?;
        let achievements = catalogue()?;
        let session = DurabilitySession {
            root: &harness.root,
            authority: &authority,
            node: &harness.node,
        };
        equip_backpack(&harness, &session, &content, &authority).await?;
        const COOKIES: &str = "oteryn:achievement/allow_cookies";
        let granting = with_achievement(
            &with_achievement(&content, OTHER_CHEST_PLACEMENT, Some(ANNIHILATOR))?,
            THIRD_CHEST_PLACEMENT,
            Some(COOKIES),
        )?;
        let first = use_request(command(1)?, OTHER_CHEST_PLACEMENT)?;
        let second = use_request(command(2)?, THIRD_CHEST_PLACEMENT)?;
        let (a, b) = {
            // Both grants issued at once: each future is polled until both are done.
            let mut first_use = std::pin::pin!(settle_chest_use(
                &session,
                &granting,
                &achievements,
                fence()?,
                first.clone()
            ));
            let mut second_use = std::pin::pin!(settle_chest_use(
                &session,
                &granting,
                &achievements,
                fence()?,
                second.clone()
            ));
            let (mut a, mut b) = (None, None);
            std::future::poll_fn(|context| {
                if a.is_none()
                    && let std::task::Poll::Ready(done) =
                        std::future::Future::poll(first_use.as_mut(), context)
                {
                    a = Some(done);
                }
                if b.is_none()
                    && let std::task::Poll::Ready(done) =
                        std::future::Future::poll(second_use.as_mut(), context)
                {
                    b = Some(done);
                }
                if a.is_some() && b.is_some() {
                    std::task::Poll::Ready(())
                } else {
                    std::task::Poll::Pending
                }
            })
            .await;
            (a.ok_or("first")?, b.ok_or("second")?)
        };
        // A node's durability root holds one connection: a grant issued while the other's pass
        // holds it is refused `Unavailable` before any write and is retried, as the `USE`
        // dispatch does. Across nodes, the EXCLUSIVE admission locks serialize the commits.
        let busy = |error: &ChestUseError| {
            matches!(
                error,
                ChestUseError::Mint(RewardClaimMintError::Unavailable(_))
                    | ChestUseError::Backpack(
                        crate::durability::item_transfer::ItemTransferError::Unavailable(_)
                    )
            )
        };
        let a = match a {
            Err(error) if busy(&error) => {
                settle_chest_use(&session, &granting, &achievements, fence()?, first.clone()).await
            }
            other => other,
        }
        .map_err(debug)?;
        let b = match b {
            Err(error) if busy(&error) => {
                settle_chest_use(&session, &granting, &achievements, fence()?, second).await
            }
            other => other,
        }
        .map_err(debug)?;
        let a = a.notice.ok_or("first notice")?;
        let b = b.notice.ok_or("second notice")?;
        assert_eq!(a.achievement_key, ANNIHILATOR);
        assert_eq!(b.achievement_key, COOKIES);
        let both = vec![COOKIES.to_owned(), ANNIHILATOR.to_owned()];
        // Whichever committed first reads only its own fact; the other reads both.
        match (a.account_fact_keys.len(), b.account_fact_keys.len()) {
            (1, 2) => {
                assert_eq!(a.account_fact_keys, [ANNIHILATOR]);
                assert_eq!(b.account_fact_keys, both);
            }
            (2, 1) => {
                assert_eq!(a.account_fact_keys, both);
                assert_eq!(b.account_fact_keys, [COOKIES]);
            }
            other => return Err(format!("watermarks not distinct: {other:?}").into()),
        }

        // A replay of a granting command and a chest without an achievement: no notice.
        let replay = settle_chest_use(&session, &granting, &achievements, fence()?, first)
            .await
            .map_err(debug)?;
        assert!(matches!(
            replay.mint,
            RewardClaimMintOutcome::AlreadyCommitted(_)
        ));
        assert_eq!(replay.notice, None);
        let plain = settle_chest_use(
            &session,
            &granting,
            &achievements,
            fence()?,
            use_request(command(3)?, CHEST_PLACEMENT)?,
        )
        .await
        .map_err(debug)?;
        assert_eq!(plain.notice, None);

        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
