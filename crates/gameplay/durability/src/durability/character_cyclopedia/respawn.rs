//! One durable World selection per server-save epoch.
use super::*;
use crate::foundation::WorldId;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use std::collections::{BTreeMap, btree_map::Entry};

const DONATION_GOAL: u64 = 10_000_000;
const RANDOM_CHANCE_PERCENT: usize = 33;

struct Draw {
    root: GameplayDecisionRoot,
    occurrence: DecisionOccurrenceId,
}

impl Draw {
    fn new(world: WorldId, epoch: u64) -> Self {
        let mut root = [0; 32];
        root[..16].copy_from_slice(world.as_bytes());
        let mut occurrence = [0; 16];
        occurrence[..8].copy_from_slice(&epoch.to_be_bytes());
        Self {
            root: GameplayDecisionRoot::from_bytes(root),
            occurrence: DecisionOccurrenceId::from_bytes(occurrence),
        }
    }

    fn below(&self, purpose: &str, bound: usize) -> Result<usize, DurabilityError> {
        let bound = bound as u64;
        // Reject the incomplete bucket so 33/100 and each candidate have exact odds.
        let threshold = bound.wrapping_neg() % bound;
        for index in 0..=u64::MAX {
            let value = deterministic_decision_u64(&self.root, self.occurrence, purpose, index)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if value >= threshold {
                return Ok((value % bound) as usize);
            }
        }
        Err(DurabilityError::InvalidStoredState)
    }

    fn pick(&self, purpose: &str, areas: &[i64]) -> Result<Option<i64>, DurabilityError> {
        if areas.is_empty() {
            return Ok(None);
        }
        Ok(Some(areas[self.below(purpose, areas.len())?]))
    }
}

#[derive(Debug, Default, PartialEq, Eq)]
struct Selection {
    donation: Option<i64>,
    random: Option<i64>,
}

fn select(
    draw: &Draw,
    pools: &BTreeMap<i64, u64>,
    previous: &BTreeMap<i64, bool>,
) -> Result<Selection, DurabilityError> {
    let eligible: Vec<_> = pools
        .keys()
        .copied()
        .filter(|area| previous.get(area) != Some(&true))
        .collect();
    let highest = eligible
        .iter()
        .map(|area| pools[area])
        .max()
        .unwrap_or_default();
    let mut donation = None;
    if highest >= DONATION_GOAL {
        let tied: Vec<_> = eligible
            .iter()
            .copied()
            .filter(|area| pools[area] == highest)
            .collect();
        donation = draw.pick("cyclopedia-donation-tie", &tied)?;
    }
    let mut random = None;
    if draw.below("cyclopedia-rapid-respawn-chance", 100)? < RANDOM_CHANCE_PERCENT {
        let remaining: Vec<_> = eligible
            .into_iter()
            .filter(|area| Some(*area) != donation)
            .collect();
        random = draw.pick("cyclopedia-rapid-respawn-area", &remaining)?;
    }
    Ok(Selection { donation, random })
}

async fn read_epoch(
    tx: &mut Type2Transaction<'_>,
    world: WorldId,
    epoch: u64,
    areas: &[i64],
    bundle: Option<&[u8]>,
) -> Result<BTreeMap<i64, bool>, DurabilityError> {
    let rows: Vec<(i64, bool)> = sqlx::query_as(
        "SELECT e.area_id,e.improved FROM game_world_cyclopedia_epochs e
         JOIN game_world_cyclopedia p USING(world_id,area_id)
         WHERE e.world_id=encode($1,'hex')::uuid AND e.area_id=ANY($2) AND e.epoch=$3::text::numeric
           AND ($4::bytea IS NULL OR (p.bundle_digest=$4 AND p.catalogue_digest=$5))",
    )
    .bind(world.as_bytes().as_slice())
    .bind(areas)
    .bind(epoch.to_string())
    .bind(bundle)
    .bind(BasicArea::content_digest().as_slice())
    .fetch_all(&mut ***tx)
    .await?;
    Ok(rows.into_iter().collect())
}

pub(super) async fn prepare(
    tx: &mut Type2Transaction<'_>,
    world: WorldId,
    bundle: [u8; 32],
    epoch: u64,
    main_areas: &BTreeSet<u32>,
    legacy_area: u32,
) -> Result<BTreeSet<u32>, DurabilityError> {
    if !main_areas.contains(&legacy_area) || main_areas.iter().any(|id| !(1..=1024).contains(id)) {
        return Err(DurabilityError::InvalidStoredState);
    }
    // The schema bounds this snapshot to 1024 canonical areas and one legacy source row.
    let area_ids: Vec<_> = main_areas.iter().copied().map(i64::from).collect();
    let mut current = read_epoch(tx, world, epoch, &area_ids, Some(&bundle)).await?;
    if current.len() != area_ids.len() {
        // Serialize only first preparation, including when no pool row exists yet.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:cyclopedia-epoch:'||encode($1,'hex'),0))")
            .bind(world.as_bytes().as_slice()).execute(&mut ***tx).await?;
        current = read_epoch(tx, world, epoch, &area_ids, Some(&bundle)).await?;
        if current.len() != area_ids.len() {
            settle(
                tx,
                world,
                bundle,
                epoch,
                &area_ids,
                i64::from(legacy_area),
                &mut current,
            )
            .await?;
        }
    }
    Ok(current
        .into_iter()
        .filter_map(|(area, improved)| improved.then_some(area as u32))
        .collect())
}

async fn settle(
    tx: &mut Type2Transaction<'_>,
    world: WorldId,
    bundle: [u8; 32],
    epoch: u64,
    area_ids: &[i64],
    legacy_area: i64,
    current: &mut BTreeMap<i64, bool>,
) -> Result<(), DurabilityError> {
    sqlx::query(
        "INSERT INTO game_world_cyclopedia(world_id,bundle_digest,catalogue_digest,area_id,pool,legacy_imported)
         SELECT encode($1,'hex')::uuid,$2,$3,area,
          CASE WHEN claim.import THEN COALESCE((SELECT pool FROM game_world_cyclopedia
            WHERE world_id=encode($1,'hex')::uuid AND area_id=0),0) ELSE 0 END,claim.import
         FROM unnest($4::bigint[]) a(area)
         CROSS JOIN LATERAL (SELECT area=$5 AND NOT EXISTS(SELECT 1 FROM game_world_cyclopedia
            WHERE world_id=encode($1,'hex')::uuid AND legacy_imported) AS import) claim
         WHERE NOT EXISTS(SELECT 1 FROM game_world_cyclopedia
            WHERE world_id=encode($1,'hex')::uuid AND area_id=area)
         ORDER BY area ON CONFLICT DO NOTHING",
    ).bind(world.as_bytes().as_slice()).bind(bundle.as_slice()).bind(BasicArea::content_digest().as_slice())
        .bind(area_ids).bind(legacy_area).execute(&mut ***tx).await?;
    let rows = sqlx::query(
        "SELECT area_id,bundle_digest,catalogue_digest,pool,legacy_imported FROM game_world_cyclopedia
         WHERE world_id=encode($1,'hex')::uuid AND area_id=ANY($2) ORDER BY area_id FOR UPDATE",
    ).bind(world.as_bytes().as_slice()).bind(area_ids).fetch_all(&mut ***tx).await?;
    let mut pools = BTreeMap::new();
    let mut imported = false;
    for row in rows {
        let area: i64 = row.try_get("area_id")?;
        imported |= area == legacy_area && row.try_get::<bool, _>("legacy_imported")?;
        pools.insert(area, storage::read_pool(&row, bundle)?);
    }
    if pools.len() != area_ids.len() {
        return Err(DurabilityError::InvalidStoredState);
    }
    // Upgrade in place: finish any old, partially recorded day without redrawing
    // or charging another pool. The original entry area's legacy day is also retained.
    let legacy = if imported {
        read_epoch(tx, world, epoch, &[0], None).await?.remove(&0)
    } else {
        None
    };
    let selection = if current.is_empty() && legacy.is_none() {
        let mut previous = BTreeMap::new();
        if let Some(prior_epoch) = epoch.checked_sub(1) {
            previous = read_epoch(tx, world, prior_epoch, area_ids, None).await?;
            if imported
                && let Entry::Vacant(entry) = previous.entry(legacy_area)
                && let Some(improved) = read_epoch(tx, world, prior_epoch, &[0], None)
                    .await?
                    .remove(&0)
            {
                entry.insert(improved);
            }
        }
        select(&Draw::new(world, epoch), &pools, &previous)?
    } else {
        Selection::default()
    };
    let mut new_areas = Vec::new();
    let mut pools_before = Vec::new();
    let mut boosts = Vec::new();
    let mut donation_winners = Vec::new();
    for (area, pool) in pools {
        let Entry::Vacant(entry) = current.entry(area) else {
            continue;
        };
        let winner = selection.donation == Some(area);
        let improved = winner
            || selection.random == Some(area)
            || (area == legacy_area && legacy == Some(true));
        new_areas.push(area);
        pools_before.push(pool as i64);
        boosts.push(improved);
        donation_winners.push(winner);
        entry.insert(improved);
    }
    sqlx::query(
        "INSERT INTO game_world_cyclopedia_epochs(world_id,area_id,epoch,improved,donation_winner,pool_before)
         SELECT encode($1,'hex')::uuid,area,$2::text::numeric,improved,winner,pool
         FROM unnest($3::bigint[],$4::boolean[],$5::boolean[],$6::bigint[]) AS e(area,improved,winner,pool)",
    ).bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(&new_areas)
        .bind(&boosts).bind(&donation_winners).bind(&pools_before).execute(&mut ***tx).await?;
    if let Some(winner) = selection.donation {
        sqlx::query(
            "UPDATE game_world_cyclopedia SET pool=0 WHERE world_id=encode($1,'hex')::uuid AND area_id=$2",
        )
        .bind(world.as_bytes().as_slice())
        .bind(winner)
        .execute(&mut ***tx)
        .await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::*;

    fn draw(epoch: u64) -> Draw {
        let world =
            WorldId::decode(&[42, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 42])
                .expect("fixture World ID");
        Draw::new(world, epoch)
    }

    #[test]
    fn rapid_respawn_chance_boundary_and_empty_candidates() {
        let pools = [(1, 0)].into();
        let no_previous = BTreeMap::new();
        let miss = draw(3);
        let hit = draw(7);
        assert_eq!(
            miss.below("cyclopedia-rapid-respawn-chance", 100).unwrap(),
            33
        );
        assert_eq!(
            select(&miss, &pools, &no_previous).unwrap(),
            Selection::default()
        );
        assert_eq!(select(&hit, &pools, &no_previous).unwrap().random, Some(1));
        assert_eq!(
            select(&hit, &pools, &[(1, true)].into()).unwrap(),
            Selection::default()
        );
    }

    #[test]
    fn rapid_respawn_replays_ties_and_picks_a_distinct_area() {
        let pools = [
            (3, DONATION_GOAL),
            (1, DONATION_GOAL),
            (2, DONATION_GOAL - 1),
        ]
        .into();
        let hit = draw(7);
        for _ in 0..2 {
            assert_eq!(
                select(&hit, &pools, &BTreeMap::new()).unwrap(),
                Selection {
                    donation: Some(1),
                    random: Some(2)
                }
            );
        }
    }
}
