//! Immutable shared effects projected from the retiring epoch's committed tallies.
use super::{ContributionKind, DurabilityError, DurabilityRoot, WorldId};
use oteryn_gameplay_world_change::contribution::{SaveContributions, SharedChanges};
use sqlx::{Postgres, Transaction};

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
        let previous: SharedChanges =
            serde_json::from_value(state).map_err(|_| DurabilityError::InvalidStoredState)?;
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
    let next = previous.at_save(&contributions);
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
