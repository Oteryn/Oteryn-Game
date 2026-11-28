use super::TestResult;
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;

use super::fixture::{Harness, debug, id};

use super::questpc8_authored_programmes_fixtures::{object_target, step, use_target};

use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};

use crate::map::{overlay::TilePos, view::MapFacts};

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
    step(owner, admitted, 1, StepDirection::North).await?;
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
    let before_gas = TilePos {
        x: 32494,
        y: 32215,
        floor: -12,
    };
    let map = owner.bundle_map().ok_or("temple map")?;
    for x in 32494..=32497 {
        let field = TilePos {
            x,
            y: 32216,
            floor: -12,
        };
        let tile = map
            .overlay
            .base()
            .tile(x, field.y, field.floor)
            .ok_or("poison tile")?;
        let id = *tile.ids().get(1).ok_or("poison field item")?;
        assert_eq!(
            map.facts
                .base_entry(field, 1, id)
                .map(|facts| facts.appearance_id),
            Some(2121)
        );
    }
    super::questpc8_mystery_geometry_fixtures::arrive(owner, admitted, before_gas).await?;
    step(owner, admitted, 2, StepDirection::South).await?;
    for command in 3..=5 {
        step(owner, admitted, command, StepDirection::East).await?;
    }
    assert_eq!(progress(h, "gas-route").await?, 1);
    super::questpc8_mystery_geometry_fixtures::arrive(
        owner,
        admitted,
        TilePos {
            x: 32526,
            y: 32156,
            floor: -12,
        },
    )
    .await?;
    step(owner, admitted, 6, StepDirection::North).await?;

    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    assert!(
        owner
            .quest_guardian_for_test("oteryn:quest-death/authored-kormarak-guardian")
            .await
            .is_none(),
        "the source spawn owns the Demon Skeleton population"
    );
    assert_eq!(progress(h, "stage").await?, 2);
    super::questpc8_mystery_geometry_fixtures::arrive(
        owner,
        admitted,
        TilePos {
            x: 32552,
            y: 32201,
            floor: -11,
        },
    )
    .await?;
    let position = TilePos {
        x: 32552,
        y: 32200,
        floor: -11,
    };
    let target = object_target(owner, position, 1)?;
    assert_eq!(
        use_target(owner, admitted, 7, &target).await?,
        UseDisposition::Rejected
    );
    let approach = TilePos {
        x: 32552,
        y: 32199,
        floor: -11,
    };
    super::questpc8_mystery_geometry_fixtures::arrive(owner, admitted, approach).await?;
    let guardian = {
        let mut runtime = owner.runtime.lock().await;
        let creature = "oteryn:creature.demon_skeleton";
        let health = runtime
            .companion_policy(creature)
            .map_err(debug)?
            .maximum_health;
        let target = runtime
            .admit_encounter_actor(
                creature,
                health,
                crate::foundation::MovementLocalPosition {
                    x: 32552,
                    y: 32198,
                    floor: 11,
                },
            )
            .map_err(debug)?;
        runtime
            .install_creature_policy(target, creature)
            .map_err(debug)?;
        target
    };
    super::grizzly_npc_fixtures::questpc8_campaign::lethal(
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
    super::questpc8_mystery_geometry_fixtures::arrive(
        owner,
        admitted,
        TilePos {
            x: 32552,
            y: 32201,
            floor: -11,
        },
    )
    .await?;
    let outputs = [(3359, 1), (3354, 1), (3276, 1), (3287, 13)];
    let mut before = Vec::new();
    for (item, _) in outputs {
        before.push(quantity(h, &format!("oteryn:item.tibia.i{item}")).await?);
    }
    assert_eq!(
        use_target(owner, admitted, 8, &target).await?,
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
        use_target(owner, admitted, 8, &target).await?,
        UseDisposition::Committed
    );
    let target = object_target(owner, position, 1)?;
    assert_eq!(
        use_target(owner, admitted, 9, &target).await?,
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
    let target = object_target(owner, position, 1)?;
    assert_eq!(
        use_target(owner, admitted, 10, &target).await?,
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
