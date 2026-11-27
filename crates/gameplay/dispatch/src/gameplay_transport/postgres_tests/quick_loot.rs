//! QUICKLOOT: production router, item owner, replay and real PostgreSQL custody.
use super::autowalk_fixtures::{read, send};
use super::fixture::{Harness, debug, id, join_two};
use super::full_world_support::command_reply;
use super::kill_reward_e2e_fixtures::scenario_thread;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::item_mint::*;
use crate::foundation::{CommandStatus, MovementLocalPosition};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    AdmittedSession, ConnectionEnd, FirstEntryOutcome, FreshAdmissionAuthority, IDLE_LIVENESS,
    UseCommand, serve_admitted,
};
use oteryn_protocol_oteryn::item_view::{self, ItemHandle, OpenContainer};
use oteryn_protocol_oteryn::quick_loot::*;
use oteryn_protocol_oteryn::world_spatial_entities::{
    EntityDetail, decode_world_spatial_entities_snapshot_with_item_handles,
};
use oteryn_protocol_oteryn::{
    MessageType, decode_snapshot_chunk, decode_state_delta, decode_wire_envelope,
};
use tokio::io::{AsyncWriteExt, DuplexStream};

#[test]
fn quick_loot_postgres_dispatch_assignment_transfer_replay_and_fences() -> TestResult {
    scenario_thread::run_async(scenario(Case::Bulk))
}

fn request(action: QuickLootAction) -> QuickLootRequest {
    QuickLootRequest {
        action,
        corpse: None,
        selected_item: None,
        review_filters: false,
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
    scenario_thread::run_async(scenario(Case::GoldPouch))
}

#[test]
fn quick_loot_pouch_relog_releases_on_one_logout_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_routed_case(admin, Case::PouchLogout)
}

#[test]
fn quick_loot_selected_item_accepts_and_refuses_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_routed_case(admin, Case::SelectedItem)
}

#[test]
fn quick_loot_stale_or_foreign_handle_refuses_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_routed_case(admin, Case::StaleHandle)
}

#[test]
fn quick_loot_admission_restores_filter_and_lists_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_routed_case(admin, Case::FilterReview)
}

#[test]
fn gold_pouch_first_assignment_from_fulfilment_id_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_routed_case(admin, Case::PouchFulfilment)
}

fn run_routed_case(admin: String, case: Case) -> TestResult {
    scenario_thread::run("quick-loot", move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()?
            .block_on(scenario(case)(admin))
    })
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Case {
    Bulk,
    GoldPouch,
    SelectedItem,
    StaleHandle,
    FilterReview,
    PouchFulfilment,
    PouchLogout,
}

fn scenario(case: Case) -> impl AsyncFnOnce(String) -> TestResult {
    async move |admin| {
        let pouch = matches!(
            case,
            Case::GoldPouch | Case::PouchFulfilment | Case::PouchLogout
        );
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
        if !matches!(case, Case::Bulk | Case::GoldPouch | Case::PouchLogout) {
            match case {
                Case::SelectedItem | Case::StaleHandle => {
                    selected_item_cases(
                        &owner,
                        admitted,
                        &harness.pool,
                        corpse_id,
                        &items,
                        [near, far],
                        case,
                    )
                    .await?
                }
                Case::FilterReview => {
                    let index = generation
                        .native_gameplay()
                        .and_then(|n| n.item_index())
                        .ok_or("item index")?;
                    let reference = |key| {
                        index
                            .definition_ref("Item", key, "definition-r1")
                            .map(std::num::NonZeroU32::get)
                            .ok_or("filter item ref")
                    };
                    filter_review_cases(
                        &owner,
                        &mut admitted,
                        reference("oteryn:item.tibia.i3031")?,
                        reference("oteryn:item.tibia.i3577")?,
                    )
                    .await?;
                }
                Case::PouchFulfilment => {
                    pouch_fulfilment_cases(&owner, &mut admitted, &harness.pool).await?
                }
                Case::Bulk | Case::GoldPouch | Case::PouchLogout => {
                    return Err("unexpected dedicated case".into());
                }
            }
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
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
        if case == Case::PouchLogout {
            pouch_relog_logout(&owner, &harness, admitted).await?;
        }
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    }
}

async fn pouch_relog_logout(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    previous: AdmittedSession,
) -> TestResult {
    use crate::durability::fresh_admission::FreshAdmissionStore;
    use crate::foundation::GameSessionState;
    use crate::gameplay_transport::composed_admission::{
        CONDITION_RELEASE_PROBE, HolderCheckoutProbe,
    };
    use crate::gameplay_transport::logout::LogoutReply;
    use oteryn_protocol_oteryn::logout::{
        COMMAND_TYPE_LOGOUT_INTENT, LogoutResult, decode_logout_result,
    };
    use std::{cell::Cell, sync::Arc, time::Duration};

    assert_eq!(
        owner.logout_session(previous).await,
        LogoutReply::Result(LogoutResult::Accepted)
    );
    let admitted =
        super::full_world_support::readmit_after_logout(harness, owner, previous).await?;
    let session = admitted.game_session_id;
    let actor = admitted.runtime_actor.ok_or("readmitted actor")?;
    let store = FreshAdmissionStore::from_root(harness.root.clone());

    let probe = Arc::new(HolderCheckoutProbe::default());
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let drive = async {
        loop {
            let frame = read(&mut client).await?;
            if decode_wire_envelope(&frame)?.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        let id = admitted.continuity.next_command_id;
        send(&mut client, id, COMMAND_TYPE_LOGOUT_INTENT, &[]).await?;
        probe.contend(&harness.root).await?;
        let (_, payload) = command_reply(&mut client, id).await?;
        client.shutdown().await?;
        let outcome = decode_logout_result(&payload).map_err(debug)?;
        assert_eq!(outcome, LogoutResult::Accepted);
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let (end, result) = tokio::time::timeout(
        Duration::from_secs(30),
        join_two(
            CONDITION_RELEASE_PROBE.scope(
                Cell::new(Some(Arc::clone(&probe))),
                serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS),
            ),
            drive,
        ),
    )
    .await?;
    result?;
    assert!(matches!(end, ConnectionEnd::AdmittedThenLoggedOut(_)));
    assert_eq!(
        store.current_session(session).await?.session_state(),
        GameSessionState::Terminal
    );
    assert!(
        owner
            .runtime
            .lock()
            .await
            .player_control_facts(actor, session)
            .is_err()
    );
    Ok(())
}

async fn selected_item_cases(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    pool: &sqlx::PgPool,
    corpse: [u8; 16],
    items: &[[u8; 16]],
    [near, far]: [[u8; 10]; 2],
    case: Case,
) -> TestResult {
    let [untouched, selected] = items else {
        return Err("selected-item fixture expected two loot entries".into());
    };
    fixture_corpse(pool, corpse, near, id(41), 0).await?;
    let corpse_identity = {
        let mut runtime = owner.runtime.lock().await;
        let creature = runtime
            .admit_pinned_test_creature(MovementLocalPosition {
                x: 7,
                y: 0,
                floor: 7,
            })
            .map_err(debug)?;
        runtime
            .commit_monster_lab_damage(creature, b"quick-loot-corpse", 20)
            .map_err(debug)?;
        let mut death = runtime.borrow_combat_death();
        let receipt = death.committed_lethal_receipt(creature).map_err(debug)?;
        death.project_committed_lethal(receipt).map_err(debug)?;
        runtime
            .bind_corpse_item(
                creature,
                corpse,
                "Item",
                "oteryn:item.tibia.i5973",
                "definition-r1",
            )
            .map_err(debug)?;
        creature.corpse_identity()
    };
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    let drive = async {
        let result: TestResult = async {
            let corpse_handle = snapshot_corpse_handle(&mut client, corpse_identity).await?;
            let mut command_id = admitted.continuity.next_command_id;
            use oteryn_protocol_oteryn::world_object::{
                COMMAND_TYPE_USE_INTENT, UseDisposition, decode_use_result, encode_use_item_intent,
            };
            send(
                &mut client,
                command_id,
                COMMAND_TYPE_USE_INTENT,
                &encode_use_item_intent(corpse_handle),
            )
            .await?;
            let (status, payload) = command_reply(&mut client, command_id).await?;
            assert_eq!(status, CommandStatus::Accepted);
            assert_eq!(
                decode_use_result(&payload).map_err(debug)?,
                UseDisposition::Committed
            );
            let opened = corpse_delta(&mut client).await?;
            assert_eq!(opened.container_handle, Some(corpse_handle));
            let [keep, take] = opened.entries.as_slice() else {
                return Err("open corpse should show both entries".into());
            };
            if case == Case::StaleHandle {
                for handle in [corpse_handle, ItemHandle::MAX] {
                    command_id += 1;
                    send_click(&mut client, command_id, handle).await?;
                    assert_eq!(
                        command_reply(&mut client, command_id).await?,
                        (CommandStatus::Rejected, vec![16, 2])
                    );
                }
                assert_eq!(corpse_items(pool, corpse).await?, items);
            } else {
                for (cell, top, reason) in [
                    (far, id(41), "corpse out of range"),
                    (near, id(99), "corpse loot rights"),
                ] {
                    fixture_corpse(pool, corpse, cell, top, 0).await?;
                    command_id += 1;
                    send_click(&mut client, command_id, take.handle).await?;
                    assert_eq!(
                        command_reply(&mut client, command_id).await?.0,
                        CommandStatus::Rejected,
                        "{reason}"
                    );
                    assert_eq!(corpse_items(pool, corpse).await?, items, "{reason}");
                }
                fixture_corpse(pool, corpse, near, id(41), 0).await?;
                command_id += 1;
                send_click(&mut client, command_id, take.handle).await?;
                assert_eq!(
                    command_reply(&mut client, command_id).await?,
                    (CommandStatus::Accepted, vec![16, 1])
                );
                assert_eq!(
                    corpse_delta(&mut client).await?,
                    OpenContainer {
                        container_handle: Some(corpse_handle),
                        entries: vec![*keep],
                    }
                );
                command_id += 1;
                send_click(&mut client, command_id, take.handle).await?;
                assert_eq!(
                    command_reply(&mut client, command_id).await?,
                    (CommandStatus::Rejected, vec![16, 2])
                );
            }
            Ok(())
        }
        .await;
        client.shutdown().await?;
        result
    };
    let (end, result) = join_two(
        serve_admitted(&mut server, admitted, owner, IDLE_LIVENESS),
        drive,
    )
    .await;
    result?;
    assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
    if case == Case::StaleHandle {
        return Ok(());
    }
    assert_eq!(corpse_items(pool, corpse).await?, [*untouched]);
    let backpack: Vec<u8> = sqlx::query_scalar(
        "SELECT uuid_send(parent_item_instance_id) FROM game_item_container_entries
         WHERE item_instance_id=encode($1,'hex')::uuid",
    )
    .bind(selected.as_slice())
    .fetch_one(pool)
    .await?;
    assert_eq!(backpack, id(70));
    Ok(())
}

// Pin the new wire shape independently of the codec so the same scenario runs on the base.
async fn send_click(client: &mut DuplexStream, command_id: u64, handle: ItemHandle) -> TestResult {
    let payload = [&[8, 1, 74, 8][..], &handle.get().to_le_bytes()].concat();
    send(client, command_id, COMMAND_TYPE_QUICK_LOOT, &payload).await
}

async fn snapshot_corpse_handle(
    client: &mut DuplexStream,
    corpse: [u8; 16],
) -> TestResult<ItemHandle> {
    let mut handle = None;
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        match envelope.message_type() {
            MessageType::SnapshotChunk => {
                for domain in decode_snapshot_chunk(envelope.payload())?.1 {
                    if domain.domain_id == 1 {
                        let snapshot = decode_world_spatial_entities_snapshot_with_item_handles(
                            domain.payload,
                        )
                        .map_err(debug)?;
                        handle = snapshot
                            .entities
                            .iter()
                            .find_map(|entry| match entry.detail {
                                EntityDetail::Object { item_handle, .. }
                                | EntityDetail::ObjectWithAppearance { item_handle, .. }
                                    if entry.entity.identity == corpse =>
                                {
                                    item_handle
                                }
                                _ => None,
                            });
                    }
                }
            }
            MessageType::SnapshotCommit => {
                return handle.ok_or_else(|| "missing corpse handle".into());
            }
            _ => {}
        }
    }
}

async fn corpse_delta(client: &mut DuplexStream) -> TestResult<OpenContainer> {
    loop {
        let frame = read(client).await?;
        let envelope = decode_wire_envelope(&frame)?;
        if envelope.message_type() == MessageType::StateDelta {
            let delta = decode_state_delta(envelope.payload())?;
            if delta.domain_id == item_view::STATE_DOMAIN_OPEN_CONTAINER {
                return item_view::decode_open_container(delta.payload).map_err(debug);
            }
        }
    }
}

async fn corpse_items(pool: &sqlx::PgPool, corpse: [u8; 16]) -> TestResult<Vec<[u8; 16]>> {
    let rows: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT uuid_send(item_instance_id) FROM game_item_corpse_container_entries
         WHERE parent_item_instance_id=encode($1,'hex')::uuid ORDER BY placement_ordinal",
    )
    .bind(corpse.as_slice())
    .fetch_all(pool)
    .await?;
    rows.into_iter()
        .map(|row| row.try_into().map_err(|_| "corpse entry UUID".into()))
        .collect()
}

async fn filter_review_cases(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    skip: u32,
    accept: u32,
) -> TestResult {
    let first_id = admitted.continuity.next_command_id;
    let add = |mode, definition| QuickLootRequest {
        filter_mode: Some(mode),
        filter_definition_ref: Some(definition),
        ..request(QuickLootAction::FilterAdd)
    };
    let edits = wire_requests(
        owner,
        admitted,
        &[
            (first_id, add(QuickLootFilterMode::Skip, skip)),
            (first_id + 1, add(QuickLootFilterMode::Accept, accept)),
        ],
    )
    .await?;
    assert_eq!(edits.len(), 2);
    assert!(edits.iter().all(|reply| reply.0 == CommandStatus::Accepted));

    // A separate serve_admitted call restores the settings from PostgreSQL.
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 98, 607])
            .ok_or("quick loot destination capability")?;
    let legacy = request(QuickLootAction::ReviewDestinations);
    let extended = quick_loot_filter_review_request();
    let queries = [
        (
            COMMAND_TYPE_QUICK_LOOT_DESTINATIONS,
            encode_quick_loot_request(&legacy).map_err(debug)?,
        ),
        (
            COMMAND_TYPE_QUICK_LOOT_DESTINATIONS,
            encode_quick_loot_request(&extended).map_err(debug)?,
        ),
    ];
    let (replies, _) =
        super::parameter_spell_fixtures::exchange(owner, admitted, &queries, false, false).await?;
    let [
        (CommandStatus::Accepted, legacy),
        (CommandStatus::Accepted, extended),
    ] = replies.as_slice()
    else {
        return Err(debug(replies));
    };
    let legacy = decode_quick_loot_destinations(legacy).map_err(debug)?;
    assert_eq!(legacy.filters, None);
    assert_eq!(
        decode_quick_loot_destinations(extended).map_err(debug)?,
        QuickLootDestinations {
            filters: Some(QuickLootFilters {
                mode: QuickLootFilterMode::Accept,
                skipped: vec![skip],
                accepted: vec![accept]
            }),
            ..legacy
        }
    );
    Ok(())
}

async fn pouch_fulfilment_cases(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    pool: &sqlx::PgPool,
) -> TestResult {
    let uuid = |seed| sqlx::types::Uuid::from_bytes(id(seed));
    let pouch = uuid(71);
    let fulfilment = uuid(240);
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    // The row shape matches the Store Inbox projection, with a distinct item and fulfilment ID.
    sqlx::query(
        "INSERT INTO store_delivery_receipts
           (operation_id, account_id, character_id, world_id, fulfilment_unit_id,
            result, recovery_generation, decided_at, item_instance_id, receipt)
         SELECT $1, $2, character_id, world_id, $3, 'COMMITTED', 1, 1, $4, decode('01','hex')
         FROM game_character_roots WHERE character_id=$5",
    )
    .bind(uuid(242))
    .bind(uuid(98))
    .bind(fulfilment.to_string())
    .bind(pouch)
    .bind(uuid(41))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;

    let assign = |id| QuickLootRequest {
        recipient: Some(QuickLootRecipient::GoldPouch),
        recipient_id: Some(id),
        ..request(QuickLootAction::Assign)
    };
    let command = admitted.continuity.next_command_id;
    assert_eq!(
        wire_requests(
            owner,
            admitted,
            &[(command, assign(*fulfilment.as_bytes()))]
        )
        .await?[0]
            .0,
        CommandStatus::Rejected
    );
    // Switch only receipt provenance before exercising the first successful assignment.
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE store_delivery_receipts r SET account_id=c.account_id
         FROM game_character_roots c WHERE r.operation_id=$1 AND c.character_id=r.character_id",
    )
    .bind(uuid(242))
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    let command = admitted.continuity.next_command_id;
    assert_eq!(
        wire_requests(
            owner,
            admitted,
            &[(command, assign(*fulfilment.as_bytes()))]
        )
        .await?[0]
            .0,
        CommandStatus::Accepted
    );
    let saved: (sqlx::types::Uuid, bool) = sqlx::query_as(
        "SELECT unassigned_container_id, unassigned_gold_pouch
         FROM game_character_quick_loot_settings WHERE character_id=$1",
    )
    .bind(uuid(41))
    .fetch_one(pool)
    .await?;
    assert_eq!(saved, (pouch, true));
    let command = admitted.continuity.next_command_id;
    assert_eq!(
        wire_requests(owner, admitted, &[(command, assign(*pouch.as_bytes()))]).await?[0].0,
        CommandStatus::Accepted
    );
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
