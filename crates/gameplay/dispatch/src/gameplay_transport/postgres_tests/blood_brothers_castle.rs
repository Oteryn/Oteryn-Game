//! Four session-backed players exercise the actual movement dispatcher on the castle symbols.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CharacterLease, CommandId, CommandRef, MovementLocalPosition};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTransition, blood_brothers_castle as rite,
};
use std::sync::Arc;

#[test]
fn blood_brothers_castle_step_in_checks_four_crystals_and_moves_the_group_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("blood_brothers_castle", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "blood_brothers_castle", false, false)
                    .await?;
            let result = exercise(&h).await;
            let cleanup = h.cleanup().await;
            result?;
            cleanup
        })
    })
}

async fn exercise(h: &Harness) -> TestResult {
    const READY: &str = "oteryn:quest-transition/blood-brothers/castle-test-admission";
    prepare_current_session(h).await?;
    let evidence = super::house_transfer::RecoverySource::start(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, pin) =
        super::death_ml_loss_composed::production_generation_with_pin(world, "content-1")?;
    let generation = controller.active().ok_or("active generation")?;
    let native = generation.native_gameplay().ok_or("native gameplay")?;
    let room = pin.qualified_room().clone();
    let first = rite::SYMBOLS[0];
    let start = (first.x - 1, first.y, first.floor);
    let fixtures = rite::SYMBOLS
        .into_iter()
        .chain(rite::DESTINATIONS)
        .map(|tile| (tile.x, tile.y, tile.floor, 0, 1900))
        .collect::<Vec<_>>();
    let map = super::quest_callbacks::source_world_spatial(start, &fixtures)?;
    let cells = map.movement_cells(room.movement_cells()).map_err(debug)?;
    let position = MovementLocalPosition {
        x: i32::from(start.0),
        y: i32::from(start.1),
        floor: -i16::from(start.2),
    };
    let channel_pin = ChannelContentPin::from_activation(
        world,
        1,
        generation.identity().server_artifact_digest(),
        generation.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        map.map_revision_digest(),
        (position.x, position.y, position.floor),
    );
    let deps =
        Dependencies::with_content_and_capacity(channel_pin, room, native.spell_book().clone(), 8)?;
    deps.runtime
        .lock()
        .await
        .initialize_pinned_test_position(deps.actor, position)
        .map_err(debug)?;
    super::tutorial_vitals_fixture::publish_native_activation(h, &pin).await?;
    super::maintain_ready(&h.root).await?;
    let mut sessions = vec![deps.admitted.game_session_id];
    for index in 0..3 {
        let peer =
            super::desert_party_rite::create_peer(h, &authority, &evidence.evidence, index).await?;
        sessions.push(
            super::activity_deadline_paging::admit_created_member(
                h,
                &evidence.evidence,
                index,
                peer,
            )
            .await?,
        );
    }
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(generation);
    owner.evidence = &evidence.evidence;
    owner.movement_cells = (&cells).into();
    let canonical = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let mission = canonical
        .catalogue()
        .track(rite::MISSION)
        .ok_or("Mission05 track")?;
    let catalogue = Arc::new(
        canonical
            .catalogue()
            .with_fixture_transition(QuestTransition {
                key: READY.into(),
                quest: mission.quest.clone(),
                effects: vec![QuestEffect {
                    track: rite::MISSION.into(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(3),
                }],
                completes: false,
                experience: None,
                world_change: None,
            })
            .map_err(debug)?,
    );
    owner.quest_catalogue = Some(Arc::clone(&catalogue));

    let mut roster = Vec::with_capacity(4);
    let mut initiator = deps.admitted;
    for (index, session) in sessions.into_iter().enumerate() {
        let actor = if index == 0 {
            deps.actor
        } else {
            let mut runtime = owner.runtime.lock().await;
            let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
            let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            let pad = rite::SYMBOLS[index];
            runtime
                .initialize_pinned_test_position(
                    actor,
                    MovementLocalPosition {
                        x: i32::from(pad.x),
                        y: i32::from(pad.y),
                        floor: -i16::from(pad.floor),
                    },
                )
                .map_err(debug)?;
            actor
        };
        let mut admitted = deps.admitted;
        admitted.game_session_id = session;
        admitted.runtime_actor = Some(actor);
        admitted.first_entry = FirstEntryOutcome::Positioned;
        owner.admit_progression_session(&mut admitted).await;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|_| "fence")?
            .ok_or("session fence")?;
        admitted.item_fence = Some(fence.item_fence());
        owner
            .runtime
            .lock()
            .await
            .bind_attacker_lease(
                actor,
                session,
                CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        let outcome = owner
            .revision_sequencer
            .acquire(fence.character_id)
            .await
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: READY.into(),
                    cause: QuestCause::Command(CommandRef::new(
                        session,
                        CommandId::new(500).map_err(debug)?,
                    )),
                },
                Arc::clone(&catalogue),
            )
            .await
            .map_err(debug)?;
        assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
        owner.admit_quest_session(&admitted).await;
        if index == 0 {
            super::desert_party_rite::select_movement(&owner, &mut admitted, false)?;
            initiator = admitted;
        }
        roster.push((actor, *fence.character_id.as_bytes()));
    }
    deps.initialize(&owner).await;
    for (index, &(_, character)) in roster.iter().take(3).enumerate() {
        seed_crystal(h, character, 180 + u8::try_from(index)?).await?;
    }

    use crate::gameplay_transport::connection::regression_routed_step;
    use oteryn_protocol_oteryn::world_spatial::StepDirection;
    let step = regression_routed_step(&owner, initiator, deps.actor, StepDirection::East).await?;
    assert_eq!(
        step.disposition,
        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
    );
    assert_eq!(
        unlocked(h).await?,
        0,
        "one missing crystal rejects the entire roster"
    );
    seed_crystal(h, roster[3].1, 183).await?;
    owner.clock_origin -= std::time::Duration::from_secs(2);
    regression_routed_step(&owner, initiator, deps.actor, StepDirection::West).await?;
    owner.clock_origin -= std::time::Duration::from_secs(2);
    regression_routed_step(&owner, initiator, deps.actor, StepDirection::East).await?;
    assert_eq!(
        unlocked(h).await?,
        4,
        "the fourth crystal unlocks all four players"
    );
    let runtime = owner.runtime.lock().await;
    for ((actor, _), destination) in roster.into_iter().zip(rite::DESTINATIONS) {
        let at = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        assert_eq!(
            (at.x, at.y, at.floor),
            (
                i32::from(destination.x),
                i32::from(destination.y),
                -i16::from(destination.floor)
            )
        );
    }
    drop(runtime);
    let crystals: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1")
        .bind(rite::CHARGED_CRYSTAL).fetch_one(&h.pool).await?;
    assert_eq!(
        crystals, 4,
        "admission checks custody without consuming the crystals"
    );
    Ok(())
}

async fn seed_crystal(h: &Harness, character: [u8; 16], tag: u8) -> TestResult {
    let item = id(tag);
    let world = id(42);
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_character_equipment_state(character_id,revision) VALUES(encode($1,'hex')::uuid,1) ON CONFLICT DO NOTHING")
        .bind(character.as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($1,'hex')::uuid)")
        .bind(item.as_slice()).bind(world.as_slice()).bind(rite::CHARGED_CRYSTAL).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_equipment_slots(character_id,world_id,slot,item_instance_id,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,6,encode($3,'hex')::uuid,encode($3,'hex')::uuid)")
        .bind(character.as_slice()).bind(world.as_slice()).bind(item.as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn unlocked(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_tracks WHERE track_key=$1 AND value=1",
    )
    .bind(rite::ENTRANCE)
    .fetch_one(&h.pool)
    .await?)
}
