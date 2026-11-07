//! Chosen Voodoo cache uses the existing death and multi-output reward owners.
use super::{TestResult, quest_world_callbacks};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc8_voodoo_three_native_witches_two_boxes_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("questpc8-voodoo", || {
        quest_world_callbacks::scenario(
            "authored-voodoo",
            Position {
                x: 32755,
                y: 31953,
                floor: -9,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_voodoo_three_native_witches_two_boxes_and_replay duration={:?}",
        started.elapsed()
    );
    Ok(())
}

use super::fixture::{Harness, debug, id};
use super::questpc8_authored_programmes::{object_target, step, use_target};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use crate::map::overlay::TilePos;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::world_object::UseDisposition;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    owner.premium_coordinator = None;
    let native = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .ok_or("native")?;
    native
        .install_companion_policies(&mut *owner.runtime.lock().await)
        .map_err(debug)?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    owner
        .runtime
        .lock()
        .await
        .bind_attacker_lease(
            admitted.runtime_actor.ok_or("actor")?,
            admitted.game_session_id,
            crate::foundation::CharacterLease::new(
                crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                    .map_err(debug)?,
                fence.character_lease_generation,
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let doll_before = quantity(h, "oteryn:item.tibia.i3209").await?;
    let wand_before = quantity(h, "oteryn:item.tibia.i3046").await?;
    step(owner, admitted, 1, StepDirection::East).await?;
    assert_eq!(progress(h,"stage").await?,1);
    step(owner, admitted, 2, StepDirection::East).await?;
    let first = TilePos {x:32757,y:31953,floor:-9};
    let target = object_target(owner, first, 1)?;
    assert_eq!(use_target(owner, admitted, 3, &target).await?,UseDisposition::Rejected);
    step(owner, admitted, 4, StepDirection::West).await?;
    super::grizzly_npc::questpc8_campaign::hunt(owner, admitted,"oteryn:creature.witch",3,4).await?;
    assert_eq!(progress(h,"kills").await?,3);
    step(owner, admitted, 5, StepDirection::East).await?;
    assert_eq!(use_target(owner, admitted, 6, &target).await?,UseDisposition::Committed);
    assert_eq!(progress(h,"stage").await?,2);
    assert_eq!(quantity(h,"oteryn:item.tibia.i3209").await?-doll_before,1);
    assert_eq!(quantity(h,"oteryn:item.tibia.i3046").await?,wand_before);
    let target = object_target(owner,first,1)?;
    assert_eq!(use_target(owner, admitted, 7, &target).await?,UseDisposition::Rejected);
    for command in 8..12 { step(owner, admitted, command, StepDirection::South).await?; }
    let second = TilePos{x:32757,y:31957,floor:-9};
    let target = object_target(owner,second,1)?;
    assert_eq!(use_target(owner, admitted, 12, &target).await?,UseDisposition::Committed);
    assert_eq!(progress(h,"stage").await?,3);
    assert_eq!(quantity(h,"oteryn:item.tibia.i3209").await?-doll_before,1);
    assert_eq!(quantity(h,"oteryn:item.tibia.i3046").await?-wand_before,1);
    assert_eq!(use_target(owner, admitted, 12, &target).await?,UseDisposition::Committed);
    let target = object_target(owner,second,1)?;
    assert_eq!(use_target(owner, admitted, 13, &target).await?,UseDisposition::Rejected);
    assert_eq!(quantity(h,"oteryn:item.tibia.i3209").await?-doll_before,1);
    assert_eq!(quantity(h,"oteryn:item.tibia.i3046").await?-wand_before,1);
    let completed:bool = sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.authored.voodoo_doll_quest'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    Ok(())
}

async fn quantity(h: &Harness,key:&str)->TestResult<i64>{
 Ok(sqlx::query_scalar("SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1").bind(key).fetch_one(&h.pool).await?)
}
async fn progress(h: &Harness,suffix:&str)->TestResult<i64>{
 Ok(sqlx::query_scalar("SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),-1)").bind(id(41).as_slice()).bind(format!("oteryn:quest-progress/chosen/voodoo/{suffix}")).fetch_one(&h.pool).await?)
}
