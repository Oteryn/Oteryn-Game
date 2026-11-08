use super::*;

pub(super) fn scenario(case: Case) -> TestResult {
    run(async |admin| {
        let key = case.key();
        let harness =
            Harness::create_with_admission_seed(admin, "novelty_items", true, false).await?;
        super::super::reset();
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                deps.admitted.world_id,
                None,
            );
        let active = controller.active().ok_or("active")?;
        let native = active.native_gameplay().ok_or("native")?;
        let index = native.item_index().ok_or("index")?;
        let oven = "oteryn:item.tibia.i2535";
        let bag = "oteryn:item.tibia.i2854";
        let reference = |key: &str| index.definition_ref("Item", key, "definition-r1");
        let (bytes, pins) = crate::map::boot::tests::bundle_with_item_ids(
            (bag, reference(bag).ok_or("bag")?.get() - 1),
            (oven, reference(oven).ok_or("oven")?.get() - 1),
        );
        let base = crate::map::load(&bytes, &pins).map_err(debug)?;
        let map = crate::map::boot::boot(
            &bytes,
            &crate::map::boot::BootPins {
                map_revision: crate::map::overlay::map_revision(&base),
                bundle: pins,
                start: crate::map::overlay::TilePos {
                    x: 6,
                    y: 0,
                    floor: -7,
                },
            },
            deps.admitted.world_id,
            deps.admitted.channel_id,
            |key| {
                let policy = native.item_policy(key, "definition-r1")?.record();
                let crate::content::ReferenceItemField::Known(physical) =
                    &policy.semantics.physical
                else {
                    return None;
                };
                Some(crate::map::facts::ItemDefinition {
                    reference: reference(key)?,
                    solid: policy.attributes.blocks_movement,
                    blocks_projectile: policy.attributes.blocks_projectile.unwrap_or(true),
                    pickupable: physical.pickupable
                        == crate::content::ReferenceItemField::Known(true),
                    door: false,
                })
            },
        )
        .map_err(debug)?;
        let cells = map
            .movement_cells(pin.qualified_room().movement_cells())
            .map_err(debug)?;
        let bundle_room = super::super::rust_lottery::bundle_action_room(
            pin.qualified_room(),
            &map,
            deps.admitted.world_id,
            &cells,
        )?;
        let content_pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            bundle_room.frame_binding().digest(),
            bundle_room.map_revision_digest(),
            (6, 0, 7),
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
            content_pin,
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
                    x: 6,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?,
                    2,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);
        let mut owner = deps.owner(&harness, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&bundle_room));
        deps.initialize(&owner).await;
        let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(
            harness.root.clone(),
        )
        .current_session_at(deps.admitted.game_session_id)
        .await?
        .0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("fence")?;
        sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1)").bind(id(41).as_slice()).execute(&harness.pool).await?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)").bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&harness.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)").bind(id(42).as_slice()).bind(id(43).as_slice()).bind(pin.identity().server_artifact_digest().as_slice()).bind(pin.identity().client_artifact_digest().as_slice()).bind(pin.frame_binding().digest().as_slice()).execute(&harness.pool).await?;
        if case.nested() {
            super::super::item_look::seed_main_backpack(&harness, &[(key, 1), (bag, 1)]).await?;
        } else {
            super::super::item_look::seed_main_backpack(&harness, &[(key, 1)]).await?;
        }
        let used = crate::gameplay_transport::item_look::typed_definition(
            index,
            reference(key).ok_or("source ref")?,
        )
        .ok_or("source definition")?;
        let seeded = id(71);
        let command_id = if case.nested() { 2 } else { 1 };
        let wanted_voodoo = match case {
            Case::VoodooSuccess | Case::GroundVoodoo => Some(true),
            Case::VoodooFailure => Some(false),
            _ => None,
        };
        let source = if case == Case::DiceStreak {
            (10_000_u32..20_000)
                .map(|nonce| {
                    let mut value = seeded;
                    value[12..].copy_from_slice(&nonce.to_be_bytes());
                    value
                })
                .find(|source| {
                    novelty_items::plan(
                        native,
                        &used,
                        *fence.game_session_id.as_bytes(),
                        command_id,
                        *source,
                    )
                    .is_some_and(|plan| plan.effects["dice_value"] == 6)
                })
                .ok_or("first six fixture")?
        } else if let Some(success) = wanted_voodoo {
            // Keep the first admitted command; choose a valid fixture identity to exercise
            // both exact source probability branches without replacing the real RNG.
            (1_u32..100_000)
                .map(|nonce| {
                    let mut value = seeded;
                    value[12..].copy_from_slice(&nonce.to_be_bytes());
                    value
                })
                .find(|source| {
                    novelty_items::plan(
                        native,
                        &used,
                        *fence.game_session_id.as_bytes(),
                        command_id,
                        *source,
                    )
                    .is_some_and(|plan| plan.effects["achievement"].is_string() == success)
                })
                .ok_or("source outcome")?
        } else {
            seeded
        };
        if source != seeded || case == Case::Hat || case.ground() || case.nested() {
            let mut tx = harness.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            if source != seeded {
                sqlx::query("UPDATE game_item_instances SET item_instance_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid").bind(source.as_slice()).bind(seeded.as_slice()).execute(&mut *tx).await?;
                sqlx::query("UPDATE game_item_container_entries SET item_instance_id=encode($1,'hex')::uuid WHERE item_instance_id=encode($2,'hex')::uuid").bind(source.as_slice()).bind(seeded.as_slice()).execute(&mut *tx).await?;
            }
            if case == Case::Hat {
                sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_character_equipment_slots(character_id,slot,item_instance_id,world_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,1,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid)").bind(fence.character_id.as_bytes().as_slice()).bind(source.as_slice()).bind(id(42).as_slice()).bind(id(79).as_slice()).execute(&mut *tx).await?;
            }
            if case.nested() {
                sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,encode($4,'hex')::uuid)")
                    .bind(source.as_slice()).bind(id(42).as_slice()).bind(id(72).as_slice()).bind(id(79).as_slice()).execute(&mut *tx).await?;
            }
            if case.ground() {
                let target = {
                    let runtime = owner.runtime.lock().await;
                    crate::durability::item_loop_move::LoopGround::from_native_owner(
                        &bundle_room,
                        &runtime,
                        MovementLocalPosition {
                            x: 6,
                            y: 0,
                            floor: 7,
                        },
                        &crate::map::overlay::map_revision(&base),
                    )
                    .map_err(debug)?
                };
                sqlx::query("DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).execute(&mut *tx).await?;
                let source_ref = [
                    crate::durability::item_loop_move::ITEM_LOOP_DROP_REF_PREFIX,
                    id(79).as_slice(),
                ]
                .concat();
                sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,$5,$6,$7,$8,1)")
                    .bind(source.as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(&target.spatial_position).bind(source_ref)
                    .bind(&target.map_revision).bind(&target.content_revision).bind(&target.placement_context).execute(&mut *tx).await?;
            }
            tx.commit().await?;
        }
        let expected = if case.ground() {
            novelty_items::plan_ground(
                native,
                &used,
                *fence.game_session_id.as_bytes(),
                command_id,
                source,
                novelty_items::GroundEffect {
                    x: 6,
                    y: 0,
                    floor: 7,
                },
                false,
            )
            .ok_or("ground source admitted")?
        } else {
            novelty_items::plan(
                native,
                &used,
                *fence.game_session_id.as_bytes(),
                command_id,
                source,
            )
            .ok_or("novelty source admitted")?
        };
        let effects: NoveltyEffects = serde_json::from_value(expected.effects.clone())?;
        let before_vitals = {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            states
                .get(&runtime, deps.actor, fence.game_session_id)
                .ok_or("player")?
                .vitals()
        };
        let far_actor = if case == Case::VoodooFarTarget {
            let mut runtime = owner.runtime.lock().await;
            let session = crate::foundation::GameSessionId::decode(&id(67)).map_err(debug)?;
            let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
            let other = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(
                    other,
                    MovementLocalPosition {
                        x: 10,
                        y: 0,
                        floor: 7,
                    },
                )
                .map_err(debug)?;
            Some(other)
        } else {
            None
        };
        let target = if matches!(
            case,
            Case::VoodooSuccess | Case::VoodooFailure | Case::GroundVoodoo | Case::VoodooFarTarget
        ) {
            let runtime = owner.runtime.lock().await;
            let entities = runtime.visible_entities();
            let me = entities
                .players
                .iter()
                .find(|player| player.actor == far_actor.unwrap_or(deps.actor))
                .ok_or("target player")?;
            Some(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: me.actor.placement_identity(),
                generation: me.generation,
            })
        } else {
            None
        };
        let mut admitted = deps.admitted;
        admitted.runtime_actor = Some(deps.actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        let capabilities: &[u32] = if case.nested() {
            &[4, 6, 11, 12, 14, 15, 18, 350]
        } else {
            &[4, 6, 11, 12, 15, 18, 350]
        };
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(owner.offered_capabilities(), capabilities)
                .ok_or("offers")?;
        assert!(admitted.continuity.selected_capabilities.contains(350));
        assert!(!admitted.continuity.selected_capabilities.contains(23));
        let intent = UseIntent {
            target: UseTarget::Item(std::num::NonZeroU64::new(1).ok_or("handle")?),
            use_with: target,
            use_with_map_item: None,
            use_with_position: None,
        };
        let payload = encode_item_use_intent(&intent).map_err(debug)?;
        if case == Case::PendingLifecycle {
            super::pending::exercise(&owner, &harness, deps.actor, fence, source, intent).await?;
            eprintln!("W19 POSTGRES_CASE_COMPLETED novelty_items case={case:?}");
            drop(owner);
            drop(authority);
            drop(seal);
            return harness.cleanup().await;
        }
        let (mut end, result, actual, wire_cues) =
            super::super::rust_lottery::exchange_with_visuals(
                &owner,
                admitted,
                command_id,
                &payload,
                reference(key),
            )
            .await?;
        let current: (i64, i16) = sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(source.as_slice()).fetch_one(&harness.pool).await?;
        if case.refused() {
            if case == Case::VoodooWithoutTarget {
                assert_eq!(result, UseDisposition::NoTarget);
            } else if case == Case::VoodooFarTarget {
                assert_eq!(result, UseDisposition::TooFar);
            } else {
                assert_ne!(result, UseDisposition::Committed);
            }
            assert_eq!(current, (1, 1));
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_item_consume_transform_records"
                )
                .fetch_one(&harness.pool)
                .await?,
                0
            );
        } else {
            assert_eq!(result, UseDisposition::Committed);
            assert_eq!(
                current,
                if expected.consume_source {
                    (0, 2)
                } else {
                    (1, 1)
                }
            );
            let (recipe, retained, hash_matches): (String, serde_json::Value, bool) = sqlx::query_as("SELECT r.recipe,r.effects,a.envelope_sha256=sha256(a.envelope) AND a.envelope_sha256=r.envelope_sha256 FROM game_item_consume_transform_records r JOIN game_item_audit_outbox a USING(event_id)").fetch_one(&harness.pool).await?;
            assert_eq!(recipe, expected.recipe);
            assert_eq!(retained, expected.effects);
            assert!(hash_matches);
            let durable = harness
                .root
                .read_character_backpack(&authority, fence.character_id)
                .await?
                .ok_or("backpack")?;
            if case.ground() || case.nested() {
                let kind: i32 = sqlx::query_scalar(
                    "SELECT source_kind FROM game_item_consume_transform_records",
                )
                .fetch_one(&harness.pool)
                .await?;
                assert_eq!(kind, if case.ground() { 3 } else { 4 });
                let rows: Vec<(String, i64)> = if case.ground() {
                    sqlx::query_as("SELECT i.definition_production_key,i.quantity FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id) WHERE i.lifecycle=1 ORDER BY g.stack_ordinal").fetch_all(&harness.pool).await?
                } else {
                    sqlx::query_as("SELECT i.definition_production_key,i.quantity FROM game_item_instances i JOIN game_item_nested_entries n USING(item_instance_id) WHERE i.lifecycle=1 ORDER BY n.placement_ordinal").fetch_all(&harness.pool).await?
                };
                if expected.consume_source {
                    assert_eq!(
                        rows,
                        expected
                            .outputs
                            .iter()
                            .map(|(definition, count)| (
                                definition.production_key.clone(),
                                i64::from(*count)
                            ))
                            .collect::<Vec<_>>()
                    );
                } else {
                    assert_eq!(rows, vec![(key.to_owned(), 1)]);
                }
                if case.ground() && !rows.is_empty() {
                    let ordinal: i64 =
                        sqlx::query_scalar("SELECT stack_ordinal FROM game_item_ground_locations")
                            .fetch_one(&harness.pool)
                            .await?;
                    assert_eq!(ordinal, 1);
                    assert_eq!(
                        harness
                            .root
                            .read_item_loop_ground(deps.admitted.world_id, deps.admitted.channel_id)
                            .await?
                            .len(),
                        rows.len()
                    );
                }
            } else if expected.consume_source {
                assert_eq!(durable.entries.len(), expected.outputs.len());
                for (entry, (definition, count)) in durable.entries.iter().zip(&expected.outputs) {
                    assert_eq!(&entry.item.definition, definition);
                    assert_eq!(entry.item.quantity, *count);
                }
            }
        }
        let (vitals, first_cues) = {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            assert!(states.pending_item_effects.is_empty());
            let state = states
                .get(&runtime, deps.actor, fence.game_session_id)
                .ok_or("player after")?;
            if let Some(costume) = &effects.costume {
                assert_eq!(
                    state.conditions.outfit_at(owner.owner_now().get()),
                    Some(costume.look_type)
                );
            }
            let vitals = state.vitals();
            let cues = match states.presentations.as_mut() {
                Some(publisher) => publisher
                    .take_for_current_publisher(&runtime)
                    .map_err(debug)?,
                None => Vec::new(),
            };
            (vitals, cues)
        };
        assert!(
            first_cues.is_empty(),
            "item cues use the live presentation journal"
        );
        let (visual_cursor, visuals) = owner
            .observe_sounds(deps.actor, fence.game_session_id, 0)
            .await
            .ok_or("item presentation")?;
        let actual_cues: Vec<_> = visuals
            .iter()
            .filter_map(|event| match event.kind {
                oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect {
                    effect_id,
                    position,
                    ..
                } => Some((effect_id, position)),
                _ => None,
            })
            .collect();
        let position = effects.ground.as_ref().map_or(
            oteryn_protocol_oteryn::world_spatial::ActorPosition {
                x: 6,
                y: 0,
                floor: 7,
            },
            |ground| oteryn_protocol_oteryn::world_spatial::ActorPosition {
                x: ground.x,
                y: ground.y,
                floor: ground.floor.abs(),
            },
        );
        let expected_cues: Vec<_> = effects
            .cues
            .iter()
            .filter(|_| !case.refused())
            .map(|cue| (u32::from(cue.effect_id), position))
            .collect();
        assert_eq!(
            actual_cues, expected_cues,
            "committed donor cues reach the live observer"
        );
        let wire_ids: Vec<_> = wire_cues
            .iter()
            .filter_map(|event| match event.kind {
                oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect {
                    effect_id,
                    position: at,
                    ..
                } => {
                    assert_eq!(at.x, position.x);
                    assert_eq!(at.y, position.y);
                    assert_eq!(at.floor, position.floor.abs());
                    Some(effect_id)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            wire_ids,
            effects
                .cues
                .iter()
                .filter(|_| !case.refused())
                .map(|cue| u32::from(cue.effect_id))
                .collect::<Vec<_>>(),
            "stationary USE sends the committed visuals over the real connection"
        );

        if case == Case::Rocket {
            assert_eq!(vitals.health, before_vitals.health - 10);
        } else {
            assert_eq!(vitals.health, before_vitals.health);
        }
        if !case.refused() {
            let original = oteryn_protocol_oteryn::world_object::decode_use_intent_target(
                end.continuity.selected_capabilities.as_slice(),
                &actual,
            )
            .map_err(debug)?;
            let replay = owner
                .use_event_item(
                    deps.actor,
                    UseCommand {
                        game_session_id: fence.game_session_id,
                        command_id,
                        item_fence: Some(fence),
                    },
                    original,
                    Some(ItemKey::Instance(source)),
                    true,
                )
                .await
                .map_err(|_| "replay unknown")?
                .ok_or("receipt replay")?;
            assert_eq!(replay.disposition, UseDisposition::Committed);
            let (replayed_end, replayed, _) =
                super::super::rust_lottery::exchange(&owner, end, command_id, &actual, None, None)
                    .await?;
            end = replayed_end;
            assert_eq!(replayed, UseDisposition::Committed);
            assert!(
                owner
                    .observe_sounds(deps.actor, fence.game_session_id, visual_cursor)
                    .await
                    .ok_or("replay visuals")?
                    .1
                    .iter().all(|event| !matches!(event.kind, oteryn_protocol_oteryn::presentation::PresentationEventKind::MagicEffect { .. })),
                "replay emits no item visual twice"
            );
            {
                let runtime = owner.runtime.lock().await;
                let mut states = owner.states.spell_states.lock().await;
                assert_eq!(
                    states
                        .get(&runtime, deps.actor, fence.game_session_id)
                        .ok_or("replay player")?
                        .vitals(),
                    vitals
                );
                if let Some(publisher) = states.presentations.as_mut() {
                    assert!(
                        publisher
                            .take_for_current_publisher(&runtime)
                            .map_err(debug)?
                            .is_empty(),
                        "historical item replay never publishes deferred presentation"
                    );
                }
            }
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_item_consume_transform_records"
                )
                .fetch_one(&harness.pool)
                .await?,
                1
            );
        }
        if case.trumpet() {
            let (duration, delta): (i64, i64) = sqlx::query_as("SELECT p.duration_millis,p.expires_at_unix_ms-r.occurred_at FROM game_spell_item_temporal_schedules p JOIN game_item_consume_transform_records r ON r.transaction_id=p.source_transaction_id").fetch_one(&harness.pool).await?;
            assert_eq!((duration, delta), (4000, 4000));
            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                loop {
                    owner.drain_source_item_deadlines().await;
                    if sqlx::query_scalar::<_, i64>(
                        "SELECT count(*) FROM game_spell_item_temporal_receipts",
                    )
                    .fetch_one(&harness.pool)
                    .await?
                        == 1
                    {
                        break;
                    }
                    tokio::time::sleep(std::time::Duration::from_millis(25)).await;
                }
                Ok::<_, sqlx::Error>(())
            })
            .await??;
            let key: String = sqlx::query_scalar("SELECT i.definition_production_key FROM game_spell_item_temporal_schedules p JOIN game_item_instances i USING(item_instance_id)").fetch_one(&harness.pool).await?;
            assert_eq!(key, "oteryn:item.tibia.i6572");
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_spell_item_temporal_receipts"
                )
                .fetch_one(&harness.pool)
                .await?,
                1
            );
        }
        if case == Case::DiceStreak {
            let mut admitted = end;
            let earned = || {
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/number_of_the_beast'",
                )
            };
            assert_eq!(earned().fetch_one(&harness.pool).await?, 0);
            for (command, face) in [(2_u64, 6_u64), (3, 2), (4, 6), (5, 6), (6, 6)] {
                let start = u32::try_from(command)? * 10_000;
                let source = (start..start + 10_000)
                    .map(|nonce| {
                        let mut value = seeded;
                        value[12..].copy_from_slice(&nonce.to_be_bytes());
                        value
                    })
                    .find(|source| {
                        novelty_items::plan(
                            native,
                            &used,
                            *fence.game_session_id.as_bytes(),
                            command,
                            *source,
                        )
                        .is_some_and(|plan| plan.effects["dice_value"] == face)
                    })
                    .ok_or("source dice face fixture")?;
                let mut tx = harness.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica")
                    .execute(&mut *tx)
                    .await?;
                sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item','oteryn:item.tibia.i5792','definition-r1',1,1,game_character_uuid_v7())")
                    .bind(source.as_slice()).bind(id(42).as_slice()).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_item_container_entries(item_instance_id,world_id,character_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,coalesce(max(placement_ordinal),0)+1,game_character_uuid_v7() FROM game_item_container_entries WHERE parent_item_instance_id=encode($4,'hex')::uuid")
                    .bind(source.as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(id(70).as_slice()).execute(&mut *tx).await?;
                tx.commit().await?;
                let (next, outcome, actual) = super::super::rust_lottery::exchange_latest_source(
                    &owner,
                    admitted,
                    command,
                    &payload,
                    reference(key),
                )
                .await?;
                assert_eq!(outcome, UseDisposition::Committed);
                admitted = next;
                let roll: i64 = sqlx::query_scalar("SELECT (effects->>'dice_value')::bigint FROM game_item_consume_transform_records WHERE command_id=$1::bigint")
                    .bind(i64::try_from(command)?).fetch_one(&harness.pool).await?;
                assert_eq!(roll, i64::try_from(face)?);
                assert_eq!(
                    earned().fetch_one(&harness.pool).await?,
                    i64::from(command == 6)
                );
                if command == 6 {
                    let (_, outcome, _) = super::super::rust_lottery::exchange(
                        &owner, admitted, command, &actual, None, None,
                    )
                    .await?;
                    assert_eq!(outcome, UseDisposition::Committed);
                }
            }
            assert_eq!(earned().fetch_one(&harness.pool).await?, 1);
            assert_eq!(
                sqlx::query_scalar::<_, i64>(
                    "SELECT count(*) FROM game_item_consume_transform_records"
                )
                .fetch_one(&harness.pool)
                .await?,
                6
            );
            let history: Vec<i64> = sqlx::query_scalar("SELECT (effects->>'dice_value')::bigint FROM game_item_consume_transform_records ORDER BY receipt_sequence")
                .fetch_all(&harness.pool).await?;
            assert_eq!(history, [6, 6, 2, 6, 6, 6]);
        }
        eprintln!("W19 POSTGRES_CASE_COMPLETED novelty_items case={case:?}");
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}
