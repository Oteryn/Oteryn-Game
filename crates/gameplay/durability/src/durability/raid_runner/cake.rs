//! Cake's independent stage-one progress. Crystal a_piece_of_cake_config.lua:
//! 97-104,374-383 and movements_extraction_spot.lua:26-49 define the two goals.
use super::*;
use crate::foundation::WorldId;
use sqlx::{Postgres, Transaction};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CakeProgress {
    pub year: i32,
    pub substances: u16,
    pub golems: u16,
}

impl CakeProgress {
    pub const fn phase_one_active(self) -> bool {
        self.substances < 200 || self.golems < 200
    }
}

/// Bind to the immutable selected reset date, sharing the caller's physical
/// transaction. Daily resets in the same annual window preserve both counters.
pub async fn phase_one_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    expected_epoch: u64,
) -> Result<Option<CakeProgress>, DurabilityError> {
    let calendar: Option<(i32, bool)> = sqlx::query_as("SELECT extract(year FROM r.activated_at AT TIME ZONE 'Europe/Berlin')::integer,extract(month FROM r.activated_at AT TIME ZONE 'Europe/Berlin')=2 AND extract(day FROM r.activated_at AT TIME ZONE 'Europe/Berlin')=21 FROM game_world_reset_active a JOIN game_world_reset_epochs r USING(world_id,reset_epoch) JOIN game_world_event_epochs e USING(world_id,reset_epoch) WHERE a.world_id=encode($1,'hex')::uuid AND a.reset_epoch=$2::text::numeric AND e.seasonal_calendar_revision=1 AND 'piece_of_cake'=ANY(e.seasonal_event_keys) FOR SHARE OF a")
        .bind(world.as_bytes().as_slice()).bind(expected_epoch.to_string()).fetch_optional(&mut **tx).await?;
    let Some((year, opening_day)) = calendar else {
        return Ok(None);
    };
    // Crystal a_piece_of_cake_config.lua:556-574 and
    // globalevents_auto_schedule.lua:10-18 initialize only on February21.
    if opening_day {
        sqlx::query("INSERT INTO game_cake_world_progress(world_id,event_year) VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).bind(year).execute(&mut **tx).await?;
    }
    let counters: Option<(i16, i16)> = sqlx::query_as("SELECT substances,golems FROM game_cake_world_progress WHERE world_id=encode($1,'hex')::uuid AND event_year=$2 FOR UPDATE")
        .bind(world.as_bytes().as_slice()).bind(year).fetch_optional(&mut **tx).await?;
    let Some((substances, golems)) = counters else {
        return Ok(None);
    };
    Ok(Some(CakeProgress {
        year,
        substances: u16::try_from(substances).map_err(|_| DurabilityError::InvalidStoredState)?,
        golems: u16::try_from(golems).map_err(|_| DurabilityError::InvalidStoredState)?,
    }))
}

/// Crystal startup/tables/tile.lua:1111-1153: these ten machine tiles contain
/// Item8886 and receive the extraction action IDs43910..43914.
pub const fn is_machine_position(position: [u16; 3]) -> bool {
    matches!(
        position,
        [32346, 31822 | 31823, 7]
            | [32350, 32275 | 32276, 7]
            | [33251, 31802 | 31803, 7]
            | [33191, 32397 | 32398, 7]
            | [32377, 32801 | 32802, 7]
    )
}

impl DurabilityRoot {
    pub async fn cake_world_progress(
        &self,
        world: WorldId,
        expected_epoch: u64,
    ) -> Result<Option<CakeProgress>, DurabilityError> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let progress = phase_one_in_transaction(&mut tx, world, expected_epoch).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(progress)
                })
            })
            .await
    }

    /// Invoked only by the current monster movement owner after an actual Cake
    /// Golem steps onto an admitted machine Item8886. The stable successful raid
    /// placement is consumed in the same transaction as the world contribution.
    pub async fn extract_cake_raid_creature(
        &self,
        node: &NodeIncarnationProof,
        identity: RaidStepIdentity,
        creature: RaidCreatureIdentity,
        expected_epoch: u64,
        machine_position: [u16; 3],
    ) -> Result<RaidWrite, DurabilityError> {
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = extract_in_transaction(
                        &mut tx,
                        &node,
                        identity,
                        creature,
                        expected_epoch,
                        machine_position,
                        true,
                    )
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await
    }
}

/// Immutable raid owner identity plus independently current node and reset fences.
/// Completed hourly waves remain eligible; their successful live placements persist.
pub async fn current_source_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    identity: RaidStepIdentity,
) -> Result<bool, DurabilityError> {
    if !effects::world_current(tx, identity.firing_id).await?
        || !effects::current(
            tx,
            node,
            identity.firing_id,
            identity.channel,
            identity.generation,
        )
        .await?
    {
        return Ok(false);
    }
    let valid:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_world_raid_channel_runs r JOIN game_world_raid_firings f USING(firing_id) WHERE r.firing_id=encode($1,'hex')::uuid AND r.channel_id=encode($2,'hex')::uuid AND r.owner_generation=$3::text::numeric AND f.raid_key='oteryn:raid.piece_of_cake')")
        .bind(identity.firing_id.as_slice()).bind(identity.channel.as_bytes().as_slice()).bind(identity.generation.get().to_string()).fetch_one(&mut **tx).await?;
    Ok(valid)
}

/// Same physical Map/Item transaction as the independently current machine read.
/// Stored receipts settle unknown COMMIT before a changed machine can refuse a new lure.
pub async fn extract_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    identity: RaidStepIdentity,
    creature: RaidCreatureIdentity,
    expected_epoch: u64,
    machine_position: [u16; 3],
    machine_present: bool,
) -> Result<RaidWrite, DurabilityError> {
    if !is_machine_position(machine_position) {
        return Ok(RaidWrite::CompareFailed);
    }
    if !current_source_in_transaction(tx, node, identity).await? {
        return Ok(RaidWrite::CompareFailed);
    }
    let world: Option<Vec<u8>> = sqlx::query_scalar("SELECT uuid_send(world_id) FROM game_world_raid_firings WHERE firing_id=encode($1,'hex')::uuid")
                .bind(identity.firing_id.as_slice()).fetch_optional(&mut **tx).await?;
    let world = world
        .and_then(|value| <[u8; 16]>::try_from(value).ok())
        .ok_or(DurabilityError::InvalidStoredState)?;
    let Some(progress) = phase_one_in_transaction(
        tx,
        WorldId::decode(&world).map_err(|_| DurabilityError::InvalidStoredState)?,
        expected_epoch,
    )
    .await?
    else {
        return Ok(RaidWrite::CompareFailed);
    };
    let existing: Option<(i32,i32,i32)> = sqlx::query_as("SELECT x,y,z FROM game_cake_lure_receipts WHERE firing_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND wave=$3 AND entry=$4 AND creature_index=$5")
                .bind(identity.firing_id.as_slice()).bind(identity.channel.as_bytes().as_slice())
                .bind(effects::small(creature.wave)?).bind(effects::small(creature.entry)?).bind(effects::small(creature.index)?)
                .fetch_optional(&mut **tx).await?;
    if let Some(position) = existing {
        return Ok(
            if position
                == (
                    i32::from(machine_position[0]),
                    i32::from(machine_position[1]),
                    i32::from(machine_position[2]),
                )
            {
                RaidWrite::Replayed
            } else {
                RaidWrite::CompareFailed
            },
        );
    }
    if !machine_present {
        return Ok(RaidWrite::CompareFailed);
    }
    if !progress.phase_one_active() || progress.golems >= 200 {
        return Ok(RaidWrite::CompareFailed);
    }
    let changed = sqlx::query("UPDATE game_world_raid_creatures c SET state=2 WHERE c.firing_id=encode($1,'hex')::uuid AND c.channel_id=encode($2,'hex')::uuid AND c.wave=$3 AND c.entry=$4 AND c.creature_index=$5 AND c.state=1 AND c.creature_key='oteryn:creature.cake_golem' AND EXISTS(SELECT 1 FROM game_world_raid_channel_runs r WHERE r.firing_id=c.firing_id AND r.channel_id=c.channel_id AND r.owner_generation=$6::text::numeric) AND EXISTS(SELECT 1 FROM game_world_raid_creature_placements p JOIN game_world_raid_effects e ON e.firing_id=p.firing_id AND e.channel_id=p.channel_id AND e.wave=p.wave AND e.action_kind=2 WHERE (p.firing_id,p.channel_id,p.wave,p.entry,p.creature_index)=(c.firing_id,c.channel_id,c.wave,c.entry,c.creature_index) AND p.placed AND e.state=2 AND e.owner_generation=$6::text::numeric)")
                .bind(identity.firing_id.as_slice()).bind(identity.channel.as_bytes().as_slice())
                .bind(effects::small(creature.wave)?).bind(effects::small(creature.entry)?).bind(effects::small(creature.index)?)
                .bind(identity.generation.get().to_string()).execute(&mut **tx).await?.rows_affected();
    if changed != 1 {
        return Ok(RaidWrite::CompareFailed);
    }
    sqlx::query("INSERT INTO game_cake_lure_receipts(firing_id,channel_id,wave,entry,creature_index,world_id,event_year,reset_epoch,x,y,z) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,encode($6,'hex')::uuid,$7,$8::text::numeric,$9,$10,$11)")
                .bind(identity.firing_id.as_slice()).bind(identity.channel.as_bytes().as_slice())
                .bind(effects::small(creature.wave)?).bind(effects::small(creature.entry)?).bind(effects::small(creature.index)?)
                .bind(world.as_slice()).bind(progress.year).bind(expected_epoch.to_string())
                .bind(i32::from(machine_position[0])).bind(i32::from(machine_position[1])).bind(i32::from(machine_position[2])).execute(&mut **tx).await?;
    sqlx::query("UPDATE game_cake_world_progress SET golems=golems+1 WHERE world_id=encode($1,'hex')::uuid AND event_year=$2 AND golems=$3")
                .bind(world.as_slice()).bind(progress.year).bind(i16::try_from(progress.golems).map_err(|_| DurabilityError::InvalidStoredState)?)
                .execute(&mut **tx).await?;
    Ok(RaidWrite::Applied)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn w05d_cake_both_independent_goals_gate_stage_two() {
        for (substances, golems, active) in [
            (0, 0, true),
            (200, 0, true),
            (0, 200, true),
            (200, 199, true),
            (199, 200, true),
            (200, 200, false),
        ] {
            assert_eq!(
                CakeProgress {
                    year: 2026,
                    substances,
                    golems
                }
                .phase_one_active(),
                active
            );
        }
    }

    #[test]
    fn w05d_cake_extraction_accepts_only_the_ten_source_machine_tiles() {
        for position in [
            [32346, 31822, 7],
            [32346, 31823, 7],
            [32350, 32275, 7],
            [32350, 32276, 7],
            [33251, 31802, 7],
            [33251, 31803, 7],
            [33191, 32397, 7],
            [33191, 32398, 7],
            [32377, 32801, 7],
            [32377, 32802, 7],
        ] {
            assert!(is_machine_position(position));
            assert!(!is_machine_position([
                position[0] - 1,
                position[1],
                position[2]
            ]));
            assert!(!is_machine_position([
                position[0],
                position[1],
                position[2] - 1
            ]));
        }
        // The baker's tile is not the extraction machine.
        assert!(!is_machine_position([32348, 32272, 7]));
    }
}
