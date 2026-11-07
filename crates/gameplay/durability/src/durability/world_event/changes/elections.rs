//! Rathleton election state is a projection of custody-backed World contributions.
use super::{DurabilityError, WorldId};
use oteryn_gameplay_quests::quest::rathleton;
use sqlx::{Postgres, Transaction};
use std::collections::BTreeMap;

pub(crate) async fn vote(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
    operation: [u8; 16],
    ballot: rathleton::Ballot,
) -> Result<(), DurabilityError> {
    let now: i64 = sqlx::query_scalar(
        "SELECT floor(extract(epoch FROM activated_at)*1000)::bigint FROM game_world_reset_epochs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric",
    )
    .bind(world.as_bytes().as_slice())
    .bind(epoch.to_string())
    .fetch_one(&mut **tx)
    .await?;
    sqlx::query("INSERT INTO game_world_change_ballots(world_id,reset_epoch,operation_id,election_period,ballot,amount) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4,$5,1)")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(operation.as_slice())
        .bind(rathleton::election_period(now)).bind(ballot.identity).execute(&mut **tx).await?;
    Ok(())
}
/// At save, close only the immediately completed period. Missing periods have no
/// votes and retain the latest winners; no scan over historical receipt rows.
pub(crate) async fn at_save(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<BTreeMap<String, String>, DurabilityError> {
    let now:i64=sqlx::query_scalar("SELECT floor(extract(epoch FROM activated_at)*1000)::bigint FROM game_world_reset_epochs WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_one(&mut **tx).await?;
    let retiring_ms: Option<i64> = sqlx::query_scalar(
        "SELECT floor(extract(epoch FROM e.activated_at)*1000)::bigint FROM game_world_change_states s JOIN game_world_reset_epochs e USING(world_id,reset_epoch) WHERE s.world_id=encode($1,'hex')::uuid AND s.reset_epoch<$2::text::numeric ORDER BY s.reset_epoch DESC LIMIT 1",
    ).bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_optional(&mut **tx).await?;
    // A long outage must still count the retiring epoch's ballots. Missing
    // calendar periods contain no active epoch, so they cannot discard those votes.
    let closed = retiring_ms.map(rathleton::election_period)
        .unwrap_or_else(|| rathleton::election_period(now) - 1)
        .min(rathleton::election_period(now) - 1);
    if let Some(value)=sqlx::query_scalar::<_,serde_json::Value>("SELECT winners FROM game_world_change_elections WHERE world_id=encode($1,'hex')::uuid AND election_period=$2")
        .bind(world.as_bytes().as_slice()).bind(closed).fetch_optional(&mut **tx).await? {
        return serde_json::from_value(value).map_err(|_|DurabilityError::InvalidStoredState);
    }
    let previous:Option<serde_json::Value>=sqlx::query_scalar("SELECT winners FROM game_world_change_elections WHERE world_id=encode($1,'hex')::uuid AND election_period<$2 ORDER BY election_period DESC LIMIT 1")
        .bind(world.as_bytes().as_slice()).bind(closed).fetch_optional(&mut **tx).await?;
    let mut winners: BTreeMap<String, String> = previous
        .map(serde_json::from_value)
        .transpose()
        .map_err(|_| DurabilityError::InvalidStoredState)?
        .unwrap_or_default();
    // Exactly12 authored options bound every lookup, independent of player count.
    let mut counts = [0u64; 12];
    for (index, ballot) in rathleton::BALLOTS.iter().enumerate() {
        let count:Option<String>=sqlx::query_scalar("SELECT tally::text FROM game_world_change_ballot_tallies WHERE world_id=encode($1,'hex')::uuid AND election_period=$2 AND ballot=$3")
            .bind(world.as_bytes().as_slice()).bind(closed).bind(ballot.identity).fetch_optional(&mut **tx).await?;
        counts[index] = count
            .map(|value| value.parse())
            .transpose()
            .map_err(|_| DurabilityError::InvalidStoredState)?
            .unwrap_or(0);
    }
    for start in [0usize, 3, 6, 9] {
        let category = rathleton::BALLOTS[start].category;
        let options = [
            (rathleton::BALLOTS[start].identity, counts[start]),
            (rathleton::BALLOTS[start + 1].identity, counts[start + 1]),
            (rathleton::BALLOTS[start + 2].identity, counts[start + 2]),
        ];
        let winner = rathleton::winner(&options, winners.get(category).map(String::as_str))
            .map(str::to_owned);
        if let Some(winner) = winner {
            winners.insert(category.into(), winner);
        }
    }
    sqlx::query("INSERT INTO game_world_change_elections(world_id,election_period,reset_epoch,winners) VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric,$4)")
        .bind(world.as_bytes().as_slice()).bind(closed).bind(epoch.to_string())
        .bind(serde_json::to_value(&winners).map_err(|_|DurabilityError::InvalidStoredState)?)
        .execute(&mut **tx).await?;
    Ok(winners)
}
