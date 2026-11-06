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
    scenario_case(name, trigger, false, Some(expected_destination))
}

fn scenario_case(
    name: &str,
    trigger: Position,
    lever_fixture: bool,
    stateless_destination: Option<Position>,
) -> TestResult {
    let monk = name.starts_with("monk-guidance");
    let qengine_reward = name.starts_with("qengine-rewards-");
    let outfit_world = name.starts_with("l1-") && name.ends_with("-wardrobe");
    let classic_npc = qengine_reward
        || matches!(
            name,
            "catalogue-npc"
                | "catalogue-npc-father"
                | "fathers-burden-handins"
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
    let sewer = name.starts_with("devil-helmet-sewer");
    let failure_landing = name.starts_with("conditional-failure-");
    let draconia_escape = name.starts_with("draconia-escape-");
    let mut callback = if draconia_escape {
        super::super::draconia_escape::authored_callback()?
    } else {
        callback(if classic_npc || sewer {
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
            name.strip_suffix("-expiry").unwrap_or(name)
        })?
    };
    if failure_landing {
        callback.selector.position = Some(trigger);
        callback.failure_teleport = Some(Position {
            x: 32818,
            y: 31599,
            floor: if name.ends_with("blocked") { -10 } else { -9 },
        });
    }
    if sewer {
        callback.key = format!("fixture:{name}");
        callback.selector.position = Some(trigger);
        callback.selector.unique = Some(if name.ends_with("unrelated") {
            25031
        } else {
            25030
        });
        callback.selector.action = None;
        callback.selector.appearance = Some(100);
        callback.lever = None;
        callback.storage.clear();
        callback.teleport = None;
        callback.offerings.clear();
        callback.passages = vec![oteryn_gameplay_quests::quest::world_callbacks::Passage {
            position: Position {
                x: 32482,
                y: 32170,
                floor: -14,
            },
            appearance: 435,
            replacement: None,
        }];
    }
    if name.ends_with("-expiry") {
        callback
            .effects
            .portals
            .as_mut()
            .ok_or("portal pair")?
            .lifetime_us = Some(60_000_000);
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
        assert_eq!(callback.selector.position, Some(trigger));
    }
    let world = world(&callback, trigger)?;
    run(async |admin| {
        let h = if qengine_reward || outfit_world {
            Harness::create_quest_reward(admin, "quest_rewards", !name.ends_with("pirate")).await
        } else {
            Harness::create_with_admission_seed(admin, "quest_world_callbacks", false, false).await
        }
        .map_err(|e| format!("callback harness: {e}"))?;
        prepare_current_session(&h)
            .await
            .map_err(|e| format!("callback session: {e}"))?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(|e| format!("callback authority: {e:?}"))?;
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
            } else if callback.effects.encounter.is_some() {
                16
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
        if lever_fixture || target_swap || failure_landing || name.ends_with("-expiry") {
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
            .map_err(|e| format!("callback admission equipment: {e:?}"))?;
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
            .map_err(|e| format!("callback starter items: {e:?}"))?;
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
        if draconia_escape {
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
                    callback.failure_teleport.ok_or("failure landing")?
                },
            )
            .await?;
        } else if sewer {
            super::super::pressure_plate_sewer::exercise(
                &mut owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                trigger,
                name,
            )
            .await?;
        } else if outfit_world {
            super::super::qengine_access::exercise(
                &mut owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                &callback,
                trigger,
            )
            .await?;
        } else if qengine_reward {
            super::super::qengine_rewards::exercise(
                &mut owner,
                &mut admitted,
                &h,
                &authority,
                name,
            )
            .await?;
        } else if callback.effects != Default::default() {
            super::super::qengine_access::exercise(
                &mut owner,
                &mut admitted,
                &h,
                deps.actor,
                fence,
                &callback,
                trigger,
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
                        context.clone(),
                        None,
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
                    stale, fresh_request, catalogue, &permit, context,
                    None,
                ).await,
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
