//! QUICKLOOT: production router, item owner, replay and real PostgreSQL custody.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::item_mint::*;
use crate::foundation::{CommandStatus, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use oteryn_protocol_oteryn::quick_loot::*;

#[test]
fn quick_loot_postgres_dispatch_assignment_transfer_replay_and_fences() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("quick-loot", scenario)
}

fn request(action: QuickLootAction) -> QuickLootRequest {
    QuickLootRequest {
        action,
        corpse: None,
        category: None,
        recipient: None,
        recipient_id: None,
        fallback_to_main: false,
        filter_mode: None,
        filter_definition_ref: None,
    }
}

#[test]
fn quick_loot_pouch_postgres_dispatch_unlimited_slots_coin_only_and_weight() -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("quick-pouch", || scenario_with_pouch(true))
}

fn scenario() -> TestResult {
    scenario_with_pouch(false)
}

fn scenario_with_pouch(pouch: bool) -> TestResult {
    run(async |admin| {
        let harness = Harness::create_with_admission_seed(admin, "quick_loot", true, false).await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
        let generation = controller.active().ok_or("active")?;
        let room = pin.qualified_room().clone();
        let frame_digest = pin.frame_binding().digest();
        let deps = Dependencies::with_content(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let (consumer, clock, activation, _) =
            crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(40));
        harness
            .root
            .latch_premium_activation(
                &activation.activation_id,
                activation.switch_over_us,
                activation.switch_over_us + 60_000_000,
            )
            .await
            .map_err(debug)?;
        let premium =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(harness.root.clone()),
            );
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(generation);
        deps.initialize(&owner).await;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice()).bind(generation.identity().client_artifact_digest().as_slice()).bind(frame_digest.as_slice()).execute(&harness.pool).await?;
        // Fixture provisioning only. All actions below travel through production owners.
        {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            for seed in [70, 71] {
                sqlx::raw_sql(sqlx::AssertSqlSafe(
                    crate::durability::item_fee_burn::test_support::item(
                        seed,
                        if pouch && seed == 71 {
                            "oteryn:item.tibia.i23721"
                        } else {
                            "oteryn:item.tibia.i2854"
                        },
                        "definition-r1",
                        1,
                    ),
                ))
                .execute(&mut *tx)
                .await?;
            }
            sqlx::query("INSERT INTO game_item_container_slots(character_id,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($2,'hex')::uuid)").bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
            if pouch {
                sqlx::query("INSERT INTO game_item_character_inbox_locations(character_id,item_instance_id,world_id,ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1)").bind(id(41).as_slice()).bind(id(71).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
                for seed in 100..121 {
                    sqlx::raw_sql(sqlx::AssertSqlSafe(
                        crate::durability::item_fee_burn::test_support::item(
                            seed,
                            "oteryn:item.tibia.i3031",
                            "definition-r1",
                            100,
                        ),
                    ))
                    .execute(&mut *tx)
                    .await?;
                    sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,encode($1,'hex')::uuid)").bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(id(71).as_slice()).bind(i64::from(seed-99)).execute(&mut *tx).await?;
                }
            } else {
                sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,encode($3,'hex')::uuid)").bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(71).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
            }
            tx.commit().await?;
        }
        let typed = |key: &str| TypedDefinitionRef {
            family: "Item".into(),
            production_key: key.into(),
            revision_ref: "definition-r1".into(),
        };
        let cause = |purpose: &str, ordinal: u32| {
            ItemMintCause::for_test(
                world,
                deps.admitted.channel_id,
                fence.scope_ownership_generation,
                99,
                1,
                TypedDefinitionRef {
                    family: "LootTable".into(),
                    production_key: "fixture:loot.quick".into(),
                    revision_ref: "definition-r1".into(),
                },
                purpose.into(),
                ordinal,
            )
        };
        let mut corpse = harness
            .root
            .freeze_item_mint(
                &authority,
                &harness.node,
                ItemMintRequest {
                    cause: cause(CORPSE_MATERIALIZATION_PURPOSE_KEY, 0),
                    item: typed("oteryn:item.tibia.i5973"),
                    quantity: 1,
                    ground: GroundPlacement {
                        spatial_position:
                            crate::durability::map_item_mint_audit::map_item_spatial_position(
                                7, 0, 7,
                            ),
                        corpse_ref: id(99).to_vec(),
                        map_revision: "map-1".into(),
                        content_revision: "content-1".into(),
                        native_room_placement_context: id(6).to_vec(),
                    },
                    content_revision: "content-1".into(),
                    ruleset_revision: "ruleset-1".into(),
                    sim_revision: "sim-1".into(),
                },
            )
            .await
            .map_err(debug)?;
        let permit = owner.spell_lane_permit().await.ok_or("permit")?;
        let corpse_id = match harness
            .root
            .commit_corpse_mint(&permit, &authority, &harness.node, &mut corpse, id(41))
            .await
            .map_err(debug)?
        {
            ItemMintOutcome::Committed(value) => value.item_instance_id,
            other => return Err(debug(other)),
        };
        let mut items = Vec::new();
        for ordinal in 1..=if pouch { 3 } else { 2 } {
            let mut entry = harness
                .root
                .freeze_corpse_loot_mint(
                    &authority,
                    &harness.node,
                    CorpseLootMintRequest {
                        cause: cause("fixture:purpose.quick", ordinal),
                        item: typed(if ordinal == 3 {
                            "oteryn:item.tibia.i3577"
                        } else {
                            "oteryn:item.tibia.i3031"
                        }),
                        quantity: if ordinal == 3 { 1 } else { 60 },
                        placement: CorpseContainerPlacement {
                            corpse_item_instance_id: corpse_id,
                            placement_ordinal: ordinal,
                        },
                        content_revision: "content-1".into(),
                        ruleset_revision: "ruleset-1".into(),
                        sim_revision: "sim-1".into(),
                    },
                )
                .await
                .map_err(debug)?;
            match harness
                .root
                .commit_corpse_loot_mint(&permit, &authority, &harness.node, &mut entry)
                .await
                .map_err(debug)?
            {
                ItemMintOutcome::Committed(value) => items.push(value.item_instance_id),
                other => return Err(debug(other)),
            }
        }
        drop(permit);
        let assign = QuickLootRequest {
            recipient: Some(if pouch {
                QuickLootRecipient::GoldPouch
            } else {
                QuickLootRecipient::Container
            }),
            recipient_id: Some(id(71)),
            ..request(QuickLootAction::Assign)
        };
        let all = request(QuickLootAction::LootAll);
        let mut admitted = deps.admitted;
        admitted.item_fence = Some(fence);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 98])
                .ok_or("capabilities")?;
        assert!(admitted.continuity.selected_capabilities.contains(98));
        assert!(
            crate::gameplay_transport::equipment_move::native_for_move(
                &*deps.runtime.lock().await,
                Some(generation),
                false,
                false
            )
            .is_some(),
            "pinned move content"
        );
        let mut plain = admitted;
        plain.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), &[6])
                .ok_or("plain capabilities")?;
        assert_eq!(
            wire_requests(&owner, &mut plain, &[(1, all)]).await?,
            vec![(
                CommandStatus::Rejected,
                QuickLootResult { items: Vec::new() }
            )],
            "unselected capability cannot produce a successful mechanic result"
        );
        let selected = admitted.continuity.selected_capabilities;
        admitted.continuity = plain.continuity;
        admitted.continuity.selected_capabilities = selected;
        assert_eq!(
            wire_requests(
                &owner,
                &mut admitted,
                &[(
                    2,
                    QuickLootRequest {
                        category: Some(QuickLootCategory::Gold),
                        ..assign
                    }
                )]
            )
            .await?[0]
                .0,
            CommandStatus::Rejected,
            "free account cannot assign categories"
        );
        let click = QuickLootRequest {
            corpse: Some(corpse_id),
            ..request(QuickLootAction::Click)
        };
        let near: [u8; 10] =
            crate::durability::map_item_mint_audit::map_item_spatial_position(7, 0, 7)
                .try_into()
                .map_err(|_| "near cell")?;
        let far: [u8; 10] =
            crate::durability::map_item_mint_audit::map_item_spatial_position(9, 0, 7)
                .try_into()
                .map_err(|_| "far cell")?;
        fixture_corpse(&harness.pool, corpse_id, far, id(41), 0).await?;
        assert_eq!(
            wire_requests(&owner, &mut admitted, &[(3, click)]).await?[0],
            (
                CommandStatus::Accepted,
                QuickLootResult { items: Vec::new() }
            ),
            "Click cannot reach a corpse two tiles away"
        );
        fixture_corpse(&harness.pool, corpse_id, near, id(99), 0).await?;
        let denied = wire_requests(&owner, &mut admitted, &[(4, click)]).await?;
        assert_eq!(denied[0].0, CommandStatus::Accepted);
        assert_eq!(denied[0].1.items.len(), items.len());
        assert!(
            denied[0].1.items.iter().all(|item| !item.accepted),
            "exclusive rights refuse every item without changing custody"
        );
        fixture_corpse(
            &harness.pool,
            corpse_id,
            near,
            if pouch { id(99) } else { id(41) },
            if pouch { 11_000 } else { 0 },
        )
        .await?;
        if !pouch {
            let reference = generation
                .native_gameplay()
                .and_then(|n| n.item_index())
                .and_then(|i| i.definition_ref("Item", "oteryn:item.tibia.i3031", "definition-r1"))
                .ok_or("coin definition ref")?
                .get();
            let filter = |action, mode| QuickLootRequest {
                filter_mode: Some(mode),
                filter_definition_ref: if matches!(
                    action,
                    QuickLootAction::FilterAdd | QuickLootAction::FilterRemove
                ) {
                    Some(reference)
                } else {
                    None
                },
                ..request(action)
            };
            let outcomes = wire_requests(
                &owner,
                &mut admitted,
                &[
                    (
                        5,
                        filter(QuickLootAction::FilterAdd, QuickLootFilterMode::Skip),
                    ),
                    (6, all),
                    (
                        7,
                        filter(QuickLootAction::FilterRemove, QuickLootFilterMode::Skip),
                    ),
                    (
                        8,
                        filter(QuickLootAction::FilterAdd, QuickLootFilterMode::Accept),
                    ),
                    (
                        9,
                        filter(QuickLootAction::FilterSelect, QuickLootFilterMode::Accept),
                    ),
                    (
                        10,
                        filter(QuickLootAction::FilterClear, QuickLootFilterMode::Accept),
                    ),
                    (11, all),
                ],
            )
            .await?;
            assert_eq!(outcomes.len(), 7);
            assert!(outcomes.iter().all(|r| r.0 == CommandStatus::Accepted));
            for index in [1, 6] {
                assert_eq!(outcomes[index].1.items.len(), items.len());
                assert!(
                    outcomes[index].1.items.iter().all(|item| !item.accepted),
                    "Loot All respects persisted skip/accept lists"
                );
            }
        }
        let loot_request = if pouch { all } else { click };
        if pouch {
            owner.premium_coordinator = Some(&premium);
            owner.admit_premium(id(40), admitted.game_session_id);
            assert!(
                owner
                    .session_premium_gate(admitted.game_session_id)
                    .await
                    .premium_current()
            );
            assert_eq!(
                wire_requests(&owner, &mut admitted, &[(5, assign)]).await?[0].0,
                CommandStatus::Accepted
            );
            fixture_overload_pouch(&harness.pool, true).await?;
            let heavy = wire_requests(&owner, &mut admitted, &[(6, all)]).await?;
            assert_eq!(heavy[0].0, CommandStatus::Accepted);
            assert_eq!(heavy[0].1.items.len(), items.len());
            assert!(
                heavy[0].1.items.iter().all(|item| !item.accepted),
                "coin pouch contents still count against capacity"
            );
            fixture_overload_pouch(&harness.pool, false).await?;
        }
        let assignment_id = admitted.continuity.next_command_id;
        let loot_id = assignment_id + 1;
        let replies = wire_requests(
            &owner,
            &mut admitted,
            &[
                (
                    assignment_id,
                    QuickLootRequest {
                        category: if pouch {
                            Some(QuickLootCategory::Gold)
                        } else {
                            None
                        },
                        ..assign
                    },
                ),
                (loot_id, loot_request),
            ],
        )
        .await?;
        assert_eq!(replies.len(), 2);
        assert_eq!(
            replies[0],
            (
                CommandStatus::Accepted,
                QuickLootResult { items: Vec::new() }
            )
        );
        assert_eq!(replies[1].0, CommandStatus::Accepted);
        assert_eq!(
            replies[1].1.items,
            items
                .iter()
                .enumerate()
                .map(|(index, item)| QuickLootItemResult {
                    item: *item,
                    accepted: index < 2
                })
                .collect::<Vec<_>>()
        );
        let quantities:Vec<i64>=sqlx::query_scalar("SELECT i.quantity FROM game_item_nested_entries e JOIN game_item_instances i USING(item_instance_id) WHERE e.parent_item_instance_id=encode($1,'hex')::uuid ORDER BY i.quantity").bind(id(71).as_slice()).fetch_all(&harness.pool).await?;
        assert_eq!(
            quantities,
            if pouch {
                let mut counts = vec![20];
                counts.extend([100; 22]);
                counts
            } else {
                vec![20, 100]
            },
            "batch conserves and merges both stacks"
        );
        let command = UseCommand {
            game_session_id: admitted.game_session_id,
            command_id: loot_id,
            item_fence: Some(fence),
        };
        assert_eq!(
            owner.quick_loot(admitted, command, loot_request).await,
            Some(replies[1].clone()),
            "lost acknowledgement replays exact outcome"
        );
        assert_eq!(
            owner
                .quick_loot(admitted, command, assign)
                .await
                .map(|r| r.0),
            Some(CommandStatus::Rejected),
            "changed intent conflicts"
        );
        if !pouch {
            assert_eq!(
                owner
                    .quick_loot(
                        admitted,
                        UseCommand {
                            command_id: 40,
                            ..command
                        },
                        QuickLootRequest {
                            category: Some(QuickLootCategory::Gold),
                            ..assign
                        }
                    )
                    .await
                    .map(|r| r.0),
                Some(CommandStatus::Rejected),
                "category sorting requires Premium"
            );
        }
        let stale = crate::durability::item_transfer::CurrentCharacterItemFence {
            connection_generation: crate::foundation::ConnectionGeneration::new(
                fence.connection_generation.get() + 1,
            )
            .map_err(debug)?,
            ..fence
        };
        assert_eq!(
            owner
                .quick_loot(
                    admitted,
                    UseCommand {
                        command_id: 9,
                        item_fence: Some(stale),
                        ..command
                    },
                    all
                )
                .await
                .map(|r| r.0),
            Some(CommandStatus::Rejected)
        );
        let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quick_loot_receipts")
            .fetch_one(&harness.pool)
            .await?;
        assert_eq!(
            receipts,
            if pouch { 6 } else { 11 },
            "stale and conflicting commands write nothing"
        );
        if !pouch {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO game_character_quick_loot_filters(character_id,accepted,item_production_key) SELECT encode($1,'hex')::uuid,false,'oteryn:item.tibia.i'||(500000+n)::text FROM generate_series(1,2048) n").bind(id(41).as_slice()).execute(&mut *tx).await?;
            tx.commit().await?;
            let reference = generation
                .native_gameplay()
                .and_then(|n| n.item_index())
                .and_then(|i| i.definition_ref("Item", "oteryn:item.tibia.i3031", "definition-r1"))
                .ok_or("coin ref")?
                .get();
            let id = admitted.continuity.next_command_id;
            let result = wire_requests(
                &owner,
                &mut admitted,
                &[(
                    id,
                    QuickLootRequest {
                        filter_mode: Some(QuickLootFilterMode::Skip),
                        filter_definition_ref: Some(reference),
                        ..request(QuickLootAction::FilterAdd)
                    },
                )],
            )
            .await?;
            assert_eq!(
                result[0].0,
                CommandStatus::Rejected,
                "max+1 filter definition is refused"
            );
            let count: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_quick_loot_filters")
                    .fetch_one(&harness.pool)
                    .await?;
            assert_eq!(
                count, 2048,
                "full list reloads without truncation or mutation"
            );
            equip_quiver_via_wire(&owner, admitted, &harness.pool).await?;
        }
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn wire_requests(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    requests: &[(u64, QuickLootRequest)],
) -> TestResult<Vec<(CommandStatus, QuickLootResult)>> {
    let mut input = Vec::new();
    for &(command_id, req) in requests {
        let payload = encode_quick_loot_request(&req).map_err(debug)?;
        let frame = oteryn_protocol_oteryn::encode_client_command(
            1,
            &oteryn_protocol_oteryn::ClientCommandValue {
                command_id,
                command_type: COMMAND_TYPE_QUICK_LOOT,
                payload: &payload,
            },
        )
        .map_err(debug)?;
        input.extend(u32::try_from(frame.len())?.to_be_bytes());
        input.extend(frame);
    }
    let output = serve_session(owner, admitted, &input).await?;
    let mut replies = Vec::new();
    let mut cursor = output.as_slice();
    while !cursor.is_empty() {
        let length = u32::from_be_bytes(cursor.get(..4).ok_or("length")?.try_into()?) as usize;
        let frame = cursor.get(4..4 + length).ok_or("frame")?;
        cursor = &cursor[4 + length..];
        let envelope = oteryn_protocol_oteryn::decode_wire_envelope(frame).map_err(debug)?;
        if envelope.message_type() == oteryn_protocol_oteryn::MessageType::CommandResult {
            let reply =
                oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
            replies.push((
                reply.status,
                decode_quick_loot_result(reply.payload).map_err(debug)?,
            ));
        }
    }
    Ok(replies)
}

// Fixture provisioning changes one independently checked reach/rights invariant at a time.
async fn fixture_corpse(
    pool: &sqlx::PgPool,
    corpse: [u8; 16],
    cell: [u8; 10],
    top: [u8; 16],
    age: i64,
) -> TestResult {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_item_ground_locations SET spatial_position=$2 WHERE item_instance_id=encode($1,'hex')::uuid").bind(corpse.as_slice()).bind(cell.as_slice()).execute(&mut *tx).await?;
    sqlx::query("UPDATE game_item_mint_receipts SET corpse_top_damage_character_id=encode($2,'hex')::uuid,materialized_at=floor(extract(epoch FROM clock_timestamp())*1000)::bigint-$3 WHERE item_instance_id=encode($1,'hex')::uuid").bind(corpse.as_slice()).bind(top.as_slice()).bind(age).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn equip_quiver_via_wire(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: crate::gameplay_transport::connection::AdmittedSession,
    pool: &sqlx::PgPool,
) -> TestResult {
    use oteryn_protocol_oteryn::item_view::*;
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (seed, key, count) in [
        (72, "oteryn:item.tibia.i35562", 1),
        (73, "oteryn:item.tibia.i3447", 1),
    ] {
        sqlx::raw_sql(sqlx::AssertSqlSafe(
            crate::durability::item_fee_burn::test_support::item(seed, key, "definition-r1", count),
        ))
        .execute(&mut *tx)
        .await?;
    }
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,2,encode($3,'hex')::uuid)").bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(72).as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($1,'hex')::uuid)").bind(id(73).as_slice()).bind(id(42).as_slice()).bind(id(72).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &[4, 6, 12],
    )
    .ok_or("equip capabilities")?;
    assert!(admitted.continuity.selected_capabilities.contains(12));
    let command_id = admitted.continuity.next_command_id;
    let handle = admitted.continuity.item_view.handle_counter + 2;
    // Fresh handles above the retained counter: backpack first, newest entry (quiver) next.
    let payload = encode_item_move_intent_for(
        &ItemMoveIntent {
            source: std::num::NonZeroU64::new(handle).ok_or("quiver handle")?,
            destination: ItemMoveDestination::Equipment(EquipmentSlot::Ammo),
        },
        ItemMoveSelection {
            equip_drop: true,
            container_tree: false,
            world_interaction: false,
        },
    )
    .map_err(debug)?;
    let frame = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id,
            command_type: 9,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    let mut input = u32::try_from(frame.len())?.to_be_bytes().to_vec();
    input.extend(frame);
    let output = serve_session(owner, &mut admitted, &input).await?;
    let mut cursor = output.as_slice();
    let mut moved = false;
    while !cursor.is_empty() {
        let length =
            u32::from_be_bytes(cursor.get(..4).ok_or("equip length")?.try_into()?) as usize;
        let envelope = oteryn_protocol_oteryn::decode_wire_envelope(
            cursor.get(4..4 + length).ok_or("equip frame")?,
        )
        .map_err(debug)?;
        cursor = &cursor[4 + length..];
        if envelope.message_type() == oteryn_protocol_oteryn::MessageType::CommandResult {
            let reply =
                oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(
                reply.status,
                CommandStatus::Accepted,
                "equip command result"
            );
            assert_eq!(
                decode_item_move_result_for(
                    reply.payload,
                    ItemMoveSelection {
                        equip_drop: true,
                        container_tree: false,
                        world_interaction: false
                    }
                )
                .map_err(debug)?,
                ItemMoveOutcome::Moved
            );
            moved = true;
        }
    }
    assert!(moved, "populated quiver equips through serve_admitted");
    let slot:i16=sqlx::query_scalar("SELECT slot FROM game_character_equipment_slots WHERE item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(pool).await?;
    assert_eq!(slot, 10);

    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (seed, key, count, ordinal) in [
        (74, "oteryn:item.tibia.i3577", 1, 3_i64),
        (75, "oteryn:item.tibia.i3447", 100, 4_i64),
    ] {
        sqlx::raw_sql(sqlx::AssertSqlSafe(
            crate::durability::item_fee_burn::test_support::item(seed, key, "definition-r1", count),
        ))
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,encode($3,'hex')::uuid)").bind(id(41).as_slice()).bind(id(70).as_slice()).bind(id(seed).as_slice()).bind(id(42).as_slice()).bind(ordinal).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
        &[4, 6, 12, 14],
    )
    .ok_or("quiver insertion capabilities")?;
    let selection = ItemMoveSelection {
        equip_drop: true,
        container_tree: true,
        world_interaction: false,
    };
    // Fresh handles: main, newest ammo, meat, assigned bag, equipped quiver.
    let base = admitted.continuity.item_view.handle_counter;
    assert_eq!(
        wire_move(
            owner,
            &mut admitted,
            ItemMoveIntent {
                source: std::num::NonZeroU64::new(base + 3).ok_or("meat handle")?,
                destination: ItemMoveDestination::Container(
                    std::num::NonZeroU64::new(base + 5).ok_or("quiver handle")?
                )
            },
            selection
        )
        .await?,
        ItemMoveOutcome::NotSupported
    );
    let base = admitted.continuity.item_view.handle_counter;
    assert_eq!(
        wire_move(
            owner,
            &mut admitted,
            ItemMoveIntent {
                source: std::num::NonZeroU64::new(base + 2).ok_or("ammo handle")?,
                destination: ItemMoveDestination::Container(
                    std::num::NonZeroU64::new(base + 5).ok_or("quiver handle")?
                )
            },
            selection
        )
        .await?,
        ItemMoveOutcome::Moved
    );
    let quantity:i64=sqlx::query_scalar("SELECT sum(i.quantity)::bigint FROM game_item_nested_entries n JOIN game_item_instances i USING(item_instance_id) WHERE n.parent_item_instance_id=encode($1,'hex')::uuid").bind(id(72).as_slice()).fetch_one(pool).await?;
    assert_eq!(
        quantity, 101,
        "ammo-only insertion merges and conserves every arrow"
    );
    Ok(())
}

async fn fixture_overload_pouch(pool: &sqlx::PgPool, add: bool) -> TestResult {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    if add {
        let (level,vocation):(i64,Option<String>)=sqlx::query_as("SELECT p.level,b.vocation FROM game_character_progression_state p LEFT JOIN game_character_build_state b USING(character_id) WHERE p.character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&mut *tx).await?;
        let capacity = oteryn_gameplay_items::item_capacity::project_global_capacity_centi_oz(
            u32::try_from(level)?,
            vocation.as_deref().unwrap_or("none"),
        )
        .map_err(debug)?;
        let rows = i64::try_from(capacity / 1000 + 1)?;
        assert!(rows < 50_000, "bounded fixture capacity");
        sqlx::query("WITH minted AS (INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) SELECT game_character_uuid_v7(),encode($1,'hex')::uuid,'Item','oteryn:item.tibia.i3031','definition-r1',100,1,game_character_uuid_v7() FROM generate_series(1,$3::bigint) RETURNING item_instance_id) INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT item_instance_id,encode($1,'hex')::uuid,encode($2,'hex')::uuid,1000+row_number() OVER(),item_instance_id FROM minted").bind(id(42).as_slice()).bind(id(71).as_slice()).bind(rows).execute(&mut *tx).await?;
    } else {
        sqlx::query("WITH removed AS (DELETE FROM game_item_nested_entries WHERE parent_item_instance_id=encode($1,'hex')::uuid AND placement_ordinal>1000 RETURNING item_instance_id) DELETE FROM game_item_instances WHERE item_instance_id IN(SELECT item_instance_id FROM removed)").bind(id(71).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

async fn serve_session(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    input: &[u8],
) -> TestResult<Vec<u8>> {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client.write_all(input).await?;
    client.shutdown().await?;
    let end = crate::gameplay_transport::connection::serve_admitted(
        &mut server,
        *admitted,
        owner,
        crate::gameplay_transport::connection::IDLE_LIVENESS,
    )
    .await;
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    match end {
        crate::gameplay_transport::connection::ConnectionEnd::AdmittedThenDisconnected(next) => {
            *admitted = next
        }
        other => return Err(format!("wire session ended: {other:?}").into()),
    }
    Ok(output)
}

async fn wire_move(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    intent: oteryn_protocol_oteryn::item_view::ItemMoveIntent,
    selection: oteryn_protocol_oteryn::item_view::ItemMoveSelection,
) -> TestResult<oteryn_protocol_oteryn::item_view::ItemMoveOutcome> {
    use oteryn_protocol_oteryn::item_view::*;
    let command_id = admitted.continuity.next_command_id;
    let payload = encode_item_move_intent_for(&intent, selection).map_err(debug)?;
    let frame = oteryn_protocol_oteryn::encode_client_command(
        1,
        &oteryn_protocol_oteryn::ClientCommandValue {
            command_id,
            command_type: 9,
            payload: &payload,
        },
    )
    .map_err(debug)?;
    let mut input = u32::try_from(frame.len())?.to_be_bytes().to_vec();
    input.extend(frame);
    let output = serve_session(owner, admitted, &input).await?;
    let mut cursor = output.as_slice();
    let mut result = None;
    while !cursor.is_empty() {
        let length = u32::from_be_bytes(cursor.get(..4).ok_or("move length")?.try_into()?) as usize;
        let envelope = oteryn_protocol_oteryn::decode_wire_envelope(
            cursor.get(4..4 + length).ok_or("move frame")?,
        )
        .map_err(debug)?;
        cursor = &cursor[4 + length..];
        if envelope.message_type() == oteryn_protocol_oteryn::MessageType::CommandResult {
            let reply =
                oteryn_protocol_oteryn::decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(reply.status, CommandStatus::Accepted);
            assert!(result.is_none());
            result = Some(decode_item_move_result_for(reply.payload, selection).map_err(debug)?);
        }
    }
    result.ok_or_else(|| "command 9 did not return a result".into())
}
