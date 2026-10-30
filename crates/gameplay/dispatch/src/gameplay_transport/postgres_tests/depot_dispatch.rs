//! Real admitted connection -> depot/stash dispatch -> fenced item owner -> PostgreSQL.
#[path = "depot_dispatch/mail_cases.rs"]
mod mail_cases;
#[path = "depot_dispatch/retrieve_cases.rs"]
mod retrieve_cases;
#[path = "depot_dispatch/stash_cases.rs"]
mod stash_cases;
#[path = "depot_dispatch/write_text_depot_session.rs"]
mod write_text_depot_session;
#[path = "depot_dispatch/write_text_ground_movement.rs"]
mod write_text_ground_movement;
#[path = "depot_dispatch/write_text_ground_nested_view.rs"]
mod write_text_ground_nested_view;
#[path = "depot_dispatch/write_text_ground_view.rs"]
mod write_text_ground_view;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest,
};
use crate::durability::item_transfer::{ItemTransferDestination, ItemTransferRequest};
use crate::durability::premium_fence::EntitlementState;
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, LivenessPolicy, UseCommand,
};
use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
use crate::gameplay_transport::item_view::{ItemKey, ItemViewContinuity, SessionItemView};
use crate::map::boot::{BootPins, boot, tests as bundle_fixture};
use crate::map::facts::ItemDefinition;
use oteryn_protocol_oteryn::depot::*;

/// The wire scenarios below run owner-side checks (SQL fixtures, lock waits, moves) while the
/// test client is not reading or acking the wire, so under CI load the idle cadence ends the
/// session mid-exchange and the client then waits out the exchange timeout. Liveness is not
/// under test here (`idle_kick` and the connection tests own it): the probe interval outlasts
/// every exchange timeout.
const SCENARIO_LIVENESS: LivenessPolicy = LivenessPolicy {
    interval: std::time::Duration::from_secs(600),
    missed_limit: 3,
};

#[test]
fn depot_session_dispatch_opens_deposits_browses_withdraws_and_replays() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false, false, false, false, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "depot e2e panicked")?
        .map_err(Into::into)
}

#[test]
fn depot_search_dispatch_retrieves_all_depot_items_until_no_room_once() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            scenario_with_premium(false, false, false, false, false, None, Mode::Retrieve)
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "depot search retrieval e2e panicked")?
        .map_err(Into::into)
}

#[test]
fn mail_dispatch_posts_a_carried_letter_into_a_mailbox_once() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            scenario_with_premium(false, false, false, false, false, None, Mode::Mail)
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "mail e2e panicked")?
        .map_err(Into::into)
}

#[test]
fn stash_session_dispatch_free_stows_withdraws_checks_capacity_and_replays() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(true, false, false, false, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "stash e2e panicked")?
        .map_err(Into::into)
}

#[test]
fn stash_session_dispatch_conserves_and_replays() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            scenario_with_premium(
                true,
                false,
                false,
                false,
                false,
                Some(EntitlementState::Active),
                Mode::Plain,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "Premium stash e2e panicked")?
        .map_err(Into::into)
}

#[test]
fn stash_session_dispatch_expired_premium_stows_withdraws_and_replays() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            scenario_with_premium(
                true,
                false,
                false,
                false,
                false,
                Some(EntitlementState::Expired),
                Mode::Plain,
            )
            .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "expired Premium stash e2e panicked")?
        .map_err(Into::into)
}

#[test]
fn write_text_ground_container_real_dispatch_observes_reach_and_custody() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false, true, false, false, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "ground container e2e panicked")?
        .map_err(Into::into)
}

#[test]
fn write_text_ground_nested_container_real_dispatch_observes_ancestry_and_reach() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false, true, true, false, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "nested Ground container e2e panicked")?
        .map_err(Into::into)
}

#[test]
fn write_text_depot_real_dispatch_revalidates_session_after_page_wait() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false, false, false, true, false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "depot session e2e panicked")?
        .map_err(Into::into)
}

#[test]
fn write_text_ground_real_dispatch_movement_closes_external_views() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false, true, true, false, true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "Ground movement view e2e panicked")?
        .map_err(Into::into)
}

/// The extra flow a scenario runs after seeding.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Plain,
    Mail,
    Retrieve,
}

fn scenario(
    stash: bool,
    ground_container: bool,
    nested_ground: bool,
    depot_session: bool,
    ground_movement: bool,
) -> TestResult {
    scenario_with_premium(
        stash,
        ground_container,
        nested_ground,
        depot_session,
        ground_movement,
        None,
        Mode::Plain,
    )
}

fn scenario_with_premium(
    stash: bool,
    ground_container: bool,
    nested_ground: bool,
    depot_session: bool,
    ground_movement: bool,
    stash_premium: Option<EntitlementState>,
    mode: Mode,
) -> TestResult {
    let mail = mode == Mode::Mail;
    let retrieve = mode == Mode::Retrieve;
    run(async |admin| {
        let harness = Harness::create_with_admission_seed(admin, "depot_dispatch", true, false)
            .await
            .map_err(|e| format!("{e:?}"))?;
        super::reset();
        let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        assert_eq!(harness.root.active_world_reset_epoch(world_id).await?, None);
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
        if ground_movement {
            deps.room = activated_pin.qualified_room().clone();
        }
        let generation = controller.active().ok_or("active generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("item index")?;
        let backpack_definition = crate::gameplay_transport::item_look::typed_definition(
            index,
            index
                .definition_ref("Item", "oteryn:item.tibia.i2854", "definition-r1")
                .ok_or("backpack ref")?,
        )
        .ok_or("backpack definition")?;
        let coin_definition = crate::gameplay_transport::item_look::typed_definition(
            index,
            index
                .definition_ref(
                    "Item",
                    if stash {
                        "oteryn:item.tibia.i3028"
                    } else {
                        "oteryn:item.tibia.i3031"
                    },
                    "definition-r1",
                )
                .ok_or("coin ref")?,
        )
        .ok_or("coin definition")?;
        let backpack_facts = generation_item_facts(native, &backpack_definition).map_err(debug)?;
        let coin_facts = generation_item_facts(native, &coin_definition).map_err(debug)?;
        let box_id = index
            .definition_ref(
                "Item",
                &backpack_definition.production_key,
                &backpack_definition.revision_ref,
            )
            .ok_or("box id")?
            .get()
            - 1;
        let coin_id = index
            .definition_ref(
                "Item",
                &coin_definition.production_key,
                &coin_definition.revision_ref,
            )
            .ok_or("coin id")?
            .get()
            - 1;
        let (bytes, mut bundle_pins) = bundle_fixture::bundle_with_item_ids(
            (&backpack_definition.production_key, box_id),
            (&coin_definition.production_key, coin_id),
        );
        let mut bundle = oteryn_world_bundle::bundle::read(&bytes).map_err(debug)?;
        let locker = bundle
            .sectors
            .iter_mut()
            .flat_map(|s| &mut s.tiles)
            .find(|t| t.x == 6)
            .ok_or("locker tile")?;
        locker.items.get_mut(1).ok_or("locker item")?.attrs.depot = Some(0);
        if mail {
            // An authored mailbox (Canary items.xml 3501) beside the start and one out of reach.
            let mut entry = bundle.manifest.palette.get(3).ok_or("item palette")?.clone();
            entry.key = "oteryn:item.tibia.i3501".into();
            entry.id = index
                .definition_ref("Item", &entry.key, "definition-r1")
                .ok_or("mailbox ref")?
                .get()
                - 1;
            bundle.manifest.palette.push(entry);
            let palette = u32::try_from(bundle.manifest.palette.len() - 1)?;
            let sector = bundle.sectors.first_mut().ok_or("sector")?;
            let mut tile = sector
                .tiles
                .iter()
                .find(|tile| tile.x == 7)
                .ok_or("start")?
                .clone();
            tile.items.get_mut(1).ok_or("start item")?.palette = palette;
            for x in [8, 10] {
                tile.x = x;
                sector.tiles.push(tile.clone());
            }
        }
        if ground_movement {
            let sector = bundle.sectors.first_mut().ok_or("sector")?;
            let mut tile = sector
                .tiles
                .iter()
                .find(|tile| tile.x == 7)
                .ok_or("start")?
                .clone();
            tile.items.truncate(1);
            for x in [8, 9] {
                tile.x = x;
                sector.tiles.push(tile.clone());
            }
        }
        let bytes =
            oteryn_world_bundle::writer::write(&bundle.manifest, &bundle.sectors, &bundle.spawns)
                .map_err(debug)?;
        bundle_pins.digest = oteryn_world_bundle::bundle::read(&bytes)
            .map_err(debug)?
            .digest;
        let base = crate::map::load(&bytes, &bundle_pins).map_err(debug)?;
        let world = boot(
            &bytes,
            &BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: bundle_pins,
                start: crate::map::overlay::TilePos {
                    x: 7,
                    y: 0,
                    floor: -7,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                Some(ItemDefinition {
                    reference: index.definition_ref("Item", key, "definition-r1")?,
                    solid: Some(key == backpack_definition.production_key),
                    blocks_projectile: false,
                    pickupable: key == coin_definition.production_key,
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let cells = world
            .movement_cells(deps.room.movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated_pin.frame_binding().digest(),
            world.map_revision_digest(),
            (7, 0, 7),
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
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let (consumer, clock, activation, mut evidence) =
            crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(40));
        if stash {
            harness
                .root
                .latch_premium_activation(
                    &activation.activation_id,
                    activation.switch_over_us,
                    activation.switch_over_us + 60_000_000,
                )
                .await
                .map_err(debug)?;
        }
        let consumer = if stash_premium == Some(EntitlementState::Expired) {
            evidence.state = EntitlementState::Expired;
            evidence.effective_until_us = evidence.authority_issued_at_us;
            let consumer = std::sync::Arc::new(crate::premium::PremiumConsumer::default());
            consumer.prove_for_test(evidence);
            assert_eq!(
                consumer.class(id(40), clock.read_current_window()),
                crate::premium::PremiumClass::Expired
            );
            consumer
        } else {
            consumer
        };
        let premium =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(harness.root.clone()),
            );
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = &cells;
        let session = deps.admitted.game_session_id;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(session)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        let command_ref = |n| -> TestResult<CommandRef> {
            Ok(CommandRef::new(session, CommandId::new(n).map_err(debug)?))
        };
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(activated_pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
        // Seed only the carried target, using the existing MINT/TRANSFER owners. The behavior
        // under test starts below at the real dispatch, with a map handle issued by domain 17.
        let mut carried = None;
        let mut stacks = Vec::new();
        let mut ground = Vec::new();
        // Retrieval: 20 carried stacks fill the main backpack; two more are carried later.
        // Mail: a letter, a parcel and its label (Canary items.xml 3505, 3503, 3507).
        let mail_items = if mail {
            ["oteryn:item.tibia.i3505", "oteryn:item.tibia.i3503", "oteryn:item.tibia.i3507"]
                .into_iter()
                .map(|key| -> TestResult<_> {
                    let definition = crate::gameplay_transport::item_look::typed_definition(
                        index,
                        index
                            .definition_ref("Item", key, "definition-r1")
                            .ok_or("mail item ref")?,
                    )
                    .ok_or("mail item definition")?;
                    let facts = generation_item_facts(native, &definition).map_err(debug)?;
                    Ok((definition, facts))
                })
                .collect::<TestResult<Vec<_>>>()?
        } else {
            Vec::new()
        };
        let seeded: u32 = if retrieve {
            22
        } else if stash || ground_container || mail {
            3
        } else {
            1
        };
        for ordinal in 0..=seeded {
            let mail_item = ordinal
                .checked_sub(1)
                .and_then(|n| mail_items.get(usize::try_from(n).ok()?));
            let mut seed = harness
                .root
                .freeze_item_mint(
                    &authority,
                    &harness.node,
                    ItemMintRequest {
                        cause: ItemMintCause::for_test(
                            deps.admitted.world_id,
                            deps.admitted.channel_id,
                            fence.scope_ownership_generation,
                            ordinal + 1,
                            1,
                            crate::durability::item_mint::TypedDefinitionRef {
                                family: "LootTable".into(),
                                production_key: "fixture:loot.map-target".into(),
                                revision_ref: "definition-r1".into(),
                            },
                            "fixture:purpose.map-target".into(),
                            0,
                        ),
                        item: if let Some((definition, _)) = mail_item {
                            definition.clone()
                        } else if ordinal == 0 || (ground_container && ordinal <= 2) {
                            backpack_definition.clone()
                        } else {
                            coin_definition.clone()
                        },
                        quantity: if ordinal == 0 || (ground_container && ordinal <= 2) || mail {
                            1
                        } else {
                            100
                        },
                        ground: GroundPlacement {
                            spatial_position:
                                crate::durability::map_item_mint_audit::map_item_spatial_position(
                                    7, 0, -7,
                                ),
                            corpse_ref: id(3).to_vec(),
                            map_revision: world.map_revision().into(),
                            content_revision: "content-1".into(),
                            native_room_placement_context: world.base().digest().to_vec(),
                        },
                        content_revision: "content-1".into(),
                        ruleset_revision: "ruleset-1".into(),
                        sim_revision: "sim-1".into(),
                    },
                )
                .await
                .map_err(debug)?;
            let permit = owner.spell_lane_permit().await.ok_or("lane")?;
            let backpack = match harness
                .root
                .commit_item_mint(&permit, &authority, &harness.node, &mut seed)
                .await
                .map_err(debug)?
            {
                ItemMintOutcome::Committed(item) => item.item_instance_id,
                other => return Err(debug(other)),
            };
            if retrieve && ordinal > 20 {
                ground.push(backpack);
                continue;
            }
            let mut transfer = harness
                .root
                .freeze_item_transfer(
                    &authority,
                    &harness.node,
                    fence,
                    ItemTransferRequest {
                        weight_generation_digest: native.source_digest(),
                        command: command_ref(100 + u64::from(ordinal))?,
                        source_item_instance_id: backpack,
                        destination: if ordinal == 0 {
                            ItemTransferDestination::ContainerSlot
                        } else {
                            ItemTransferDestination::MainBackpack
                        },
                        item: if let Some((_, facts)) = mail_item {
                            facts.clone()
                        } else if ordinal == 0 || (ground_container && ordinal <= 2) {
                            backpack_facts.clone()
                        } else {
                            coin_facts.clone()
                        },
                        backpack: (ordinal != 0).then(|| backpack_facts.clone()),
                        content_revision: "content-1".into(),
                        ruleset_revision: "ruleset-1".into(),
                        sim_revision: "sim-1".into(),
                    },
                    &NativeItemWeights(native),
                )
                .await
                .map_err(debug)?;
            harness
                .root
                .commit_item_transfer(
                    &permit,
                    &authority,
                    &harness.node,
                    fence,
                    &mut transfer,
                    &NativeItemWeights(native),
                )
                .await
                .map_err(debug)?;
            drop(permit);
            if ordinal > 0 {
                stacks.push(backpack);
            }
            if ordinal == 1 {
                carried = Some(backpack);
            }
        }
        deps.initialize(&owner).await;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 18, 37, 62, 153])
                .ok_or("selected")?;
        assert!(
            admitted
                .continuity
                .selected_capabilities
                .contains(CAPABILITY_DEPOT_V1)
        );
        // Initial inventory issues the same stable session handles the real connection will issue.
        let mut view = SessionItemView::resume(ItemViewContinuity::default());
        view.snapshot(
            owner
                .observe_character_inventory(deps.actor, session)
                .await
                .ok_or("inventory")?,
            None,
        )
        .map_err(debug)?;
        let handle = view
            .table()
            .handle(&ItemKey::Instance(carried.ok_or("carried")?))
            .ok_or("source handle")?;
        if retrieve {
            let handles = stacks
                .iter()
                .map(|item| {
                    view.table()
                        .handle(&ItemKey::Instance(*item))
                        .map(|handle| (handle, *item))
                        .ok_or("stack handle")
                })
                .collect::<Result<Vec<_>, _>>()?;
            retrieve_cases::deposit(&owner, admitted, &handles).await?;
            // Two more stacks reach the main backpack through the TRANSFER owner: one slot stays.
            for (n, item) in (200..).zip(&ground) {
                let permit = owner.spell_lane_permit().await.ok_or("lane")?;
                let mut transfer = harness
                    .root
                    .freeze_item_transfer(
                        &authority,
                        &harness.node,
                        fence,
                        ItemTransferRequest {
                            weight_generation_digest: native.source_digest(),
                            command: command_ref(n)?,
                            source_item_instance_id: *item,
                            destination: ItemTransferDestination::MainBackpack,
                            item: coin_facts.clone(),
                            backpack: Some(backpack_facts.clone()),
                            content_revision: "content-1".into(),
                            ruleset_revision: "ruleset-1".into(),
                            sim_revision: "sim-1".into(),
                        },
                        &NativeItemWeights(native),
                    )
                    .await
                    .map_err(debug)?;
                harness
                    .root
                    .commit_item_transfer(
                        &permit,
                        &authority,
                        &harness.node,
                        fence,
                        &mut transfer,
                        &NativeItemWeights(native),
                    )
                    .await
                    .map_err(debug)?;
            }
            retrieve_cases::exercise(&owner, admitted, &harness.pool, &handles).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        if mail {
            let [letter, parcel, label] = stacks[..] else {
                return Err("mail items".into());
            };
            mail_cases::exercise(&owner, admitted, &harness.pool, letter, parcel, label).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        if ground_container {
            if !nested_ground {
                write_text_ground_view::exercise(
                    &owner,
                    admitted,
                    carried.ok_or("ground container")?,
                    *stacks.get(2).ok_or("Ground non-container")?,
                    index
                        .definition_ref(
                            "Item",
                            &backpack_definition.production_key,
                            &backpack_definition.revision_ref,
                        )
                        .ok_or("bag reference")?,
                )
                .await?;
            } else {
                write_text_ground_nested_view::exercise(
                    &owner,
                    admitted,
                    &harness.pool,
                    carried.ok_or("Ground root")?,
                    *stacks.get(1).ok_or("nested Ground container")?,
                    ground_movement,
                    index
                        .definition_ref(
                            "Item",
                            &backpack_definition.production_key,
                            &backpack_definition.revision_ref,
                        )
                        .ok_or("bag reference")?,
                )
                .await?;
            }
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        if stash {
            if stash_premium.is_some() {
                owner.premium_coordinator = Some(&premium);
                owner.admit_premium(id(40), session);
            }
            assert_eq!(
                owner.session_premium_gate(session).await.premium_current(),
                stash_premium == Some(EntitlementState::Active)
            );
            let other_handles = stacks
                .iter()
                .skip(1)
                .map(|item| {
                    view.table()
                        .handle(&ItemKey::Instance(*item))
                        .ok_or("stack handle")
                })
                .collect::<Result<Vec<_>, _>>()?;
            return stash_cases::exercise(
                &owner,
                admitted,
                handle,
                &other_handles,
                &harness.pool,
                carried.ok_or("source")?,
                deps.actor,
                fence,
            )
            .await;
        }
        if depot_session {
            let deposit = DepotIntent {
                town: Some(0),
                box_number: 17,
                page: 0,
                action: DepotAction::Deposit(handle),
            };
            let moved = owner
                .depot(
                    deps.actor,
                    UseCommand {
                        game_session_id: session,
                        command_id: 3,
                        item_fence: Some(fence),
                    },
                    deposit,
                    Some(ItemKey::Instance(carried.ok_or("carried")?)),
                )
                .await
                .ok_or("page race deposit")?;
            assert_eq!(moved.reply.outcome, DepotOutcome::Moved);
            assert_eq!(moved.items, vec![(1, carried.ok_or("carried")?)]);
            let search = harness
                .root
                .read_depot_search(&authority, fence.character_id, 3031)
                .await?
                .ok_or("depot search result")?;
            assert_eq!(
                (search.client_id, search.depot, search.stash, search.inbox),
                (3031, 100, 0, 0)
            );
            write_text_depot_session::exercise(&owner, admitted, &harness.pool, deposit).await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        let browse = DepotIntent {
            town: None,
            box_number: 17,
            page: 0,
            action: DepotAction::Browse,
        };
        let deposit = DepotIntent {
            town: Some(0),
            action: DepotAction::Deposit(handle),
            ..browse
        };
        let withdraw = DepotIntent {
            town: Some(0),
            action: DepotAction::Withdraw(1),
            ..browse
        };
        let mut input = Vec::new();
        for (command_id, intent) in [
            (1, browse),
            (2, deposit),
            (
                3,
                DepotIntent {
                    town: Some(0),
                    ..browse
                },
            ),
            (7, withdraw),
            (8, withdraw),
        ] {
            if command_id == 7 {
                use oteryn_protocol_oteryn::depot_search::{self as search, Request};
                for (id, request) in [
                    (4, Request::Search { client_id: 3031 }),
                    (5, Request::Detail { client_id: 3031 }),
                    (
                        6,
                        Request::Search {
                            client_id: u32::MAX,
                        },
                    ),
                ] {
                    let payload = search::encode_request(request).map_err(debug)?;
                    let frame = oteryn_protocol_oteryn::encode_client_command(
                        1,
                        &oteryn_protocol_oteryn::ClientCommandValue {
                            command_id: id,
                            command_type: search::COMMAND_TYPE_DEPOT_SEARCH,
                            payload: &payload,
                        },
                    )
                    .map_err(debug)?;
                    input.extend(u32::try_from(frame.len())?.to_be_bytes());
                    input.extend(frame);
                }
            }
            let payload = encode_depot_intent(intent).map_err(debug)?;
            let frame = oteryn_protocol_oteryn::encode_client_command(
                1,
                &oteryn_protocol_oteryn::ClientCommandValue {
                    command_id,
                    command_type: COMMAND_TYPE_DEPOT_INTENT,
                    payload: &payload,
                },
            )
            .map_err(debug)?;
            input.extend(u32::try_from(frame.len())?.to_be_bytes());
            input.extend(frame);
        }
        // After the withdrawal the coins are carried: retrieval finds nothing in custody.
        for (id, all) in [(9, false), (10, true)] {
            use oteryn_protocol_oteryn::depot_search as search;
            let payload = search::encode_request(search::Request::Retrieve {
                client_id: 3031,
                all,
            })
            .map_err(debug)?;
            let frame = oteryn_protocol_oteryn::encode_client_command(
                1,
                &oteryn_protocol_oteryn::ClientCommandValue {
                    command_id: id,
                    command_type: search::COMMAND_TYPE_DEPOT_SEARCH,
                    payload: &payload,
                },
            )
            .map_err(debug)?;
            input.extend(u32::try_from(frame.len())?.to_be_bytes());
            input.extend(frame);
        }
        let (end, output) =
            super::weather_wire::fixture_access::serve_input(&owner, admitted, &input).await?;
        let mut replies = Vec::new();
        let mut search_replies = Vec::new();
        let mut cursor = output.as_slice();
        while !cursor.is_empty() {
            let length =
                u32::from_be_bytes(cursor.get(..4).ok_or("frame length")?.try_into()?) as usize;
            let frame = cursor.get(4..4 + length).ok_or("frame")?;
            cursor = &cursor[4 + length..];
            let envelope = oteryn_protocol_oteryn::decode_wire_envelope(frame).map_err(debug)?;
            if envelope.message_type() == oteryn_protocol_oteryn::MessageType::CommandResult {
                let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())
                    .map_err(debug)?;
                if matches!(result.command_id, 4..=6 | 9 | 10) {
                    assert_eq!(
                        result.status,
                        oteryn_protocol_oteryn::CommandStatus::Accepted
                    );
                    search_replies.push(
                        oteryn_protocol_oteryn::depot_search::decode_reply(result.payload)
                            .map_err(debug)?,
                    );
                    continue;
                }
                replies.push((
                    result.status,
                    decode_depot_reply(result.payload).map_err(debug)?,
                ));
            }
        }
        use oteryn_protocol_oteryn::depot_search::{Outcome, TierCount};
        assert_eq!(search_replies.len(), 5, "{end}");
        assert_eq!(
            search_replies.iter().map(|r| r.outcome).collect::<Vec<_>>(),
            [
                Outcome::Found,
                Outcome::Found,
                Outcome::NotFound,
                Outcome::NotFound,
                Outcome::NotFound
            ]
        );
        assert_eq!(search_replies[0], search_replies[1]);
        let found = search_replies[0].results.first().ok_or("search result")?;
        assert_eq!(
            (found.client_id, found.depot, found.stash, found.inbox),
            (3031, 100, 0, 0)
        );
        assert_eq!(
            found.tiers,
            [TierCount {
                tier: 0,
                count: 100
            }]
        );
        assert!(
            search_replies[2..]
                .iter()
                .all(|r| r.retrieved == 0 && r.results.is_empty())
        );
        assert_eq!(replies.len(), 5, "{end}");
        assert_eq!(
            replies.iter().map(|(_, r)| r.outcome).collect::<Vec<_>>(),
            [
                DepotOutcome::Opened,
                DepotOutcome::Moved,
                DepotOutcome::Opened,
                DepotOutcome::Moved,
                DepotOutcome::Stale
            ],
            "{end}"
        );
        assert!(
            replies
                .iter()
                .all(|(status, _)| *status == oteryn_protocol_oteryn::CommandStatus::Accepted)
        );
        assert_eq!(replies[2].1.entries.len(), 1);
        assert_eq!(replies[2].1.entries[0].ordinal, 1);
        assert_eq!(replies[2].1.entries[0].count, 100);
        assert_eq!(replies[3].1.total, 0);
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_loop_move_receipts WHERE depot_town=0 AND depot_box=17",
        )
        .fetch_one(&harness.pool)
        .await?;
        assert_eq!(receipts, 2);
        let counter:(String,i32)=sqlx::query_as("SELECT next_ordinal::text,committed FROM game_character_depot_box_counters WHERE town_id=0 AND box_number=17").fetch_one(&harness.pool).await?;
        assert_eq!(counter, ("2".into(), 0));
        let reloaded = harness
            .root
            .read_character_backpack(&authority, fence.character_id)
            .await
            .map_err(debug)?
            .ok_or("backpack reload")?;
        assert_eq!(reloaded.entries.len(), 1);
        assert_eq!(reloaded.entries[0].item.quantity, 100);
        assert_eq!(
            reloaded.entries[0].item.item_instance_id,
            carried.ok_or("carried")?
        );
        // Simulate loss of the connection's outcome cache: replay is durable before handles.
        let replay = owner
            .depot(
                deps.actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 2,
                    item_fence: Some(fence),
                },
                deposit,
                None,
            )
            .await
            .ok_or("durable replay")?
            .reply;
        assert_eq!(replay.outcome, DepotOutcome::Moved);
        // Reusing an ordinal after withdrawal cannot move another item; a wrong town fails closed.
        let wrong = owner
            .depot(
                deps.actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 11,
                    item_fence: Some(fence),
                },
                DepotIntent {
                    town: Some(1),
                    ..browse
                },
                None,
            )
            .await
            .ok_or("town refusal")?
            .reply;
        assert_eq!(wrong.outcome, DepotOutcome::Stale);
        let replay_command = UseCommand {
            game_session_id: session,
            command_id: 2,
            item_fence: Some(fence),
        };
        let changed = owner
            .depot(
                deps.actor,
                replay_command,
                DepotIntent {
                    box_number: 1,
                    ..deposit
                },
                None,
            )
            .await
            .ok_or("conflicting replay")?
            .reply;
        assert_eq!(changed.outcome, DepotOutcome::Rejected);
        let stale = crate::durability::item_transfer::CurrentCharacterItemFence {
            connection_generation: crate::foundation::ConnectionGeneration::new(2)
                .map_err(debug)?,
            ..fence
        };
        let refused = owner
            .depot(
                deps.actor,
                UseCommand {
                    item_fence: Some(stale),
                    command_id: 12,
                    ..replay_command
                },
                deposit,
                Some(ItemKey::Instance(carried.ok_or("carried")?)),
            )
            .await
            .ok_or("stale fence")?
            .reply;
        assert_eq!(refused.outcome, DepotOutcome::Rejected);
        // The native page preserves exact identities from storage; neither the wire
        // ordinal nor its definition can be used as an instance identity. In particular,
        // this second deposit gets a new ordinal while keeping the same ItemInstance.
        let observation = owner
            .depot(
                deps.actor,
                UseCommand {
                    command_id: 14,
                    ..replay_command
                },
                deposit,
                Some(ItemKey::Instance(carried.ok_or("carried")?)),
            )
            .await
            .ok_or("selection deposit")?;
        assert_eq!(observation.reply.outcome, DepotOutcome::Moved);
        assert_eq!(observation.items, vec![(2, carried.ok_or("carried")?)]);
        assert_eq!(observation.reply.entries[0].ordinal, observation.items[0].0);
        let observation = owner
            .depot(
                deps.actor,
                UseCommand {
                    command_id: 15,
                    ..replay_command
                },
                browse,
                None,
            )
            .await
            .ok_or("selection browse")?;
        assert_eq!(observation.reply.outcome, DepotOutcome::Opened);
        assert_eq!(observation.items, vec![(2, carried.ok_or("carried")?)]);
        // Restore custody through the fenced owner; an empty current page retains no IDs.
        let observation = owner
            .depot(
                deps.actor,
                UseCommand {
                    command_id: 16,
                    ..replay_command
                },
                DepotIntent {
                    action: DepotAction::Withdraw(2),
                    ..withdraw
                },
                None,
            )
            .await
            .ok_or("selection withdraw")?;
        assert_eq!(observation.reply.outcome, DepotOutcome::Moved);
        assert!(observation.items.is_empty());
        assert!(observation.reply.entries.is_empty());
        {
            let mut runtime = owner.runtime.lock().await;
            let mut movement = runtime.borrow_movement_position();
            for x in (2..7).rev() {
                let snapshot = movement.read(deps.actor).map_err(debug)?;
                movement
                    .commit_cardinal(snapshot, MovementLocalPosition { x, y: 0, floor: 7 })
                    .map_err(debug)?;
            }
        }
        let far = owner
            .depot(
                deps.actor,
                UseCommand {
                    command_id: 13,
                    ..replay_command
                },
                browse,
                None,
            )
            .await
            .ok_or("reach refusal")?
            .reply;
        assert_eq!(far.outcome, DepotOutcome::NoLocker);
        let replay = owner
            .depot(deps.actor, replay_command, deposit, None)
            .await
            .ok_or("replay outside locker")?;
        assert!(replay.items.is_empty());
        let replay = replay.reply;
        assert_eq!(replay.outcome, DepotOutcome::Moved);
        assert_eq!(
            replay.town, None,
            "terminal replay grants no open locker view"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
