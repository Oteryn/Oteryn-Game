//! Crystal brain mission through framed production USE and durable Ground custody.
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::{Harness, debug}};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::{self, world_callbacks::{Callback, Position}};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) const NAME: &str = "brainout";
pub(super) const SOURCE: Position = Position { x: 33022, y: 32329, floor: -10 };
pub(super) const START: Position = Position { x: 33022, y: 32330, floor: -10 };
const KEY: &str = "oteryn:quest-callback/gravedigger-brain";

#[test]
fn brainout_ground_reward_atomic_replay_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    assert!(!admin.is_empty());
    run(NAME)
}
#[test]
fn brainout_ground_reward_full_inventory_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    assert!(!admin.is_empty());
    run("brainout-full")
}
#[test]
fn brainout_ground_reward_wrong_quest_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else { return Ok(()) };
    assert!(!admin.is_empty());
    run("brainout-wrong")
}
fn run(name: &'static str) -> TestResult {
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(name, move || {
        super::quest_world_callbacks::scenario(name, SOURCE, false)
    });
    eprintln!("POSTGRES_CASE_COMPLETED {name} duration={:?} success={}", started.elapsed(), result.is_ok());
    result
}
pub(super) fn callback() -> TestResult<Callback> {
    quest::world_callbacks::load()?.into_iter().find(|c| c.key == KEY)
        .ok_or_else(|| "BRAINOUT production callback is missing".into())
}
pub(super) fn catalogue(original: &quest::QuestStateCatalogue, callback: &Callback, wrong: bool) -> TestResult<quest::QuestStateCatalogue> {
    // Arrange the NPC prerequisite only; the production USE owns the brain mission.
    let tracks = callback.storage.iter().map(|storage| {
        let mut track = original.track(&storage.track).cloned().ok_or("brain mission track")?;
        track.initial = if storage.preserve && !wrong { 1 } else { track.initial };
        Ok(track)
    }).collect::<Result<Vec<_>, &str>>()?;
    quest::QuestStateCatalogue::new(original.content_revision(), tracks,
        vec![original.transition(KEY).cloned().ok_or("brain mission transition")?]).map_err(debug)
}
pub(super) fn world(native: &crate::content::native_gameplay::NativeGameplayState) -> TestResult<crate::map::boot::BundleWorld> {
    super::source_world_crop::world(
        crate::map::overlay::TilePos { x:33019,y:32328,floor:-10 },
        crate::map::overlay::TilePos { x:33026,y:32334,floor:-10 },
        crate::map::overlay::TilePos { x:START.x,y:START.y,floor:START.floor }, native)
}
pub(super) async fn exercise(owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession,
    h: &Harness, fence: CurrentCharacterGameplayFence, callback: &Callback, name: &str) -> TestResult {
    let source = callback.selector.native.as_ref().ok_or("native brain control")?;
    let (key, placement) = {
        let map = owner.bundle_map().ok_or("map")?;
        let key = oteryn_world_bundle::bundle::placement_key(SOURCE.floor,SOURCE.x,SOURCE.y,source.ordinal).ok_or("control placement")?;
        let mut placement = map.overlay.base().digest().to_vec(); placement.extend_from_slice(&key.to_be_bytes());
        (key,placement)
    };
    let payload = |revision| oteryn_protocol_oteryn::world_object::encode_use_intent(&WorldObjectTarget { placement:placement.clone(),expected_revision:revision }).map_err(debug);
    let use_ = async |owner: &mut ComposedFreshAdmission<'_, '_, '_>, admitted: &mut AdmittedSession, command, revision| -> TestResult<UseDisposition> {
        owner.clock_origin -= std::time::Duration::from_secs(2);
        let reply = Box::pin(super::quest_world_callbacks::wire(owner,admitted,command,2,payload(revision)?)).await?;
        oteryn_protocol_oteryn::world_object::decode_use_result(&reply).map_err(|e| debug(e).into())
    };
    let first = &callback.offerings[0];
    let second = &callback.offerings[1];
    super::quest_world_callbacks::input_item(owner,fence.item_fence(),first,8701,false).await?;
    owner.states.loop_ground.invalidate();
    assert_eq!(use_(owner,admitted,1,0).await?,UseDisposition::Rejected,"one brain refuses");
    assert_eq!(counts(h).await?,(1,0,0),"missing input must roll back");
    super::quest_world_callbacks::input_item(owner,fence.item_fence(),second,8702,false).await?;
    if name.ends_with("-full") {
        for n in 0..20 {
            super::quest_world_callbacks::input_item(owner,fence.item_fence(),&quest::world_callbacks::Offering {
                position:START,item:"oteryn:item.tibia.i3031".into(),count:1,
            },8800+n,true).await?;
        }
        let entries:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid")
            .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(entries,20,"full backpack fixture");
    }
    owner.states.loop_ground.invalidate();
    if name.ends_with("-wrong") {
        assert_eq!(use_(owner,admitted,2,0).await?,UseDisposition::Rejected);
        assert_eq!(counts(h).await?,(2,0,0));
        return Ok(());
    }
    assert_eq!(use_(owner,admitted,2,0).await?,UseDisposition::Committed);
    assert_eq!(counts(h).await?,(0,1,1));
    let state=owner.root.read_character_quest_state(owner.character,fence.character_id).await.map_err(debug)?;
    assert_eq!(state.tracks().get(&callback.storage[1].track),Some(&1));
    let ground:(Vec<u8>,i64)=sqlx::query_as("SELECT g.spatial_position,i.quantity FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i19078' AND i.lifecycle=1")
        .fetch_one(&h.pool).await?;
    let mut expected=33022_i32.to_be_bytes().to_vec();expected.extend_from_slice(&32332_i32.to_be_bytes());expected.extend_from_slice(&(-10_i16).to_be_bytes());
    assert_eq!(ground,(expected,1));
    let carried:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries c JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i19078'").fetch_one(&h.pool).await?;
    assert_eq!(carried,0,"donor always creates the reward on ground");
    assert_eq!(use_(owner,admitted,2,0).await?,UseDisposition::Committed,"same command replays");
    assert_eq!(counts(h).await?,(0,1,1));
    assert_eq!(owner.bundle_map().ok_or("map")?.facts.object_revision(key),1);
    assert_eq!(use_(owner,admitted,3,1).await?,UseDisposition::Rejected,"new command cannot repeat quest");
    assert_eq!(counts(h).await?,(0,1,1));
    Ok(())
}
async fn counts(h:&Harness)->TestResult<(i64,i64,i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i9659'),(SELECT count(*) FROM game_item_instances WHERE lifecycle=1 AND definition_production_key='oteryn:item.tibia.i19078'),(SELECT count(*) FROM game_character_quest_receipts)").fetch_one(&h.pool).await?)
}
