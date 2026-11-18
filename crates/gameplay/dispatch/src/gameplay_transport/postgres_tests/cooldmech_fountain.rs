//! The Well of Life action settles the once-only receipt and restores both pools.
use super::fixture::Harness;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult, quest_world_callbacks as world};
use crate::map::view::MapFacts;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

pub(super) const NAME: &str = "cooldmech-fountain";
const WELL: Position = Position {
    x: 32805,
    y: 32318,
    floor: -11,
};

#[test]
fn cooldmech_fountain_use_restores_vitals_and_grants_achievement_once_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run(NAME, || world::scenario(NAME, WELL, false))
}

pub(super) fn geometry(trigger: Position) -> TestResult<Callback> {
    let mut fixture = super::authored_world_rewards::placeholder(trigger)?;
    fixture.selector.appearance = Some(1922);
    fixture.selector.unique = None;
    fixture.selector.action = Some(8815);
    Ok(fixture)
}

pub(super) async fn exercise(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    harness: &Harness,
) -> TestResult {
    let key =
        oteryn_world_bundle::bundle::placement_key(WELL.floor, WELL.x, WELL.y, 1).ok_or("well")?;
    let mut placement = owner
        .bundle_map()
        .ok_or("map")?
        .overlay
        .base()
        .digest()
        .to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let actor = admitted.runtime_actor.ok_or("actor")?;
    let session = admitted.game_session_id;
    for command in [1, 2] {
        {
            let runtime = owner.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            states
                .get_mut(&runtime, actor, session)
                .ok_or("spell actor")?
                .restore_saved_vitals(1, 0);
        }
        let target = WorldObjectTarget {
            placement: placement.clone(),
            expected_revision: owner.bundle_map().ok_or("map")?.facts.object_revision(key),
        };
        let replies = super::quest_callbacks::wire_use_targets_retained(
            owner,
            admitted,
            &[(command, &target)],
        )
        .await?;
        let expected = if command == 1 {
            UseDisposition::Committed
        } else {
            UseDisposition::Rejected
        };
        assert_eq!(replies.first().ok_or("USE reply")?.1, expected);
        let runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        let vitals = states
            .get(&runtime, actor, session)
            .ok_or("spell actor")?
            .vitals();
        if command == 1 {
            assert_eq!(
                (vitals.health, vitals.mana),
                (vitals.max_health, vitals.max_mana)
            );
        } else {
            assert_eq!((vitals.health, vitals.mana), (1, 0));
        }
    }
    let grants: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/fountain_of_life'",
    ).fetch_one(&harness.pool).await?;
    assert_eq!(grants, 1);
    Ok(())
}
