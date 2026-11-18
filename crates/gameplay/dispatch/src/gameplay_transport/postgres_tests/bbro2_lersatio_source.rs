//! Mirror USE, timed repair, area admission and the retained native return portal.
use super::annihilator_route::{position, staged_step as step, use_target};
use super::fixture::debug;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::content::native_gameplay::NativeGameplayState;
use crate::map::{boot::BundleMap, overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, authored_bindings::Bindings, world_callbacks::Position,
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use oteryn_protocol_oteryn::world_spatial::StepDirection;

const QUEST: &str = "oteryn:quest.blood_brothers_quest";
const COUNTER: &str =
    "oteryn:quest-progress/crystalserver/quest/u8_4/blood_brothers/mirror_counter";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/blood_brothers_quest/actions_lersatio_mirrors.lua:3-79";
const MIRROR: Position = Position {
    x: 32965,
    y: 31457,
    floor: -2,
};
const STATUE: Position = Position {
    x: 32967,
    y: 31457,
    floor: -2,
};
pub(super) const START: Position = Position {
    x: 32965,
    y: 31458,
    floor: -2,
};

#[test]
fn lersatio_repaired_mirror_uses_admit_two_players_and_native_exit_returns_both() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::annihilator_route::blood_brothers_mirror()
}

pub(super) fn catalogue(revision: &str) -> TestResult<QuestStateCatalogue> {
    let stage = |counter: u8| {
        let encounter = (counter == 1).then(|| serde_json::json!({
            "minimum_level":0, "players":[],
            "staging_area":{
                "minimum":{"x":32954,"y":31447,"floor":-2},
                "maximum":{"x":32972,"y":31465,"floor":-2},
                "destination":{"x":32967,"y":31459,"floor":-1}
            },
            "room_min":{"x":32965,"y":31458,"floor":-1},
            "room_max":{"x":32969,"y":31463,"floor":-1},
            "monsters":[
                {"position":{"x":32967,"y":31459,"floor":-1},"creature":"oteryn:creature.lersatio"},
                {"position":{"x":32966,"y":31459,"floor":-1},"creature":"oteryn:creature.vampire"},
                {"position":{"x":32968,"y":31459,"floor":-1},"creature":"oteryn:creature.vampire"}
            ]
        }));
        serde_json::json!({
            "key":if counter == 0 { "first-mirror" } else { "second-mirror" },
            "completes":false,
            "trigger":{"kind":"map_use","callback":{
                "key":format!("oteryn:quest-callback/bbro2-lersatio-mirror-{counter}"),
                "quest":QUEST,
                "trigger":"use",
                "selector":{"native":{"position":MIRROR,"ordinal":2,"appearance":8667}},
                "reversible":false,"return_on_failure":false,"requires_world_admission":true,
                "passages":[],"offerings":[],
                "storage":[{"track":COUNTER,"minimum":counter,"maximum":counter,"set":1-counter}],
                "world_requirements":[{"position":MIRROR,"appearance":8667}],
                "effects":{
                    "transform":8721,
                    "reset_after_ms":30_000,
                    "target_transform":{"position":STATUE,"from":8325,"to":8326,"if_present":true},
                    "encounter":encounter
                },
                "sources":[SOURCE]
            }}
        })
    };
    let document = serde_json::json!({"schema":1,"quests":[{
        "quest":QUEST,
        "recipe_sha256":"cdabe288fd28476a9a10c823ce20c27abaec70bb193039480b6148c45a3c4c65",
        "sources":[SOURCE],
        "tracks":[{"key":COUNTER,"initial":0,"minimum":0,"maximum":1}],
        "stages":[stage(0),stage(1)]
    }]});
    let base = QuestStateCatalogue::empty(revision).ok_or("quest catalogue")?;
    Ok(Bindings::parse(&serde_json::to_vec(&document)?)?.attach(&base)?)
}

pub(super) fn world(native: &NativeGameplayState) -> TestResult<crate::map::boot::BundleWorld> {
    use super::l6_world_pickups::{
        ExpectedTeleport, ExpectedTile, NativeCrop, native_scene_with_appearances,
    };
    const FLOOR_1: &[u8] =
        include_bytes!("../../../../../../content/world/placements/region-z01-x128-y122.b3");
    const FLOOR_2: &[u8] =
        include_bytes!("../../../../../../content/world/placements/region-z02-x128-y122.b3");
    native_scene_with_appearances(
        native,
        vec![
            NativeCrop {
                bytes: FLOOR_2,
                region: (2, 128, 122),
                bounds: (32964, 31455, 32969, 31460),
                start: (START.x, START.y),
                expected: vec![
                    ExpectedTile {
                        position: (MIRROR.x, MIRROR.y),
                        ids: vec![8308, 8205, 8667],
                        counted_entry: None,
                    },
                    ExpectedTile {
                        position: (STATUE.x, STATUE.y),
                        ids: vec![8308, 8698, 8325],
                        counted_entry: None,
                    },
                ],
            },
            NativeCrop {
                bytes: FLOOR_1,
                region: (1, 128, 122),
                bounds: (32965, 31458, 32970, 31464),
                start: (32967, 31459),
                expected: vec![ExpectedTile {
                    position: (32967, 31463),
                    ids: vec![8308, 1959],
                    counted_entry: None,
                }],
            },
            NativeCrop {
                bytes: FLOOR_1,
                region: (1, 128, 122),
                bounds: (32966, 31479, 32969, 31482),
                start: (32967, 31480),
                expected: Vec::new(),
            },
        ],
        &[ExpectedTeleport {
            position: (32967, 31463, 1),
            ordinal: 1,
            destination: (32967, 31480, 1),
        }],
        &[8326, 8721],
    )
}

fn placement(map: &BundleMap, position: Position) -> TestResult<(WorldObjectTarget, u16)> {
    let at = TilePos {
        x: position.x,
        y: position.y,
        floor: position.floor,
    };
    let key = crate::map::view::placement_key(at, 2).ok_or("placement key")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let base = map
        .overlay
        .base()
        .tile(at.x, at.y, at.floor)
        .ok_or("source tile")?;
    let id = *base.ids().get(2).ok_or("source appearance entry")?;
    let appearance = map
        .facts
        .base_entry(at, 2, id)
        .ok_or("current source appearance")?
        .appearance_id;
    Ok((
        WorldObjectTarget {
            placement,
            expected_revision: map.facts.object_revision(key),
        },
        appearance,
    ))
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    members: &mut [AdmittedSession],
    map: &BundleMap,
) -> TestResult {
    let [first, other] = members else {
        return Err("two durable admissions".into());
    };
    owner.admit_quest_session(first).await;
    owner.admit_quest_session(other).await;
    let fence = owner
        .current_quest_fence(first.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("session")?;
    let (mirror, appearance) = placement(map, MIRROR)?;
    assert_eq!(appearance, 8667);
    assert_eq!(
        use_target(owner, first, &mirror).await?,
        UseDisposition::Committed
    );
    assert_eq!(placement(map, MIRROR)?.1, 8721);
    assert_eq!(placement(map, STATUE)?.1, 8326);
    let due = map
        .facts
        .next_movement_decay_time()
        .ok_or("mirror repair")?;
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(due), 1);
    assert_eq!(placement(map, MIRROR)?.1, 8667);
    assert_eq!(placement(map, STATUE)?.1, 8326);
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(state.tracks().get(COUNTER), Some(&1));
    assert_eq!(
        use_target(owner, first, &placement(map, MIRROR)?.0).await?,
        UseDisposition::Committed
    );
    let state = owner
        .root
        .read_character_quest_state(owner.character, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(state.tracks().get(COUNTER), Some(&0));
    assert_eq!(position(owner, first).await?, (32967, 31459, 1));
    assert_eq!(position(owner, other).await?, (32967, 31459, 1));
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .visible_entities()
            .creatures
            .len(),
        3
    );

    // The activating USE owns the same 30-second repair as the first break.
    assert_eq!(placement(map, MIRROR)?.1, 8721);
    let second_due = map
        .facts
        .next_movement_decay_time()
        .ok_or("group mirror repair")?;
    assert!(second_due > due);
    assert_eq!(map.facts.expire_movement_items(second_due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(second_due), 1);
    assert_eq!(placement(map, MIRROR)?.1, 8667);
    assert_eq!(placement(map, STATUE)?.1, 8326);

    for direction in [StepDirection::South; 4] {
        step(owner, first, direction).await?;
    }
    assert_eq!(position(owner, first).await?, (32967, 31480, 1));
    step(owner, first, StepDirection::East).await?;
    for direction in [StepDirection::South; 4] {
        step(owner, other, direction).await?;
    }
    assert_eq!(position(owner, other).await?, (32967, 31480, 1));
    eprintln!("POSTGRES_CASE_COMPLETED bbro2_lersatio_two_players_mirror_repair_and_exit");
    Ok(())
}
