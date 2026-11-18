//! Runs the timed growing-birch projection through the normal USE dispatcher.
use super::fixture::debug;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, quest_world_callbacks};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::{overlay::TilePos, view::MapFacts};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) const NAME: &str = "fknow-earth-birch";
const AT: Position = Position {
    x: 32736,
    y: 32116,
    floor: -10,
};
const PLANTING_SOIL: u16 = 23814;
const SEEDLING: u16 = 23816;
const BIRCH: u16 = 23815;
const SOIL: u16 = 26482;

#[test]
fn fknow_earth_birch_expires_to_soil_after_use() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        quest_world_callbacks::scenario(NAME, AT, false)
    })
}

pub(super) fn callback() -> TestResult<Callback> {
    stage("plant-birch", PLANTING_SOIL, SEEDLING, "415-426")
}

pub(super) fn watering() -> TestResult<Callback> {
    stage("grow-birch", SEEDLING, BIRCH, "425-431")
}

fn stage(name: &str, from: u16, to: u16, lines: &str) -> TestResult<Callback> {
    Ok(serde_json::from_value(serde_json::json!({
        "key": format!("oteryn:quest-callback/forgotten-knowledge/{name}"),
        "quest": "oteryn:quest.forgotten_knowledge_quest",
        "trigger": "use",
        "selector": { "position": AT, "appearance": from },
        "reversible": false,
        "passages": [],
        "effects": { "transform": to, "reset_after_ms": 600000, "reset_appearance": SOIL },
        "storage": [],
        "offerings": [],
        "return_on_failure": false,
        "sources": [format!("crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/forgotten_knowledge/actions_plant.lua:{lines}")]
    }))?)
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    quest_world_callbacks::world_with_objects(callback, AT, &[], &[BIRCH, SEEDLING, SOIL])
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let map = owner.bundle_map().ok_or("map")?;
    let pos = TilePos {
        x: AT.x,
        y: AT.y,
        floor: AT.floor,
    };
    let key = oteryn_world_bundle::bundle::placement_key(AT.floor, AT.x, AT.y, 1)
        .ok_or("birch placement")?;
    let item = *map
        .overlay
        .base()
        .tile(AT.x, AT.y, AT.floor)
        .ok_or("birch tile")?
        .ids()
        .get(1)
        .ok_or("birch item")?;
    let appearance = || -> TestResult<u16> {
        Ok(map
            .facts
            .base_entry(pos, 1, item)
            .ok_or("birch facts")?
            .appearance_id)
    };
    assert_eq!(appearance()?, PLANTING_SOIL);
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    for (revision, placed, now) in [(0, SEEDLING, 5_000_000), (1, BIRCH, 6_000_000)] {
        owner
            .states
            .test_owner_time
            .store(now + 1, std::sync::atomic::Ordering::Relaxed);
        super::m44_source_lifecycle::wait_for_world_action(owner, admitted).await?;
        let payload = oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: revision,
        })
        .map_err(debug)?;
        let command = admitted.continuity.next_command_id;
        let reply = quest_world_callbacks::wire(owner, admitted, command, 2, payload).await?;
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(debug)?,
            UseDisposition::Committed,
            "birch stage at placement revision {revision}"
        );
        assert_eq!(appearance()?, placed);
    }
    let planting_due = map
        .facts
        .next_movement_decay_time()
        .ok_or("planting timer")?;
    assert_eq!(planting_due, 605_000_000);
    assert_eq!(map.facts.expire_movement_items(planting_due), 0);
    assert_eq!(appearance()?, BIRCH, "the first timer cannot undo watering");

    let due = map.facts.next_movement_decay_time().ok_or("birch timer")?;
    assert_eq!(due, 606_000_000);
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(due), 1);
    assert_eq!(appearance()?, SOIL);
    Ok(())
}

pub(super) const TREE_NAME: &str = "fknow-earth-world-tree";
const TREE: Position = Position {
    x: 32737,
    y: 32116,
    floor: -10,
};
const CROWN: Position = Position {
    x: 32737,
    y: 32114,
    floor: -10,
};
const FOREST_GROUND: Position = Position {
    x: 32732,
    y: 32113,
    floor: -11,
};
const PHIAL: &str = "oteryn:item.tibia.i23810";

#[test]
fn fknow_earth_world_tree_use_step_and_expiry() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run(TREE_NAME, || {
        quest_world_callbacks::scenario(TREE_NAME, TREE, false)
    })
}

pub(super) fn tree_callback() -> TestResult<Callback> {
    use oteryn_gameplay_quests::quest::world_callbacks::{
        EARTH_TREE_PORTAL, EARTH_TREE_REQUIREMENTS, EARTH_TREE_RETURN,
    };
    let storage: Vec<_> = EARTH_TREE_REQUIREMENTS.into_iter().map(|(track, minimum)| {
        serde_json::json!({"track": track, "minimum": minimum, "maximum": i32::MAX, "set": minimum, "preserve": true})
    }).collect();
    Ok(serde_json::from_value(serde_json::json!({
        "key": "oteryn:quest-callback/forgotten-knowledge/water-world-tree",
        "quest": "oteryn:quest.forgotten_knowledge_quest",
        "trigger": "use",
        "selector": {"position": TREE, "appearance": 6180},
        "reversible": false,
        "requires_world_admission": true,
        "passages": [],
        "storage": storage,
        "offerings": [],
        "return_on_failure": false,
        "item_on_target": {"item": PHIAL, "count": 1, "reusable": true, "selected_instance": true},
        "effects": {"scene": {
            "open": [
                {"position": CROWN, "remove": 6189, "create": 3617, "passable": true},
                {"position": FOREST_GROUND, "remove": 10605, "create": 4515, "passable": true},
                {"position": EARTH_TREE_PORTAL, "create": 11108, "passable": true}
            ],
            "close": [],
            "evacuate_open": [{"from": EARTH_TREE_PORTAL, "players_to": EARTH_TREE_RETURN, "creatures_to": EARTH_TREE_RETURN, "all": false}]
        }},
        "sources": ["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/forgotten_knowledge/actions_plant.lua:437-495"]
    }))?)
}

pub(super) fn tree_world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_gameplay_quests::quest::world_callbacks::{
        EARTH_TREE_CHAMBER, EARTH_TREE_PORTAL, EARTH_TREE_RETURN,
    };
    quest_world_callbacks::world_cells(
        callback,
        TREE,
        6180,
        1,
        &[(CROWN, 6189), (FOREST_GROUND, 10605)],
        &[3617, 4515, 775, 11108],
        None,
        &[EARTH_TREE_PORTAL, EARTH_TREE_CHAMBER, EARTH_TREE_RETURN],
    )
}

pub(super) fn tree_catalogue(
    source: &oteryn_gameplay_quests::quest::QuestStateCatalogue,
    callback: &Callback,
) -> TestResult<oteryn_gameplay_quests::quest::QuestStateCatalogue> {
    use oteryn_gameplay_quests::quest::{QuestStateCatalogue, world_callbacks::primary_transition};
    let tracks = callback
        .storage
        .iter()
        .map(|condition| {
            source
                .track(&condition.track)
                .cloned()
                .ok_or("world tree track")
        })
        .collect::<Result<Vec<_>, _>>()?;
    QuestStateCatalogue::new(
        source.content_revision(),
        tracks,
        vec![primary_transition(callback)?],
    )
    .map_err(debug)
    .map_err(Into::into)
}

pub(super) async fn exercise_tree(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &super::fixture::Harness,
) -> TestResult {
    use oteryn_gameplay_quests::quest::world_callbacks::{
        EARTH_TREE_CHAMBER, EARTH_TREE_PORTAL, EARTH_TREE_REQUIREMENTS, Offering,
    };
    use oteryn_protocol_oteryn::world_object::{
        MapUseTarget, UseIntent, UseTarget, encode_item_use_intent,
    };
    use oteryn_protocol_oteryn::world_spatial::{
        StepDirection, StepDisposition, decode_step_result, encode_step_intent,
    };
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    // The two prerequisite tracks represent historical progress before the phial use.
    for update in [
        "UPDATE game_character_roots SET character_revision=2 WHERE character_id=encode($1,'hex')::uuid AND character_revision=1",
        "UPDATE game_character_progression_state SET character_revision=2 WHERE character_id=encode($1,'hex')::uuid AND character_revision=1",
    ] {
        sqlx::query(update)
            .bind(fence.character_id.as_bytes().as_slice())
            .execute(&mut *tx)
            .await?;
    }
    for (track, value) in EARTH_TREE_REQUIREMENTS {
        sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision)
            SELECT character_id,$2,'oteryn:quest.forgotten_knowledge_quest',$3,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).bind(track).bind(value).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            &[4, 6, 11, 15, 18],
        )
        .ok_or("capabilities")?;
    quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: TREE,
            item: PHIAL.into(),
            count: 1,
        },
        950,
        true,
    )
    .await?;
    let reference = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .and_then(|native| native.item_index())
        .and_then(|index| index.definition_ref("Item", PHIAL, "definition-r1"))
        .ok_or("phial definition")?;
    let map = owner.bundle_map().ok_or("world tree map")?;
    let tile = |p: Position| TilePos {
        x: p.x,
        y: p.y,
        floor: p.floor,
    };
    let appearance = |p: Position| -> TestResult<u16> {
        let item = *map
            .overlay
            .base()
            .tile(p.x, p.y, p.floor)
            .ok_or("world tree cell")?
            .ids()
            .get(1)
            .ok_or("world tree item")?;
        Ok(map
            .facts
            .base_entry(tile(p), 1, item)
            .ok_or("world tree appearance")?
            .appearance_id)
    };
    assert_eq!(appearance(CROWN)?, 6189);
    assert_eq!(appearance(FOREST_GROUND)?, 10605);
    let key = oteryn_world_bundle::bundle::placement_key(TREE.floor, TREE.x, TREE.y, 1)
        .ok_or("world tree placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(std::num::NonZeroU64::MIN),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: 0,
        })),
        use_with_position: None,
    })
    .map_err(debug)?;
    let (next, result, _) = super::item_craft::exchange(
        owner,
        *admitted,
        admitted.continuity.next_command_id,
        &payload,
        Some(reference),
    )
    .await?;
    *admitted = next;
    assert_eq!(result, UseDisposition::Committed);
    let active: bool = sqlx::query_scalar(
        "SELECT expires_at > statement_timestamp()
         FROM game_forgotten_knowledge_tree_windows WHERE world_id=encode($1,'hex')::uuid",
    )
    .bind(admitted.world_id.as_bytes().as_slice())
    .fetch_one(&harness.pool)
    .await?;
    assert!(active);
    assert_eq!(appearance(CROWN)?, 3617);
    assert_eq!(appearance(FOREST_GROUND)?, 4515);
    assert_eq!(
        map.facts
            .quest_portal_destination(tile(EARTH_TREE_PORTAL), owner.owner_now().get()),
        Some(tile(EARTH_TREE_CHAMBER))
    );
    let actor = admitted.runtime_actor.ok_or("world tree player")?;
    let command = admitted.continuity.next_command_id;
    let result = quest_world_callbacks::wire(
        owner,
        admitted,
        command,
        1,
        encode_step_intent(StepDirection::South),
    )
    .await?;
    assert_eq!(
        decode_step_result(&result).map_err(debug)?,
        StepDisposition::Moved
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        quest_world_callbacks::local(EARTH_TREE_CHAMBER)
    );
    let due = map
        .facts
        .next_movement_decay_time()
        .ok_or("world tree expiry")?;
    assert_eq!(map.facts.expire_movement_items(due - 1), 0);
    assert_eq!(map.facts.expire_movement_items(due), 1);
    assert_eq!(appearance(CROWN)?, 6189);
    assert_eq!(appearance(FOREST_GROUND)?, 10605);
    assert_eq!(
        map.facts
            .quest_portal_destination(tile(EARTH_TREE_PORTAL), due),
        None
    );
    Ok(())
}

/// Use the existing canonical phial definition and exact Crystal item binding in the test activation.
pub(super) fn tree_generation(
    world: crate::foundation::WorldId,
) -> TestResult<super::quest_catalogue_chests::CatalogueActivation> {
    use sha2::Digest;
    let project = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let load = |path: &str| -> TestResult<serde_json::Value> {
        Ok(serde_json::from_slice(&std::fs::read(project.join(path))?)?)
    };
    let definitions = load("content/items/definitions/items-12500-12999.json")?;
    let definition = definitions["records"]
        .as_array()
        .ok_or("item definitions")?
        .iter()
        .map(|row| &row["definition"])
        .find(|row| row["identity"]["key"] == PHIAL)
        .ok_or("canonical phial")?;
    let bindings = load("imports/crystalserver/bindings/items.json")?;
    let binding = bindings["bindings"]
        .as_array()
        .ok_or("item bindings")?
        .iter()
        .find(|row| row["target"]["key"] == PHIAL)
        .ok_or("phial source binding")?;
    let mut input = super::death_ml_loss_composed::production_input_with_source("content-1", None)?;
    let pin = input.item_profiles.as_mut().ok_or("native item profiles")?;
    let mut document: serde_json::Value = serde_json::from_slice(&pin.bytes)?;
    let records = document["records"].as_array_mut().ok_or("item profiles")?;
    // The profile schema uses the same nonblocking flask attributes for carried quest tools.
    let attributes = records
        .iter()
        .find(|row| row["production_definition"]["production_key"] == "oteryn:item.tibia.i7253")
        .ok_or("carried flask attributes")?["attributes"]
        .clone();
    let index = records.partition_point(|row| {
        row["production_binding"]["external_id"]
            .as_str()
            .and_then(|id| id.parse::<u32>().ok())
            .is_some_and(|id| id < 23810)
    });
    records.insert(index, serde_json::json!({
        "admission": {
            "legal_destinations": ["CharacterInventory", "Ground"],
            "materializable": definition["materializable"],
            "stack_class": definition["stack_class"]
        },
        "attributes": attributes,
        "authoring": {
            "item": {"family": "Item", "key": "candidate:item/23810", "revision": "spell-p2-r21"},
            "taxonomy": {"primary": "Quest Items"}
        },
        "production_binding": binding,
        "production_binding_qualification": null,
        "production_definition": {"family": "Item", "production_key": PHIAL, "revision_ref": definition["identity"]["revision"]},
        "semantics": definition["semantics"]
    }));
    pin.bytes = serde_json::to_vec(&document)?;
    pin.sha256 = sha2::Sha256::digest(&pin.bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    crate::content::native_gameplay::test_support::try_activate_input(world, &input)
        .map_err(debug)
        .map_err(Into::into)
}
