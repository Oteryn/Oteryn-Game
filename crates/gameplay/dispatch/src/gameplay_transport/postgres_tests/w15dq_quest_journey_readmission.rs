//! Admitted Paradox Tower and Way of the Monk journeys through successor admission.
//!
//! The client replay handoff is the framed server output from each terminal and successor
//! connection. With E2E_ALL_CAPTURE_DIR set, the scenario writes it only after both agree.
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult,
    fixture::{Harness, debug, id, run},
    l6_paradox_story_fixtures as story, paradox_tower_journey_fixtures as tower,
    prepare_current_session, quest_world_callbacks_fixtures as wire,
};
use crate::{
    content::NativeEntryStart,
    durability::{item_mint::TypedDefinitionRef, starter_grant::StarterTemplate},
    foundation::{ChannelContentPin, ChannelRuntimeV1, NodeId},
    gameplay_transport::{
        SeamRooms,
        capabilities::SelectedCapabilities,
        connection::{
            ControllerBinding, FirstEntryOutcome, FreshAdmissionAuthority, LogoutReply,
            SessionContinuity,
        },
        item_ref_admission::NativeItemWeights,
        last_position::AdmissionPosition,
    },
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt as _;
use oteryn_gameplay_quests::quest::{loader, log, monk_shrines, world_callbacks};
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, MessageType, decode_command_result, decode_snapshot_chunk,
    decode_state_delta, decode_wire_envelope, encode_client_command,
    item_view::{
        CAPABILITY_ITEM_EQUIP_DROP_V1, CAPABILITY_ITEM_VIEW_MOVE_V1,
        STATE_DOMAIN_CHARACTER_INVENTORY, decode_character_inventory_with_equip_drop,
    },
    logout::LogoutResult,
    quest_log::{
        CAPABILITY_QUEST_LOG_V1, COMMAND_TYPE_QUEST_LOG_QUERY, QuestLine, QuestLogQuery,
        QuestLogView, STATE_DOMAIN_QUEST_LOG, decode_quest_log, encode_quest_log_query,
    },
    world_spatial_entities::CAPABILITY_WORLD_SPATIAL_ENTITIES,
};
use std::{path::Path, sync::Arc, time::Duration};

const PARADOX: &str = "oteryn:quest.the_paradox_tower_quest";
const TALONS: &str = "oteryn:quest-callback/paradox-tower/talons";

#[derive(Clone, Copy)]
enum Cohort {
    Paradox,
    Pilgrimage,
}

impl Cohort {
    fn key(self) -> &'static str {
        match self {
            Self::Paradox => PARADOX,
            Self::Pilgrimage => monk_shrines::QUEST,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Paradox => "w15dq_paradox",
            Self::Pilgrimage => "w15dq_pilgrimage",
        }
    }
}

#[test]
fn w15dq_paradox_journey_reward_and_journal_survive_fresh_admission() -> TestResult {
    run_cohort(Cohort::Paradox)
}

#[test]
fn w15dq_monk_pilgrimage_reward_and_journal_survive_fresh_admission() -> TestResult {
    run_cohort(Cohort::Pilgrimage)
}

fn run_cohort(cohort: Cohort) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e_fixtures::scenario_thread::run(cohort.name(), move || {
        run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, cohort.name(), false, false).await?;
            eprintln!("POSTGRES_DATABASE_READY {}", cohort.name());
            let result =
                tokio::time::timeout(Duration::from_secs(180), Box::pin(journey(&h, cohort))).await;
            let cleanup = h.cleanup().await;
            result.map_err(|_| format!("{} journey timed out", cohort.name()))??;
            cleanup
        })
    })
}

async fn journey(h: &Harness, cohort: Cohort) -> TestResult {
    use crate::gameplay_transport::spell_premium_coordinator::{
        SpellPremiumCoordinator, purchase_fixture,
    };

    let initial_request = prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut deps = Dependencies::new()?;
    let world_id = deps.admitted.world_id;
    let channel_id = deps.admitted.channel_id;
    let (controller, activated) =
        super::death_ml_loss_composed_fixtures::production_generation_with_pin(
            world_id,
            "content-1",
        )?;
    let generation = controller.active().ok_or("active generation")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let (world, start) = match cohort {
        Cohort::Paradox => (tower::world(native)?, story::start()),
        Cohort::Pilgrimage => {
            let guidance = wire::callback("monk-guidance")?;
            let trigger = guidance.selector.position.ok_or("guidance placement")?;
            let start = world_callbacks::Position {
                x: trigger.x - 1,
                ..trigger
            };
            (wire::world(&guidance, trigger)?, start)
        }
    };
    let start = wire::local(start);
    let cells = world
        .movement_cells(activated.qualified_room().movement_cells())
        .map_err(debug)?;
    let frame = match cohort {
        Cohort::Paradox => world.frame_binding_digest(world_id),
        Cohort::Pilgrimage => activated.frame_binding().digest(),
    };
    let map = world.map_revision_digest();
    let room = activated
        .qualified_room()
        .bundle_view(
            cells.clone(),
            frame,
            map,
            NativeEntryStart {
                x: start.x,
                y: start.y,
                floor: start.floor,
            },
        )
        .ok_or("cohort bundle room")?;
    let content = ChannelContentPin::from_activation(
        world_id,
        activated.activation_sequence(),
        generation.identity().server_artifact_digest(),
        generation.identity().client_artifact_digest(),
        frame,
        map,
        (start.x, start.y, start.floor),
    );
    let mut runtime = ChannelRuntimeV1::from_committed_assignment(
        world_id,
        channel_id,
        NodeId::decode(&id(1)).map_err(debug)?,
        1,
        1,
        1,
        "runtime-scope-assignment:1",
        8, // One player, six NPCs and the timed Ghoul.
        content,
    )
    .map_err(debug)?;
    runtime.use_classic_creature_deaths().map_err(debug)?;
    native
        .install_companion_policies(&mut runtime)
        .map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(deps.admitted.game_session_id)
        .map_err(debug)?;
    deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime
        .initialize_pinned_test_position(deps.actor, start)
        .map_err(debug)?;
    deps.runtime = tokio::sync::Mutex::new(runtime);

    let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
    h.root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        )
        .await?;
    let premium = SpellPremiumCoordinator::new(
        consumer,
        Some(clock),
        Some(activation),
        Some(h.root.clone()),
    );
    let loaded = loader::load_embedded_quest_state("content-1").map_err(debug)?;
    let journal = log::loader::load_embedded_quest_log(&loaded)?;
    let quest_index = u32::try_from(
        journal
            .content
            .active()
            .quests()
            .iter()
            .position(|quest| quest.key == cohort.key())
            .ok_or("cohort journal index")?,
    )?;

    let mut owner = deps.owner(h, &authority);
    owner.premium_coordinator = Some(&premium);
    owner.active_generation = Some(generation);
    owner.rooms = SeamRooms::new(
        matches!(cohort, Cohort::Paradox).then_some(&room),
        Some(&room),
    );
    owner.movement_cells = (&cells).into();
    owner.quest_catalogue = Some(Arc::new(loaded.catalogue().clone()));
    owner.quest_log_content = Some(
        crate::gameplay_transport::quest_log::QuestLogLines::new(journal.content)
            .ok_or("cohort journal bounds")?,
    );

    let mut admitted = deps.admitted;
    let session = admitted.game_session_id;
    let fence = owner
        .current_quest_fence_durable(session)
        .await?
        .ok_or("first quest fence")?;
    let store = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone());
    let current = store.current_session(session).await?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(deps.actor, session, current.current_character_lease())
        .map_err(debug)?;
    admitted.runtime_actor = Some(deps.actor);
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence = Some(fence.item_fence());

    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(generation.identity().server_artifact_digest().as_slice())
        .bind(generation.identity().client_artifact_digest().as_slice())
        .bind(frame.as_slice()).execute(&h.pool).await?;
    h.root
        .initialize_admission_equipment(
            &authority,
            &h.node,
            &fence.item_fence(),
            generation.identity().server_artifact_digest(),
            initial_request.operation(),
        )
        .await
        .map_err(debug)?;
    h.root
        .grant_starter_items(
            &authority,
            &h.node,
            &fence.item_fence(),
            &NativeItemWeights(native),
            &StarterTemplate {
                definition: TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: "oteryn:item.tibia.i2854".into(),
                    revision_ref: "definition-r1".into(),
                },
                ..StarterTemplate::main()
            },
        )
        .await?;
    owner.admit_progression_session(&mut admitted).await;
    deps.initialize(&owner).await;
    owner.admit_premium(id(40), session);
    assert!(owner.session_premium_gate(session).await.premium_current());
    owner.admit_quest_session(&admitted).await;
    assert!(owner.track_restored(session, deps.actor, &AdmissionPosition::FirstLogin, fence));

    let initial = h
        .root
        .read_character_quest_state(&authority, fence.character_id)
        .await?;
    assert!(
        !initial.states().contains_key(cohort.key()),
        "cohort starts without quest progress"
    );
    let before = tower::carried(&owner, &admitted, h).await?;
    // Reuse the admitted journeys' entry, prerequisite, terminal and replay assertions.
    match cohort {
        Cohort::Paradox => {
            Box::pin(tower::exercise(
                &mut owner,
                &mut admitted,
                h,
                &authority,
                false,
            ))
            .await?
        }
        Cohort::Pilgrimage => {
            Box::pin(super::monk_shrines_fixtures::exercise(
                &mut owner,
                &mut admitted,
                h,
                &authority,
                &world,
                false,
                false,
            ))
            .await?
        }
    }
    let after = tower::carried(&owner, &admitted, h).await?;
    let terminal = h
        .root
        .read_character_quest_state(&authority, fence.character_id)
        .await?;
    assert!(
        terminal
            .states()
            .get(cohort.key())
            .is_some_and(|quest| quest.completed)
    );
    let talons = if matches!(cohort, Cohort::Paradox) {
        let callbacks = world_callbacks::load_for(loaded.catalogue())?;
        let reward = callbacks
            .iter()
            .find(|callback| callback.key == TALONS && callback.completes)
            .and_then(|callback| callback.item_reward.as_ref())
            .ok_or("Paradox terminal reward binding")?;
        let quantity = |items: &[(String, i64)]| {
            items
                .iter()
                .filter(|(item, _)| item == &reward.item)
                .map(|(_, count)| count)
                .sum::<i64>()
        };
        assert_eq!(
            quantity(&after) - quantity(&before),
            i64::from(reward.count)
        );
        let definition = native
            .item_index()
            .ok_or("Item index")?
            .definition_ref("Item", &reward.item, "definition-r1")
            .ok_or("Talon item reference")?;
        Some((definition, quantity(&after)))
    } else {
        assert_eq!(terminal.tracks().get(monk_shrines::FINAL_TRACK), Some(&1));
        None
    };

    let (terminal_line, _, terminal_capture) = readback(&owner, admitted, quest_index).await?;
    assert!(
        terminal_line.completed,
        "terminal quest line did not report completion"
    );

    owner.clock_origin = owner
        .clock_origin
        .checked_sub(Duration::from_secs(120))
        .ok_or("logout clock")?;
    assert_eq!(
        owner.logout_session(admitted).await,
        LogoutReply::Result(LogoutResult::Accepted)
    );
    let (session, request) = super::full_world_support::relogin_request(h).await?;
    assert_ne!(session, admitted.game_session_id);
    let current = store.current_session(session).await?;
    assert!(current.current_character_lease().generation() > fence.character_lease_generation);
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        runtime.commit_fresh_session(reservation).map_err(debug)?
    };
    let transport = current.current_transport().ok_or("successor transport")?;
    let account_id = admitted.controller.ok_or("initial controller")?.account_id;
    owner.admit_premium(account_id, session);
    let (positioned, item_fence, position) =
        Box::pin(owner.initialize_first_entry(&request, session, transport, actor)).await;
    assert!(
        matches!(
            positioned,
            FirstEntryOutcome::Positioned | FirstEntryOutcome::Reconciled
        ),
        "successor first entry: {positioned:?}"
    );
    let position = position.ok_or("successor position")?;
    let new_quest_fence = owner
        .current_quest_fence_durable(session)
        .await?
        .ok_or("successor quest fence")?;
    assert!(owner.track_restored(session, actor, &position, new_quest_fence));
    let item_fence = item_fence.ok_or("successor item fence")?;
    assert_eq!(item_fence.character_id, fence.character_id);
    let mut next = AdmittedSession {
        game_session_id: session,
        runtime_actor: Some(actor),
        first_entry: positioned,
        item_fence: Some(item_fence),
        controller: Some(ControllerBinding {
            transport,
            account_id,
        }),
        continuity: SessionContinuity::FRESH,
        ..admitted
    };
    owner.admit_progression_session(&mut next).await;
    owner.admit_quest_session(&next).await;
    assert_eq!(tower::carried(&owner, &next, h).await?, after);

    let fresh = h
        .root
        .read_character_quest_state(&authority, item_fence.character_id)
        .await?;
    assert_eq!(
        fresh.states().get(cohort.key()),
        terminal.states().get(cohort.key())
    );
    assert_eq!(fresh.tracks(), terminal.tracks());

    let (line, inventory, capture) = readback(&owner, next, quest_index).await?;
    assert_eq!(
        line, terminal_line,
        "successor quest line lost terminal progress"
    );
    match cohort {
        Cohort::Paradox => {
            for mission in [
                "The Feared Hugo",
                "Favorite colour: Green",
                "The Secret of Mathemagics",
            ] {
                assert!(
                    line.missions
                        .iter()
                        .any(|row| row.name == mission && row.done),
                    "{mission}"
                );
            }
            let (definition, expected) = talons.ok_or("expected Talon reward")?;
            let count: i64 = inventory
                .entries
                .iter()
                .filter(|item| item.item_definition_ref == definition)
                .map(|item| i64::from(item.count.get()))
                .sum();
            assert_eq!(count, expected, "successor inventory snapshot");
        }
        Cohort::Pilgrimage => {
            let (report, shrines) = line.missions.split_first().ok_or("pilgrimage missions")?;
            assert!(report.done && report.text.contains("reported"));
            assert_eq!(shrines.len(), monk_shrines::NAMES.len());
            assert!(shrines.iter().all(|shrine| shrine.done));

            use oteryn_gameplay_progression::{
                wheel::{WheelRuleset, available_points},
                wheel_gem_data::WheelGemData,
            };
            let data = WheelGemData::embedded().map_err(debug)?;
            let ruleset = Arc::new(
                WheelRuleset::from_catalogue(data.revision(), data.wheel()).map_err(debug)?,
            );
            let wheel = h
                .root
                .read_wheel_presentation(&authority, &h.node, item_fence, ruleset, &premium)
                .await?;
            assert_eq!(
                wheel.budget.unlock_points,
                available_points(wheel.level)
                    + monk_shrines::final_points().ok_or("pilgrimage reward points")?,
                "successor Wheel budget includes the final award exactly once",
            );
        }
    }
    assert_eq!(
        owner.logout_session(next).await,
        LogoutReply::Result(LogoutResult::Accepted)
    );
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        for (phase, bytes) in [("terminal", terminal_capture), ("fresh", capture)] {
            std::fs::write(
                Path::new(&root).join(format!("{}-{phase}.wire", cohort.name())),
                bytes,
            )?;
        }
    }
    Ok(())
}

async fn readback(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: AdmittedSession,
    index: u32,
) -> TestResult<(
    QuestLine,
    oteryn_protocol_oteryn::item_view::CharacterInventory,
    Vec<u8>,
)> {
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        owner.offered_capabilities(),
        &[
            CAPABILITY_ITEM_VIEW_MOVE_V1,
            CAPABILITY_WORLD_SPATIAL_ENTITIES,
            CAPABILITY_ITEM_EQUIP_DROP_V1,
            CAPABILITY_QUEST_LOG_V1,
        ],
    )
    .ok_or("successor journal/inventory capabilities")?;
    let command_id = admitted.continuity.next_command_id;
    let command = encode_client_command(
        admitted.continuity.connection_generation,
        &ClientCommandValue {
            command_id,
            command_type: COMMAND_TYPE_QUEST_LOG_QUERY,
            payload: &encode_quest_log_query(&QuestLogQuery::Quest { quest_index: index })
                .map_err(debug)?,
        },
    )
    .map_err(debug)?;
    let mut framed = u32::try_from(command.len())?.to_be_bytes().to_vec();
    framed.extend(command);
    let (_, output) =
        super::weather_wire_fixtures::fixture_access::serve_input(owner, admitted, &framed).await?;
    let mut frames = output.as_slice();
    let mut line = None;
    let mut inventory = None;
    let mut accepted = false;
    while !frames.is_empty() {
        let bytes = crate::gameplay_transport::tcp_tls::read_frame(&mut frames).await?;
        let envelope = decode_wire_envelope(&bytes).map_err(debug)?;
        match envelope.message_type() {
            MessageType::SnapshotChunk => {
                let (_, domains) = decode_snapshot_chunk(envelope.payload()).map_err(debug)?;
                for domain in domains {
                    if domain.domain_id == STATE_DOMAIN_CHARACTER_INVENTORY {
                        inventory = Some(
                            decode_character_inventory_with_equip_drop(domain.payload, true)
                                .map_err(debug)?,
                        );
                    }
                }
            }
            MessageType::CommandResult => {
                let reply = decode_command_result(envelope.payload()).map_err(debug)?;
                assert_eq!(reply.command_id, command_id);
                accepted = reply.status == CommandStatus::Accepted;
            }
            MessageType::StateDelta => {
                let delta = decode_state_delta(envelope.payload()).map_err(debug)?;
                if delta.domain_id == STATE_DOMAIN_QUEST_LOG
                    && let Some(QuestLogView::Line(view)) =
                        decode_quest_log(delta.payload).map_err(debug)?.view
                {
                    assert_eq!(view.quest_index, index);
                    line = Some(view);
                }
            }
            _ => {}
        }
    }
    assert!(
        accepted,
        "successor quest log query did not receive Accepted"
    );
    Ok((
        line.ok_or("successor quest line delta")?,
        inventory.ok_or("successor inventory snapshot")?,
        output,
    ))
}
