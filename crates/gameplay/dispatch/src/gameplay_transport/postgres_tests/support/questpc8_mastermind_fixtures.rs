use super::fixture::{Harness, id};

use super::questpc8_authored_programmes_fixtures::{object_target, step, use_target};

use super::TestResult;

use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};

use crate::map::{overlay::TilePos, view::MapFacts};

use oteryn_protocol_oteryn::world_object::UseDisposition;

use oteryn_protocol_oteryn::world_spatial::StepDirection;

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
) -> TestResult {
    let ids = [7440, 7159, 7158, 3031, 2853];
    let mut before = Vec::new();
    for item in ids {
        before.push(quantity(h, item).await?);
    }
    let premium = owner.premium_coordinator.take();
    step(owner, admitted, 1, StepDirection::East).await?;
    assert_eq!(
        progress(h).await?,
        -1,
        "absent native Premium authority cannot start the journey"
    );
    owner.premium_coordinator = premium;
    owner.admit_premium(id(40), admitted.game_session_id);
    step(owner, admitted, 2, StepDirection::West).await?;
    step(owner, admitted, 3, StepDirection::East).await?;
    assert_eq!(progress(h).await?, 1);
    step(owner, admitted, 4, StepDirection::North).await?;
    let first = TilePos {
        x: 32216,
        y: 31168,
        floor: -11,
    };
    let target = object_target(owner, first, 1)?;
    assert_eq!(
        use_target(owner, admitted, 5, &target).await?,
        UseDisposition::Committed
    );
    assert_eq!(progress(h).await?, 2);
    assert_eq!(quantity(h, 7440).await? - before[0], 1);
    assert_eq!(quantity(h, 7159).await?, before[1]);
    assert_eq!(quantity(h, 7158).await?, before[2]);
    assert_eq!(quantity(h, 3031).await? - before[3], 78);
    assert_bag(h, &[(7440, 1), (3031, 78)]).await?;
    let target = object_target(owner, first, 1)?;
    assert_eq!(
        use_target(owner, admitted, 6, &target).await?,
        UseDisposition::Rejected
    );
    super::questpc8_mystery_geometry_fixtures::arrive(
        owner,
        admitted,
        TilePos {
            x: 32146,
            y: 31120,
            floor: -9,
        },
    )
    .await?;
    assert_eq!(progress(h).await?, 2, "fixture move earns no progress");
    let entrance = TilePos {
        x: 32145,
        y: 31120,
        floor: -9,
    };
    let map = owner.bundle_map().ok_or("Formorgar map")?;
    let key = crate::map::view::placement_key(entrance, 0).ok_or("mine entrance key")?;
    let tile = map
        .overlay
        .base()
        .tile(entrance.x, entrance.y, entrance.floor)
        .ok_or("native mine entrance")?;
    let ground_id = *tile.ids().first().ok_or("mine entrance ground")?;
    assert_eq!(
        map.facts
            .base_entry(entrance, 0, ground_id)
            .map(|facts| facts.appearance_id),
        Some(6683)
    );
    let step_bindings: Vec<_> = owner
        .callback_content()
        .iter()
        .filter(|callback| {
            callback.trigger == oteryn_gameplay_quests::quest::world_callbacks::Trigger::StepIn
                && crate::gameplay_transport::quest_callbacks::matches(map, callback, key)
        })
        .map(|callback| callback.key.as_str())
        .collect();
    assert_eq!(
        step_bindings,
        ["oteryn:quest-callback/authored-mastermind-second-route"]
    );
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    step(owner, admitted, 7, StepDirection::West).await?;
    assert_eq!(progress(h).await?, 3, "return to the mine entrance");
    let second = TilePos {
        x: 32084,
        y: 31062,
        floor: -10,
    };
    super::questpc8_mystery_geometry_fixtures::arrive(
        owner,
        admitted,
        TilePos {
            x: 32084,
            y: 31063,
            floor: -10,
        },
    )
    .await?;
    let target = object_target(owner, second, 2)?;
    let premium = owner.premium_coordinator.take();
    assert_eq!(
        use_target(owner, admitted, 8, &target).await?,
        UseDisposition::Rejected
    );
    assert_eq!(progress(h).await?, 3);
    assert_eq!(quantity(h, 7440).await? - before[0], 1);
    assert_eq!(quantity(h, 7159).await?, before[1]);
    assert_eq!(quantity(h, 7158).await?, before[2]);
    assert_eq!(quantity(h, 3031).await? - before[3], 78);
    owner.premium_coordinator = premium;
    owner.admit_premium(id(40), admitted.game_session_id);
    assert_eq!(
        use_target(owner, admitted, 9, &target).await?,
        UseDisposition::Committed
    );
    assert_eq!(progress(h).await?, 4);
    assert_bag(
        h,
        &[(7440, 1), (3031, 100), (3031, 23), (7159, 1), (7158, 2)],
    )
    .await?;
    for ((item, amount), initial) in [(7440, 2), (7159, 1), (7158, 2), (3031, 201), (2853, 2)]
        .into_iter()
        .zip(&before)
    {
        assert_eq!(quantity(h, item).await? - initial, amount);
    }
    assert_eq!(
        use_target(owner, admitted, 9, &target).await?,
        UseDisposition::Committed
    );
    let target = object_target(owner, second, 2)?;
    assert_eq!(
        use_target(owner, admitted, 10, &target).await?,
        UseDisposition::Rejected
    );
    for ((item, amount), initial) in [(7440, 2), (7159, 1), (7158, 2), (3031, 201), (2853, 2)]
        .into_iter()
        .zip(&before)
    {
        assert_eq!(quantity(h, item).await? - initial, amount);
    }
    let completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.authored.mastermind_potion_quest'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
    assert!(completed);
    {
        let mut sessions = owner
            .states
            .quest_sessions
            .lock()
            .map_err(|_| "quest cache")?;
        sessions.remove(&admitted.game_session_id);
    }
    owner.admit_quest_session(admitted).await;
    let target = object_target(owner, second, 2)?;
    assert_eq!(
        use_target(owner, admitted, 11, &target).await?,
        UseDisposition::Rejected
    );
    for ((item, amount), initial) in [(7440, 2), (7159, 1), (7158, 2), (3031, 201), (2853, 2)]
        .into_iter()
        .zip(&before)
    {
        assert_eq!(quantity(h, item).await? - initial, amount);
    }
    Ok(())
}

async fn assert_bag(h: &Harness, expected: &[(u16, i64)]) -> TestResult {
    let children: Vec<(String, i64)> = sqlx::query_as(
        "SELECT item.definition_production_key, item.quantity
         FROM game_item_instances item
         JOIN game_item_nested_entries entry USING(item_instance_id)
         WHERE entry.parent_item_instance_id = (
             SELECT item_instance_id FROM game_item_instances
             WHERE definition_production_key='oteryn:item.tibia.i2853' AND lifecycle=1
             ORDER BY item_instance_id DESC LIMIT 1
         ) ORDER BY entry.placement_ordinal",
    )
    .fetch_all(&h.pool)
    .await?;
    let expected: Vec<_> = expected
        .iter()
        .map(|(id, count)| (format!("oteryn:item.tibia.i{id}"), *count))
        .collect();
    assert_eq!(children, expected);
    Ok(())
}

async fn quantity(h: &Harness, item: u16) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1").bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}

async fn progress(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT COALESCE((SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/chosen/mastermind/stage'),-1)").bind(id(41).as_slice()).fetch_one(&h.pool).await?)
}
