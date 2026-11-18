//! Crystal 201189f8 actions_mission03_the_keeper.lua:10–16.
//! The terminal offering is exercised independently of the shared switch count.
use super::{
    AdmittedSession, ComposedFreshAdmission, TestResult,
    fixture::{Harness, debug},
};
use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue,
    authored_bindings::{Bindings, Trigger},
    world_callbacks::{Offering, Position},
};
use oteryn_protocol_oteryn::world_object::{
    MapUseTarget, UseDisposition, UseIntent, UseTarget, WorldObjectTarget, encode_item_use_intent,
};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{num::NonZeroU64, sync::Arc};

pub(super) const NAME: &str = "qewrath-keeper-summon";
const QUEST: &str = "oteryn:quest.wrath_of_the_emperor_quest";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_6/wrath_of_the_emperor/questline";
const KEEPER: &str = "oteryn:creature.the_keeper";
const POISON: &str = "oteryn:item.tibia.i11364";
const SOURCE: &str = "crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/wrath_of_the_emperor/actions_mission03_the_keeper.lua:10-16";
const SPAWN: Position = Position {
    x: 33171,
    y: 31058,
    floor: -11,
};
// Source selects a plant by AID. The dispatch fixture places that bound
// object beside the donor's fixed summon cell.
const PLANT: Position = Position { x: 33170, ..SPAWN };

#[test]
fn keeper_terminal_plant_use_consumes_the_selected_poison_and_summons_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    super::kill_reward_e2e::scenario_thread::run(NAME, || {
        super::quest_world_callbacks::scenario(NAME, PLANT, false)
    })
}

fn fixture() -> TestResult<Bindings> {
    let callback = json!({
        "key":"oteryn:quest-callback/wrath/keeper-summon","quest":QUEST,"trigger":"use",
        "selector":{"position":PLANT,"action":8026,"appearance":100},
        "reversible":false,"requires_world_admission":true,
        "passages":[],"offerings":[],"return_on_failure":false,
        "storage":[{"track":QUESTLINE,"minimum":0,"maximum":34,"set":0,"preserve":true}],
        "item_on_target":{"item":POISON,"count":1,"selected_instance":true},
        "effects":{
            "offering_spawn":{"creature":KEEPER,"position":SPAWN,"magic":11},
            "magic":{"position":PLANT,"effect":8}
        },
        "sources":[SOURCE]
    });
    let digest = Sha256::digest(serde_json::to_vec(&callback)?)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let input = json!({"schema":1,"quests":[{
        "quest":QUEST,"recipe_sha256":digest,"sources":[SOURCE],
        "tracks":[{"key":QUESTLINE,"initial":7,"minimum":0,"maximum":34}],
        "stages":[{"key":"keeper-summon","completes":false,"trigger":{"kind":"map_use","callback":callback}}],
    }]});
    Ok(Bindings::parse(&serde_json::to_vec(&input)?)?)
}

pub(super) fn world() -> TestResult<crate::map::boot::BundleWorld> {
    let fixture = fixture()?;
    let Trigger::MapUse { callback } = &fixture.quests[0].stages[0].trigger else {
        return Err("Keeper plant binding".into());
    };
    super::quest_world_callbacks::world(callback, PLANT)
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let empty = QuestStateCatalogue::empty("content-1").ok_or("catalogue")?;
    owner.quest_catalogue = Some(Arc::new(fixture()?.attach(&empty)?));
    let session = admitted.game_session_id;
    owner.refresh_quest_session(session, false).await;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            position: PLANT,
            item: POISON.into(),
            count: 1,
        },
        612,
        true,
    )
    .await?;
    let map = owner.bundle_map().ok_or("map")?;
    let key = crate::map::view::placement_key(
        crate::map::overlay::TilePos {
            x: PLANT.x,
            y: PLANT.y,
            floor: PLANT.floor,
        },
        1,
    )
    .ok_or("plant")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let intent = encode_item_use_intent(&UseIntent {
        target: UseTarget::Item(NonZeroU64::MIN),
        use_with: None,
        use_with_position: None,
        use_with_map_item: Some(MapUseTarget::Object(WorldObjectTarget {
            placement,
            expected_revision: 0,
        })),
    })
    .map_err(debug)?;
    let reference = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|index| index.definition_ref("Item", POISON, "definition-r1"))
        .ok_or("poison definition")?;
    let command = admitted.continuity.next_command_id;
    let (next, disposition, _) =
        super::item_craft::exchange(owner, *admitted, command, &intent, Some(reference)).await?;
    *admitted = next;
    assert_eq!(disposition, UseDisposition::Committed);
    let runtime = owner.runtime.lock().await;
    let bodies = runtime
        .positioned_melee_census()
        .map_err(debug)?
        .into_iter()
        .filter(|(actor, position, _)| {
            runtime.matches_live_creature_identity(*actor, KEEPER.as_bytes())
                && position.position() == super::quest_world_callbacks::local(SPAWN)
        })
        .count();
    assert_eq!(bodies, 1);
    drop(runtime);
    let inputs: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_consumptions WHERE transition_key='oteryn:quest-callback/wrath/keeper-summon' AND quantity=1").fetch_one(&h.pool).await?;
    assert_eq!(inputs, 1);
    Ok(())
}
