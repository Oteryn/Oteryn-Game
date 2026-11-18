//! Crystal's level-only entrance through the ordinary movement owner.
use super::fixture::{Harness, debug, id};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::{CharacterId, CharacterLease, MovementLocalPosition};
use crate::map::{boot, overlay::TilePos};
use oteryn_gameplay_quests::quest::world_callbacks::Callback;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};

#[test]
fn feaster_entrance_checks_level_without_quest_state() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run_async(exercise);
    eprintln!(
        "POSTGRES_CASE_COMPLETED feaster_entrance duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

async fn exercise(admin: String) -> TestResult {
    let h = Harness::create_with_admission_seed(admin, "feaster_level", true, false).await?;
    prepare_current_session(&h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let initial = Dependencies::new()?;
    let world_id = initial.admitted.world_id;
    let (controller, activated) =
        super::death_ml_loss_composed::production_generation_with_pin(world_id, "content-1")?;
    let active = controller.active().ok_or("active generation")?;
    let room = activated.qualified_room();
    let (bytes, pins) = boot::tests::bundle();
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let world = boot::boot(
        &bytes,
        &boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: TilePos {
                x: 1,
                y: 0,
                floor: -7,
            },
        },
        world_id,
        initial.admitted.channel_id,
        boot::tests::items,
    )
    .map_err(debug)?;
    let cells = world.movement_cells(room.movement_cells()).map_err(debug)?;
    for (level, expected_x) in [(249_i64, 1), (250, 3)] {
        // Seed boundary levels using the same isolated transaction as the boss activity cases.
        let mut seed = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *seed)
            .await?;
        let changed = sqlx::query(
            "UPDATE game_character_progression_state SET level=$1,total_experience=$2 \
             WHERE character_id=encode($3,'hex')::uuid",
        )
        .bind(level)
        .bind((level - 1) * (50 * level * level - 250 * level + 600) / 3)
        .bind(id(41).as_slice())
        .execute(&mut *seed)
        .await?;
        assert_eq!(changed.rows_affected(), 1);
        seed.commit().await?;
        let pin = ChannelContentPin::from_activation(
            world_id,
            1,
            active.identity().server_artifact_digest(),
            active.identity().client_artifact_digest(),
            world.frame_binding_digest(world_id),
            world.map_revision_digest(),
            (1, 0, 7),
        );
        let deps = Dependencies::with_content(pin, room.clone(), initial.spells.clone())?;
        let origin = deps
            .runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                MovementLocalPosition {
                    x: 1,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        owner.movement_cells = (&cells).into();
        assert!(
            owner
                .states
                .quest_callbacks
                .fixture
                .set(vec![portal()?])
                .is_ok()
        );
        let session = deps.admitted.game_session_id;
        let fence = owner
            .current_quest_fence(session)
            .await
            .map_err(|()| "session fence")?
            .ok_or("current session fence")?;
        deps.runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                session,
                CharacterLease::new(
                    CharacterId::decode(fence.character_id.as_bytes()).map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        deps.initialize(&owner).await;
        let (step, _) = owner
            .ordinary_paced_step(
                deps.actor,
                deps.admitted.game_session_id,
                StepDirection::East,
                Some(origin),
                None,
            )
            .await;
        assert_eq!(step.disposition, StepDisposition::Moved);
        let position = deps
            .runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?;
        assert_eq!(position.position().x, expected_x, "level {level}");
    }
    let rows: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM game_character_quest_states), \
                (SELECT count(*) FROM game_character_quest_receipts)",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(rows, (0, 0));
    drop(authority);
    drop(seal);
    h.cleanup().await
}

fn portal() -> TestResult<Callback> {
    Ok(serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/feaster-level-entrance-fixture",
        "quest": "oteryn:quest.feaster_of_souls_quest",
        "trigger": "step_in",
        "selector": {"position":{"x":2,"y":0,"floor":-7},"ordinal":0},
        "lever": null,
        "reversible": false,
        "passages": [],
        "storage": [],
        "teleport": {"x":3,"y":0,"floor":-7},
        "effects": {"teleport_effect": 11},
        "offerings": [],
        "minimum_level": 250,
        "return_on_failure": true,
        "failure_message": "You need at least level 250 to enter.",
        "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/feaster_of_souls/actions_entrances.lua:17-33"]
    }))?)
}
