//! The split Edron machine through the production world-object USE owner.
use super::fixture::{Harness, debug, id, run};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{ChannelContentPin, CommandId, CommandRef, WorldId};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::map::overlay::TilePos;
use crate::map::view::{MapFacts, placement_key};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTransition,
    elemental_spheres::{MACHINE_GEMS, QUEST},
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::sync::Arc;

const LEFT: TilePos = TilePos {
    x: 33268,
    y: 31830,
    floor: -10,
};
const RIGHT: TilePos = TilePos { x: 33269, ..LEFT };
const START: (i32, i32, i16) = (33268, 31831, 10);
const SEED: &str = "oteryn:quest-transition/fixture/sphere-machine-gems";

#[test]
fn espher_sphere_machine_toggles_both_halves_and_spends_twenty_gems_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run("espher-sphere-machine", || {
        run(async |admin| {
            let h =
                Harness::create_with_admission_seed(admin, "espher_sphere_machine", false, false)
                    .await?;
            let result = exercise(&h).await;
            h.cleanup().await?;
            result
        })
    })
}

async fn exercise(h: &Harness) -> TestResult {
    prepare_current_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let world_id = WorldId::decode(&id(42)).map_err(debug)?;
    let (controller, activated) =
        super::death_ml_loss_composed::production_generation_with_pin(world_id, "content-1")?;
    let active = controller.active().ok_or("active generation")?;
    let native = active.native_gameplay().ok_or("native content")?;
    use super::l6_world_pickups::{NativeCrop, native_scene_with_appearances};
    let world = native_scene_with_appearances(
        native,
        vec![
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z10-x129-y124.b3"
                )),
                region: (10, 129, 124),
                bounds: (33267, 31829, 33271, 31833),
                start: (33268, 31831),
                expected: Vec::new(),
            },
            NativeCrop {
                bytes: include_bytes!(concat!(
                    env!("CARGO_MANIFEST_DIR"),
                    "/../../../content/world/placements/region-z13-x130-y125.b3"
                )),
                region: (13, 130, 125),
                bounds: (33330, 32075, 33333, 32078),
                start: (33331, 32076),
                expected: Vec::new(),
            },
        ],
        &[],
        &[846, 847],
    )?;
    let cells = world
        .movement_cells(activated.qualified_room().movement_cells())
        .map_err(debug)?;
    let frame = world.frame_binding_digest(world_id);
    let revision = world.map_revision_digest();
    let room = activated
        .qualified_room()
        .bundle_view(
            cells.clone(),
            frame,
            revision,
            crate::content::NativeEntryStart {
                x: START.0,
                y: START.1,
                floor: START.2,
            },
        )
        .ok_or("bundle room")?;
    let pin = ChannelContentPin::from_activation(
        world_id,
        activated.activation_sequence(),
        active.identity().server_artifact_digest(),
        active.identity().client_artifact_digest(),
        frame,
        revision,
        START,
    );
    let deps = Dependencies::with_content(
        pin,
        activated.qualified_room().clone(),
        crate::spell::cast::v1_spell_book().map_err(debug)?,
    )?;
    let mut admitted = deps.admitted;
    let actor = deps.actor;
    let session = admitted.game_session_id;
    let mut owner = deps.owner(h, &authority);
    owner.active_generation = Some(active);
    owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&room), Some(&room));
    owner.movement_cells = (&cells).into();
    let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(debug)?;
    let catalogue = Arc::new(
        loaded
            .catalogue()
            .with_fixture_transition(QuestTransition {
                key: SEED.into(),
                quest: QUEST.into(),
                completes: false,
                experience: None,
                world_change: None,
                effects: vec![QuestEffect {
                    track: MACHINE_GEMS.into(),
                    from: QuestComparison::Between(-1, 19),
                    effect: QuestEffectKind::Set(23),
                }],
            })
            .map_err(debug)?,
    );
    owner.quest_catalogue = Some(Arc::clone(&catalogue));
    let fence = super::death_ml_loss_composed::current_fence(&owner, session).await?;
    {
        let mut runtime = deps.runtime.lock().await;
        runtime
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        owner
            .states
            .spell_states
            .lock()
            .await
            .initialize(
                &runtime,
                actor,
                session,
                crate::spell::cast::CharacterCastFacts {
                    vocation: crate::spell::Vocation::Druid,
                    ..crate::gameplay_transport::actor_spell::tests::FACTS
                },
                (0, 0),
                owner.owner_now(),
            )
            .ok_or("druid projection")?;
    }
    owner.admit_progression_session(&mut admitted).await;
    let left = machine_key(&owner, LEFT, 842)?;
    let right = machine_key(&owner, RIGHT, 843)?;
    assert_eq!(
        click(&mut owner, &admitted, 710, left).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        (
            machine_appearance(&owner, left)?,
            machine_appearance(&owner, right)?
        ),
        (846, 847)
    );
    assert_eq!(gems(h).await?, -1);
    assert_eq!(
        click(&mut owner, &admitted, 711, right).await?,
        UseDisposition::Committed
    );
    assert_eq!(
        (
            machine_appearance(&owner, left)?,
            machine_appearance(&owner, right)?
        ),
        (842, 843)
    );
    let fence = super::death_ml_loss_composed::current_fence(&owner, session).await?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let seeded = slot
        .commit_quest_transition(
            owner.root,
            &authority,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: SEED.into(),
                cause: QuestCause::Command(CommandRef::new(
                    session,
                    CommandId::new(712).map_err(debug)?,
                )),
            },
            catalogue,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(seeded, QuestTransitionOutcome::Committed(_)),
        "{seeded:?}"
    );
    drop(slot);
    assert_eq!(
        click(&mut owner, &admitted, 713, right).await?,
        UseDisposition::Committed
    );
    assert_eq!(gems(h).await?, 3);
    assert_eq!(
        (
            machine_appearance(&owner, left)?,
            machine_appearance(&owner, right)?
        ),
        (846, 847)
    );
    let position = deps
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position();
    assert_eq!((position.x, position.y, position.floor), (33331, 32076, 13));
    Ok(())
}

async fn gems(h: &Harness) -> TestResult<i64> {
    let value = sqlx::query_scalar(
        "SELECT value FROM game_character_quest_tracks
         WHERE character_id=encode($1,'hex')::uuid AND track_key=$2",
    )
    .bind(id(41).as_slice())
    .bind(MACHINE_GEMS)
    .fetch_optional(&h.pool)
    .await?;
    Ok(value.unwrap_or(-1))
}

fn machine_key(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    at: TilePos,
    appearance: u16,
) -> TestResult<u64> {
    let map = owner.bundle_map().ok_or("machine bundle")?;
    let tile = map
        .overlay
        .base()
        .tile(at.x, at.y, at.floor)
        .ok_or("machine tile")?;
    tile.ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .enumerate()
        .find_map(|(ordinal, (id, _))| {
            let ordinal = u8::try_from(ordinal).ok()?;
            let facts = map.facts.base_entry(at, ordinal, *id)?;
            (facts.appearance_id == appearance)
                .then(|| placement_key(at, ordinal))
                .flatten()
        })
        .ok_or_else(|| format!("missing machine {appearance} at {at:?}").into())
}

fn machine_appearance(owner: &ComposedFreshAdmission<'_, '_, '_>, key: u64) -> TestResult<u16> {
    let map = owner.bundle_map().ok_or("machine bundle")?;
    let (at, ordinal) = crate::map::view::placement_of(key).ok_or("machine placement")?;
    let tile = map
        .overlay
        .base()
        .tile(at.x, at.y, at.floor)
        .ok_or("machine tile")?;
    let (id, _) = tile
        .ids()
        .iter()
        .zip(tile.depths())
        .filter(|(_, depth)| **depth == 0)
        .nth(usize::from(ordinal))
        .ok_or("machine ordinal")?;
    Ok(map
        .facts
        .base_entry(at, ordinal, *id)
        .ok_or("machine facts")?
        .appearance_id)
}

async fn click(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    id: u64,
    key: u64,
) -> TestResult<UseDisposition> {
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let map = owner.bundle_map().ok_or("machine bundle")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let target = WorldObjectTarget {
        placement,
        expected_revision: map.facts.object_revision(key),
    };
    let fence =
        super::death_ml_loss_composed::current_fence(owner, admitted.game_session_id).await?;
    Ok(owner
        .use_object(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: id,
                item_fence: Some(fence.item_fence()),
            },
            target,
        )
        .await
        .disposition)
}
