//! Skulled and Ray of Light through production NPC and object dispatch.
use super::TestResult;
use super::full_world_support::world_scenario;
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};

#[test]
fn skulled_requires_zao_and_premium_then_starts_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx-unnatural-selection", || {
        world_scenario!(
            "qcx_unnatural_selection",
            |h,
             _request,
             _seal,
             authority,
             world_id,
             native,
             world,
             cells,
             room,
             pin,
             deps,
             actor,
             session,
             owner,
             fence,
             admitted| {
                use oteryn_gameplay_quests::quest::{
                    loader::load_embedded_quest_state, unnatural_selection,
                };
                let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
                h.root
                    .latch_premium_activation(
                        &activation.activation_id,
                        activation.switch_over_us,
                        activation.switch_over_us + 60_000_000,
                    )
                    .await
                    .map_err(debug)?;
                let premium = SpellPremiumCoordinator::new(
                    consumer,
                    Some(clock),
                    Some(activation),
                    Some(h.root.clone()),
                );
                owner.quest_catalogue = Some(Arc::new(
                    load_embedded_quest_state("content-1")
                        .map_err(debug)?
                        .catalogue()
                        .clone(),
                ));
                owner.admit_quest_session(&admitted).await;
                let lazaran = install_lazaran(&owner, &admitted).await?;

                // With Premium and no Zao prerequisite, the mission cannot start.
                owner.premium_coordinator = Some(&premium);
                owner.admit_premium(id(40), session);
                assert!(owner.session_premium_gate(session).await.premium_current());
                attempt(&mut owner, &mut admitted, lazaran, 100).await?;
                assert_eq!(stage(&h, unnatural_selection::QUESTLINE_TRACK).await?, None);

                // Seed only the external Zao-access prerequisite.
                let mut tx = h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica")
                    .execute(&mut *tx)
                    .await?;
                sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,3,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice())
                .bind("oteryn:quest-progress/quest/u8_54/the_new_frontier/mission03")
                .bind("oteryn:quest.the_new_frontier_quest")
                .execute(&mut *tx).await?;
                tx.commit().await?;
                owner.refresh_quest_session(session, false).await;

                owner.premium_coordinator = None;
                assert!(!owner.session_premium_gate(session).await.premium_current());
                attempt(&mut owner, &mut admitted, lazaran, 200).await?;
                assert_eq!(stage(&h, unnatural_selection::QUESTLINE_TRACK).await?, None);

                owner.premium_coordinator = Some(&premium);
                let reply = attempt(&mut owner, &mut admitted, lazaran, 300).await?;
                assert_eq!(reply, ["You hero of our tribe if bring back holy skull!"]);
                assert_eq!(
                    stage(&h, unnatural_selection::QUESTLINE_TRACK).await?,
                    Some(1)
                );
                assert_eq!(
                    stage(
                        &h,
                        "oteryn:quest-progress/quest/u8_54/unnatural_selection/mission01"
                    )
                    .await?,
                    Some(1)
                );
                attempt(&mut owner, &mut admitted, lazaran, 400).await?;
                let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-reward/unnatural-selection-skulled-start'")
                .fetch_one(&h.pool).await?;
                assert_eq!(receipts, 1);
                Ok(())
            }
        )
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED qcx_unnatural_selection duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

async fn attempt(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
    first: u64,
) -> TestResult<Vec<String>> {
    use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
    use oteryn_protocol_oteryn::npc_service::NpcTalkIntent;
    let mut reply = Vec::new();
    for (offset, text) in ["hi", "help", "yes", "mission", "yes", "bye"]
        .into_iter()
        .enumerate()
    {
        owner.clock_origin = owner
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(2))
            .ok_or("clock")?;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let outcome = owner
            .npc_talk(
                admitted.runtime_actor.ok_or("actor")?,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id: first + u64::try_from(offset)?,
                    item_fence: Some(fence.item_fence()),
                },
                NpcTalkIntent {
                    npc_actor: npc,
                    text: text.into(),
                },
            )
            .await;
        assert!(
            !outcome.unknown,
            "NPC dispatch must return a known outcome for {text}"
        );
        if offset == 4 {
            reply = outcome
                .conversation
                .map(|conversation| conversation.lines)
                .unwrap_or_default();
        }
    }
    Ok(reply)
}

#[test]
fn ray_of_light_uses_the_native_red_crystal() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run_async(ray);
    eprintln!(
        "POSTGRES_CASE_COMPLETED qcx_unnatural_selection_ray duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

async fn ray(admin: String) -> TestResult {
    use super::full_world_support::*;
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use crate::map::overlay::TilePos;
    use crate::map::view::MapFacts;
    use oteryn_gameplay_quests::quest::{loader::load_embedded_quest_state, unnatural_selection};
    use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

    const CRYSTAL: TilePos = TilePos {
        x: 32982,
        y: 31431,
        floor: -1,
    };
    const START: TilePos = TilePos {
        x: 32981,
        y: 31430,
        floor: -1,
    };
    const MISSION: &str = "oteryn:quest-progress/quest/u8_54/unnatural_selection/mission05";
    reset();
    let h = Harness::create_with_admission_seed(admin, "qcx_unnatural_selection_ray", false, false)
        .await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, activated) =
        super::death_ml_loss_composed::production_generation_with_pin(world_id, "content-1")?;
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native gameplay")?;
    let world = super::source_world_crop::compiled_region(START, CRYSTAL, START, native)?;
    let cells = world
        .movement_cells(activated.qualified_room().movement_cells())
        .map_err(debug)?;
    let frame = world.frame_binding_digest(world_id);
    let start = crate::content::NativeEntryStart {
        x: i32::from(START.x),
        y: i32::from(START.y),
        floor: 1,
    };
    let room = activated
        .qualified_room()
        .bundle_view(cells.clone(), frame, world.map_revision_digest(), start)
        .ok_or("bundle room")?;
    let pin = ChannelContentPin::from_activation(
        world_id,
        1,
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        frame,
        world.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    let mut deps = Dependencies::with_content(pin, room.clone(), native.spell_book().clone())?;
    deps.combat_room = Some(room);
    let mut owner = deps.owner(&h, &authority);
    owner.movement_cells = (&cells).into();
    owner.active_generation = Some(active);
    owner.quest_catalogue = Some(Arc::new(
        load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    ));
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    let session = admitted.game_session_id;
    let fence = super::death_ml_loss_composed::current_fence(&owner, session).await?;
    {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                deps.actor,
                session,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
    }
    track(&owner, &deps)?;
    admitted.item_fence = Some(fence.item_fence());

    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice())
        .bind(active.identity().server_artifact_digest().as_slice())
        .bind(active.identity().client_artifact_digest().as_slice())
        .bind(frame.as_slice()).execute(&h.pool).await?;
    owner.admit_progression_session(&mut admitted).await;
    deps.initialize(&owner).await;

    // Ulala's Mission 5 briefing is a fixture precondition. Only normal USE
    // can advance Mission05 past 1 in this test.
    let catalogue = owner.quest_catalogue.as_ref().ok_or("quest catalogue")?;
    for (ordinal, step) in [(900, "mission05/npc_1"), (901, "questline/npc_9")] {
        let fence = super::death_ml_loss_composed::current_fence(&owner, session).await?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let outcome = slot
            .commit_quest_transition(
                owner.root,
                &authority,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: format!(
                        "oteryn:quest-transition/quest/u8_54/unnatural_selection/{step}"
                    ),
                    cause: QuestCause::Command(CommandRef::new(
                        session,
                        CommandId::new(ordinal).map_err(debug)?,
                    )),
                },
                catalogue.clone(),
            )
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
    }
    owner.admit_quest_session(&admitted).await;
    owner
        .states
        .chat
        .admit(session, "Crystal witness", owner.owner_now());

    let map = owner.bundle_map().ok_or("bundle map")?;
    let key = oteryn_world_bundle::bundle::placement_key(CRYSTAL.floor, CRYSTAL.x, CRYSTAL.y, 2)
        .ok_or("crystal placement")?;
    assert!(
        !map.facts.quest_callback_source_bound(key),
        "retained B3 lacks startup UID 1052"
    );
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let fence = super::death_ml_loss_composed::current_fence(&owner, session).await?;
    let outcome = owner
        .use_object(
            deps.actor,
            UseCommand {
                game_session_id: session,
                command_id: 1,
                item_fence: Some(fence.item_fence()),
            },
            WorldObjectTarget {
                placement,
                expected_revision: map.facts.object_revision(key),
            },
        )
        .await;
    assert_eq!(outcome.disposition, UseDisposition::Committed);
    assert_eq!(stage(&h, MISSION).await?, Some(2));
    assert_eq!(
        stage(&h, unnatural_selection::QUESTLINE_TRACK).await?,
        Some(11)
    );
    Ok(())
}

async fn stage(h: &super::fixture::Harness, track: &str) -> TestResult<Option<i64>> {
    Ok(
        sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
            .bind(track)
            .fetch_optional(&h.pool)
            .await?,
    )
}

async fn install_lazaran(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &super::AdmittedSession,
) -> TestResult<oteryn_protocol_oteryn::world_spatial_entities::EntityRef> {
    use super::full_world_support::*;
    const LAZARAN: &str = "oteryn:npc.lazaran";
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let model = catalogue.service_model_for_project().map_err(debug)?;
    owner
        .states
        .chat
        .install_npc_talk(Arc::new(NpcTalkBook::from_model_with_native_quests(
            &model,
            &[LAZARAN],
        )));
    owner
        .states
        .chat
        .admit(admitted.game_session_id, "Quest witness", owner.owner_now());
    owner.states.chat.set_npc_service(admitted.game_session_id);
    let mut runtime = owner.runtime.lock().await;
    let at = runtime
        .read_actor_position(admitted.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    runtime
        .place_npcs(
            &NpcPlacementTable {
                npcs: vec![NpcPlacements {
                    key: LAZARAN.into(),
                    placements: vec![NpcPlacement {
                        x: u16::try_from(at.x)?,
                        y: u16::try_from(at.y + 1)?,
                        floor: -i8::try_from(at.floor)?,
                        direction: MovementFacing::South,
                    }],
                }],
            },
            |_| true,
            true,
        )
        .map_err(debug)?;
    runtime
        .visible_entities()
        .npcs
        .into_iter()
        .find_map(|npc| {
            (runtime.npc_key(npc.actor) == Some(LAZARAN)).then_some(EntityRef {
                identity: npc.actor.placement_identity(),
                generation: npc.generation,
            })
        })
        .ok_or_else(|| "placed Lazaran".into())
}
