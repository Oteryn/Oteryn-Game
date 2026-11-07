//! Chosen Kormarak cache uses the existing death and multi-output reward owners.
use super::{TestResult, quest_world_callbacks};
use oteryn_gameplay_quests::quest::world_callbacks::Position;

#[test]
fn questpc8_kormarak_native_guardian_body_and_four_outputs() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("questpc8-kormarak", || {
        quest_world_callbacks::scenario(
            "authored-kormarak",
            Position {
                x: 32340,
                y: 32125,
                floor: -10,
            },
            false,
        )
    })?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED questpc8_kormarak_native_guardian_body_and_four_outputs duration={:?}",
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
    assert!(
        owner
            .quest_guardian_for_test("oteryn:quest-death/authored-kormarak-guardian")
            .await
            .is_none()
    );
    step(owner, admitted, 1, StepDirection::East).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(progress(h, "stage").await?, 1);
    assert!(
        owner
            .quest_guardian_for_test("oteryn:quest-death/authored-kormarak-guardian")
            .await
            .is_none()
    );
    step(owner, admitted, 2, StepDirection::East).await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    let guardian = owner
        .quest_guardian_for_test("oteryn:quest-death/authored-kormarak-guardian")
        .await
        .ok_or("the native objective must create its guardian")?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(
        owner
            .quest_guardian_for_test("oteryn:quest-death/authored-kormarak-guardian")
            .await,
        Some(guardian),
        "refresh cannot duplicate the guardian"
    );
    assert_eq!(progress(h, "stage").await?, 2);
    let position = TilePos {
        x: 32342,
        y: 32125,
        floor: -10,
    };
    let target = object_target(owner, position, 2)?;
    assert_eq!(
        use_target(owner, admitted, 3, &target).await?,
        UseDisposition::Rejected
    );
    super::grizzly_npc::questpc8_campaign::lethal(
        owner,
        admitted,
        guardian,
        "oteryn:creature.demon_skeleton",
        3,
        1,
    )
    .await?;
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert_eq!(progress(h, "kills").await?, 1);
    assert!(
        owner
            .quest_guardian_for_test("oteryn:quest-death/authored-kormarak-guardian")
            .await
            .is_none()
    );
    let outputs = [(3359, 1), (3354, 1), (3276, 1), (3287, 13)];
    let mut before = Vec::new();
    for (item, _) in outputs {
        before.push(quantity(h, &format!("oteryn:item.tibia.i{item}")).await?);
    }
    assert_eq!(
        use_target(owner, admitted, 4, &target).await?,
        UseDisposition::Committed
    );
    assert_eq!(progress(h, "stage").await?, 3);
    for ((item, amount), initial) in outputs.into_iter().zip(&before) {
        assert_eq!(
            quantity(h, &format!("oteryn:item.tibia.i{item}")).await? - initial,
            amount
        );
    }
    assert_eq!(
        use_target(owner, admitted, 4, &target).await?,
        UseDisposition::Committed
    );
    let target = object_target(owner, position, 2)?;
    assert_eq!(
        use_target(owner, admitted, 5, &target).await?,
        UseDisposition::Rejected
    );
    for ((item, amount), initial) in outputs.into_iter().zip(&before) {
        assert_eq!(
            quantity(h, &format!("oteryn:item.tibia.i{item}")).await? - initial,
            amount
        );
    }
    owner
        .states
        .quest_sessions
        .lock()
        .map_err(|_| "quest cache")?
        .remove(&admitted.game_session_id);
    owner.admit_quest_session(admitted).await;
    let target = object_target(owner, position, 2)?;
    assert_eq!(
        use_target(owner, admitted, 6, &target).await?,
        UseDisposition::Rejected
    );
    for ((item, amount), initial) in outputs.into_iter().zip(&before) {
        assert_eq!(
            quantity(h, &format!("oteryn:item.tibia.i{item}")).await? - initial,
            amount
        );
    }
    let completed: bool = sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.authored.kingdom_of_kormarak_quest'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    Ok(())
}

async fn quantity(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1").bind(key).fetch_one(&h.pool).await?)
}
async fn progress(h: &Harness, suffix: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2),-1)").bind(id(41).as_slice()).bind(format!("oteryn:quest-progress/chosen/kormarak/{suffix}")).fetch_one(&h.pool).await?)
}
