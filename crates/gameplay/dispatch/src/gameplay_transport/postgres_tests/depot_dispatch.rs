//! Real admitted connection -> depot/stash dispatch -> fenced item owner -> PostgreSQL.
#[path = "depot_dispatch/stash_cases.rs"]
mod stash_cases;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::item_mint::{
    GroundPlacement, ItemMintCause, ItemMintOutcome, ItemMintRequest,
};
use crate::durability::item_transfer::{ItemTransferDestination, ItemTransferRequest};
use crate::foundation::{ChannelRuntimeV1, CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::gameplay_transport::item_ref_admission::{NativeItemWeights, generation_item_facts};
use crate::gameplay_transport::item_view::{ItemKey, ItemViewContinuity, SessionItemView};
use crate::map::boot::{BootPins, boot, tests as bundle_fixture};
use crate::map::facts::ItemDefinition;
use oteryn_protocol_oteryn::depot::*;

#[test]
fn depot_session_dispatch_opens_deposits_browses_withdraws_and_replays() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(false).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "depot e2e panicked")?
        .map_err(Into::into)
}

#[test]
fn stash_session_dispatch_conserves_and_replays() -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario(true).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "stash e2e panicked")?
        .map_err(Into::into)
}

fn scenario(stash: bool) -> TestResult {
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
        let (consumer, clock, activation, _) =
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
        for ordinal in 0..=if stash { 3 } else { 1 } {
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
                        item: if ordinal == 0 {
                            backpack_definition.clone()
                        } else {
                            coin_definition.clone()
                        },
                        quantity: if ordinal == 0 { 1 } else { 100 },
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
                        item: if ordinal == 0 {
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
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 18, 37])
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
        if stash {
            use oteryn_protocol_oteryn::stash::{StashAction, StashIntent, StashOutcome};
            let free = owner
                .stash(
                    deps.actor,
                    UseCommand {
                        game_session_id: session,
                        command_id: 900,
                        item_fence: Some(fence),
                    },
                    StashIntent {
                        page: 0,
                        action: StashAction::Stow(handle),
                    },
                    Some(ItemKey::Instance(carried.ok_or("source")?)),
                    true,
                )
                .await
                .ok_or("free refusal")?;
            assert_eq!(free.outcome, StashOutcome::PremiumRequired);
            owner.premium_coordinator = Some(&premium);
            owner.admit_premium(id(40), session);
            assert!(owner.session_premium_gate(session).await.premium_current());
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
            (4, withdraw),
            (5, withdraw),
        ] {
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
        let (end, output) =
            super::weather_wire::fixture_access::serve_input(&owner, admitted, &input).await?;
        let mut replies = Vec::new();
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
                replies.push((
                    result.status,
                    decode_depot_reply(result.payload).map_err(debug)?,
                ));
            }
        }
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
            .ok_or("durable replay")?;
        assert_eq!(replay.outcome, DepotOutcome::Moved);
        // Reusing an ordinal after withdrawal cannot move another item; a wrong town fails closed.
        let wrong = owner
            .depot(
                deps.actor,
                UseCommand {
                    game_session_id: session,
                    command_id: 6,
                    item_fence: Some(fence),
                },
                DepotIntent {
                    town: Some(1),
                    ..browse
                },
                None,
            )
            .await
            .ok_or("town refusal")?;
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
            .ok_or("conflicting replay")?;
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
                    command_id: 7,
                    ..replay_command
                },
                deposit,
                Some(ItemKey::Instance(carried.ok_or("carried")?)),
            )
            .await
            .ok_or("stale fence")?;
        assert_eq!(refused.outcome, DepotOutcome::Rejected);
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
                    command_id: 8,
                    ..replay_command
                },
                browse,
                None,
            )
            .await
            .ok_or("reach refusal")?;
        assert_eq!(far.outcome, DepotOutcome::NoLocker);
        let replay = owner
            .depot(deps.actor, replay_command, deposit, None)
            .await
            .ok_or("replay outside locker")?;
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
