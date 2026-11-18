//! Two independent characters use one Channel poison counter and one native corpse.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult};
use crate::gameplay_transport::{
    capabilities::SelectedCapabilities, connection::FreshAdmissionAuthority,
};
use oteryn_gameplay_quests::quest::{
    world_callbacks::{Callback, Offering, Position},
    wrath_keeper::{CORPSE, CREATURE, POISON_USE, SPAWN, TAIL_HANDOFF},
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, decode_use_result,
    encode_item_use_intent,
};

pub(super) const NAME: &str = "qengine-rewards-wrath-keeper";
pub(super) const PLANT: Position = Position {
    y: SPAWN.y - 1,
    ..SPAWN
};
const FIRST: Position = Position {
    x: PLANT.x - 1,
    ..PLANT
};
const SECOND: Position = Position {
    x: PLANT.x + 1,
    ..PLANT
};

#[test]
fn wrathm1_keeper_shared_poison_and_two_character_corpse_handoff() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    assert!(!admin.is_empty());
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, PLANT, false)
    })
}

pub(super) fn callback() -> TestResult<Callback> {
    oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|callback| callback.key == POISON_USE)
        .ok_or_else(|| "registered plant-poison callback".into())
}

pub(super) fn world(callback: &Callback) -> TestResult<crate::map::boot::BundleWorld> {
    let mut geometry = callback.clone();
    // Crystal items.xml:28302 names the corrupted plantation appearance.
    // This test viewport places the script's ActionID adjacent to its spawn.
    geometry.selector.appearance = Some(11373);
    super::quest_world_callbacks::world_with_cells(&geometry, PLANT, &[FIRST, SECOND, SPAWN])
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    deps: &Dependencies,
    h: &Harness,
) -> TestResult {
    use super::quest_world_callbacks::{input_item, local, wire};
    let (_, mut second) =
        super::grizzly_boss::vacancy::second_player(owner, deps, h, local(SECOND)).await?;
    let caps = SelectedCapabilities::select(owner.offered_capabilities(), &[4, 6, 11, 15, 18])
        .ok_or("Keeper USE capabilities")?;
    assert!(
        caps.contains(oteryn_protocol_oteryn::world_interaction::CAPABILITY_WORLD_INTERACTION_V1)
    );
    admitted.continuity.selected_capabilities = caps.clone();
    second.continuity.selected_capabilities = caps;
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native gameplay")?;
    let second_fence = second.item_fence.ok_or("second item fence")?;
    let template = crate::durability::starter_grant::StarterTemplate {
        definition: crate::durability::item_mint::TypedDefinitionRef {
            family: "Item".into(),
            production_key: "oteryn:item.tibia.i2854".into(),
            revision_ref: "definition-r1".into(),
        },
        ..crate::durability::starter_grant::StarterTemplate::main()
    };
    h.root
        .grant_starter_items(
            owner.character,
            &h.node,
            &second_fence,
            &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
            &template,
        )
        .await
        .map_err(debug)?;
    let definition = native
        .item_index()
        .and_then(|index| index.definition_ref("Item", "oteryn:item.tibia.i11364", "definition-r1"))
        .ok_or("plant poison definition")?;
    let offering = Offering {
        position: FIRST,
        item: "oteryn:item.tibia.i11364".into(),
        count: 1,
    };
    input_item(
        owner,
        admitted.item_fence.ok_or("item fence")?,
        &offering,
        71,
        true,
    )
    .await?;
    input_item(
        owner,
        second_fence,
        &Offering {
            position: SECOND,
            ..offering
        },
        72,
        true,
    )
    .await?;
    let map = owner.bundle_map().ok_or("Keeper test map")?;
    let key = crate::map::view::placement_key(
        crate::map::overlay::TilePos {
            x: PLANT.x,
            y: PLANT.y,
            floor: PLANT.floor,
        },
        1,
    )
    .ok_or("plant placement")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let use_poison = encode_item_use_intent(&UseIntent {
        target: UseTarget::ItemByDefinition(definition),
        use_with: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: 0,
        })),
        use_with_position: None,
    })
    .map_err(debug)?;
    for count in 1..=6 {
        owner.clock_origin -= std::time::Duration::from_secs(2);
        let player = if count == 6 {
            &mut second
        } else {
            &mut *admitted
        };
        let result = wire(
            owner,
            player,
            player.continuity.next_command_id,
            2,
            use_poison.clone(),
        )
        .await?;
        assert_eq!(
            decode_use_result(&result).map_err(debug)?,
            UseDisposition::Committed,
            "poison use {count}"
        );
        assert_eq!(
            super::questpc7_outfit_items::quantity(h, 11364).await?,
            if count < 6 { 2 } else { 1 }
        );
    }
    let keeper = owner
        .runtime
        .lock()
        .await
        .positioned_melee_census()
        .map_err(debug)?
        .into_iter()
        .find(|(_, at, session)| session.is_none() && at.position() == local(SPAWN))
        .map(|(actor, _, _)| actor)
        .ok_or("the sixth use spawns the Keeper")?;
    super::questpc7_native_deaths::lethal_creature(owner, admitted, keeper, 7400, CREATURE).await?;
    let corpse: sqlx::types::Uuid = sqlx::query_scalar(
        "SELECT i.item_instance_id FROM game_item_instances i JOIN game_item_ground_locations g USING(item_instance_id)
         WHERE i.definition_production_key=$1 AND i.lifecycle=1 AND octet_length(g.corpse_ref)>0"
    ).bind(CORPSE).fetch_one(&h.pool).await?;
    let identity = owner
        .runtime
        .lock()
        .await
        .visible_corpses()
        .into_iter()
        .find(|body| body.item.production_key == CORPSE)
        .map(|body| body.identity)
        .ok_or("native Keeper corpse projection")?;
    let bound = owner
        .runtime
        .lock()
        .await
        .bound_corpse(identity)
        .ok_or("native corpse binding")?;
    assert_eq!(bound.item.item_instance_id, *corpse.as_bytes());
    for player in [&mut *admitted, &mut second] {
        claim_tail(
            owner,
            player,
            h,
            crate::gameplay_transport::item_view::ItemKey::Entity(identity),
        )
        .await?;
    }
    let grants: i64 = sqlx::query_scalar(
        "SELECT count(DISTINCT character_id) FROM game_character_quest_receipts WHERE transition_key=$1"
    ).bind(TAIL_HANDOFF).fetch_one(&h.pool).await?;
    assert_eq!(grants, 2);
    let final_tracks: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_quest_tracks
         WHERE (track_key=$1 AND value=8) OR (track_key=$2 AND value=2)",
    )
    .bind(oteryn_gameplay_quests::quest::wrath_sleeping_dragon::QUESTLINE)
    .bind(oteryn_gameplay_quests::quest::wrath_keeper::MISSION)
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(final_tracks, 4);
    assert_eq!(super::questpc7_outfit_items::quantity(h, 11367).await?, 2);
    Ok(())
}

async fn claim_tail(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    player: &mut AdmittedSession,
    h: &Harness,
    corpse: crate::gameplay_transport::item_view::ItemKey,
) -> TestResult {
    use crate::gameplay_transport::connection::UseCommand;
    use oteryn_protocol_oteryn::item_view::ItemHandle;
    let fence = owner
        .current_quest_fence(player.game_session_id)
        .await
        .map_err(|()| "quest fence")?
        .ok_or("quest fence")?;
    sqlx::query(
        "INSERT INTO game_character_quest_tracks
         (character_id,track_key,quest_key,value,committed_character_revision)
         SELECT character_id,$2,$3,7,character_revision FROM game_character_roots
         WHERE character_id=$1 ON CONFLICT (character_id,track_key) DO UPDATE
         SET value=EXCLUDED.value,committed_character_revision=EXCLUDED.committed_character_revision",
    )
    .bind(sqlx::types::Uuid::from_bytes(*fence.character_id.as_bytes()))
    .bind(oteryn_gameplay_quests::quest::wrath_sleeping_dragon::QUESTLINE)
    .bind(oteryn_gameplay_quests::quest::wrath_sleeping_dragon::QUEST)
    .execute(&h.pool).await?;
    owner
        .refresh_quest_session(player.game_session_id, false)
        .await;
    let actor = player.runtime_actor.ok_or("player actor")?;
    let outcome = owner
        .use_quest_tool(
            actor,
            UseCommand {
                game_session_id: player.game_session_id,
                command_id: 7500,
                item_fence: Some(fence.item_fence()),
            },
            UseIntent {
                target: UseTarget::Item(ItemHandle::new(1).ok_or("corpse handle")?),
                use_with: None,
                use_with_map_item: None,
                use_with_position: None,
            },
            Some(corpse),
            None,
        )
        .await
        .map_err(|()| "corpse USE")?
        .ok_or("corpse route")?;
    assert_eq!(outcome.disposition, UseDisposition::Committed);
    Ok(())
}

/// The content lane owns publishing these two Item profiles. The test generation
/// admits the existing physical facts and immutable source bindings through the
/// same compiler used by a live Channel.
pub(super) fn generation(
    world: crate::foundation::WorldId,
) -> TestResult<(
    crate::content::ContentActivationController,
    crate::content::NativeEntryContentPin,
)> {
    use serde_json::{Value, json};
    use sha2::{Digest, Sha256};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
    let read = |path: &str| -> TestResult<Value> {
        Ok(serde_json::from_slice(&std::fs::read(root.join(path))?)?)
    };
    let items = read("content/items/definitions/items-01000-01499.json")?;
    let bindings = read("imports/crystalserver/bindings/items.json")?;
    let mut input = super::death_ml_loss_composed::production_input_with_source("content-1", None)?;
    let profiles = input.item_profiles.as_mut().ok_or("native item profiles")?;
    let mut document: Value = serde_json::from_slice(&profiles.bytes)?;
    let records = document["records"]
        .as_array_mut()
        .ok_or("native item records")?;
    for id in ["11364", "11367"] {
        let key = format!("oteryn:item.tibia.i{id}");
        if records
            .iter()
            .any(|row| row["production_definition"]["production_key"] == key)
        {
            continue;
        }
        let definition = items["records"]
            .as_array()
            .ok_or("canonical item records")?
            .iter()
            .map(|row| &row["definition"])
            .find(|row| row["identity"]["key"] == key)
            .filter(|row| row["materializable"] == true)
            .ok_or("materializable Keeper item")?;
        let binding = bindings["bindings"]
            .as_array()
            .ok_or("Crystal Item bindings")?
            .iter()
            .find(|row| row["external_id"] == id && row["target"] == definition["identity"])
            .ok_or("exact Crystal Item binding")?;
        records.push(json!({
            "authoring": {"item": definition["identity"]},
            "semantics": definition["semantics"],
            "attributes": {},
            "admission": {
                "materializable": true,
                "stack_class": definition["stack_class"],
                "legal_destinations": ["CharacterInventory", "Ground"]
            },
            "production_definition": {
                "family": "Item", "production_key": key, "revision_ref": definition["identity"]["revision"]
            },
            "production_binding": binding
        }));
    }
    profiles.bytes = serde_json::to_vec(&document)?;
    profiles.sha256 = Sha256::digest(&profiles.bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    crate::content::native_gameplay::test_support::try_activate_input(world, &input)
        .map_err(|error| debug(error).into())
}
