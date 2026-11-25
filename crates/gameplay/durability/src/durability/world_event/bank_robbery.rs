//! World-owned Bank Robbery selection and Channel projection binding.
use super::*;
use crate::durability::character_progression::assert_runtime_scope_fence;
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use serde::Deserialize;
use sha2::{Digest, Sha256};

const HOUR_MS: i64 = 3_600_000;
const MAX_HOURS_PER_POLL: i64 = 24;

/// Reserved carrier wave number for the robbery's one normal-map cohort.
pub const RAID_WAVE: u16 = u16::MAX;
/// Native Item key used by the qualified robber loot tables.
pub const BAG: &str = "oteryn:item.tibia.i13429";

#[derive(Deserialize)]
struct Index {
    bank_robbery: Config,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    hourly_roll: Roll,
    city_weights_parity: String,
    spawn_cells_parity: String,
    source: String,
    cities: [City; 4],
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Roll {
    numerator: u32,
    denominator: u32,
    parity: String,
}

/// Source-qualified city and its exact normal-map placements.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct City {
    pub key: String,
    weight: u32,
    pub robber: String,
    pub cell: [u16; 3],
    pub escort: Option<Escort>,
    escort_parity: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Escort {
    pub creature: String,
    pub cells: [[u16; 3]; 4],
}

fn config() -> Result<&'static Config, DurabilityError> {
    static CONFIG: std::sync::OnceLock<Option<Config>> = std::sync::OnceLock::new();
    CONFIG.get_or_init(|| {
        let config = serde_json::from_str::<Index>(include_str!(
            "../../../../../../content/encounters/mini-world-changes/index.json"
        )).ok()?.bank_robbery;
        let pending = "PARITY_PENDING";
        if config.hourly_roll.numerator == 0
            || config.hourly_roll.numerator > config.hourly_roll.denominator
            || config.hourly_roll.parity != pending
            || config.city_weights_parity != pending
            || config.spawn_cells_parity != pending
            || config.source.is_empty()
        {
            return None;
        }
        let mut keys = std::collections::BTreeSet::new();
        let mut robbers = std::collections::BTreeSet::new();
        for city in &config.cities {
            if !keys.insert(&city.key)
                || !robbers.insert(&city.robber)
                || city.weight == 0
                || !city.robber.starts_with("oteryn:creature.")
                || city.cell[2] > 15
            {
                return None;
            }
            let valid_escort = match (&city.escort, city.escort_parity.as_deref()) {
                (Some(escort), None) if escort.creature.starts_with("oteryn:creature.") => {
                    let mut cells = std::collections::BTreeSet::from([city.cell]);
                    escort.cells.iter().all(|cell| cell[2] == city.cell[2] && cells.insert(*cell))
                }
                (None, Some("PARITY_PENDING")) => true,
                _ => false,
            };
            if !valid_escort {
                return None;
            }
        }
        Some(config)
    }).as_ref().ok_or(DurabilityError::InvalidStoredState)
}

/// The authored placements, in selection order.
pub fn cities() -> Result<&'static [City; 4], DurabilityError> {
    Ok(&config()?.cities)
}

/// A committed live robbery on the polling Channel.
#[derive(Debug, Clone, Copy)]
pub struct Projection {
    pub firing_id: [u8; 16],
    pub city: &'static City,
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
}

impl Projection {
    /// The durable Channel claim is valid only for this runtime generation.
    pub fn matches(self, scope: RuntimeScopeRefV1, generation: ScopeOwnershipGeneration) -> bool {
        scope == self.scope && generation == self.generation
    }
}


fn draw(world: WorldId, epoch: u64, hour: i64, domain: u8, upper: u64) -> u64 {
    let limit = u64::MAX - u64::MAX % upper;
    let mut counter = 0_u64;
    loop {
        let mut hash = Sha256::new();
        hash.update(b"oteryn:bank-robbery:hourly:v1\0");
        hash.update(world.as_bytes());
        hash.update(epoch.to_be_bytes());
        hash.update(hour.to_be_bytes());
        hash.update([domain]);
        hash.update(counter.to_be_bytes());
        let digest = hash.finalize();
        let mut first = [0; 8];
        first.copy_from_slice(&digest[..8]);
        let sample = u64::from_be_bytes(first);
        if sample < limit {
            return sample % upper;
        }
        counter = counter.wrapping_add(1);
    }
}

fn select(config: &Config, world: WorldId, epoch: u64, hour: i64) -> Option<&City> {
    #[cfg(any(test, feature = "test-support"))]
    if let Ok(forced) = TEST_CITY.try_with(|key| *key) {
        return config.cities.iter().find(|city| city.key == forced);
    }
    if draw(world, epoch, hour, 0, u64::from(config.hourly_roll.denominator))
        >= u64::from(config.hourly_roll.numerator)
    {
        return None;
    }
    let mut choice = draw(
        world,
        epoch,
        hour,
        1,
        config.cities.iter().map(|city| u64::from(city.weight)).sum(),
    );
    config.cities.iter().find(|city| {
        let weight = u64::from(city.weight);
        if choice < weight {
            true
        } else {
            choice -= weight;
            false
        }
    })
}

#[cfg(any(test, feature = "test-support"))]
tokio::task_local! {
    static TEST_CITY: &'static str;
}

/// Force a city in the scoped PostgreSQL test future.
#[cfg(any(test, feature = "test-support"))]
pub async fn with_test_roll<T>(city: &'static str, test: impl Future<Output = T>) -> T {
    TEST_CITY.scope(city, test).await
}

impl DurabilityRoot {
    /// Establish the World boot basis on its first fenced native poll. Later
    /// Channels and restarts use the same epoch row and draw cursor.
    pub async fn poll_bank_robbery(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        epoch: u64,
        loaded_cities: [bool; 4],
    ) -> Result<Option<Projection>, DurabilityError> {
        let config = config()?;
        let channel = scope.channel_id().ok_or(DurabilityError::InvalidStoredState)?;
        let world = scope.world_id();
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !assert_runtime_scope_fence(&mut tx, scope, generation, &node).await? {
                return Ok(None);
            }
            let world_id = world.as_bytes().as_slice();
            let channel_id = channel.as_bytes().as_slice();
            let epoch_id = epoch.to_string();
            let activated: Option<i64> = sqlx::query_scalar(
                "SELECT activated_at_unix_ms FROM game_world_reset_read_locked(encode($1,'hex')::uuid) WHERE reset_epoch=$2::text::numeric"
            ).bind(world_id).bind(&epoch_id).fetch_optional(&mut *tx).await?;
            let Some(activated) = activated else { return Ok(None); };
            let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint").fetch_one(&mut *tx).await?;
            // Keep the prior boot basis across committed saves. A World with
            // no native robbery poll in any previous epoch starts its clock now.
            sqlx::query("INSERT INTO game_bank_robbery_occurrences(world_id,reset_epoch,boot_ms,last_checked_hour)
                SELECT encode($1,'hex')::uuid,$2::text::numeric,coalesce(prior.boot_ms,$3),
                       greatest(0,($4-coalesce(prior.boot_ms,$3))/$5)
                FROM (SELECT 1) seed
                LEFT JOIN LATERAL (
                    SELECT boot_ms FROM game_bank_robbery_occurrences
                    WHERE world_id=encode($1,'hex')::uuid AND reset_epoch<$2::text::numeric
                    ORDER BY reset_epoch DESC LIMIT 1
                ) prior ON true ON CONFLICT DO NOTHING")
                .bind(world_id).bind(&epoch_id).bind(now).bind(activated).bind(HOUR_MS)
                .execute(&mut *tx).await?;
            let row = sqlx::query("SELECT boot_ms,last_checked_hour,state,city,robber,
                       uuid_send(firing_id) AS firing,channel_id=encode($3,'hex')::uuid AS owns
                    FROM game_bank_robbery_occurrences WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric FOR UPDATE")
                    .bind(world_id).bind(&epoch_id).bind(channel_id).fetch_one(&mut *tx).await?;
            let state: i16 = row.try_get("state")?;
            let owns: Option<bool> = row.try_get("owns")?;
            let mut selected = match state {
                0 => None,
                1 => {
                    let city: &str = row.try_get("city")?;
                    let robber: &str = row.try_get("robber")?;
                    let firing: &[u8] = row.try_get("firing")?;
                    let city = config.cities.iter().position(|c| c.key == city && c.robber == robber)
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    Some((city, firing.try_into().map_err(|_| DurabilityError::InvalidStoredState)?))
                }
                2..=4 => {
                    commit_semantic_transaction(tx, deadline).await?;
                    return Ok(None);
                }
                _ => return Err(DurabilityError::InvalidStoredState),
            };
            if state == 0 {
                let boot: i64 = row.try_get("boot_ms")?;
                let checked: i64 = row.try_get("last_checked_hour")?;
                let last = ((now-boot)/HOUR_MS).min(checked.saturating_add(MAX_HOURS_PER_POLL));
                if last > checked {
                    let mut checked_through = last;
                    for hour in checked + 1..=last {
                        if let Some(city) = select(config, world, epoch, hour) {
                            let index = config.cities.iter().position(|c| c.key == city.key)
                                .ok_or(DurabilityError::InvalidStoredState)?;
                            let firing = crate::durability::starter_grant::new_uuid(&mut tx).await?;
                            selected = Some((index, firing));
                            checked_through = hour;
                            break;
                        }
                    }
                    let city = selected.as_ref().map(|(index, _)| &config.cities[*index]);
                    let firing = selected.as_ref().map(|(_, firing)| firing.as_slice());
                    sqlx::query("UPDATE game_bank_robbery_occurrences
                        SET last_checked_hour=$3, state=$4, city=$5,robber=$6,firing_id=encode($7,'hex')::uuid
                        WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
                        .bind(world_id).bind(&epoch_id).bind(checked_through).bind(i16::from(selected.is_some()))
                        .bind(city.map(|city| city.key.as_str())).bind(city.map(|city| city.robber.as_str())).bind(firing)
                        .execute(&mut *tx).await?;
                }
            }
            let mut projection = None;
            if let Some((index, firing_id)) = selected
                && loaded_cities[index] && owns != Some(false)
            {
                if owns.is_none() {
                    sqlx::query("UPDATE game_bank_robbery_occurrences SET channel_id=encode($3,'hex')::uuid
                        WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric")
                        .bind(world_id).bind(&epoch_id).bind(channel_id).execute(&mut *tx).await?;
                }
                projection = Some(Projection { firing_id, city: &config.cities[index], scope, generation });
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok(projection)
        })).await
    }
}

/// Fence the committed kill against the selected Channel and current save.
/// The surrounding corpse transaction rolls the transition back on any mint failure.
pub(crate) async fn claim_death(
    tx: &mut Transaction<'_, Postgres>,
    scope: RuntimeScopeRefV1,
    firing: [u8; 16],
) -> Result<Option<String>, DurabilityError> {
    let Some(channel) = scope.channel_id() else { return Ok(None); };
    let world = scope.world_id();
    let world_id = world.as_bytes().as_slice();
    let epoch: Option<String> = sqlx::query_scalar(
        "SELECT reset_epoch::text FROM game_world_reset_read_locked(encode($1,'hex')::uuid)"
    ).bind(world_id).fetch_optional(&mut **tx).await?;
    let Some(epoch) = epoch else { return Ok(None); };
    sqlx::query_scalar("UPDATE game_bank_robbery_occurrences SET state=2
        WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric
          AND firing_id=encode($3,'hex')::uuid AND channel_id=encode($4,'hex')::uuid AND state=1
        RETURNING reset_epoch::text")
        .bind(world_id).bind(epoch).bind(firing.as_slice()).bind(channel.as_bytes().as_slice())
        .fetch_optional(&mut **tx).await.map_err(Into::into)
}

pub(crate) async fn bind_bags(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: &str,
    bags: &[(u32, [u8; 16])],
) -> Result<(), DurabilityError> {
    for &(ordinal, bag) in bags {
        let ordinal = i16::try_from(ordinal).map_err(|_| DurabilityError::InvalidStoredState)?;
        sqlx::query("INSERT INTO game_bank_robbery_bags(world_id,reset_epoch,ordinal,item_instance_id)
            VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,encode($4,'hex')::uuid)")
            .bind(world.as_bytes().as_slice()).bind(epoch).bind(ordinal).bind(bag.as_slice())
            .execute(&mut **tx).await?;
    }
    Ok(())
}
