//! Crystal 201189f8 actions_ghost_tear.lua:3-43 consumes one selected tear on the exact statue.
use super::fixture::{Harness, debug, id};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::foundation::GameSessionId;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{self, Callback, Offering, Position};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use serde_json::{Value, json};

pub(super) const NAME: &str = "arthei-ghost-tear";
const MISSION: &str = "oteryn:quest-progress/quest/u8_4/blood_brothers/mission10";
const TEAR: &str = "oteryn:item.tibia.i8746";
const ARTHEI: &str = "oteryn:creature.arthei";
const VAMPIRE: &str = "oteryn:creature.vampire";
const STATUE: Position = Position {
    x: 32953,
    y: 31440,
    floor: -2,
};
const LANDING: Position = Position {
    x: 32953,
    y: 31444,
    floor: -1,
};
const ARTHEI_POSITION: Position = Position {
    x: 32953,
    y: 31441,
    floor: -1,
};

#[test]
fn arthei_statue_consumes_exact_tear_and_moves_two_players_to_spawned_room() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, STATUE, false)
    })
}

fn binding() -> Value {
    json!({
        "key":"oteryn:quest-callback/fixture-arthei-ghost-tear",
        "quest":"oteryn:quest.blood_brothers_quest", "trigger":"use",
        "selector":{"position":STATUE,"appearance":8325,
            "native":{"position":STATUE,"ordinal":2,"appearance":8325}},
        "lever":null,"reversible":false,"teleport":null,"offerings":[],"passages":[],
        "item_on_target":{"item":TEAR,"count":1,"selected_instance":true},
        "return_on_failure":false,"failure_message":null,"requires_world_admission":true,
        "storage":[{"track":MISSION,"minimum":-1,"maximum":1,"set":1,"preserve":true}],
        "effects":{
            "source_must_match":true,"transform":8326,"source_magic":16,
            "player_speech":"YES... THAT FEELING... IS... HUNGER!!",
            "encounter_group":{
                "minimum":{"x":32941,"y":31431,"floor":-2},
                "maximum":{"x":32957,"y":31447,"floor":-2},
                "destination":LANDING
            },
            "stationary_encounter":[
                {"creature":ARTHEI,"position":ARTHEI_POSITION},
                {"creature":VAMPIRE,"position":Position { x:32952, ..ARTHEI_POSITION }},
                {"creature":VAMPIRE,"position":Position { x:32954, ..ARTHEI_POSITION }}
            ]
        },
        "sources":["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/blood_brothers_quest/actions_ghost_tear.lua:3-43"]
    })
}

pub(super) fn callback() -> TestResult<Callback> {
    Ok(serde_json::from_value(binding())?)
}

pub(super) fn world(row: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    super::quest_world_callbacks::world_ordinal(row, STATUE, 8325, 2)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let base = owner.quest_catalogue.as_deref().ok_or("quest catalogue")?;
    let fixture = json!({"schema":1,"callbacks":[binding()]}).to_string();
    owner.quest_catalogue = Some(std::sync::Arc::new(
        world_callbacks::with_fixture_callbacks(base.clone(), &fixture)?,
    ));
    super::qengine_outfit_stories::seed_quest_prerequisites(
        h,
        owner,
        admitted,
        &[(MISSION.into(), 1)],
        123,
    )
    .await?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(debug)?
        .ok_or("quest fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: STATUE,
            item: TEAR.into(),
            count: 1,
        },
        87,
        true,
    )
    .await?;
    let actor = admitted.runtime_actor.ok_or("player actor")?;
    let (peer, tear) = {
        let mut runtime = owner.runtime.lock().await;
        let reference = crate::gameplay_transport::item_ref_admission::pinned_index(
            &runtime,
            owner.active_generation,
        )
        .and_then(|index| index.definition_ref("Item", TEAR, "definition-r1"))
        .ok_or("tear definition")?;
        let session = GameSessionId::decode(&id(238)).map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let peer = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(
                peer,
                super::quest_world_callbacks::local(Position { x: 32954, ..STATUE }),
            )
            .map_err(debug)?;
        (peer, reference)
    };
    let map = owner.bundle_map().ok_or("map")?;
    let key = oteryn_world_bundle::bundle::placement_key(STATUE.floor, STATUE.x, STATUE.y, 2)
        .ok_or("statue placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let intent = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        })),
        use_with_position: None,
    })
    .map_err(debug)?;
    let command = admitted.continuity.next_command_id;
    let (next, outcome, _) =
        super::item_craft::exchange(owner, *admitted, command, &intent, Some(tear)).await?;
    *admitted = next;
    assert_eq!(outcome, UseDisposition::Committed);
    assert_eq!(super::m39_bone_journey::stage(h, MISSION).await?, Some(1));
    assert_eq!(super::m39_bone_journey::carried(h, 8746).await?, 0);
    let spent: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key='oteryn:quest-callback/fixture-arthei-ghost-tear'",
    ).fetch_one(&h.pool).await?;
    assert_eq!(spent, 1);
    let destination = super::quest_world_callbacks::local(LANDING);
    let runtime = owner.runtime.lock().await;
    for member in [actor, peer] {
        assert_eq!(
            runtime
                .read_actor_position(member)
                .map_err(debug)?
                .position(),
            destination
        );
    }
    let creatures = runtime.visible_entities().creatures;
    for (creature, count) in [(ARTHEI, 1), (VAMPIRE, 2)] {
        assert_eq!(
            creatures.iter().filter(|row| runtime.matches_live_creature_identity(row.actor, creature.as_bytes())).count(),
            count
        );
    }
    Ok(())
}
