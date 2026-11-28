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
    super::kill_reward_e2e_fixtures::scenario_thread::run("quick-loot", scenario)
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
    super::kill_reward_e2e_fixtures::scenario_thread::run("quick-pouch", || {
        scenario_with_loot(LootScenario::Pouch)
    })
}

fn scenario() -> TestResult {
    scenario_with_loot(LootScenario::Ordinary)
}

#[test]
fn quick_loot_postgres_dispatch_nested_corpse_items_keep_rights_order_and_custody() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("quick-loot-nested", || {
        scenario_with_loot(LootScenario::Nested)
    })
}

#[test]
fn quick_loot_postgres_clock_retires_skipped_nested_corpse_contents() -> TestResult {
    super::kill_reward_e2e_fixtures::scenario_thread::run("quick-loot-nested-decay", || {
        scenario_with_loot(LootScenario::NestedDecay)
    })
}

#[derive(Clone, Copy)]
enum LootScenario {
    Ordinary,
    Pouch,
    Nested,
    NestedDecay,
}

fn scenario_with_loot(scenario: LootScenario) -> TestResult {
    let pouch = matches!(scenario, LootScenario::Pouch);
    let nested = matches!(scenario, LootScenario::Nested | LootScenario::NestedDecay);
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
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                world,
                "content-1",
            )?;
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
        deps.runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                current.current_game_session_id(),
                current.current_character_lease(),
            )
            .map_err(debug)?;
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
        let corpse_id = *corpse.item_instance_id();
        let items: Vec<[u8; 16]> = if nested {
            let specs = [
                (None, 1, "oteryn:item.tibia.i3031", 60),
                (None, 2, "oteryn:item.tibia.i2854", 1),
                (Some(1), 1, "oteryn:item.tibia.i2854", 1),
                (Some(2), 1, "oteryn:item.tibia.i3031", 60),
            ];
            let mut children: Vec<ItemMintCandidate> = Vec::with_capacity(specs.len());
            for (index, (parent, ordinal, key, quantity)) in specs.into_iter().enumerate() {
                let parent = parent.map_or(corpse_id, |index: usize| {
                    *children[index].item_instance_id()
                });
                let entry = harness
                    .root
                    .freeze_kill_loot_entry(
                        &authority,
                        &harness.node,
                        CorpseLootMintRequest {
                            cause: cause("fixture:purpose.quick", u32::try_from(index + 1)?),
                            item: typed(key),
                            quantity,
                            placement: CorpseContainerPlacement {
                                corpse_item_instance_id: parent,
                                placement_ordinal: ordinal,
                                fluid_subtype: None,
                            },
                            content_revision: "content-1".into(),
                            ruleset_revision: "ruleset-1".into(),
                            sim_revision: "sim-1".into(),
                        },
                    )
                    .await
                    .map_err(debug)?;
                children.push(entry);
            }
            let outcomes = harness
                .root
                .commit_kill_loot_mints(
                    &permit,
                    &authority,
                    &harness.node,
                    &mut corpse,
                    &mut children,
                    id(41),
                    None,
                    None,
                    None,
                    None,
                )
                .await
                .map_err(debug)?;
            assert_eq!(outcomes.len(), children.len() + 1);
            assert!(
                outcomes
                    .iter()
                    .all(|outcome| matches!(outcome, ItemMintOutcome::Committed(_)))
            );
            children
                .iter()
                .map(|entry| *entry.item_instance_id())
                .collect()
        } else {
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
                                fluid_subtype: None,
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
            items
        };
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
        if nested {
            quick_loot_nested(
                &owner,
                &mut admitted,
                &harness.pool,
                corpse_id,
                &items,
                assign,
                matches!(scenario, LootScenario::NestedDecay),
            )
            .await?;
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
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
            let command_id = admitted.continuity.next_command_id;
            let result = wire_requests(
                &owner,
                &mut admitted,
                &[(
                    command_id,
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
            owner.premium_coordinator = Some(&premium);
            owner.admit_premium(id(40), admitted.game_session_id);
            quick_loot_destination_management(&owner, &mut admitted, &harness.pool).await?;
            equip_quiver_via_wire(&owner, admitted, &harness.pool).await?;
        }
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn quick_loot_nested(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    pool: &sqlx::PgPool,
    corpse: [u8; 16],
    items: &[[u8; 16]],
    assign: QuickLootRequest,
    decay: bool,
) -> TestResult {
    let [direct, outer, inner, nested] = items else {
        return Err("expected four loot mints".into());
    };
    sqlx::query("INSERT INTO game_character_quick_loot_filters(character_id,accepted,item_production_key) VALUES(encode($1,'hex')::uuid,false,'oteryn:item.tibia.i2854')")
        .bind(id(41).as_slice()).execute(pool).await?;

    if !decay {
        sqlx::raw_sql(include_str!(
            "../../../../../../apps/game-server/migrations/3355_quick_loot_corpse_descendants.sql"
        ))
        .execute(pool)
        .await?;
        quick_loot_postorder_bounds(pool, corpse).await?;
    }
    let near = crate::durability::map_item_mint_audit::map_item_spatial_position(7, 0, 7)
        .try_into()
        .map_err(|_| "near cell")?;
    let loot = request(QuickLootAction::LootAll);
    fixture_corpse(pool, corpse, near, id(99), 0).await?;
    let denied = wire_requests(owner, admitted, &[(1, assign), (2, loot)]).await?;
    assert_eq!(denied[0].0, CommandStatus::Accepted);
    assert_eq!(denied[1].0, CommandStatus::Accepted);
    assert!(denied[1].1.items.iter().all(|item| !item.accepted));
    assert!(
        denied[1].1.items.iter().any(|item| item.item == *nested),
        "a nested coin is reported as refused while the corpse is exclusive"
    );
    let parent: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(parent_item_instance_id) FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(nested.as_slice()).fetch_one(pool).await?;
    assert_eq!(parent.as_slice(), inner);

    fixture_corpse(pool, corpse, near, id(41), 0).await?;
    let fence = admitted.item_fence.ok_or("item fence")?;
    let schedule = owner
        .root
        .read_scoped_corpse_decay_schedule(owner.character, fence.runtime_scope)
        .await?;
    assert!(
        schedule
            .corpses
            .iter()
            .any(|entry| entry.corpse_item_instance_id == corpse)
    );

    sqlx::query("UPDATE game_character_quick_loot_filters SET item_production_key='oteryn:item.tibia.i3031' WHERE character_id=encode($1,'hex')::uuid AND NOT accepted")
        .bind(id(41).as_slice()).execute(pool).await?;
    let skipped = wire_requests(owner, admitted, &[(3, loot)]).await?;
    assert_eq!(skipped[0].0, CommandStatus::Accepted);
    assert!(
        skipped[0].1.items.iter().all(|item| !item.accepted),
        "moving a bag must not bypass its contents' filter"
    );

    if decay {
        fixture_corpse(pool, corpse, near, id(41), 60_001).await?;
        owner.drain_corpse_deadlines().await;
        let remaining = owner
            .root
            .read_scoped_corpse_decay_schedule(owner.character, fence.runtime_scope)
            .await?;
        assert!(
            remaining
                .corpses
                .iter()
                .all(|entry| entry.corpse_item_instance_id != corpse),
            "the production decay clock must remove a corpse with skipped nested loot"
        );
        let receipts: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM game_item_decay_retire_receipts
             WHERE corpse_item_instance_id=encode($1,'hex')::uuid AND cause_kind='CORPSE_DECAY'",
        )
        .bind(corpse.as_slice())
        .fetch_one(pool)
        .await?;
        assert_eq!(receipts, i64::try_from(items.len() + 1)?);
        for (item, parent) in [(nested, inner), (inner, outer)] {
            let envelope: Vec<u8> = sqlx::query_scalar(
                "SELECT a.envelope FROM game_item_decay_retire_receipts r
                 JOIN game_item_audit_outbox a USING(event_id,transaction_id)
                 WHERE r.item_instance_id=encode($1,'hex')::uuid AND r.cause_kind='CORPSE_DECAY'",
            )
            .bind(item.as_slice())
            .fetch_one(pool)
            .await?;
            let (_, retirement) =
                crate::durability::item_decay_retire_audit::decode_decay_retire_envelope(&envelope)
                    .map_err(debug)?;
            let edge = retirement
                .ground_container_entry
                .ok_or("nested retirement audit edge")?;
            assert_eq!(edge.parent_item_instance_id.as_slice(), parent);
            assert_eq!(
                edge.tree_root_item_instance_id.as_deref(),
                Some(corpse.as_slice())
            );
        }
        return Ok(());
    }

    sqlx::query("UPDATE game_character_quick_loot_filters SET item_production_key='oteryn:item.tibia.i2854' WHERE character_id=encode($1,'hex')::uuid AND NOT accepted")
        .bind(id(41).as_slice()).execute(pool).await?;
    let p = sqlx::raw_sql(
        "CREATE FUNCTION fixture_fail_nested_quick_loot() RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN
           IF NEW.command_id=21 AND EXISTS(
             SELECT 1 FROM game_item_loop_move_receipts r JOIN game_item_loop_move_lines l USING(transaction_id)
             WHERE r.game_session_id=NEW.game_session_id AND r.command_id=NEW.command_id
               AND r.quick_loot_ordinal=2 AND l.from_kind=5
           ) THEN
             RAISE EXCEPTION 'fixture abort after the nested item write';
           END IF;
           RETURN NEW;
         END $$;
         CREATE TRIGGER fixture_fail_nested_quick_loot BEFORE INSERT ON game_quick_loot_receipts
         FOR EACH ROW EXECUTE FUNCTION fixture_fail_nested_quick_loot()",
    );
    p.execute(pool).await?;
    let failed = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 21,
        item_fence: Some(fence),
    };
    assert_eq!(owner.quick_loot(*admitted, failed, loot).await, None);
    let rolled_back: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(parent_item_instance_id) FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid",
    ).bind(nested.as_slice()).fetch_one(pool).await?;
    assert_eq!(rolled_back.as_slice(), inner);
    let move_count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_item_loop_move_receipts WHERE command_id=21")
            .fetch_one(pool)
            .await?;
    assert_eq!(move_count, 0);

    let replies = wire_requests(owner, admitted, &[(4, loot)]).await?;
    assert_eq!(replies[0].0, CommandStatus::Accepted);
    assert_eq!(
        replies[0].1.items,
        [
            (*direct, true),
            (*nested, true),
            (*inner, false),
            (*outer, false)
        ]
        .map(|(item, accepted)| QuickLootItemResult { item, accepted }),
        "Loot All takes both coins and leaves the skipped bags in the corpse",
    );
    let command = UseCommand {
        game_session_id: admitted.game_session_id,
        command_id: 4,
        item_fence: Some(fence),
    };
    assert_eq!(
        owner.quick_loot(*admitted, command, loot).await,
        Some(replies[0].clone())
    );
    let corpse_items = owner
        .root
        .read_corpse_contents(owner.character, corpse)
        .await?
        .ok_or("corpse contents")?;
    assert_eq!(
        corpse_items
            .iter()
            .map(|entry| entry.item.item_instance_id)
            .collect::<Vec<_>>(),
        [*outer]
    );
    assert!(
        owner
            .root
            .read_item_source_location(owner.character, *direct)
            .await?
            .is_none()
    );
    let after = owner
        .root
        .read_scoped_corpse_decay_schedule(owner.character, fence.runtime_scope)
        .await?;
    assert_eq!(after.corpses, schedule.corpses);
    let target = owner
        .root
        .read_item_loop_container(owner.character, fence.character_id, id(71))
        .await?
        .ok_or("loot destination")?;
    assert_eq!(target.parent, Some(id(70)));
    let mut quantities: Vec<_> = target
        .entries
        .iter()
        .map(|entry| entry.item.quantity)
        .collect();
    quantities.sort_unstable();
    assert_eq!(quantities, [20, 100]);
    assert!(
        target
            .entries
            .iter()
            .any(|entry| entry.item.item_instance_id == *nested)
    );
    let proofs: Vec<(i16, i16)> = sqlx::query_as(
        "SELECT r.quick_loot_ordinal,l.from_kind
           FROM game_item_loop_move_receipts r
           JOIN game_item_loop_move_lines l USING (transaction_id)
          WHERE r.command_id=$1::text::numeric AND l.from_kind=5 ORDER BY r.quick_loot_ordinal",
    )
    .bind(command.command_id.to_string())
    .fetch_all(pool)
    .await?;
    assert_eq!(
        proofs,
        [(1, 5), (2, 5)],
        "both moves have one corpse-source proof after replay"
    );

    let parent: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(parent_item_instance_id) FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(inner.as_slice()).fetch_one(pool).await?;
    assert_eq!(parent.as_slice(), outer);
    sqlx::query(
        "DELETE FROM game_character_quick_loot_filters WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(id(41).as_slice())
    .execute(pool)
    .await?;
    let emptied = wire_requests(owner, admitted, &[(5, loot)]).await?;
    assert_eq!(emptied[0].0, CommandStatus::Accepted);
    assert_eq!(
        emptied[0].1.items,
        [*inner, *outer].map(|item| QuickLootItemResult {
            item,
            accepted: true
        }),
        "each bag moves after its last child",
    );
    Ok(())
}

// The extra rows are only a stress fixture for the query used by the command.
async fn quick_loot_postorder_bounds(pool: &sqlx::PgPool, corpse: [u8; 16]) -> TestResult {
    let mut tx = pool.begin().await?;
    sqlx::raw_sql(
        "SET LOCAL session_replication_role=replica;
         CREATE TEMP TABLE fixture_loot_tree ON COMMIT DROP AS
         SELECT n, game_character_uuid_v7() AS id, game_character_uuid_v7() AS mint_transaction
         FROM generate_series(1,1309) n;",
    )
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id)
         SELECT id,encode($1,'hex')::uuid,'Item','oteryn:item.tibia.i2854','definition-r1',1,1,mint_transaction FROM fixture_loot_tree",
    ).bind(id(42).as_slice()).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_item_corpse_container_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id)
         SELECT id,encode($1,'hex')::uuid,encode($2,'hex')::uuid,CASE WHEN n=1301 THEN 11 ELSE n+2 END,mint_transaction
         FROM fixture_loot_tree WHERE n<=8 OR n=1301",
    ).bind(id(42).as_slice()).bind(corpse.as_slice()).execute(&mut *tx).await?;
    sqlx::query(
        "INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id)
         SELECT child.id,encode($1,'hex')::uuid,parent.id,CASE WHEN child.n>1301 THEN 1 ELSE (child.n-1)%8+1 END,child.mint_transaction
         FROM fixture_loot_tree child JOIN fixture_loot_tree parent
           ON parent.n=CASE WHEN child.n>1301 THEN child.n-1 ELSE (child.n-1)/8 END
         WHERE child.n<>1301",
    ).bind(id(42).as_slice()).execute(&mut *tx).await?;
    let traverse = "SELECT item FROM game_corpse_loot_postorder(encode($1,'hex')::uuid,$2) ORDER BY visit_ordinal";
    let limit = oteryn_gameplay_items::quick_loot::MAX_TRANSFERS_PER_COMMAND;
    let limited: Vec<sqlx::types::Uuid> = sqlx::query_scalar(traverse)
        .bind(corpse.as_slice())
        .bind(i32::try_from(limit)?)
        .fetch_all(&mut *tx)
        .await?;
    let full: Vec<sqlx::types::Uuid> = sqlx::query_scalar(traverse)
        .bind(corpse.as_slice())
        .bind(4097_i32)
        .fetch_all(&mut *tx)
        .await?;
    let depth_edges: Vec<sqlx::types::Uuid> =
        sqlx::query_scalar("SELECT id FROM fixture_loot_tree WHERE n>=1308 ORDER BY n")
            .fetch_all(&mut *tx)
            .await?;
    assert!(full.contains(&depth_edges[0]));
    assert!(!full.contains(&depth_edges[1]));
    assert_eq!(limited.len(), limit);
    assert_eq!(limited, full[..limit]);
    tx.rollback().await?;
    Ok(())
}

async fn quick_loot_destination_management(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    pool: &sqlx::PgPool,
) -> TestResult {
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let coin = id(119);
    // Crystal coins follow their Obtain category. Use gold with no matching
    // stack in the destination so this test can assert the physical parent.
    sqlx::raw_sql(sqlx::AssertSqlSafe(
        crate::durability::item_fee_burn::test_support::item(
            119,
            "oteryn:item.tibia.i3043",
            "definition-r1",
            1,
        ),
    ))
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,parent_item_instance_id,item_instance_id,world_id,placement_ordinal,placed_transaction_id)
                 VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,20,encode($3,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(70).as_slice()).bind(coin.as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 98, 607])
            .ok_or("quick loot destination capability")?;
    assert!(admitted.continuity.selected_capabilities.contains(607));
    let assign = QuickLootRequest {
        recipient: Some(QuickLootRecipient::Container),
        recipient_id: Some(id(71)),
        ..request(QuickLootAction::Assign)
    };
    let review = request(QuickLootAction::ReviewDestinations);
    let commands = [
        QuickLootRequest {
            action: QuickLootAction::AssignAllLoot,
            ..assign
        },
        review,
        QuickLootRequest {
            category: Some(QuickLootCategory::Boots),
            ..request(QuickLootAction::UnassignLoot)
        },
        QuickLootRequest {
            action: QuickLootAction::AssignObtain,
            category: Some(QuickLootCategory::Gold),
            ..assign
        },
        review,
        request(QuickLootAction::Distribute),
    ];
    // A fresh domain 9 snapshot allocates the backpack handle first.
    let backpack = std::num::NonZeroU64::new(admitted.continuity.item_view.handle_counter + 1)
        .ok_or("backpack handle")?;
    let commands: Vec<_> = commands
        .iter()
        .map(|request| {
            let payload = if request.action == QuickLootAction::Distribute {
                encode_quick_loot_view_request(request, backpack)
            } else {
                encode_quick_loot_request(request)
            }
            .map_err(debug)?;
            Ok((request.action.command_type(), payload))
        })
        .collect::<TestResult<_>>()?;
    let distribution_id = admitted.continuity.next_command_id + (commands.len() - 1) as u64;
    let (replies, _) =
        super::parameter_spell_fixtures::exchange(owner, admitted, &commands, false, false).await?;
    assert_eq!(replies.len(), commands.len());
    for ((kind, _), (status, _)) in commands.iter().zip(&replies) {
        assert_eq!(
            *status,
            CommandStatus::Accepted,
            "quick-loot command {kind}"
        );
    }
    let first = decode_quick_loot_destinations(&replies[1].1).map_err(debug)?;
    assert_eq!(first.loot.len(), QuickLootCategory::ALL.len() + 1);
    assert!(first.loot.iter().all(|r| r.recipient_id == Some(id(71))));
    let after = decode_quick_loot_destinations(&replies[4].1).map_err(debug)?;
    assert!(
        !after
            .loot
            .iter()
            .any(|r| r.category == Some(QuickLootCategory::Boots))
    );
    assert_eq!(after.obtain.len(), 1);
    assert_eq!(after.obtain[0].category, Some(QuickLootCategory::Gold));
    assert_eq!(after.obtain[0].recipient_id, Some(id(71)));
    let distribution = decode_quick_loot_result(&replies[5].1).map_err(debug)?;
    assert!(distribution.items.contains(&QuickLootItemResult {
        item: coin,
        accepted: true
    }));
    let actual: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(parent_item_instance_id) FROM game_item_nested_entries WHERE item_instance_id=encode($1,'hex')::uuid")
        .bind(coin.as_slice()).fetch_one(pool).await?;
    assert_eq!(actual, id(71));
    // Keep the Channel's command cursor and resend the original command on the new stream.
    let (retry, _) = super::parameter_spell_fixtures::exchange_from(
        owner,
        admitted,
        &commands[5..],
        false,
        false,
        distribution_id,
    )
    .await?;
    assert_eq!(
        retry,
        [(CommandStatus::DuplicateReplay, replies[5].1.clone())]
    );

    // A cold ingress must recover the same durable result even if the new item view cannot
    // resolve the original handle. An unresolved handle on its own cannot authorize a move.
    let (kind, payload) = &commands[5];
    let durable_retry = owner
        .quick_loot_destinations(
            *admitted,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: distribution_id,
                item_fence: admitted.item_fence,
            },
            (*kind, payload),
            None,
        )
        .await;
    assert_eq!(durable_retry, Some(replies[5].clone()));

    Ok(())
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
            manual_sort: false,
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
                        world_interaction: false,
                        manual_sort: false,
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
        manual_sort: false,
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
                    std::num::NonZeroU64::new(base + 5).ok_or("quiver handle")?,
                    None
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
                    std::num::NonZeroU64::new(base + 5).ok_or("quiver handle")?,
                    None
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
