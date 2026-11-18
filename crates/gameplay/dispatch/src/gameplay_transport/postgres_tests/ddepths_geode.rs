use super::TestResult;
use super::fixture::{Harness, debug, run};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::map::overlay::TilePos;
use crate::map::view::{MapFacts, placement_key};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget,
};

const FRAGMENT: &str = "oteryn:item.tibia.i27509";
const GEODE: TilePos = TilePos {
    x: 33459,
    y: 32267,
    floor: -15,
};
const ENTRANCE: TilePos = TilePos {
    x: 33460,
    y: 32267,
    floor: -15,
};
const LANDING: TilePos = TilePos {
    x: 33650,
    y: 32310,
    floor: -15,
};

#[test]
fn ddepths_geode_use_opens_portal_on_thirtieth_fragment_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    if admin.is_empty() {
        return Err("empty PostgreSQL admin URL".into());
    }
    super::kill_reward_e2e::scenario_thread::run("ddepths-geode-use", || {
        run(async |admin| {
            super::reset();
            let scene = super::srx3_soul_war_world_fixture::source_world(MovementLocalPosition {
                x: 33460,
                y: 32268,
                floor: 15,
            })?;
            let h = Harness::create_with_admission_seed(admin, "ddepths_geode_use", false, false)
                .await?;
            let result = exercise(&h, &scene).await;
            let cleanup = h.cleanup().await;
            result?;
            cleanup
        })
    })
}

async fn exercise(
    h: &Harness,
    scene: &super::srx3_activity_source_fixture::SourceScene,
) -> TestResult {
    let deps = scene.dependencies()?;
    let evidence = super::house_transfer::RecoverySource::start(h).await?;
    super::house_transfer::prepare_owned_session(h).await?;
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let mut owner = deps.owner(h, &authority);
    owner.evidence = &evidence.evidence;
    owner.active_generation = scene.controller.active();
    owner.rooms = crate::gameplay_transport::SeamRooms::new(None, Some(&scene.room));
    owner.quest_catalogue = Some(std::sync::Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?
            .catalogue()
            .clone(),
    ));
    super::boss_activity::issue_content_and_reset(h, &scene.pin).await?;
    let mut admitted = deps.admitted;
    super::srx3_activity_source_fixture::prepare_player(h, &owner, &mut admitted, 1999).await?;
    // Source 27509 is retained but held for production materialization. Seed its exact
    // typed custody as a previously obtained fragment, leaving the real USE owner intact.
    super::item_look::seed_main_backpack(h, &[(FRAGMENT, 31)]).await?;
    let actor = admitted.runtime_actor.ok_or("admitted actor")?;
    let reference = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", FRAGMENT, "definition-r1"))
        .ok_or("retained fragment reference")?;
    let key = placement_key(GEODE, 2).ok_or("retained geode placement")?;

    for command_id in 1..=31 {
        owner.clock_origin -= std::time::Duration::from_millis(500);
        let map = owner.bundle_map().ok_or("published World")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let result = owner
            .tame_mount(
                actor,
                UseCommand {
                    game_session_id: admitted.game_session_id,
                    command_id,
                    item_fence: admitted.item_fence,
                },
                UseIntent {
                    target: UseTarget::ItemByDefinition(reference),
                    use_with: None,
                    use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
                        placement,
                        expected_revision: map.facts.object_revision(key),
                    })),
                    use_with_position: None,
                },
                None,
                None,
                false,
            )
            .await
            .map_err(|()| "native geode USE failed")?;
        assert_eq!(
            result.map(|outcome| outcome.disposition),
            Some(if command_id <= 30 {
                UseDisposition::Committed
            } else {
                UseDisposition::Rejected
            }),
            "geode contribution {command_id}",
        );
        assert_eq!(
            map.facts
                .quest_portal_destination(ENTRANCE, owner.owner_now().get()),
            (command_id >= 30).then_some(LANDING),
        );
    }
    let map = owner.bundle_map().ok_or("published World")?;
    assert_eq!(
        map.facts
            .quest_portal_destination(ENTRANCE, owner.owner_now().get() + 8_000_000),
        None
    );
    let remainder: i64 = sqlx::query_scalar(
        "SELECT quantity FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1",
    ).bind(FRAGMENT).fetch_one(&h.pool).await?;
    assert_eq!(remainder, 1);
    Ok(())
}
