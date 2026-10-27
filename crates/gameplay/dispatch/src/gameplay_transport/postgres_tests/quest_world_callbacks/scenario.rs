use super::*;

pub(super) fn scenario(name: &str, trigger: Position, lever_fixture: bool) -> TestResult {
    let target_swap = name == "fish-napping-key-target-swap";
    let mut callback = callback(if target_swap {
        "fish-napping-key"
    } else {
        name
    })?;
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
    let world = world(&callback, trigger)?;
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "quest_world_callbacks", false, false)
            .await?;
        prepare_current_session(&h).await?;
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
            2,
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
        if lever_fixture || target_swap {
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
        if callback.item_on_target.is_some()
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
