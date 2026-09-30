// D39 PG cases: `interaction_chest_use::settle_chest_use` resolves the placed chest and the
// reward and backpack facts from the current Content generation, builds the GAME-INTERACTION
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
    ReferenceItemStackClass, ReferencePlayableContentSource, Sha256HexDigest, SpatialAddress,
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
    ChestUseError, ChestUseOutcome, ChestUseRequest, chest_use_occurrence, prepare_chest_use,
    settle_chest_use,
};
use crate::item_transfer_postgres_cases::{
    CHARACTER, Harness, SESSION, TestResult, configured_admin, debug, id, runtime, scope, uuid_text,
};
use ReferenceItemField::{Known, Unknown};
use std::collections::BTreeMap;

const COIN: &str = "oteryn:chestuse.pg.coin";
const BACKPACK: &str = "oteryn:chestuse.pg.backpack";
const CHEST: &str = "oteryn:chestuse.pg.chest";
const ABSENT: &str = "oteryn:chestuse.pg.absent-from-content";
const CHEST_PLACEMENT: &str = "oteryn:chestuse.pg.placement.chest";
const SECOND_CHEST_PLACEMENT: &str = "oteryn:chestuse.pg.placement.chest.second";
/// The resolved definition of both chest placements (§5.3).
const CHEST_DEFINITION: &str = "Item:oteryn:chestuse.pg.chest@definition-r1";
const DOOR_PLACEMENT: &str = "oteryn:chestuse.pg.placement.door";
const CLAIM: &str = "oteryn:chestuse.pg.claim.first";
const OTHER_CLAIM: &str = "oteryn:chestuse.pg.claim.second";
const THIRD_CLAIM: &str = "oteryn:chestuse.pg.claim.third";
const FOURTH_CLAIM: &str = "oteryn:chestuse.pg.claim.fourth";
/// An earnable record of the embedded catalogue (`content/achievements/`), at revision "1".
const ANNIHILATOR: &str = "oteryn:achievement/annihilator";
/// The harness Character's account (`item_transfer_postgres_cases::seed_character`).
const ACCOUNT: u8 = 40;

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
/// container-slot pattern) and a `CHEST` item, plus three placements: two chests (Item
/// placements) and a `LocalObject` door. `ABSENT` is never a definition key.
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
            SECOND_CHEST_PLACEMENT,
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

/// The runtime catalogue embedded from `content/achievements/`.
fn catalogue() -> TestResult<AchievementCatalogue> {
    Ok(AchievementCatalogue::embedded().map_err(debug)?)
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
        achievement: None,
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

        let request = use_request(command(1)?, CHEST_PLACEMENT, CLAIM, COIN, 7)?;
        let first = settle_chest_use(&session, &content, &achievements, fence()?, request.clone())
            .await
            .map_err(debug)?;
        assert!(matches!(first.mint, RewardClaimMintOutcome::Committed(_)));
        assert_eq!(
            first.child,
            chest_use_occurrence(&request, CHEST_DEFINITION)?
        );
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

        // §17: a changed intent under the same CommandRef conflicts.
        let changed = use_request(command(1)?, CHEST_PLACEMENT, CLAIM, COIN, 8)?;
        let conflict = settle_chest_use(&session, &content, &achievements, fence()?, changed).await;
        assert!(matches!(
            conflict,
            Err(ChestUseError::Mint(RewardClaimMintError::ConflictingCause))
        ));
        // §17 / D40: the chest is part of the intent. The same CommandRef and claim on
        // another chest conflicts; it never replays the first chest's outcome.
        let other_chest = use_request(command(1)?, SECOND_CHEST_PLACEMENT, CLAIM, COIN, 7)?;
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

        // D42: a new command on the claimed `once` claim is refused with nothing written.
        let again = settle_chest_use(
            &session,
            &content,
            &achievements,
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
            &achievements,
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
            let outcome =
                settle_chest_use(&session, &content, &achievements, fence()?, request).await;
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
            &achievements,
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
        let request = use_request(command(1)?, CHEST_PLACEMENT, CLAIM, COIN, 3)?;
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
                use_request(command(2)?, chest, CLAIM, COIN, 3)?,
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
            use_request(command(2)?, CHEST_PLACEMENT, CLAIM, COIN, 3)?,
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
            use_request(command(3)?, CHEST_PLACEMENT, OTHER_CLAIM, COIN, 1)?,
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
            use_request(command(4)?, CHEST_PLACEMENT, OTHER_CLAIM, COIN, 1)?,
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
            use_request(command(5)?, CHEST_PLACEMENT, THIRD_CLAIM, COIN, 1)?,
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

        // A chest without an achievement: the claim commits and no achievement row is written.
        let plain = settle_chest_use(
            &session,
            &content,
            &achievements,
            fence()?,
            use_request(command(1)?, CHEST_PLACEMENT, CLAIM, COIN, 1)?,
        )
        .await
        .map_err(debug)?;
        assert!(matches!(plain.mint, RewardClaimMintOutcome::Committed(_)));
        assert_eq!(achievement_rows(&harness).await?, (vec![], vec![]));

        // A chest with an achievement: the server resolves the key in the embedded catalogue
        // (Annihilator, revision "1") and the MINT grants it to the account with the claim.
        let mut with_achievement =
            use_request(command(2)?, SECOND_CHEST_PLACEMENT, OTHER_CLAIM, COIN, 1)?;
        with_achievement.achievement = Some(ANNIHILATOR.into());
        let granted = settle_chest_use(
            &session,
            &content,
            &achievements,
            fence()?,
            with_achievement.clone(),
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

        // A replay returns the first outcome and grants nothing again; the same command
        // without the achievement is another intent and conflicts.
        let replay = settle_chest_use(
            &session,
            &content,
            &achievements,
            fence()?,
            with_achievement.clone(),
        )
        .await
        .map_err(debug)?;
        assert_eq!(committed(&replay)?, committed(&granted)?);
        let mut without = with_achievement;
        without.achievement = None;
        let conflict = settle_chest_use(&session, &content, &achievements, fence()?, without).await;
        assert!(
            matches!(
                conflict,
                Err(ChestUseError::Mint(RewardClaimMintError::ConflictingCause))
            ),
            "{conflict:?}"
        );
        assert_eq!(achievement_rows(&harness).await?, granted_rows);

        // A retired achievement: the claim commits and nothing is recorded for it.
        let mut retired = use_request(command(3)?, CHEST_PLACEMENT, THIRD_CLAIM, COIN, 1)?;
        retired.achievement = Some("oteryn:achievement/the_more_the_merrier".into());
        let retired = settle_chest_use(&session, &content, &achievements, fence()?, retired)
            .await
            .map_err(debug)?;
        assert!(matches!(retired.mint, RewardClaimMintOutcome::Committed(_)));
        assert_eq!(achievement_rows(&harness).await?, granted_rows);

        // A key the catalogue lacks is refused before any write, whatever the caller names.
        let mut absent = use_request(command(4)?, CHEST_PLACEMENT, FOURTH_CLAIM, COIN, 1)?;
        absent.achievement = Some("oteryn:achievement/not_in_the_catalogue".into());
        let absent = settle_chest_use(&session, &content, &achievements, fence()?, absent).await;
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
