//! QREADY1: source-bound USE, exact native backpack reward, second-claim refusal and durable replay.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

const DREAMERS_BOOK: (u16, u16, i8) = (32850, 32285, -14);

// Compile the retained repository regions without injecting chest UIDs or appearances.
pub(super) fn retained_world(
    chest: (u16, u16, i8),
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    retained_world_from(
        chest,
        native,
        &[
            (-1, 0), (0, 1), (1, 0), (0, -1), (-1, 1), (1, 1), (-1, -1), (1, -1),
        ],
    )
}

fn retained_world_from(
    chest: (u16, u16, i8),
    native: &crate::content::native_gameplay::NativeGameplayState,
    approaches: &[(i32, i32)],
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle_compiler::{bundle, compile, source};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let project = source::project_regions(&root, &[(chest.0 / 256, chest.1 / 256)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let (_, mut pins) = crate::map::boot::tests::bundle();
    let identity = oteryn_world_bundle::bundle::read(&crate::map::boot::tests::bundle().0)?
        .manifest
        .identity;
    let compiled = compile::compile(
        &compile::Input {
            regions: &project.regions,
            palette: &palette,
            identity,
            world: source::world_extent(&root)?,
            build_class: bundle::BuildClass::NonProduction,
            draft_areas: Vec::new(),
            families: &project.families,
        },
        &registry,
    )?;
    pins.digest = compiled.digest;
    let base = crate::map::load(&compiled.bytes, &pins).map_err(debug)?;
    for &(dx, dy) in approaches {
        let start = crate::map::overlay::TilePos {
            x: u16::try_from(i32::from(chest.0) + dx)?,
            y: u16::try_from(i32::from(chest.1) + dy)?,
            floor: chest.2,
        };
        if !base
            .tile(start.x, start.y, start.floor)
            .is_some_and(|tile| tile.walkable())
        {
            continue;
        }
        match crate::map::boot::boot(
            &compiled.bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins.clone(),
                start,
            },
            crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
            crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
            |key| {
                use crate::content::{ReferenceItemField, ReferenceItemType};
                let index = native.item_index()?;
                let revision = index.revision(key)?;
                let reference = index.definition_ref("Item", key, revision)?;
                let profile = native
                    .item_policy(key, revision)
                    .map(|policy| policy.record());
                let pickupable = profile.is_some_and(|profile| {
                    matches!(
                    &profile.semantics.physical, ReferenceItemField::Known(physical)
                    if physical.pickupable == ReferenceItemField::Known(true))
                });
                let door = profile.is_some_and(|profile| matches!(
                    &profile.semantics.classification, ReferenceItemField::Known(classification)
                    if classification.item_type == ReferenceItemField::Known(ReferenceItemType::Door)));
                let attributes = profile.map(|profile| &profile.attributes);
                Some(crate::map::facts::ItemDefinition {
                    reference,
                    pickupable,
                    door,
                    solid: attributes.and_then(|a| a.blocks_movement),
                    blocks_projectile: attributes.and_then(|a| a.blocks_projectile).unwrap_or(true),
                })
            },
        ) {
            Ok(world) => return Ok(world),
            Err(crate::map::boot::BootRefusal::StartNotEnterable) => continue,
            Err(error) => return Err(debug(error)),
        }
    }
    Err("source chest has no enterable adjacent tile".into())
}

pub(super) fn claim_case(
    claim: &str,
    chest: (u16, u16, i8),
    choices: &[(u16, u16)],
    appearance: u16,
    reward: &str,
    quantity: u32,
) -> TestResult {
    claim_case_with_reward_check(
        claim,
        chest,
        choices,
        appearance,
        reward,
        quantity,
        RewardCheck::Item,
    )
}

pub(super) enum RewardCheck {
    Item,
    Book(&'static str),
    Lantern,
}

pub(super) fn claim_case_with_reward_check(
    claim: &str,
    chest: (u16, u16, i8),
    choices: &[(u16, u16)],
    appearance: u16,
    reward: &str,
    quantity: u32,
    check: RewardCheck,
) -> TestResult {
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "qready1_claim", false, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated_pin) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let outfox = claim == "oteryn:reward-claim.quest.u8_1.to_outfox_afox_quest.mining_helmet";
        // An empty choice list requests the shipped region. Doublet also uses
        // the shipped ground entry (ordinal zero).
        let world = if outfox {
            retained_world_from(chest, native, &[(0, -2)])?
        } else if choices.is_empty() || claim == "oteryn:reward-claim.kv.doublet" {
            retained_world(chest, native)?
        } else {
            super::quest_callbacks::source_world_choices(chest, choices, appearance)?
        };
        let mut start = world.start();
        if chest == DREAMERS_BOOK && matches!(check, RewardCheck::Book(_)) {
            // Seed the approach position a player can stand at after picking up
            // the loose map book. Keep the shipped chest and region intact.
            start.x = chest.0;
            start.y = chest.1 + 1;
            let approach = world
                .base()
                .tile(start.x, start.y, start.floor)
                .ok_or("book approach")?;
            assert!(approach.walkable());
        }
        let key = if matches!(check, RewardCheck::Book(_)) {
            // Send USE to the visible chest even when the reward binder cannot attach it.
            let tile = world
                .base()
                .tile(chest.0, chest.1, chest.2)
                .ok_or("shipped chest tile")?;
            let mut entries =
                (0..=u8::MAX).filter(|&ordinal| tile.appearance(ordinal) == Some(appearance));
            let ordinal = entries.next().ok_or("shipped chest appearance")?;
            assert!(
                entries.next().is_none(),
                "ambiguous shipped chest appearance"
            );
            oteryn_world_bundle::bundle::placement_key(chest.2, chest.0, chest.1, ordinal)
                .ok_or("shipped chest placement")?
        } else {
            let report = crate::content::world_reward_claims::bind_embedded_reward_claims(
                world.base().as_ref(),
                &|_| true,
            )?;
            assert!(
                report
                    .served
                    .iter()
                    .any(|entry| entry.key.as_str() == claim),
                "{claim}: {:?}",
                report.unbound,
            );
            report
                .served
                .iter()
                .any(|entry| entry.key.as_str() == claim),
            "{claim}: {:?}",
            report.unbound
        );
        let entry_cells = if outfox {
            activated_pin.qualified_room().movement_cells()
        } else {
            deps.room.movement_cells()
        };
        let cells = world.movement_cells(entry_cells).map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated_pin.frame_binding().digest(),
            world.map_revision_digest(),
            (
                i32::from(start.x),
                i32::from(start.y),
                -i16::from(start.floor),
            ),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: i32::from(start.x),
                    y: i32::from(start.y),
                    floor: -i16::from(start.floor),
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(activated_pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
        let weights = NativeItemWeights(native);
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        harness
            .root
            .initialize_admission_equipment(
                &authority,
                &harness.node,
                &fence,
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        let template = crate::durability::starter_grant::StarterTemplate {
            definition: TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2854".into(),
                revision_ref: "definition-r1".into(),
            },
            ..crate::durability::starter_grant::StarterTemplate::main()
        };
        harness
            .root
            .grant_starter_items(&authority, &harness.node, &fence, &weights, &template)
            .await
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        let mut placement = world.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        // The second USE is a new command on the live connection. Rebuild the owner
        // for the replay so no Foundation command-result cache can satisfy it.
        let catalogue = std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        );
        let mut journey_owner = if outfox {
            let mut owner = deps.owner(&harness, &authority);
            owner.active_generation = Some(generation);
            owner.quest_catalogue = Some(std::sync::Arc::clone(&catalogue));
            owner.movement_cells = (&cells).into();
            owner.admit_progression_session(&mut admitted).await;
            owner.admit_quest_session(&admitted).await;
            deps.initialize(&owner).await;
            enter_mining_helmet_room(&mut owner, &mut admitted).await?;
            Some(owner)
        } else {
            None
        };
        let first = admitted.continuity.next_command_id;
        for (commands, expected) in [
            (
                &[first, first + 1][..],
                &[UseDisposition::Committed, UseDisposition::NothingToUse][..],
            ),
            (&[first][..], &[UseDisposition::Committed][..]),
        ] {
            let owner = if let Some(owner) = journey_owner.take() {
                owner
            } else {
                let mut owner = deps.owner(&harness, &authority);
                owner.active_generation = Some(generation);
                owner.quest_catalogue = Some(std::sync::Arc::clone(&catalogue));
                owner.movement_cells = (&cells).into();
                owner.admit_progression_session(&mut admitted).await;
                owner.admit_quest_session(&admitted).await;
                deps.initialize(&owner).await;
                owner
            };
            let actual = if outfox && commands.len() == 1 {
                // The new owner has no socket history. Exercise the same USE
                // dispatcher directly with the original durable command ID.
                let fence = owner
                    .current_quest_fence(admitted.game_session_id)
                    .await
                    .map_err(|()| "Mining Helmet replay fence")?
                    .ok_or("Mining Helmet replay fence")?;
                let actor = admitted.runtime_actor.ok_or("Mining Helmet actor")?;
                let command = UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: first,
                    item_fence: Some(fence.item_fence()),
                };
                vec![owner.use_object(actor, command, target.clone()).await.disposition]
            } else {
                super::quest_callbacks::wire_uses(&owner, admitted, commands, &target)
                    .await
                    .map_err(|e| format!("claim={claim} commands={commands:?}: {e}"))?
            };
            assert_eq!(actual, expected);
            let bag = harness
                .root
                .read_character_backpack(&authority, fence.character_id)
                .await
                .map_err(debug)?
                .ok_or("bag")?;
            assert_eq!(bag.entries.len(), 1);
            assert_eq!(bag.entries[0].item.quantity, quantity);
            assert_eq!(bag.entries[0].item.definition.production_key, reward);
            if let RewardCheck::Book(digest) = check {
                let text: (String, i64) = sqlx::query_as(
                    "SELECT encode(sha256(convert_to(text,'UTF8')),'hex'),text_revision::bigint \
                     FROM game_item_texts WHERE item_instance_id=encode($1,'hex')::uuid",
                )
                .bind(bag.entries[0].item.item_instance_id.as_slice())
                .fetch_one(&harness.pool)
                .await?;
                assert_eq!(text, (digest.into(), 1), "{claim}");
            }
            if matches!(check, RewardCheck::Lantern) && commands == [1] {
                use crate::gameplay_transport::connection::FreshAdmissionAuthority;
                use oteryn_protocol_oteryn::world_object::{
                    UseIntent, UseTarget, encode_item_use_intent,
                };

                deps.runtime
                    .lock()
                    .await
                    .bind_attacker_lease(
                        deps.actor,
                        admitted.game_session_id,
                        crate::foundation::CharacterLease::new(
                            crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                                .map_err(debug)?,
                            fence.character_lease_generation,
                        )
                        .map_err(debug)?,
                    )
                    .map_err(debug)?;
                super::track(&owner, &deps)?;
                connection.runtime_actor = Some(deps.actor);
                connection.continuity.selected_capabilities =
                    crate::gameplay_transport::capabilities::SelectedCapabilities::select(
                        owner.offered_capabilities(),
                        &[4, 6, 11, 15, 18],
                    )
                    .ok_or("lantern capabilities")?;
                let payload = encode_item_use_intent(&UseIntent {
                    target: UseTarget::Item(std::num::NonZeroU64::MIN),
                    use_with: None,
                    use_with_map_item: None,
                    use_with_position: None,
                })
                .map_err(debug)?;
                let index = native.item_index().ok_or("native item index")?;
                for (from, to) in [
                    ("oteryn:item.tibia.i23738", "oteryn:item.tibia.i23734"),
                    ("oteryn:item.tibia.i23734", "oteryn:item.tibia.i23738"),
                ] {
                    super::m44_source_lifecycle_fixtures::wait_for_world_action(
                        &owner,
                        &connection,
                    )
                    .await?;
                    let reference = index
                        .definition_ref("Item", from, "definition-r1")
                        .ok_or("lantern definition")?;
                    let (next, disposition, _) = super::item_craft_fixtures::exchange(
                        &owner,
                        connection,
                        connection.continuity.next_command_id,
                        &payload,
                        Some(reference),
                    )
                    .await?;
                    assert_eq!(disposition, UseDisposition::Committed, "{from} -> {to}");
                    let bag = harness
                        .root
                        .read_character_backpack(&authority, fence.character_id)
                        .await
                        .map_err(debug)?
                        .ok_or("lantern bag")?;
                    assert_eq!(bag.entries.len(), 1);
                    assert_eq!(bag.entries[0].item.quantity, 1);
                    assert_eq!(bag.entries[0].item.definition.production_key, to);
                    connection = next;
                }
            }
            if claim == "oteryn:reward-claim.kv.timering3" {
                let partial_charges: Option<i32> = sqlx::query_scalar(
                    "SELECT charges FROM game_item_timed_states WHERE item_instance_id=encode($1,'hex')::uuid",
                )
                .bind(bag.entries[0].item.item_instance_id.as_slice())
                .fetch_optional(&harness.pool)
                .await?;
                assert_eq!(partial_charges, None, "the amulet arrives at full charges");
                let (timed, _) = crate::gameplay_transport::timed_item_content::facts(
                    native,
                    &bag.entries[0].item.definition,
                )
                .ok_or("Elven Amulet timed definition")?;
                assert_eq!(
                    timed.full.charges(),
                    Some(50),
                    "Shadowthorn Elven Amulet charges"
                );
            }
            let quest = match claim {
                "oteryn:reward-claim.kv.adoreduh" => Some("oteryn:quest.adorned_uh_rune_quest"),
                "oteryn:reward-claim.kv.barbarianaxe1" => Some("oteryn:quest.barbarian_axe_quest"),
                "oteryn:reward-claim.kv.battleaxe" => Some("oteryn:quest.battle_axe_quest"),
                "oteryn:reward-claim.kv.bloodherb" => Some("oteryn:quest.blood_herb_quest"),
                "oteryn:reward-claim.kv.captain_iglues" => {
                    Some("oteryn:quest.captain_iglues_treasure_quest")
                }
                _ => None,
            };
            if let Some(quest) = quest {
                let completed: (i64, i64) = sqlx::query_as(
                    "SELECT (SELECT count(*) FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 AND completed_character_revision IS NOT NULL), (SELECT count(*) FROM game_character_quest_receipts WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 AND completes)",
                ).bind(fence.character_id.as_bytes().as_slice()).bind(quest).fetch_one(&harness.pool).await?;
                assert_eq!(
                    completed,
                    (1, 1),
                    "{quest}: native source claim must record completion exactly once"
                );
            }
        }
        let claims: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claims WHERE character_id=encode($1,'hex')::uuid AND claim_production_key=$2), (SELECT count(*) FROM game_reward_claim_mint_receipts)")
            .bind(fence.character_id.as_bytes().as_slice()).bind(claim).fetch_one(&harness.pool).await?;
        assert_eq!(claims, (1, 1));
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn enter_mining_helmet_room(
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
) -> TestResult {
    use crate::gameplay_transport::capabilities::SelectedCapabilities;
    use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
    use oteryn_protocol_oteryn::world_spatial::StepDirection;

    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
            .ok_or("Mining Helmet room capabilities")?;
    let map = owner.bundle_map().ok_or("Mining Helmet map")?;
    let door = crate::map::view::placement_key(
        crate::map::overlay::TilePos {
            x: 32467,
            y: 31969,
            floor: -5,
        },
        1,
    )
    .ok_or("Mining Helmet door")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&door.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: 0,
    };
    let first = admitted.continuity.next_command_id;
    let locked = super::quest_callbacks::wire_use_targets_retained(owner, admitted, &[(first, &target)])
        .await?;
    assert_eq!(locked[0].1, UseDisposition::Locked);

    let [budrik] = super::classic_quests_fixtures::install_with_offsets(
        owner,
        admitted,
        ["oteryn:npc.budrik"],
        Some([(-1, 0)]),
    )
    .await?;
    for (command, word) in [(9000, "hi"), (9001, "mission"), (9002, "yes")] {
        assert_eq!(
            super::qengine_rewards_fixtures::talk(owner, admitted, budrik, command, word).await?,
            NpcIntentDisposition::Ok,
            "{word}"
        );
    }
    let next = admitted.continuity.next_command_id;
    let opened = super::quest_callbacks::wire_use_targets_retained(owner, admitted, &[(next, &target)])
        .await?;
    assert_eq!(opened[0].1, UseDisposition::Committed);
    super::desert_earned_route_fixtures::step(owner, admitted, StepDirection::South).await
}

#[test]
fn qready1_adorned_uh_rune_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.adoreduh",
                (33136, 31601, -15),
                &[(33136, 6025)],
                2469,
                "oteryn:item.tibia.i11603",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_adorned_uh_rune_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_barbarian_axe_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.barbarianaxe1",
                (33184, 31945, -11),
                &[(33184, 6026), (33185, 6027)],
                2469,
                "oteryn:item.tibia.i3317",
                1,
            )
            .map_err(|e| e.to_string())?;
            claim_case(
                "oteryn:reward-claim.kv.barbarianaxe1",
                (33185, 31945, -11),
                &[(33184, 6026), (33185, 6027)],
                2469,
                "oteryn:item.tibia.i3307",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_barbarian_axe_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_battle_axe_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.battleaxe",
                (32305, 32254, -9),
                &[(32305, 6180)],
                4285,
                "oteryn:item.tibia.i3266",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_battle_axe_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_blood_herb_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.bloodherb",
                (32769, 31968, -7),
                &[(32769, 6111)],
                3634,
                "oteryn:item.tibia.i3734",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_blood_herb_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_captain_iglues_treasure_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.captain_iglues",
                (32039, 32121, -13),
                &[(32039, 6117)],
                2472,
                "oteryn:item.tibia.i3579",
                2,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_captain_iglues_treasure_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_circle_room_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.circleroom1",
                (32495, 31992, -14),
                &[(32495, 6122)],
                2469,
                "oteryn:item.tibia.i3279",
                1,
            )
            .map_err(|e| e.to_string())?;
            claim_case(
                "oteryn:reward-claim.kv.circleroom2",
                (32497, 31992, -14),
                &[(32497, 6123)],
                2469,
                "oteryn:item.tibia.i3323",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_circle_room_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_combat_knife_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.combatknife",
                (32102, 32235, -8),
                &[(32102, 5025)],
                2473,
                "oteryn:item.tibia.i3292",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_combat_knife_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_crusader_helmet_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.crusaderhelmet",
                (32427, 31943, -14),
                &[(32427, 6164)],
                3990,
                "oteryn:item.tibia.i3391",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_crusader_helmet_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_crystal_wand_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.crystalwand1",
                (32479, 31611, -15),
                &[(32479, 6124)],
                2472,
                "oteryn:item.tibia.i3068",
                1,
            )
            .map_err(|e| e.to_string())?;
            claim_case(
                "oteryn:reward-claim.kv.crystalwand2",
                (32481, 31611, -15),
                &[(32481, 6125)],
                2472,
                "oteryn:item.tibia.i11609",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_crystal_wand_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_darashia_dragon_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.quest.u8_5.darashia_dragon",
                (33233, 32280, -12),
                &[(33233, 6285)],
                4286,
                "oteryn:item.tibia.i3052",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_darashia_dragon_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_dark_armor_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.darkarmor",
                (33178, 31870, -12),
                &[(33178, 6028)],
                4311,
                "oteryn:item.tibia.i3383",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_dark_armor_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_demona_ring_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.demonaring1",
                (32504, 31596, -14),
                &[(32504, 6126)],
                2472,
                "oteryn:item.tibia.i3049",
                1,
            )
            .map_err(|e| e.to_string())?;
            claim_case(
                "oteryn:reward-claim.kv.demonaring2",
                (32515, 31596, -14),
                &[(32515, 6127)],
                2472,
                "oteryn:item.tibia.i3051",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_demona_ring_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_double_hero_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.doublehero1",
                (33109, 31679, -13),
                &[(33109, 6032)],
                2469,
                "oteryn:item.tibia.i3039",
                1,
            )
            .map_err(|e| e.to_string())?;
            claim_case(
                "oteryn:reward-claim.kv.doublehero2",
                (33110, 31679, -13),
                &[(33110, 6033)],
                2469,
                "oteryn:item.tibia.i3093",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_double_hero_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_doublet_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.doublet",
                (32084, 32181, -8),
                &[(32084, 6181)],
                408,
                "oteryn:item.tibia.i3379",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_doublet_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_elephant_tusk_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.quest.u7_5.elephant_tusk",
                (32922, 32755, -7),
                &[(32922, 6185)],
                2472,
                "oteryn:item.tibia.i3044",
                2,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_elephant_tusk_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_explorer_brooch_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.explorerbrooch",
                (32636, 31873, -10),
                &[(32636, 6019)],
                4240,
                "oteryn:item.tibia.i4871",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_explorer_brooch_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn qready1_griffin_shield_quest_exact_reward_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result: TestResult = std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            claim_case(
                "oteryn:reward-claim.kv.griffinshield1",
                (32498, 31721, -15),
                &[(32498, 6165)],
                4240,
                "oteryn:item.tibia.i3433",
                1,
            )
            .map_err(|e| e.to_string())?;
            claim_case(
                "oteryn:reward-claim.kv.griffinshield2",
                (32500, 31721, -15),
                &[(32500, 6166)],
                4024,
                "oteryn:item.tibia.i3313",
                1,
            )
            .map_err(|e| e.to_string())?;
            claim_case(
                "oteryn:reward-claim.kv.griffinshield3",
                (32503, 31724, -15),
                &[(32503, 6167)],
                4024,
                "oteryn:item.tibia.i3323",
                1,
            )
            .map_err(|e| e.to_string())?;
            Ok(())
        })?
        .join()
        .map_err(|_| "qready1 dispatch panicked")?
        .map_err(Into::into);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qready1_griffin_shield_quest duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

#[test]
fn time_ring_shadowthorn_source_boxes_mint_exact_rewards_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("time-ring-source-boxes", || {
        for (claim, x, reward) in [
            ("oteryn:reward-claim.kv.timering1", 33038, "oteryn:item.tibia.i3076"),
            ("oteryn:reward-claim.kv.timering2", 33039, "oteryn:item.tibia.i3053"),
            ("oteryn:reward-claim.kv.timering3", 33040, "oteryn:item.tibia.i3082"),
        ] {
            claim_case(claim, (x, 32171, -9), &[], 2472, reward, 1)?;
        }
        Ok(())
    })
}

#[test]
fn outfox_shipped_cache_mints_mining_helmet_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run("outfox-source-cache", || {
        claim_case(
            "oteryn:reward-claim.quest.u8_1.to_outfox_afox_quest.mining_helmet",
            (32467, 31970, -5),
            &[],
            2469,
            "oteryn:item.tibia.i139",
            1,
        )
    })
}
