//! Crystal 201189f8, data-global/scripts/quests/the_new_frontier/action_beaver.lua:12-44
//! and data-global/monster/mammals/thieving_squirrel.lua:63-65.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::{Harness, debug}};
use oteryn_gameplay_quests::quest::{QuestStateCatalogue, authored_bindings::Bindings, world_callbacks::{Offering, Position}};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use serde_json::json;
use std::sync::Arc;

pub(super) const NAME: &str = "qengine-rewards-frontier-tree-bait";
const AT: Position = Position {x: 33294, y: 31677, floor: -15};
const QUEST: &str = "oteryn:quest.authored.draccoon_herald_outfits_quest";
const TRACK: &str = "oteryn:quest-progress/tree-bait-fixture/marked";
const BAIT: &str = "oteryn:item.tibia.i9843";
const THIEF: &str = "oteryn:creature.thieving_squirrel";

#[test]
fn bait_use_spawns_a_thief_and_the_same_creature_returns_bait_on_death() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    drop(admin);
    super::qengine_rewards::scenario(NAME)
}

pub(super) async fn exercise(owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession, h: &Harness) -> TestResult {
    let mut callback = super::authored_world_rewards::placeholder(AT)?;
    callback.key = "oteryn:quest-callback/tree-bait-fixture/mark".into();
    callback.requires_world_admission = true;
    callback.storage[0].track = TRACK.into();
    callback.sources = vec!["crystal:201189f871d23597b82caf88fd611454a2ddefb8:data-global/scripts/quests/the_new_frontier/action_beaver.lua:12-44".into()];
    let mut callback = serde_json::to_value(callback)?;
    callback["item_on_target"] = json!({"item":BAIT,"count":1,"replacement":null,"target_replacement":null});
    callback["effects"] = json!({"spawn_creatures":[{"creature":THIEF,"count":1}]});
    let document = json!({"schema":1,"quests":[{"quest":QUEST,
        "recipe_sha256":"2222222222222222222222222222222222222222222222222222222222222222",
        "sources":["synthetic target-item spawn engine fixture"],
        "tracks":[{"key":TRACK,"initial":0,"minimum":0,"maximum":1}],
        "stages":[{"key":"mark","completes":false,"trigger":{"kind":"map_use","callback":callback}}]}]});
    let bindings = Bindings::parse(&serde_json::to_vec(&document)?)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("revision")?;
    owner.quest_catalogue = Some(Arc::new(bindings.attach(&base)?));
    let fence = owner.current_quest_fence(admitted.game_session_id).await.map_err(|()| "fence")?.ok_or("fence")?;
    use super::secret_service_dock_fire::use_target;
    assert_eq!(use_target(owner, admitted, AT).await?, UseDisposition::Rejected);
    assert!(owner.runtime.lock().await.current_live_creature_census().map_err(debug)?.is_empty());
    super::quest_world_callbacks::input_item(owner, fence.item_fence(), &Offering {
        position: AT, item: BAIT.into(), count: 1,
    }, 4620, true).await?;
    assert_eq!(use_target(owner, admitted, AT).await?, UseDisposition::Committed);
    assert_eq!(super::qengine_voucher_stories::quantity(h, 9843).await?, 0);
    let thief = {
        let runtime = owner.runtime.lock().await;
        let actors = runtime.current_live_creature_census().map_err(debug)?;
        let [thief] = actors.as_slice() else { return Err("exactly one spawned thief".into()) };
        *thief
    };
    assert_eq!(super::qengine_kill_stories::queue_death_using(owner, admitted, THIEF, 20000, Some(thief)).await?, thief);
    super::qengine_kill_stories::drain_deaths(owner, admitted, &[]).await?;
    let bait_in_corpse: i64 = sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_corpse_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.lifecycle=1 AND i.definition_production_key=$1")
        .bind(BAIT).fetch_one(&h.pool).await?;
    assert_eq!(bait_in_corpse, 1);
    let marked: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE track_key=$1").bind(TRACK).fetch_one(&h.pool).await?;
    assert_eq!(marked, 1);
    Ok(())
}
