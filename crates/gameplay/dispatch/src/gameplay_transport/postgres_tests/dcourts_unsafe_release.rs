//! The production action-ID listener runs after STEP and checks the current PostgreSQL bait guard.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::map::{boot, overlay::TilePos};
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use oteryn_world_bundle::bundle;
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.the_dream_courts_quest";
const BAIT: &str = "oteryn:quest-progress/quest/u12_00/the_dream_courts/unsafe_release/has_bait";
const START: TilePos = TilePos { x: 1, y: 0, floor: -7 };

#[test]
fn dcourts_step_summons_lucifuga_only_with_bait_and_respects_cooldown() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            // The shared runner supplies the same configured admin route.
            assert!(!admin.is_empty());
            run(async |admin| {
                let h = Harness::create_with_admission_seed(admin, "dcourts_bait", true, false).await?;
                let result = exercise(&h).await;
                h.cleanup().await?;
                result
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Unsafe Release PostgreSQL thread panicked")?
        .map_err(Into::into)
}

async fn exercise(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
    let base_deps = Dependencies::new()?;
    let world_id = base_deps.admitted.world_id;
    let channel_id = base_deps.admitted.channel_id;
    let (controller, activation) =
        super::death_ml_loss_composed::production_generation_with_pin(world_id, "content-1")?;
    let active = controller.active().ok_or("active generation")?;
    let (bytes, mut pins) = boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    // The test introduces Crystal's action ID on a walkable exercise tile.
    // This is a test placement, not a claim about the retained World map.
    let tile = data.sectors.iter_mut().flat_map(|sector| &mut sector.tiles)
        .find(|tile| tile.x == 2 && tile.y == START.y).ok_or("step fixture tile")?;
    tile.items[0].attrs.action = Some(23120);
    let bytes = bundle::write(&data.manifest, &data.sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    let world = boot::boot(
        &bytes, &boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins, start: START,
        },
        world_id, channel_id,
        |key| Some(crate::map::facts::ItemDefinition {
            reference: std::num::NonZeroU32::new(match key {
                "item:box" => 1, "item:coin" => 2, _ => return None,
            })?,
            solid: Some(key == "item:box"),
            blocks_projectile: false,
            pickupable: key == "item:coin",
            door: false,
        }),
    ).map_err(debug)?;
    let qualified = activation.qualified_room();
    let cells = world.movement_cells(qualified.movement_cells()).map_err(debug)?;
    let frame = world.frame_binding_digest(world_id);
    let room = qualified.bundle_view(
        cells.clone(), frame, world.map_revision_digest(),
        crate::content::NativeEntryStart { x: 1, y: 0, floor: 7 },
    ).ok_or("bundle room")?;
    let pin = ChannelContentPin::from_activation(
        world_id, 1, active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(), frame, world.map_revision_digest(), (1, 0, 7),
    );
    let deps = Dependencies::with_content_and_capacity(pin, room, base_deps.spells, 8)?;
    let actor = deps.actor;
    let session = deps.admitted.game_session_id;
    let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
        .current_session_at(session).await?.0;
    {
        let mut runtime = deps.runtime.lock().await;
        runtime.bind_attacker_lease(actor, session, current.current_character_lease()).map_err(debug)?;
        runtime.initialize_pinned_test_position(actor, MovementLocalPosition { x: 1, y: 0, floor: 7 })
            .map_err(debug)?;
    }

    sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(active.identity().server_artifact_digest().as_slice())
        .bind(active.identity().client_artifact_digest().as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    owner.movement_cells = (&cells).into();
    owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&deps.room), Some(&deps.room));
    owner.quest_catalogue = Some(Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?.catalogue().clone(),
    ));
    let mut admitted = deps.admitted;
    admitted.first_entry = FirstEntryOutcome::Positioned;
    owner.admit_progression_session(&mut admitted).await;
    let step = |direction| owner.paced_step(actor, session, direction);
    assert_eq!(step(StepDirection::East).await.0.disposition, StepDisposition::Moved);
    assert!(owner.runtime.lock().await.current_live_creature_census().map_err(debug)?.is_empty());
    assert_eq!(step(StepDirection::West).await.0.disposition, StepDisposition::Moved);

    sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision)
        SELECT character_id,$2,$3,1,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).bind(BAIT).bind(QUEST).execute(&h.pool).await?;
    assert_eq!(step(StepDirection::East).await.0.disposition, StepDisposition::Moved);
    {
        let mut runtime = owner.runtime.lock().await;
        let census = runtime.current_live_creature_census().map_err(debug)?;
        let [spider] = census.as_slice() else {
            return Err(format!("expected one spider, got {census:?}").into());
        };
        assert_eq!(runtime.read_actor_position(*spider).map_err(debug)?.position().x, 2);
        runtime.remove_encounter_actor(*spider).map_err(debug)?;
    }
    assert_eq!(step(StepDirection::West).await.0.disposition, StepDisposition::Moved);
    assert_eq!(step(StepDirection::East).await.0.disposition, StepDisposition::Moved);
    assert!(owner.runtime.lock().await.current_live_creature_census().map_err(debug)?.is_empty());
    let bait: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1")
        .bind(BAIT).fetch_one(&h.pool).await?;
    assert_eq!(bait, 1);
    Ok(())
}
