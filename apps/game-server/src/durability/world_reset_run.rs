//! MAP-OVERLAY-1c World reset orchestration (ADR-0021 §4.7 steps 3 and 4).
//!
//! The node runs a reset only at boot, before any Channel runtime, listener or Spell lane of
//! this incarnation exists: a running node drains and restarts (step 1), and the control plane
//! assigns every World Channel a fresh generation (step 2) whose closed readiness guard and
//! released sessions the census proves. A crash anywhere repeats the same run: each frozen
//! candidate resumes with its persisted identity, every committed retirement replays, and an
//! activated occurrence leaves no RETIRING job behind.
use super::character_authority::ReconciledCharacterAuthority;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::spell_owner_commit::SpellLane;
use super::world_reset::{
    WorldResetActivationDisposition, WorldResetCensus, WorldResetError, census_retiring_world_reset,
};
use super::world_reset_port::{
    DrainedChannelBinding, WorldDrainBinding, WorldDrainWitness, WorldResetHouseSource,
    world_drain_cohort_digest,
};
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{ChannelId, WorldId};
use sqlx::Row;
use std::time::{Duration, Instant};

/// `MAP01-RESET-RETIRE-MS`: qualification budget of one whole reset run, from the first
/// retirement pass to the committed activation. It is measured and alarmed, never a cutoff:
/// the World stays closed until the run completes (ADR-0021 §4.7 Duration).
pub(crate) const MAP01_RESET_RETIRE_MS: u64 = 600_000;

/// Unknown activation commits are retried with the same frozen identity (historical replay
/// precedes every current-authority check). Restart repeats the whole run after this bound.
const ACTIVATION_ATTEMPTS: u32 = 3;

/// The measured outcome of one completed reset run.
#[derive(Debug)]
pub(crate) struct WorldResetRun {
    pub(crate) disposition: WorldResetActivationDisposition,
    pub(crate) reset_epoch: u64,
    pub(crate) retired: u64,
    pub(crate) elapsed: Duration,
}

impl WorldResetRun {
    /// Whether the run stayed within `MAP01-RESET-RETIRE-MS`.
    pub(crate) fn within_budget(&self) -> bool {
        self.elapsed <= Duration::from_millis(MAP01_RESET_RETIRE_MS)
    }
}

impl DurabilityRoot {
    /// The World's RETIRING reset epoch, if any. Data only: the census and every retirement
    /// recheck the job under their own locks.
    pub(crate) async fn read_retiring_world_reset(
        &self,
        world: WorldId,
    ) -> Result<Option<u64>, WorldResetError> {
        let epoch: Option<String> = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    let epoch = sqlx::query_scalar(
                        "SELECT reset_epoch::text FROM game_world_reset_jobs \
                         WHERE world_id=encode($1,'hex')::uuid AND phase='RETIRING'",
                    )
                    .bind(world.as_bytes().as_slice())
                    .fetch_optional(&mut *tx)
                    .await?;
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(epoch)
                })
            })
            .await?;
        epoch
            .map(|value| {
                value
                    .parse()
                    .map_err(|_| WorldResetError::InvalidStoredState)
            })
            .transpose()
    }

    /// Every World Channel this node holds, sorted by Channel bytes, with its D1233 cohort
    /// digest. Data for a drain witness, never drain proof itself: only a holder that knows no
    /// in-memory runtime of these Channels exists may wrap it. A Channel held by any other
    /// node, or not assigned, refuses with `NotCurrent`.
    pub(crate) async fn read_world_drain_binding(
        &self,
        node: &NodeIncarnationProof,
        world: WorldId,
    ) -> Result<WorldDrainBinding, WorldResetError> {
        let fact = node.fact();
        type Row = (Vec<u8>, String, i16, Option<Vec<u8>>, Option<String>);
        let rows: Vec<Row> = self
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    let rows = sqlx::query(
                        "SELECT uuid_send(channel_id),ownership_generation::text,state,\
                         uuid_send(holder_node_id),holder_registration_revision::text \
                         FROM game_runtime_scope_assignments \
                         WHERE world_id=encode($1,'hex')::uuid AND scope_kind=1 \
                         ORDER BY uuid_send(channel_id)",
                    )
                    .bind(world.as_bytes().as_slice())
                    .fetch_all(&mut *tx)
                    .await?;
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    rows.into_iter()
                        .map(|row| {
                            Ok((
                                row.try_get(0)?,
                                row.try_get(1)?,
                                row.try_get(2)?,
                                row.try_get(3)?,
                                row.try_get(4)?,
                            ))
                        })
                        .collect::<Result<_, DurabilityError>>()
                })
            })
            .await?;
        let mut channels = Vec::with_capacity(rows.len());
        for (channel, generation, state, holder, revision) in rows {
            let revision = revision
                .map(|value| value.parse::<u64>())
                .transpose()
                .map_err(|_| WorldResetError::InvalidStoredState)?;
            if state != 1
                || holder.as_deref() != Some(fact.node_id().as_bytes().as_slice())
                || revision != Some(fact.registration_revision())
            {
                return Err(WorldResetError::NotCurrent);
            }
            let channel =
                ChannelId::decode(&channel).map_err(|_| WorldResetError::InvalidStoredState)?;
            channels.push(DrainedChannelBinding {
                scope: crate::foundation::RuntimeScopeRefV1::channel(world, channel),
                generation: generation
                    .parse()
                    .map_err(|_| WorldResetError::InvalidStoredState)?,
            });
        }
        let cohort_digest = world_drain_cohort_digest(world, fact, &channels)
            .map_err(|_| WorldResetError::DrainBindingRefused)?;
        Ok(WorldDrainBinding {
            world,
            node: fact,
            channels,
            cohort_digest,
        })
    }

    /// The census of the World's RETIRING reset, read in its own transaction. The value is
    /// frozen occurrence data; no lock outlives this call.
    pub(crate) async fn census_world_reset(
        &self,
        node: &NodeIncarnationProof,
        world: WorldId,
    ) -> Result<Option<WorldResetCensus>, WorldResetError> {
        let mut context = node;
        self.try_issue_semantic_pass()?
            .run_with_context(&mut context, move |holder, deadline, node| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    let census = census_retiring_world_reset(&mut tx, node, world).await;
                    tx.rollback().await?;
                    Ok(census)
                })
            })
            .await?
    }

    /// Steps 3 and 4 of the World's RETIRING reset: retire every Ground item of every drained
    /// Channel, one transaction per item, then activate epoch N+1 atomically. `Ok(None)` means
    /// no reset is RETIRING. Every error leaves the durable run resumable: run it again.
    ///
    /// `drain` must be the witness of a holder with no Channel runtime of this World (boot
    /// quiescence); `houses` the qualified native House source of both catalogues.
    pub(crate) async fn run_world_reset(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: WorldId,
        houses: &(dyn WorldResetHouseSource + Sync),
        drain: &(dyn WorldDrainWitness + Sync),
    ) -> Result<Option<WorldResetRun>, WorldResetError> {
        let started = Instant::now();
        let Some(census) = self.census_world_reset(node, world).await? else {
            return Ok(None);
        };
        let binding = drain.drained();
        if binding.world != world || binding.node != node.fact() {
            return Err(WorldResetError::DrainBindingRefused);
        }
        let mut retired = 0_u64;
        for channel in &binding.channels {
            let crate::foundation::RuntimeScopeRefV1::Channel {
                world_id,
                channel_id,
            } = channel.scope
            else {
                return Err(WorldResetError::DrainBindingRefused);
            };
            if world_id != world {
                return Err(WorldResetError::DrainBindingRefused);
            }
            // Boot owns no lane yet: a fresh lane holds no parked attempt.
            let lane = SpellLane::new(world, channel_id);
            let permit = lane
                .acquire()
                .await
                .map_err(|_| WorldResetError::NotCurrent)?;
            while let Some(candidate) = self
                .freeze_next_world_reset_item(&permit, authority, node, world, channel_id)
                .await?
            {
                self.commit_world_reset_item(&permit, authority, node, &candidate)
                    .await?;
                retired = retired
                    .checked_add(1)
                    .ok_or(WorldResetError::CapacityExceeded)?;
            }
        }
        // A verified empty census of the same occurrence permits activation (step 4).
        let after = self
            .census_world_reset(node, world)
            .await?
            .ok_or(WorldResetError::NotCurrent)?;
        let identity = after.activation_identity();
        if after.reset_epoch != census.reset_epoch
            || after.target_digest != census.target_digest
            || after.ground_root_count != 0
            || after.ground_corpse_entry_count != 0
        {
            return Err(WorldResetError::NotCurrent);
        }
        let mut attempt = 0;
        let disposition = loop {
            attempt += 1;
            match self
                .commit_world_reset_activation(node, &identity, houses, drain)
                .await
            {
                Err(WorldResetError::Unavailable(_)) if attempt < ACTIVATION_ATTEMPTS => {}
                result => break result?,
            }
        };
        Ok(Some(WorldResetRun {
            disposition,
            reset_epoch: census.reset_epoch,
            retired,
            elapsed: started.elapsed(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::MAP01_RESET_RETIRE_MS;

    /// The budget is the registered `MAP01-RESET-RETIRE-MS` row.
    #[test]
    fn retire_budget_is_the_registered_limit() -> Result<(), Box<dyn std::error::Error>> {
        let registry: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../docs/contracts/RESOURCE_LIMITS_REGISTRY.json"
        ))?;
        let row = registry["entries"]
            .as_array()
            .ok_or("entries")?
            .iter()
            .find(|entry| entry["id"] == "MAP01-RESET-RETIRE-MS")
            .ok_or("MAP01-RESET-RETIRE-MS row")?;
        assert_eq!(row["hard_maximum"].as_u64(), Some(MAP01_RESET_RETIRE_MS));
        assert_eq!(row["unit"], "milliseconds");
        Ok(())
    }
}
