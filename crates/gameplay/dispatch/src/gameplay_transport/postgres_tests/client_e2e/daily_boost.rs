use super::super::fixture::{Harness, debug};
use super::TestResult;
use super::support::{self, Case, count, exchange};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::daily_boost as wire;

#[test]
fn client_e2e_daily_boost() -> TestResult {
    support::scenario(Case::DailyBoost)
}

pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    // PlayLink::daily_boost_request -> daily_boost::request sends command 177 with
    // empty payload; daily_boost::apply_result retains these exact result bytes.
    let first = exchange(
        owner,
        admitted,
        1,
        wire::COMMAND_TYPE_DAILY_BOOST_QUERY,
        &[],
    )
    .await?;
    let choice = owner
        .runtime
        .lock()
        .await
        .daily_boost()
        .cloned()
        .ok_or("daily choice")?;
    assert_eq!(
        first,
        wire::encode_view(&wire::DailyBoostView {
            reset_epoch: 0,
            creature_key: choice.creature.clone(),
            boss_key: choice.boss.clone()
        })
        .map_err(debug)?
    );
    let stored: (String, String) =
        sqlx::query_as("SELECT creature_key,boss_key FROM game_world_daily_boosts")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(stored, (choice.creature, choice.boss));
    assert_eq!(
        exchange(
            owner,
            admitted,
            2,
            wire::COMMAND_TYPE_DAILY_BOOST_QUERY,
            &[]
        )
        .await?,
        first
    );
    assert_eq!(
        count(h, "SELECT count(*) FROM game_world_daily_boosts").await?,
        1
    );
    Ok(())
}
