use super::annihilator_route_fixtures::{step, use_target};
use super::fixture::debug;
use super::quest_world_callbacks_fixtures::local;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::map::{boot::BundleMap, view::MapFacts};
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use oteryn_protocol_oteryn::world_spatial::StepDirection::{North, South};
use std::sync::atomic::Ordering;

pub(super) async fn demon_helmet(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    members: &mut [AdmittedSession],
    map: &BundleMap,
) -> TestResult {
    let keys = [33210, 33211, 33212].map(|x| {
        crate::map::view::placement_key(
            crate::map::overlay::TilePos {
                x,
                y: 31630,
                floor: -13,
            },
            1,
        )
    });
    let [Some(left), Some(center), Some(right)] = keys else {
        return Err("wall placements".into());
    };
    let hidden = || [left, center, right].map(|key| map.facts.base_hidden(key));

    step(owner, &mut members[0], North).await?;
    assert_eq!(hidden(), [false; 3], "one occupied plate");
    step(owner, &mut members[1], North).await?;
    assert_eq!(hidden(), [true; 3], "both exact plates occupied");
    step(owner, &mut members[2], North).await?;
    assert_position(owner, &members[2], 33211, 31630).await?;
    step(owner, &mut members[0], South).await?;
    assert_eq!(hidden(), [false; 3], "a player left the first plate");
    assert_position(owner, &members[2], 33211, 31631).await?;
    step(owner, &mut members[0], North).await?;
    assert_eq!(hidden(), [true; 3], "the same plates can reopen the wall");
    step(owner, &mut members[1], South).await?;
    assert_eq!(hidden(), [false; 3], "either player can close the wall");
    Ok(())
}

pub(super) async fn claimed_station(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    members: &mut [AdmittedSession],
    lever: &WorldObjectTarget,
    pool: &sqlx::PgPool,
) -> TestResult {
    let fence = owner
        .current_quest_fence(members[3].game_session_id)
        .await
        .map_err(|()| "station character fence")?
        .ok_or("station character")?;
    // Seed the persisted once-claim in the isolated fixture. Admission must read
    // that actual owner row, independently of the approach door and its state.
    let mut tx = pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "INSERT INTO game_reward_claims (character_id, claim_family, claim_production_key, \
         claim_revision_ref, claimed_transaction_id, claimed_at) \
         VALUES (encode($1,'hex')::uuid, 'RewardClaim', \
         'oteryn:reward-claim.quest.u7_24.the_annihilator.reward', 'reward-claim-r1', \
         encode($2,'hex')::uuid, floor(extract(epoch FROM clock_timestamp()))::bigint)",
    )
    .bind(fence.character_id.as_bytes().as_slice())
    .bind(super::fixture::id(249).as_slice())
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    assert_eq!(
        use_target(owner, &mut members[0], lever).await?,
        UseDisposition::NotPossible
    );
    for (member, x) in members.iter().zip([33225, 33224, 33223, 33222]) {
        assert_position(owner, member, x, 31671).await?;
    }
    let map = owner.bundle_map().ok_or("source map")?;
    assert_eq!(
        super::annihilator_route_fixtures::target(map, 33226, 31671)?.expected_revision,
        lever.expected_revision
    );
    assert!(
        owner
            .runtime
            .lock()
            .await
            .visible_entities()
            .creatures
            .is_empty()
    );
    Ok(())
}

pub(super) async fn arena_deadline(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    members: &[AdmittedSession],
    map: &BundleMap,
) -> TestResult {
    let admitted_at = owner.owner_now().get();
    // One member has passed the reward door and a different session now occupies
    // their vacated cell. Neither should move when this run expires.
    let closed = crate::map::floor::BundleDoors(&map.facts);
    {
        let mut runtime = owner.runtime.lock().await;
        for (member, (x, y)) in [(&members[0], (33227, 31657)), (&members[4], (33222, 31659))] {
            let actor = member.runtime_actor.ok_or("actor")?;
            let proof = crate::map::floor::prepare_quest_encounter_ejection(
                &runtime,
                &owner.movement_cells,
                actor,
                member.game_session_id,
                runtime.read_actor_position(actor).map_err(debug)?,
                local(Position { x, y, floor: -13 }),
                &closed,
            )
            .map_err(debug)?;
            runtime.commit_source_step(proof).map_err(debug)?;
        }
    }

    owner
        .states
        .test_owner_time
        .store(admitted_at + 600_000_000, Ordering::Relaxed);
    owner.drain_quest_encounter_deadlines().await;
    for (member, x) in members[1..4].iter().zip([33221, 33220, 33219]) {
        assert_position(owner, member, x, 31659).await?;
    }

    owner
        .states
        .test_owner_time
        .store(admitted_at + 600_000_001, Ordering::Relaxed);
    owner.drain_quest_encounter_deadlines().await;
    assert_position(owner, &members[0], 33227, 31657).await?;
    assert_position(owner, &members[4], 33222, 31659).await?;
    for member in &members[1..4] {
        assert_position(owner, member, 33227, 31659).await?;
    }
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .visible_entities()
            .creatures
            .len(),
        6
    );
    owner.drain_quest_encounter_deadlines().await;
    for member in &members[1..4] {
        assert_position(owner, member, 33227, 31659).await?;
    }
    Ok(())
}

async fn assert_position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    member: &AdmittedSession,
    x: i32,
    y: i32,
) -> TestResult {
    let at = owner
        .runtime
        .lock()
        .await
        .read_actor_position(member.runtime_actor.ok_or("actor")?)
        .map_err(debug)?
        .position();
    assert_eq!((at.x, at.y, at.floor), (x, y, 13));
    Ok(())
}
