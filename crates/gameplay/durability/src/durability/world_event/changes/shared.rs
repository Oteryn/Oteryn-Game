//! Immutable shared effects projected from the retiring epoch's committed tallies.
use super::{ContributionKind, DurabilityError, DurabilityRoot, WorldId};
use oteryn_gameplay_world_change::contribution::{SaveContributions, SharedChanges};
use sqlx::{Postgres, Transaction};
use sha2::Digest;

pub(super) async fn record(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<SharedChanges, DurabilityError> {
    if let Some(state) = read(tx, world, epoch).await? {
        return Ok(state);
    }
    // The leading World/epoch primary key supports this bounded backwards lookup.
    // Reset publication fences the retiring epoch's contributors before this call.
    let previous: Option<(String, serde_json::Value)> = sqlx::query_as(
        "SELECT reset_epoch::text,state FROM game_world_change_states \
         WHERE world_id=encode($1,'hex')::uuid AND reset_epoch<$2::text::numeric \
         ORDER BY reset_epoch DESC LIMIT 1",
    )
    .bind(world.as_bytes().as_slice())
    .bind(epoch.to_string())
    .fetch_optional(&mut **tx)
    .await?;
    let (previous, contributions) = if let Some((previous_epoch, state)) = previous {
        let previous_epoch = previous_epoch
            .parse()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let mut previous: SharedChanges =
            serde_json::from_value(state).map_err(|_| DurabilityError::InvalidStoredState)?;
        if let Some(live) = super::actions::for_save(tx, world, previous_epoch).await? {
            previous = live.overlay(previous);
        }
        let contributions = SaveContributions {
            coal: super::tally(tx, world, previous_epoch, ContributionKind::Coal).await?,
            medicine: super::tally(tx, world, previous_epoch, ContributionKind::Medicine).await?,
            corpses: super::tally(tx, world, previous_epoch, ContributionKind::Corpses).await?,
            winning_ballot: None,
        };
        (previous, contributions)
    } else {
        (SharedChanges::default(), SaveContributions::default())
    };
    let mut next = previous.at_save(&contributions);
    next.election_winners = super::elections::at_save(tx, world, epoch).await?;
    // Shared immutable World/reset selection; neither Channel nor process entropy
    // can choose another dog location. Source probability: retained W15 92/1000.
    let draw = sha2::Sha256::digest([b"oteryn.noodles.daily.v1\0".as_slice(),
        world.as_bytes().as_slice(), &epoch.to_be_bytes()].concat());
    // A disappearance is a saved-day selection. Unreturned escorts are released
    // at reset; a successful return earns the following day with Dog Lady.
    next.noodles.missing = false;
    if !next.noodles.dog_lady && u64::from_be_bytes([draw[0],draw[1],draw[2],draw[3],draw[4],draw[5],draw[6],draw[7]]) % 1000 < 92 {
        next.noodles.disappear();
        next.noodles.roaming_index = (u64::from_be_bytes([draw[8],draw[9],draw[10],draw[11],draw[12],draw[13],draw[14],draw[15]]) % 13) as u8;
    }
    let state = serde_json::to_value(&next).map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query(
        "INSERT INTO game_world_change_states(world_id,reset_epoch,state) \
         VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3) ON CONFLICT DO NOTHING",
    )
    .bind(world.as_bytes().as_slice())
    .bind(epoch.to_string())
    .bind(state)
    .execute(&mut **tx)
    .await?;
    // A concurrent observer must agree with the one immutable save projection.
    let stored = read(tx, world, epoch)
        .await?
        .ok_or(DurabilityError::InvalidStoredState)?;
    if stored != next {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(stored)
}

async fn read(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<Option<SharedChanges>, DurabilityError> {
    let state: Option<serde_json::Value> = sqlx::query_scalar(
        "SELECT state FROM game_world_change_states \
         WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric",
    )
    .bind(world.as_bytes().as_slice())
    .bind(epoch.to_string())
    .fetch_optional(&mut **tx)
    .await?;
    state
        .map(|state| serde_json::from_value(state).map_err(|_| DurabilityError::InvalidStoredState))
        .transpose()
}

/// Current-epoch observation; callers still require their own mutation fences.
pub async fn load(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<Option<SharedChanges>, DurabilityError> {
    if super::load(tx, world, epoch).await?.is_none() {
        return Ok(None);
    }
    read(tx, world, epoch).await
}

impl DurabilityRoot {
    pub async fn shared_world_changes(
        &self,
        world: WorldId,
        epoch: u64,
    ) -> Result<Option<SharedChanges>, DurabilityError> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::begin_semantic_transaction(holder, deadline).await?;
                    let shared = load(&mut tx, world, epoch).await?;
                    super::commit_semantic_transaction(tx, deadline).await?;
                    Ok(shared)
                })
            })
            .await
    }
}
