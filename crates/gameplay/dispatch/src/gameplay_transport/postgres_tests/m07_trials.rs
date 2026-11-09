//! Source Lion trial admission through real USE and the durable Quest/Item owners.
use super::TestResult;
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn m07_lion_strength_offering_and_shared_reset() -> TestResult {
    trial("lions-rock-strength-first", Position { x:33137, y:32291, floor:-8 })
}



fn trial(name: &'static str, position: Position) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("m07-lion-trial", move || {
        super::quest_world_callbacks::scenario(name, position, false)
    })
}

// The shared scenario supplies the admitted character and exact source geometry.
#[allow(clippy::too_many_arguments)]
pub(super) async fn exercise(
    owner: &mut super::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut super::AdmittedSession,
    h: &super::fixture::Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
    callback: &oteryn_gameplay_quests::quest::world_callbacks::Callback,
    trigger: Position,
) -> TestResult {
    use super::fixture::debug;
    use crate::map::view::MapFacts;
    use oteryn_protocol_oteryn::world_object::{WorldObjectTarget,UseDisposition,encode_use_intent,decode_use_result};
    use oteryn_gameplay_quests::quest::{lions_rock,world_callbacks::Offering};
    let key = oteryn_world_bundle::bundle::placement_key(trigger.floor,trigger.x,trigger.y,1).ok_or("trial key")?;
    let map = owner.bundle_map().ok_or("trial map")?;
    let mut placement = map.overlay.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let payload = encode_use_intent(&WorldObjectTarget {placement:placement.clone(),expected_revision:0}).map_err(debug)?;
    let index = lions_rock::binding(&callback.key).ok_or("trial binding")?;
    let before = owner.root.read_character_quest_state(authority,fence.character_id).await.map_err(debug)?;
    assert_eq!(before.tracks().get(lions_rock::QUESTLINE).copied().unwrap_or(0),0);
    let reply = super::quest_world_callbacks::wire(owner,admitted,1,2,payload.clone()).await?;
    assert_eq!(decode_use_result(&reply).map_err(debug)?,UseDisposition::Rejected);
    let counts: (i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts),(SELECT count(*) FROM game_quest_callback_consumptions)").fetch_one(&h.pool).await?;
    assert_eq!(counts,(0,0),"missing input cannot advance the trial");
    let item = callback.item_on_target.as_ref().ok_or("trial input")?;
    super::quest_world_callbacks::input_item(owner,fence.item_fence(),&Offering {position:trigger,item:item.item.clone(),count:1},60,true).await?;
    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(1)).ok_or("trial pacing")?;
    let reply = super::quest_world_callbacks::wire(owner,admitted,2,2,payload.clone()).await?;
    assert_eq!(decode_use_result(&reply).map_err(debug)?,UseDisposition::Committed);
    let state = owner.root.read_character_quest_state(authority,fence.character_id).await.map_err(debug)?;
    assert_eq!(state.tracks().get(lions_rock::QUESTLINE),Some(&2));
    assert_eq!(state.tracks().get(lions_rock::FLAGS[index]),Some(&1));
    let input: (i64,i64,bool) = sqlx::query_as("SELECT i.quantity,i.lifecycle::bigint,sha256(c.proof)=c.proof_sha256 FROM game_quest_callback_consumptions c JOIN game_item_instances i USING(item_instance_id) WHERE c.transition_key=$1").bind(&callback.key).fetch_one(&h.pool).await?;
    assert_eq!(input,(0,2,true));
    let p = callback.passages.first().ok_or("pagoda")?.position;
    let pos = crate::map::overlay::TilePos{x:p.x,y:p.y,floor:p.floor};
    let pagoda = oteryn_world_bundle::bundle::placement_key(p.floor,p.x,p.y,1).ok_or("pagoda key")?;
    let map = owner.bundle_map().ok_or("trial map")?;
    assert_eq!(map.facts.base_entry(pos,1,1).ok_or("lit pagoda")?.appearance_id,2075);
    let due = map.facts.next_movement_decay_time().ok_or("shared reset deadline")?;
    assert_eq!(map.facts.expire_movement_items(due-1),0);
    assert_eq!(map.facts.object_revision(pagoda),1);
    let replay = super::quest_world_callbacks::wire(owner,admitted,2,2,payload).await?;
    assert_eq!(decode_use_result(&replay).map_err(debug)?,UseDisposition::Committed);
    let map = owner.bundle_map().ok_or("trial map")?;
    assert_eq!(map.facts.next_movement_decay_time(),Some(due),"replay must not extend the shared deadline");
    assert_eq!(map.facts.expire_movement_items(due),1);
    assert_eq!(map.facts.base_entry(pos,1,1).ok_or("reset pagoda")?.appearance_id,2074);
    assert_eq!(map.facts.object_revision(pagoda),2);
    assert_eq!(map.facts.expire_movement_items(due),0);
    // A fresh offering after the shared reset still cannot repeat this personal trial.
    super::quest_world_callbacks::input_item(owner,fence.item_fence(),&Offering{position:trigger,item:item.item.clone(),count:1},61,true).await?;
    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(1)).ok_or("trial pacing")?;
    let payload = encode_use_intent(&WorldObjectTarget {placement,expected_revision:1}).map_err(debug)?;
    let reply = super::quest_world_callbacks::wire(owner,admitted,3,2,payload).await?;
    assert_eq!(decode_use_result(&reply).map_err(debug)?,UseDisposition::Rejected);
    let counts: (i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_character_quest_receipts),(SELECT count(*) FROM game_quest_callback_consumptions)").fetch_one(&h.pool).await?;
    assert_eq!(counts,(1,1));
    let state = owner.root.read_character_quest_state(authority,fence.character_id).await.map_err(debug)?;
    assert_eq!(state.tracks().get(lions_rock::QUESTLINE),Some(&2));
    assert_eq!(state.tracks().get(lions_rock::FLAGS[index]),Some(&1));
    eprintln!("M07 POSTGRES_CASE_COMPLETED real_USE missing_input atomic_trial_input shared_pagoda_reset replay_no_extension personal_once");
    Ok(())
}
