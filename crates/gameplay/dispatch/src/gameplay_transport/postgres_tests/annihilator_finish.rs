//! Literal source chest selection, shared claim replay, exit and claimed-door refusal.
use super::annihilator_route::{step, target, use_target};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, fixture::debug};
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::*;
use oteryn_protocol_oteryn::world_spatial::StepDirection::{East, North, South};

const CLAIM: &str = "oteryn:reward-claim.quest.u7_24.the_annihilator.reward";
const REWARD: &str = "oteryn:item.tibia.i3388";

async fn position(
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
    assert_eq!(
        (at.x, at.y, at.floor),
        (x, y, 13),
        "actual source route position"
    );
    Ok(())
}

pub(super) async fn outer_gate(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    member: &mut AdmittedSession,
    map: &crate::map::boot::BundleMap,
) -> TestResult {
    position(owner, member, 33213, 31671).await?;
    assert_eq!(
        use_target(owner, member, &target(map, 33214, 31671)?).await?,
        UseDisposition::Committed,
        "native source gate of expertise"
    );
    step(owner, member, East).await?;
    position(owner, member, 33214, 31671).await?;
    step(owner, member, East).await?;
    position(owner, member, 33215, 31671).await
}

async fn durable_counts(pool: &sqlx::PgPool) -> TestResult {
    let counts: (i64, i64, i64, i64) = sqlx::query_as("SELECT \
        (SELECT count(*) FROM game_reward_claims WHERE claim_production_key=$1), \
        (SELECT count(*) FROM game_reward_claim_mint_receipts), \
        (SELECT count(*) FROM game_item_instances WHERE definition_production_key=$2), \
        (SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/annihilator')")
        .bind(CLAIM).bind(REWARD).fetch_one(pool).await?;
    assert_eq!(
        counts,
        (1, 1, 1, 1),
        "one physical reward, shared claim, receipt and achievement"
    );
    Ok(())
}

pub(super) async fn complete(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    member: &mut AdmittedSession,
    map: &crate::map::boot::BundleMap,
    pool: &sqlx::PgPool,
) -> TestResult {
    position(owner, member, 33224, 31659).await?;
    assert_eq!(
        use_target(owner, member, &target(map, 33225, 31659)?).await?,
        UseDisposition::Committed,
        "source reward-room door"
    );
    // The captured statues at33226,31658 and33234,31658 are actual blockers.
    for direction in [East, East, East, North, North] {
        step(owner, member, direction).await?;
    }
    position(owner, member, 33227, 31657).await?;
    let chest = target(map, 33227, 31656)?;
    let command = member.continuity.next_command_id;
    assert_eq!(
        use_target(owner, member, &chest).await?,
        UseDisposition::Committed,
        "earn the actual source armor chest"
    );
    durable_counts(pool).await?;
    let replay = super::quest_world_callbacks::wire(
        owner,
        member,
        command,
        2,
        encode_use_intent(&chest).map_err(debug)?,
    )
    .await?;
    assert_eq!(
        decode_use_result(&replay).map_err(debug)?,
        UseDisposition::Committed,
        "literal same-command replay"
    );
    assert_eq!(
        use_target(owner, member, &chest).await?,
        UseDisposition::NothingToUse
    );
    step(owner, member, East).await?;
    assert_eq!(
        use_target(owner, member, &target(map, 33229, 31656)?).await?,
        UseDisposition::NothingToUse,
        "other source choice shares the durable claim"
    );
    durable_counts(pool).await?;
    let fence = owner
        .current_quest_fence(member.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("session")?;
    let backpack = owner
        .root
        .read_character_backpack(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("backpack")?;
    let rewards = backpack
        .entries
        .iter()
        .filter(|entry| entry.item.definition.production_key == REWARD)
        .collect::<Vec<_>>();
    assert_eq!(rewards.len(), 1);
    assert_eq!(rewards[0].item.quantity, 1);
    // Stay below the captured display at33230,31657 and west of the east statue.
    step(owner, member, South).await?;
    for _ in 0..5 {
        step(owner, member, East).await?;
    }
    step(owner, member, South).await?;
    step(owner, member, East).await?;
    position(owner, member, 33234, 31659).await?;
    assert_eq!(
        use_target(owner, member, &target(map, 33235, 31659)?).await?,
        UseDisposition::Committed,
        "source exit-side door"
    );
    for _ in 0..2 {
        step(owner, member, East).await?;
    }
    for _ in 0..4 {
        step(owner, member, North).await?;
    }
    position(owner, member, 33213, 31671).await?;
    outer_gate(owner, member, map).await?;
    let door = target(map, 33216, 31671)?;
    assert_eq!(
        use_target(owner, member, &door).await?,
        UseDisposition::NotPossible,
        "the real claimed reward seals re-entry"
    );
    position(owner, member, 33215, 31671).await?;
    let lever = crate::map::view::placement_key(
        crate::map::overlay::TilePos {
            x: 33226,
            y: 31671,
            floor: -13,
        },
        1,
    )
    .ok_or("lever")?;
    assert_eq!(
        map.facts.object_revision(lever),
        1,
        "daily source lever remains spent in this reset epoch"
    );
    durable_counts(pool).await?;
    eprintln!(
        "ANNIHILATOR_COMPLETE physical_reward=3388 shared_claims=1 achievements=1 source_exit=33213,31671,13 reentry=refused"
    );
    Ok(())
}
