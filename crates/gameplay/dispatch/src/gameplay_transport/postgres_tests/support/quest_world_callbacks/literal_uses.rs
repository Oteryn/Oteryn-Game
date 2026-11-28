//! Literal source USE callbacks must select the clicked object above ground.
use super::*;

pub(in super::super) fn position((x, y, floor): (u16, u16, i8)) -> Position {
    Position { x, y, floor }
}

pub(in super::super) fn exercise(
    callback: Callback,
    appearance: u16,
    ordinal: u8,
    stage: Option<i64>,
    main_era_save: bool,
) -> TestResult {
    exercise_case(callback, appearance, ordinal, stage, main_era_save, None)
}

pub(in super::super) fn postman_mailbox(
    callback: Callback,
    stage: i64,
    selected: &str,
) -> TestResult {
    let native = callback.selector.native.as_ref().ok_or("native mailbox")?;
    let (appearance, ordinal) = (native.appearance, native.ordinal);
    exercise_case(
        callback,
        appearance,
        ordinal,
        Some(stage),
        false,
        Some(selected),
    )
}

fn exercise_case(
    callback: Callback,
    appearance: u16,
    ordinal: u8,
    stage: Option<i64>,
    main_era_save: bool,
    mailbox_selection: Option<&str>,
) -> TestResult {
    let started = std::time::Instant::now();
    let trigger = callback
        .selector
        .position
        .or_else(|| callback.selector.native.as_ref().map(|native| native.position))
        .ok_or("position")?;
    let world = if mailbox_selection.is_some() {
        // The native mailbox in the bundle has no legacy startup action.
        let mut base = callback.clone();
        base.selector.action = None;
        world_ordinal(&base, trigger, appearance, ordinal)?
    } else {
        world_ordinal(&callback, trigger, appearance, ordinal)?
    };
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
            super::super::death_ml_loss_composed_fixtures::production_generation_with_pin(
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
        owner.movement_cells = (&cells).into();
        owner.quest_catalogue = Some(std::sync::Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        {
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

        owner
            .states
            .chat
            .admit(admitted.game_session_id, "Traveller", owner.owner_now());
        let mut historical_hash = None;
        if let Some(stage) = stage {
            use oteryn_gameplay_quests::quest::{
                QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
                QuestTransition,
            };
            let storage = callback.storage.first().ok_or("guard storage")?;
            // Advance the fixture revision before loading a mailbox's persisted
            // predecessor. The actual Postman tracks are owned by their callbacks.
            let (quest, track) = if mailbox_selection.is_some() {
                ("oteryn:quest.fixture_mailbox_setup", "oteryn:quest-progress/fixture/mailbox-setup")
            } else {
                (callback.quest.as_str(), storage.track.as_str())
            };
            let seed = "oteryn:quest-transition/fixture/passage-stage";
            let catalogue = std::sync::Arc::new(
                QuestStateCatalogue::new(
                    "content-1",
                    vec![QuestTrack {
                        key: track.into(),
                        quest: quest.into(),
                        initial: -1,
                        min: -1,
                        max: 13,
                    }],
                    vec![QuestTransition {
                        world_change: None,
                        key: seed.into(),
                        quest: quest.into(),
                        effects: vec![QuestEffect {
                            track: track.into(),
                            from: QuestComparison::Between(-1, 13),
                            effect: QuestEffectKind::Set(stage),
                        }],
                        completes: false,
                        experience: None,
                    }],
                )
                .map_err(debug)?,
            );
            let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
            let seeded = slot
                .commit_quest_transition(
                    owner.root,
                    owner.character,
                    owner.holder,
                    fence,
                    crate::durability::quest_state::QuestTransitionRequest {
                        transition_key: seed.into(),
                        cause: crate::durability::quest_state::QuestCause::Command(
                            crate::foundation::CommandRef::new(
                                admitted.game_session_id,
                                crate::foundation::CommandId::new(900).map_err(debug)?,
                            ),
                        ),
                    },
                    catalogue.clone(),
                )
                .await
                .map_err(|error| format!("seed {}: {error:?}", callback.key))?;
            assert!(matches!(
                seeded,
                crate::durability::quest_state::QuestTransitionOutcome::Committed(_)
            ));
            drop(slot);
            if main_era_save {
                let main = owner.quest_catalogue.as_deref().ok_or("main catalogue")?;
                let active = main
                    .with_fixture_transition(
                        catalogue
                            .transition(seed)
                            .ok_or("fixture transition")?
                            .clone(),
                    )
                    .map_err(debug)?;
                let (active, hash) =
                    if callback.quest == "oteryn:quest.children_of_the_revolution_quest" {
                        // The fully loaded deployed definition at 4740f9a8.
                        // Keep this fixture pinned even when later content is active.
                        (
                            active,
                            [
                                0xe9, 0x1b, 0x65, 0xcb, 0x6e, 0x0a, 0x69, 0xe9, 0xb4, 0x47, 0x3b,
                                0xb8, 0x3d, 0xb9, 0x70, 0xa1, 0x15, 0xbd, 0x63, 0x1a, 0xcb, 0xfa,
                                0xe3, 0x4b, 0x7a, 0x7e, 0x5f, 0x8d, 0x74, 0x0e, 0x52, 0xc2,
                            ],
                        )
                    } else {
                        (
                            active
                                .with_fixture_predecessor(main, &callback.quest)
                                .map_err(debug)?,
                            main.definition_hash(&callback.quest).ok_or("main hash")?,
                        )
                    };
                assert_ne!(active.definition_hash(&callback.quest), Some(hash));
                // Restore the historical pin in both members of the seeded save.
                let mut save = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role = replica")
                    .execute(&mut *save)
                    .await?;
                let character = fence.character_id.as_bytes().as_slice();
                let state = sqlx::query("UPDATE game_character_quest_states SET pinned_content_revision='content-main-era', definition_hash=$1 WHERE quest_key=$2 AND character_id=encode($3,'hex')::uuid")
                    .bind(hash.as_slice()).bind(&callback.quest).bind(character).execute(&mut *save).await?;
                let receipt = sqlx::query("UPDATE game_character_quest_receipts SET pinned_content_revision='content-main-era', definition_hash=$1 WHERE quest_key=$2 AND character_id=encode($3,'hex')::uuid")
                    .bind(hash.as_slice()).bind(&callback.quest).bind(character).execute(&mut *save).await?;
                assert_eq!(state.rows_affected(), 1);
                assert_eq!(receipt.rows_affected(), 1);
                save.commit().await?;
                historical_hash = Some(hash);
                owner.quest_catalogue = Some(std::sync::Arc::new(active));
            } else if mailbox_selection.is_none() {
                owner.quest_catalogue = Some(catalogue);
            }
        }
        let key = oteryn_world_bundle::bundle::placement_key(
            trigger.floor,
            trigger.x,
            trigger.y,
            ordinal,
        )
        .ok_or("key")?;
        let mut placement = owner
            .bundle_map()
            .ok_or("map")?
            .overlay
            .base()
            .digest()
            .to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let target = WorldObjectTarget {
            placement,
            expected_revision: 0,
        };
        if let Some(selected) = mailbox_selection {
            let result = super::super::postman_mailbox_fixtures::exercise(
                &h, &mut owner, admitted, &callback, target, stage.ok_or("mailbox stage")?, selected,
            )
            .await;
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await?;
            return result;
        }
        let before = owner
            .root
            .read_character_quest_state(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        if main_era_save {
            let stored = before
                .states()
                .get(&callback.quest)
                .ok_or("main-era state")?;
            assert!(!stored.completed);
            assert_eq!(stored.pinned_content_revision, "content-main-era");
            assert_eq!(Some(stored.definition_hash), historical_hash);
            assert_eq!(
                before.tracks().get(&callback.storage[0].track).copied(),
                stage
            );
        }
        // xmin also catches a write that assigns the existing value back to a row.
        let quest_rows =
            "SELECT 'state', xmin::text, to_jsonb(s) FROM game_character_quest_states s
            WHERE character_id=encode($1,'hex')::uuid
            UNION ALL SELECT 'track', xmin::text, to_jsonb(t) FROM game_character_quest_tracks t
            WHERE character_id=encode($1,'hex')::uuid
            UNION ALL SELECT 'receipt', xmin::text, to_jsonb(r) FROM game_character_quest_receipts r
            WHERE character_id=encode($1,'hex')::uuid
            ORDER BY 1, 3";
        let character = fence.character_id.as_bytes().as_slice();
        let rows_before: Vec<(String, String, serde_json::Value)> = sqlx::query_as(quest_rows)
            .bind(character)
            .fetch_all(&h.pool)
            .await?;
        let allowed = callback.storage.first().is_none_or(|storage| {
            stage.is_some_and(|stage| (storage.minimum..=storage.maximum).contains(&stage))
        });
        if callback.trigger == oteryn_gameplay_quests::quest::world_callbacks::Trigger::StepIn {
            use oteryn_protocol_oteryn::world_spatial::{
                COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition,
                decode_step_result, encode_step_intent,
            };
            let reply = wire(
                &owner,
                &mut admitted,
                1,
                COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
                encode_step_intent(StepDirection::East),
            )
            .await?;
            assert_eq!(
                decode_step_result(&reply).map_err(debug)?,
                StepDisposition::Moved
            );
        } else {
            let reply = wire(
                &owner,
                &mut admitted,
                1,
                2,
                oteryn_protocol_oteryn::world_object::encode_use_intent(&target).map_err(debug)?,
            )
            .await?;
            assert_eq!(
                oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
                if allowed {
                    UseDisposition::Committed
                } else {
                    UseDisposition::Rejected
                }
            );
        }
        assert_eq!(
            owner
                .runtime
                .lock()
                .await
                .read_actor_position(deps.actor)
                .map_err(debug)?
                .position(),
            if allowed {
                local(callback.teleport.ok_or("destination")?)
            } else {
                origin
            },
            "{} stage {stage:?}",
            callback.key,
        );
        let messages = owner.states.chat.drain(admitted.game_session_id);
        let expected_message = if allowed {
            &callback.success_message
        } else {
            &callback.failure_message
        };
        if let Some(expected_message) = expected_message {
            assert_eq!(messages.iter().filter(|line| matches!(line,
                oteryn_protocol_oteryn::chat::ChatLine::Private {text, ..} if text == expected_message)).count(), 1);
        }
        let after = owner
            .root
            .read_character_quest_state(&authority, fence.character_id)
            .await
            .map_err(debug)?;
        assert_eq!(before, after, "passage changed durable quest state");
        let rows_after: Vec<(String, String, serde_json::Value)> = sqlx::query_as(quest_rows)
            .bind(character)
            .fetch_all(&h.pool)
            .await?;
        assert_eq!(rows_after, rows_before, "passage wrote quest rows");
        eprintln!(
            "QUEST_CALLBACK_PASSAGE_POSTGRES_CASE_COMPLETED {} stage={stage:?} allowed={allowed} tracks_unchanged receipts_unchanged revision_unchanged duration={:?}",
            callback.key,
            started.elapsed()
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
