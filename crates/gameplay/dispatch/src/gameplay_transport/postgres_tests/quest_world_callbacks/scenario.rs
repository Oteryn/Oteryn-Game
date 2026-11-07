use super::*;

pub(crate) fn scenario(name: &str, trigger: Position, lever_fixture: bool) -> TestResult {
    scenario_case(name, trigger, lever_fixture, None)
}

#[allow(
    dead_code,
    reason = "per-quest stateless PG cases are independently picked after the common harness"
)]
pub(crate) fn stateless_scenario(
    name: &str,
    trigger: Position,
    expected_destination: Position,
) -> TestResult {
    // Retain the existing STEP contract for old callers. Source USE families
    // are selected explicitly, never inferred from changed content.
    assert_eq!(
        callback(name)?.trigger,
        if name == "deeper-fibula-well" || name.starts_with("between-the-lines-") {
            oteryn_gameplay_quests::quest::world_callbacks::Trigger::Use
        } else {
            oteryn_gameplay_quests::quest::world_callbacks::Trigger::StepIn
        }
    );
    scenario_case(name, trigger, false, Some(expected_destination))
}

fn scenario_case(
    name: &str,
    trigger: Position,
    lever_fixture: bool,
    stateless_destination: Option<Position>,
) -> TestResult {
    let route = name.starts_with("draconia-route-");
    let container_stock = name.starts_with("draconia-container-stock-");
    let departure = name == "departure-map-regression";
    let dropdown = name.starts_with("source-dropdown");
    let devil = name.starts_with("devil-helmet-source");
    let failure_landing = name.starts_with("conditional-failure-");
    let draconia_escape = name.starts_with("draconia-escape-");
    let draconia_exit = name.starts_with("draconia-exit-");
    let object_mutation = name.starts_with("object-mutation-");
    let mintwallin = name == "mintwallin-source";
    let monk = name.starts_with("monk-guidance");
    let elathriel = name.starts_with("draconia-elathriel-");
    let dragonlord = name.starts_with("draconia-dragonlord-");
    let classic_npc = elathriel
        || dragonlord
        || matches!(
            name,
            "catalogue-npc"
                | "catalogue-npc-father"
                | "fathers-burden-handins"
                | "hunting-for-tokens"
                | "marlin-trophy-exchange"
                | "catalogue-npc-hero"
                | "catalogue-npc-legacy"
                | "catalogue-npc-scoped"
                | "classic-quests-npc"
                | "classic-rottin-disrepair"
                | "threatened-dreams-npc"
                | "conditional-quest-greetings"
        );
    let rotten_purchase = name == "rotten-blood-access-purchase";
    let target_swap = name == "fish-napping-key-target-swap";
    let mut callback = callback(
        if classic_npc
            || container_stock
            || route
            || departure
            || object_mutation
            || mintwallin
            || dropdown
            || devil
            || draconia_escape
            || draconia_exit
        {
            "behemoth"
        } else if failure_landing {
            "liquid-black-shortcut"
        } else if rotten_purchase {
            "rotten-blood/sacrifice"
        } else if monk {
            "monk-guidance"
        } else if target_swap {
            "fish-napping-key"
        } else {
            name
        },
    )?;
    if failure_landing {
        callback.selector.position = Some(trigger);
    }
    if route {
        callback = super::super::draconia_route::fixture(callback, name)?;
    }
    if target_swap {
        callback
            .item_on_target
            .as_mut()
            .ok_or("item")?
            .target_replacement = Some(7777);
    }
    if lever_fixture {
        callback.trigger = oteryn_gameplay_quests::quest::world_callbacks::Trigger::Use;
        callback.selector.unique = Some(25012);
        callback.selector.appearance = Some(2772);
        callback.lever = Some([2772, 2773]);
    }
    if stateless_destination.is_some() {
        match callback.trigger {
            oteryn_gameplay_quests::quest::world_callbacks::Trigger::StepOut => {
                return Err("stateless destination scenario requires entry or USE".into());
            }
            oteryn_gameplay_quests::quest::world_callbacks::Trigger::StepIn => {
                assert_eq!(callback.selector.position, Some(trigger));
            }
            oteryn_gameplay_quests::quest::world_callbacks::Trigger::Use => {
                assert!(
                    callback
                        .selector
                        .position
                        .is_none_or(|source| source == trigger)
                );
                if name.starts_with("between-the-lines-") {
                    assert_eq!(callback.quest, "oteryn:quest.between_the_lines_quest");
                    assert_eq!(callback.selector.position, Some(trigger));
                    assert!(callback.selector.appearance.is_some());
                } else {
                    assert!(
                        callback.selector.unique.is_some() || callback.selector.action.is_some()
                    );
                }
            }
        }
    }
    if departure {
        callback.trigger = oteryn_gameplay_quests::quest::world_callbacks::Trigger::StepIn;
        callback.selector.appearance = None;
        callback.reversible = true;
    }
    if object_mutation {
        callback
            .passages
            .push(oteryn_gameplay_quests::quest::world_callbacks::Passage {
                position: Position {
                    x: trigger.x + 6,
                    ..trigger
                },
                appearance: 1270,
                replacement: None,
            });
    }
    if mintwallin {
        callback.objects = Some(
            oteryn_gameplay_quests::quest::world_callbacks::ObjectEffects {
                consume_trigger: true,
                mutations: [32105, 32106]
                    .into_iter()
                    .map(
                        |y| oteryn_gameplay_quests::quest::world_callbacks::ObjectMutation {
                            position: Position {
                                x: 32592,
                                y,
                                floor: -14,
                            },
                            remove: None,
                            create: Some(1270),
                        },
                    )
                    .collect(),
            },
        );
        callback.selector.unique = None;
        callback.selector.action = Some(30008);
        callback.passages = (32593..=32601)
            .map(
                |x| oteryn_gameplay_quests::quest::world_callbacks::Passage {
                    position: Position {
                        x,
                        y: 32104,
                        floor: -14,
                    },
                    appearance: 1271,
                    replacement: None,
                },
            )
            .collect();
        callback
            .passages
            .push(oteryn_gameplay_quests::quest::world_callbacks::Passage {
                position: Position {
                    x: 32590,
                    y: 32104,
                    floor: -14,
                },
                appearance: 1270,
                replacement: None,
            });
    }
    if dropdown {
        callback.selector.appearance = Some(435);
        callback.teleport = Some(Position {
            floor: trigger.floor - 1,
            ..trigger
        });
    }
    if devil {
        callback.selector.unique = Some(if name.ends_with("unrelated") {
            25031
        } else {
            25030
        });
        callback.selector.action = None;
        callback.selector.appearance = Some(100);
        callback.lever = None;
        callback.passages.clear();
        callback.objects = Some(
            oteryn_gameplay_quests::quest::world_callbacks::ObjectEffects {
                consume_trigger: false,
                mutations: vec![
                    oteryn_gameplay_quests::quest::world_callbacks::ObjectMutation {
                        position: Position {
                            x: 32482,
                            y: 32170,
                            floor: -14,
                        },
                        remove: None,
                        create: Some(435),
                    },
                ],
            },
        );
        callback.teleport = (!name.ends_with("blocked")).then_some(Position {
            x: 32482,
            y: 32170,
            floor: -15,
        });
        if name.ends_with("existing") {
            callback
                .passages
                .push(oteryn_gameplay_quests::quest::world_callbacks::Passage {
                    position: Position {
                        x: 32482,
                        y: 32170,
                        floor: -14,
                    },
                    appearance: 435,
                    replacement: None,
                });
        }
    }
    if draconia_escape {
        callback.selector.unique = Some(35012);
        callback.selector.action = None;
        callback.selector.position = None;
        callback.selector.appearance =
            (trigger.x == 32815 && trigger.y == 31599 && trigger.floor == -9).then_some(1949);
        callback.lever = None;
        callback.passages.clear();
        callback.teleport = Some(Position {
            x: 32701,
            y: 31639,
            floor: -6,
        });
        callback.offerings = vec![oteryn_gameplay_quests::quest::world_callbacks::Offering {
            position: Position {
                x: 32816,
                y: 31601,
                floor: -9,
            },
            item: "oteryn:item.tibia.i3206".into(),
            count: 1,
        }];
    }
    if draconia_exit {
        callback.selector.unique = Some(35011);
        callback.selector.position = None;
        callback.selector.action = None;
        callback.selector.appearance = Some(
            if trigger.x == 32805 && trigger.y == 31587 && trigger.floor == -1 {
                1949
            } else {
                100
            },
        );
        callback.lever = None;
        callback.passages = super::super::draconia_exit::pattern()
            .into_iter()
            .enumerate()
            .map(|(i, (position, wanted))| {
                oteryn_gameplay_quests::quest::world_callbacks::Passage {
                    position,
                    appearance: if super::super::draconia_exit::mismatch(name) == Some(i) {
                        if wanted == 2772 { 2773 } else { 2772 }
                    } else {
                        wanted
                    },
                    replacement: None,
                }
            })
            .collect();
        callback.teleport = Some(Position {
            x: 32701,
            y: 31639,
            floor: -6,
        });
    }
    if container_stock {
        callback.selector.unique = None;
        callback.selector.action = None;
        callback.selector.appearance = Some(5972);
        callback.passages.clear();
        callback.offerings.clear();
        callback.teleport = None;
    }
    let world = if container_stock {
        super::source_route_world(&callback, trigger, &[])?
    } else if route {
        let cells =
            super::super::draconia_route::gate(super::super::draconia_route::source(name)?.0)
                .map(|gate| {
                    vec![
                        Position {
                            x: gate.x - 1,
                            ..gate
                        },
                        Position {
                            x: gate.x + 1,
                            ..gate
                        },
                    ]
                })
                .unwrap_or_default();
        super::source_route_world(&callback, trigger, &cells)?
    } else if draconia_exit {
        super::world_with_palette(
            &callback,
            trigger,
            &[
                (2772, oteryn_world_bundle::bundle::TerrainKind::Common),
                (2773, oteryn_world_bundle::bundle::TerrainKind::Common),
            ],
        )?
    } else if devil {
        super::world_with_palette(
            &callback,
            trigger,
            &[(435, oteryn_world_bundle::bundle::TerrainKind::Common)],
        )?
    } else if departure {
        super::world_ordinal(&callback, trigger, 2772, 1)?
    } else {
        world(&callback, trigger)?
    };
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "quest_world_callbacks", false, false)
            .await?;
        if name.starts_with("draconia-") {
            let version: String = sqlx::query_scalar("SELECT current_setting('server_version')")
                .fetch_one(&h.pool)
                .await?;
            println!(
                "POSTGRES_CASE_STARTED {name} server_version={version} native_trigger={trigger:?}"
            );
        }
        if dropdown {
            println!("POSTGRES_CASE_STARTED source_dropdown database_created=true");
        }
        let devil_departure = if devil {
            println!("POSTGRES_CASE_STARTED devil_helmet_source database_created=true");
            Some(super::callback("devil-helmet-sewer-departure")?)
        } else {
            None
        };
        let callback = if draconia_exit {
            println!("POSTGRES_CASE_STARTED draconia_exit database_created=true");
            let authored = super::super::draconia_exit::authored()?;
            let production = super::callback("draconia-exit-35011")?;
            assert_eq!(
                serde_json::to_value(&authored)?,
                serde_json::to_value(&production)?
            );
            production
        } else if draconia_escape {
            println!("POSTGRES_CASE_STARTED draconia_escape database_created=true");
            let production = super::callback("draconia-escape-35012")?;
            assert_eq!(
                serde_json::to_value(&production)?,
                serde_json::to_value(super::super::draconia_escape::authored_callback()?)?,
                "runtime callback preserves its authored source packet"
            );
            production
        } else if failure_landing {
            println!("POSTGRES_CASE_STARTED conditional_failure_landing database_created=true");
            let mut value = serde_json::to_value(&callback)?;
            value["failure_teleport"] = serde_json::json!({"x":32818,"y":31599,"floor":if name.ends_with("blocked") {-10}else{-9}});
            serde_json::from_value::<Callback>(value)?
        } else if devil {
            super::callback("devil-helmet-sewer-entry")?
        } else if object_mutation {
            println!("POSTGRES_CASE_STARTED quest_object_mutations database_created=true");
            let mut value = serde_json::to_value(&callback)?;
            value["lever"] = serde_json::Value::Null;
            value["passages"] = serde_json::json!([]);
            value["objects"] = serde_json::json!({
                "consume_trigger": name != "object-mutation-repeat",
                "mutations": [
                    {"position": callback.passages[0].position, "remove":1791, "create":null},
                    {"position": {"x":trigger.x+1,"y":trigger.y,"floor":trigger.floor},
                     "remove":null,"create":if name == "object-mutation-refusal" {65535} else {1270}}
                ]
            });
            serde_json::from_value::<Callback>(value)?
        } else if mintwallin {
            println!("POSTGRES_CASE_STARTED mintwallin_source database_created=true");
            super::callback("mintwallin-cyclops-lever")?
        } else {
            callback.clone()
        };
        let departure_callback = if departure {
            println!("POSTGRES_CASE_STARTED quest_departure database_created=true");
            let mut value = serde_json::to_value(&callback)?;
            value["key"] = "oteryn:quest-callback/departure-map-regression".into();
            value["trigger"] = "step_out".into();
            Some(serde_json::from_value::<Callback>(value)?)
        } else {
            None
        };
        prepare_current_session(&h).await?;
        if (route
            && super::super::draconia_route::gate(super::super::draconia_route::source(name)?.0)
                .is_some())
            || (container_stock && name.ends_with("-epoch-observer"))
        {
            h.seed_pvp_target().await?;
            super::super::appearance_change_postgres::observer_visibility_tests::admit(&h).await?;
        }
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let cells = world
            .movement_cells(activated.qualified_room().movement_cells())
            .map_err(debug)?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            world.map_revision_digest(),
            (
                i32::from(trigger.x) - 1,
                i32::from(trigger.y),
                -i16::from(trigger.floor),
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
            // These two cases keep five source NPCs and one player alive.
            if matches!(name, "catalogue-npc-hero" | "catalogue-npc-scoped") {
                6
            } else if classic_npc {
                5
            } else if monk {
                4
            } else {
                2
            },
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        let origin = local(Position {
            x: trigger.x - 1,
            ..trigger
        });
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = &cells;
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        if draconia_escape {
            owner.quest_catalogue =
                Some(super::super::draconia_escape::catalogue(&callback, name)?);
        }
        if draconia_exit && name.ends_with("live") {
            assert!(
                owner
                    .states
                    .quest_callbacks
                    .fixture
                    .set(vec![
                        callback.clone(),
                        super::super::draconia_exit::live_lever(&callback)?
                    ])
                    .is_ok()
            );
        }
        if departure {
            let mut entry = callback.clone();
            entry.key = "oteryn:quest-callback/departure-entry-feedback".into();
            entry.selector.position = Some(Position {
                x: trigger.x - 1,
                ..trigger
            });
            entry.selector.unique = None;
            entry.lever = None;
            entry.reversible = false;
            entry.passages.clear();
            entry.success_message = Some("The destination entry was processed.".into());
            assert!(
                owner
                    .states
                    .quest_callbacks
                    .fixture
                    .set(vec![
                        callback.clone(),
                        departure_callback.ok_or("departure callback")?,
                        entry,
                    ])
                    .is_ok()
            );
        } else if devil {
            assert!(
                owner
                    .states
                    .quest_callbacks
                    .fixture
                    .set(vec![
                        callback.clone(),
                        devil_departure.ok_or("departure callback")?
                    ])
                    .is_ok()
            );
        } else if lever_fixture || target_swap || object_mutation || mintwallin || failure_landing {
            assert!(
                owner
                    .states
                    .quest_callbacks
                    .fixture
                    .set(vec![callback.clone()])
                    .is_ok()
            );
        }
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        admitted.item_fence = Some(fence.item_fence());
        sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fresh = super::super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root
            .initialize_admission_equipment(
                &authority,
                &h.node,
                &fence.item_fence(),
                generation.identity().server_artifact_digest(),
                fresh.operation(),
            )
            .await
            .map_err(debug)?;
        let native = generation.native_gameplay().ok_or("native")?;
        let template = crate::durability::starter_grant::StarterTemplate {
            definition: crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2854".into(),
                revision_ref: "definition-r1".into(),
            },
            ..crate::durability::starter_grant::StarterTemplate::main()
        };
        h.root
            .grant_starter_items(
                &authority,
                &h.node,
                &fence.item_fence(),
                &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                &template,
            )
            .await
            .map_err(debug)?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
        if route {
            super::super::draconia_route::exercise(&deps, &owner, &mut admitted, &h, name).await?;
        } else if draconia_exit {
            super::super::draconia_exit::exercise(&owner, &mut admitted, &h, deps.actor, name)
                .await?;
        } else if draconia_escape {
            super::super::draconia_escape::exercise(
                &owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                &callback,
                name,
            )
            .await?;
        } else if failure_landing {
            super::super::conditional_callback_failure::exercise(
                &owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                if name.ends_with("blocked") {
                    trigger
                } else {
                    Position {
                        x: 32818,
                        y: 31599,
                        floor: -9,
                    }
                },
            )
            .await?;
        } else if devil {
            super::super::devil_helmet_source::exercise(
                &owner,
                &mut admitted,
                &h,
                deps.actor,
                trigger,
                name,
            )
            .await?;
        } else if dropdown {
            super::super::source_dropdown::exercise(
                &owner,
                &admitted,
                &h,
                deps.actor,
                trigger,
                name.ends_with("added"),
            )
            .await?;
        } else if mintwallin {
            super::super::mintwallin_source::exercise(&owner, &admitted, &h, trigger).await?;
        } else if object_mutation {
            super::super::quest_object_mutations::exercise(&owner, &admitted, &h, name, trigger)
                .await?;
        } else if departure {
            super::super::quest_departure::exercise(
                &owner,
                &mut admitted,
                &h,
                deps.actor,
                &callback,
            )
            .await?;
        } else if let Some(expected_destination) = stateless_destination {
            super::stateless::exercise(
                &owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                &callback,
                expected_destination,
            )
            .await?;
        } else if name == "hunting-for-tokens" {
            super::super::raid_tokens::exercise(&mut owner, &mut admitted, &h).await?;
        } else if name == "marlin-trophy-exchange" {
            super::super::marlin_trophy::exercise(&mut owner, &mut admitted, &h, &authority)
                .await?;
        } else if container_stock && name.contains("-wire-") {
            super::super::draconia_stock_wire::exercise(&owner, &mut admitted, &h, name).await?;
        } else if container_stock && name.contains("-epoch-") {
            super::super::draconia_stock_epochs::exercise(&deps, &mut owner, &admitted, &h, name)
                .await?;
        } else if container_stock {
            super::super::draconia_container_stock::exercise(
                &owner, &admitted, &h, deps.actor, name,
            )
            .await?;
        } else if elathriel {
            super::super::draconia_elathriel::exercise(&mut owner, &mut admitted, &h, name).await?;
        } else if dragonlord {
            super::super::draconia_dragonlord::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                name,
            )
            .await?;
        } else if matches!(
            name,
            "catalogue-npc-father" | "catalogue-npc-legacy" | "fathers-burden-handins"
        ) {
            super::super::npc_program::exercise_father(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                name == "catalogue-npc-legacy",
                name == "fathers-burden-handins",
            )
            .await?;
        } else if matches!(name, "catalogue-npc-hero" | "catalogue-npc-scoped") {
            super::super::npc_program::exercise_scoped(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                name == "catalogue-npc-hero",
            )
            .await?;
        } else if name == "catalogue-npc" {
            super::super::npc_program::exercise(&mut owner, &mut admitted, &h, &authority).await?;
        } else if name == "conditional-quest-greetings" {
            super::super::conditional_quest_greetings::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            )
            .await?;
        } else if name == "threatened-dreams-npc" {
            super::super::threatened_dreams::exercise(&mut owner, &mut admitted, &h, &authority)
                .await?;
        } else if name == "classic-rottin-disrepair" {
            super::super::classic_quests::exercise_rottin(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
            )
            .await?;
        } else if classic_npc {
            super::super::classic_quests::exercise(&mut owner, &mut admitted, &h, &authority)
                .await?;
        } else if matches!(
            name,
            "outlaw-camp-entrance"
                | "banshees-first-seal-return"
                | "banshees-second-seal"
                | "banshees-final-battle-exit"
        ) {
            super::super::classic_quests::exercise_step(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                &world,
                &callback,
                trigger,
                origin,
            )
            .await?;
        } else if monk {
            super::super::monk_shrines::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                &world,
                name.ends_with("full-backpack"),
                name.ends_with("capacity"),
            )
            .await?;
        } else if callback.key == oteryn_gameplay_quests::quest::rotten_blood::SACRIFICE {
            super::rotten_blood::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                fence,
                &callback,
                trigger,
                rotten_purchase,
            )
            .await?;
        } else if callback.item_on_target.is_some()
            || name == "outlaw-camp-door"
            || name == "travelling-trader-delivery"
        {
            super::actions::exercise(
                &owner,
                &mut admitted,
                &h,
                &authority,
                fence,
                &callback,
                trigger,
                origin,
            )
            .await?;
        } else if !lever_fixture && !callback.offerings.is_empty() {
            for (n, altar) in callback.offerings.iter().take(3).enumerate() {
                offering(&owner, fence.item_fence(), altar, u64::try_from(n)? + 1).await?;
            }
            let east = oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                oteryn_protocol_oteryn::world_spatial::StepDirection::East,
            );
            wire(&owner, &mut admitted, 1, 1, east.clone()).await?;
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                local(trigger)
            );
            let burned: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(burned, 0);
            offering(&owner, fence.item_fence(), &callback.offerings[3], 4).await?;
            wire(
                &owner,
                &mut admitted,
                2,
                1,
                oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                    oteryn_protocol_oteryn::world_spatial::StepDirection::West,
                ),
            )
            .await?;
            wire(&owner, &mut admitted, 3, 1, east).await?;
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                local(callback.teleport.ok_or("teleport")?)
            );
            let burned: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(burned, 4);
            let copy = h
                .root
                .read_character_quest_state(&authority, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(copy.tracks().get(&callback.storage[0].track), Some(&1));
        } else if name == "lions-rock-inner-sanctum-entry" {
            super::lions_rock::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                fence,
                &callback,
                origin,
            )
            .await?;
        } else if name == "unnatural-selection-dance-2" {
            super::unnatural_selection::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                fence,
                &callback,
                trigger,
            )
            .await?;
        } else if name == "liquid-black-shortcut" {
            // The source shortcut has no text. Exercise the generic authored refusal message
            // without changing that source binding in production content.
            let mut message_callback = callback.clone();
            message_callback.failure_message = Some("Finish the visitor mission first.".into());
            assert!(
                owner
                    .states
                    .quest_callbacks
                    .fixture
                    .set(vec![message_callback])
                    .is_ok()
            );
            owner
                .states
                .chat
                .admit(admitted.game_session_id, "Callback", owner.owner_now());
            let step = oteryn_protocol_oteryn::world_spatial::encode_step_intent(
                oteryn_protocol_oteryn::world_spatial::StepDirection::East,
            );
            wire(&owner, &mut admitted, 1, 1, step.clone()).await?;
            assert!(owner.states.chat.drain(admitted.game_session_id).iter().any(|line|
                matches!(line, oteryn_protocol_oteryn::chat::ChatLine::Private { text, .. } if text == "Finish the visitor mission first.")));
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                origin
            );
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            for (n, key) in ["npc_1", "action_2", "action_1", "npc_2"]
                .into_iter()
                .enumerate()
            {
                use crate::durability::quest_state::{
                    QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
                };
                let request = QuestTransitionRequest {
                    transition_key: format!(
                        "oteryn:quest-transition/quest/u9_4/liquid_black_quest/visitor/{key}"
                    ),
                    cause: QuestCause::Command(crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(100 + u64::try_from(n)?)
                            .map_err(debug)?,
                    )),
                };
                assert!(matches!(
                    slot.commit_quest_transition(
                        owner.root,
                        owner.character,
                        owner.holder,
                        fence,
                        request,
                        owner.quest_catalogue.clone().ok_or("catalogue")?
                    )
                    .await
                    .map_err(debug)?,
                    QuestTransitionOutcome::Committed(_)
                ));
            }
            drop(slot);
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("clock")?;
            wire(&owner, &mut admitted, 2, 1, step).await?;
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                local(callback.teleport.ok_or("teleport")?)
            );
            let copy = h
                .root
                .read_character_quest_state(&authority, fence.character_id)
                .await
                .map_err(debug)?;
            assert_eq!(copy.tracks().get(&callback.storage[0].track), Some(&5));
        } else {
            let key =
                oteryn_world_bundle::bundle::placement_key(trigger.floor, trigger.x, trigger.y, 1)
                    .ok_or("key")?;
            let target = |revision| {
                let mut placement = world.base().digest().to_vec();
                placement.extend_from_slice(&key.to_be_bytes());
                WorldObjectTarget {
                    placement,
                    expected_revision: revision,
                }
            };
            if lever_fixture {
                for (n, altar) in callback.offerings.iter().take(3).enumerate() {
                    offering(&owner, fence.item_fence(), altar, u64::try_from(n)? + 1).await?;
                }
            }
            let result = wire(
                &owner,
                &mut admitted,
                1,
                2,
                oteryn_protocol_oteryn::world_object::encode_use_intent(&target(0))
                    .map_err(debug)?,
            )
            .await?;
            let expected = if lever_fixture {
                UseDisposition::Rejected
            } else {
                UseDisposition::Committed
            };
            assert_eq!(
                oteryn_protocol_oteryn::world_object::decode_use_result(&result).map_err(debug)?,
                expected
            );
            if lever_fixture {
                let untouched: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
                        .fetch_one(&h.pool)
                        .await?;
                assert_eq!(untouched, 0);
                let copy = h
                    .root
                    .read_character_quest_state(&authority, fence.character_id)
                    .await
                    .map_err(debug)?;
                assert!(!copy.tracks().contains_key(&callback.storage[0].track));
                offering(&owner, fence.item_fence(), &callback.offerings[3], 4).await?;
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(2))
                    .ok_or("clock")?;
                let context = {
                    let runtime = deps.runtime.lock().await;
                    crate::durability::quest_world_callback::QuestWorldContext {
                        carried_tool: None,
                        map_revision: crate::map::overlay::map_revision(world.base()),
                        content_revision: format!(
                            "sha256:{}",
                            runtime
                                .content_pin()
                                .server_artifact_digest()
                                .iter()
                                .map(|b| format!("{b:02x}"))
                                .collect::<String>()
                        ),
                        placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
                        reset_epoch: owner.reset_epoch,
                        actor_placement: deps.actor.placement_identity(),
                        movement_revision: runtime
                            .read_actor_position(deps.actor)
                            .map_err(debug)?
                            .revision(),
                    }
                };
                owner
                    .states
                    .quest_callbacks
                    .interrupt_after_commit
                    .store(true, std::sync::atomic::Ordering::Release);
                let result = wire(
                    &owner,
                    &mut admitted,
                    2,
                    2,
                    oteryn_protocol_oteryn::world_object::encode_use_intent(&target(0))
                        .map_err(debug)?,
                )
                .await;
                assert!(
                    result.is_err(),
                    "an unknown result must close without an acknowledgement"
                );
                assert!(owner.has_pending_quest_callback(admitted.game_session_id));
                assert_eq!(
                    deps.runtime
                        .lock()
                        .await
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position(),
                    origin
                );
                let burned: i64 =
                    sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions")
                        .fetch_one(&h.pool)
                        .await?;
                assert_eq!(burned, 4);
                // Another session must not settle or project this disconnected player's
                // pending callback when its own command stream opens.
                let mut other = admitted;
                other.game_session_id =
                    crate::foundation::GameSessionId::decode(&id(51)).map_err(debug)?;
                other.continuity = crate::gameplay_transport::connection::SessionContinuity::FRESH;
                other.item_fence = None;
                {
                    let mut runtime = deps.runtime.lock().await;
                    let reservation = runtime
                        .reserve_fresh_session(other.game_session_id)
                        .map_err(debug)?;
                    let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
                    runtime
                        .initialize_pinned_test_position(
                            actor,
                            local(Position {
                                x: trigger.x - 2,
                                ..trigger
                            }),
                        )
                        .map_err(debug)?;
                    other.runtime_actor = Some(actor);
                }
                assert!(
                    wire(&owner, &mut other, 1, 999, Vec::new())
                        .await?
                        .is_empty()
                );
                assert!(owner.has_pending_quest_callback(admitted.game_session_id));
                assert_eq!(
                    deps.runtime
                        .lock()
                        .await
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position(),
                    origin
                );
                // The next real dispatch reconciles the original USE before allowing input.
                let replay = wire(
                    &owner,
                    &mut admitted,
                    2,
                    2,
                    oteryn_protocol_oteryn::world_object::encode_use_intent(&target(0))
                        .map_err(debug)?,
                )
                .await?;
                assert_eq!(
                    oteryn_protocol_oteryn::world_object::decode_use_result(&replay)
                        .map_err(debug)?,
                    UseDisposition::Committed
                );
                assert!(!owner.has_pending_quest_callback(admitted.game_session_id));
                use crate::durability::quest_state::{
                    QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
                };
                let request = QuestTransitionRequest {
                    transition_key: callback.key.clone(),
                    cause: QuestCause::Use(crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(2).map_err(debug)?,
                    )),
                };
                let catalogue = owner.quest_catalogue.clone().ok_or("catalogue")?;
                let permit = owner.spell_lane_permit().await.ok_or("lane")?;
                let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
                assert!(matches!(
                    slot.commit_world_callback(
                        owner.root,
                        owner.character,
                        owner.holder,
                        fence,
                        request.clone(),
                        catalogue.clone(),
                        &permit,
                        context.clone()
                    )
                    .await
                    .map_err(debug)?,
                    QuestTransitionOutcome::AlreadyCommitted(_)
                ));
                // A generic quest caller cannot bypass the physical altar proof.
                assert!(matches!(slot.commit_quest_transition(owner.root, owner.character, owner.holder,
                    fence, request.clone(), catalogue.clone()).await,
                    Err(crate::durability::character_progression::CharacterProgressionError::InvalidInput)));
                let mut stale = fence;
                stale.character_lease_generation += 1;
                let fresh_request = QuestTransitionRequest {
                    cause: QuestCause::Use(crate::foundation::CommandRef::new(
                        admitted.game_session_id,
                        crate::foundation::CommandId::new(999).map_err(debug)?,
                    )),
                    ..request
                };
                assert!(matches!(slot.commit_world_callback(owner.root, owner.character, owner.holder,
                    stale, fresh_request, catalogue, &permit, context).await,
                    Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)));
                drop(slot);
                drop(permit);
                let (proofs, receipts, remaining): (i64, i64, i64) = sqlx::query_as(
                    "SELECT (SELECT count(*) FROM game_quest_callback_consumptions), \
                     (SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1), \
                     (SELECT count(*) FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key=$2)")
                    .bind(&callback.key).bind(&callback.offerings[0].item).fetch_one(&h.pool).await?;
                assert_eq!((proofs, receipts, remaining), (4, 1, 0));
                let copy = h
                    .root
                    .read_character_quest_state(&authority, fence.character_id)
                    .await
                    .map_err(debug)?;
                assert_eq!(copy.tracks().get(&callback.storage[0].track), Some(&1));
                assert_eq!(
                    deps.runtime
                        .lock()
                        .await
                        .read_actor_position(deps.actor)
                        .map_err(debug)?
                        .position(),
                    local(callback.teleport.ok_or("teleport")?)
                );
            }
            if !lever_fixture {
                for passage in &callback.passages {
                    assert!(world.enterable(crate::map::overlay::TilePos {
                        x: passage.position.x,
                        y: passage.position.y,
                        floor: passage.position.floor
                    }));
                }
                owner.clock_origin = owner
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(2))
                    .ok_or("clock")?;
                let result = wire(
                    &owner,
                    &mut admitted,
                    2,
                    2,
                    oteryn_protocol_oteryn::world_object::encode_use_intent(&target(1))
                        .map_err(debug)?,
                )
                .await?;
                assert_eq!(
                    oteryn_protocol_oteryn::world_object::decode_use_result(&result)
                        .map_err(debug)?,
                    if callback.reversible {
                        UseDisposition::Committed
                    } else {
                        UseDisposition::NothingToUse
                    }
                );
                if callback.reversible {
                    for passage in &callback.passages {
                        assert!(!world.enterable(crate::map::overlay::TilePos {
                            x: passage.position.x,
                            y: passage.position.y,
                            floor: passage.position.floor,
                        }));
                    }
                }
            }
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
