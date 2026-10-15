//! Private transactional DATA core. No public entry, commit, native fence substitute or action.
//! CP must put the actual recovery/admission/occurrence locks before this named World-row lock.
use super::*;
use sqlx::{Postgres, Row, Transaction};

/// Private proposed DDL, installed ONLY by the isolated test fixture. Production migration,
/// schema admission, grants and deferred protection of immutable columns remain CP work.
#[cfg(test)]
pub(super) const PRIVATE_SCHEMA: &str = r"
CREATE TABLE game_world_raid_schedules (
 world_id UUID NOT NULL, raid_key TEXT NOT NULL, revision NUMERIC(20,0) NOT NULL CHECK(revision>=0),
 payload BYTEA NOT NULL CHECK(octet_length(payload)<=1048576), PRIMARY KEY(world_id,raid_key));
CREATE TABLE game_world_raid_firings (
 world_id UUID NOT NULL, raid_key TEXT NOT NULL, ordinal NUMERIC(20,0) NOT NULL CHECK(ordinal>=0),
 firing_id BYTEA NOT NULL UNIQUE CHECK(octet_length(firing_id)=16),
 payload BYTEA NOT NULL CHECK(octet_length(payload)<=1048576), PRIMARY KEY(world_id,raid_key,ordinal),
 FOREIGN KEY(world_id,raid_key) REFERENCES game_world_raid_schedules(world_id,raid_key));
CREATE TABLE game_world_raid_channel_runs (
 world_id UUID NOT NULL, raid_key TEXT NOT NULL, firing_ordinal NUMERIC(20,0) NOT NULL,
 channel_id BYTEA NOT NULL CHECK(octet_length(channel_id)=16),
 payload BYTEA NOT NULL CHECK(octet_length(payload)<=128),
 PRIMARY KEY(world_id,raid_key,firing_ordinal,channel_id),
 FOREIGN KEY(world_id,raid_key,firing_ordinal) REFERENCES game_world_raid_firings(world_id,raid_key,ordinal));
";

#[derive(Debug)]
pub(super) enum StoreError {
    Invalid(RaidError),
    Sql(sqlx::Error),
}
impl std::fmt::Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(e) => write!(f, "{e}"),
            Self::Sql(e) => write!(f, "{e}"),
        }
    }
}
impl std::error::Error for StoreError {}
impl From<sqlx::Error> for StoreError {
    fn from(e: sqlx::Error) -> Self {
        Self::Sql(e)
    }
}
impl From<RaidError> for StoreError {
    fn from(e: RaidError) -> Self {
        Self::Invalid(e)
    }
}
type StoreResult<T> = std::result::Result<T, StoreError>;
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Cas {
    Applied,
    Replayed,
    CompareFailed,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FiringPayload {
    ordinal: u64,
    id: [u8; 16],
    fired_at_ms: i64,
    reset_epoch: u64,
    bundle_digest: [u8; 32],
    close_at_ms: i64,
    state: FiringState,
    targets: Vec<[u8; 16]>,
}
impl From<&Firing> for FiringPayload {
    fn from(f: &Firing) -> Self {
        Self {
            ordinal: f.ordinal,
            id: f.id,
            fired_at_ms: f.fired_at_ms,
            reset_epoch: f.reset_epoch,
            bundle_digest: f.bundle_digest,
            close_at_ms: f.close_at_ms,
            state: f.state,
            targets: f.targets.keys().map(|id| *id.as_bytes()).collect(),
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
enum RunPayload {
    Claimed(u64),
    Done(u64),
    Lost(Option<u64>),
}
impl From<Run> for RunPayload {
    fn from(r: Run) -> Self {
        match r {
            Run::Claimed(g) => Self::Claimed(g.get()),
            Run::Done(g) => Self::Done(g.get()),
            Run::Lost(g) => Self::Lost(g.map(ScopeOwnershipGeneration::get)),
        }
    }
}
impl RunPayload {
    fn decode(self) -> StoreResult<Run> {
        let generation = |g| {
            ScopeOwnershipGeneration::new(g)
                .map_err(|_| StoreError::Invalid(RaidError::InvalidStoredState))
        };
        Ok(match self {
            Self::Claimed(g) => Run::Claimed(generation(g)?),
            Self::Done(g) => Run::Done(generation(g)?),
            Self::Lost(g) => Run::Lost(g.map(generation).transpose()?),
        })
    }
}
fn encode<T: Serialize>(value: &T) -> StoreResult<Vec<u8>> {
    serde_json::to_vec(value).map_err(|_| RaidError::InvalidStoredState.into())
}
fn decode<T: for<'de> Deserialize<'de>>(bytes: &[u8]) -> StoreResult<T> {
    if bytes.len() > 1_048_576 {
        return Err(RaidError::InvalidStoredState.into());
    }
    serde_json::from_slice(bytes).map_err(|_| RaidError::InvalidStoredState.into())
}
fn unsigned(value: String) -> StoreResult<u64> {
    value
        .parse()
        .map_err(|_| RaidError::InvalidStoredState.into())
}

/// Caller owns the transaction and its isolation/locks. For a consistent read outside apply,
/// use REPEATABLE READ or take the same named lock; this never asserts current authority.
pub(super) async fn load(
    tx: &mut Transaction<'_, Postgres>,
    key: &RaidKey,
) -> StoreResult<Option<Snapshot>> {
    if !key.valid() {
        return Err(RaidError::InvalidInput.into());
    }
    let Some(row)=sqlx::query("SELECT revision::text,payload FROM game_world_raid_schedules WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2")
        .bind(key.world.as_bytes().as_slice()).bind(&key.raid).fetch_optional(&mut **tx).await? else{return Ok(None)};
    let schedule: Schedule = decode(&row.try_get::<Vec<u8>, _>("payload")?)?;
    if schedule.revision != unsigned(row.try_get("revision")?)? {
        return Err(RaidError::InvalidStoredState.into());
    }
    let row=sqlx::query("SELECT ordinal::text,firing_id,payload FROM game_world_raid_firings WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2 ORDER BY game_world_raid_firings.ordinal DESC LIMIT 1")
        .bind(key.world.as_bytes().as_slice()).bind(&key.raid).fetch_optional(&mut **tx).await?;
    let latest = if let Some(row) = row {
        let payload: FiringPayload = decode(&row.try_get::<Vec<u8>, _>("payload")?)?;
        if payload.ordinal != unsigned(row.try_get("ordinal")?)?
            || row.try_get::<Vec<u8>, _>("firing_id")? != payload.id
        {
            return Err(RaidError::InvalidStoredState.into());
        }
        let mut targets = BTreeMap::new();
        for bytes in payload.targets {
            let channel = ChannelId::decode(&bytes).map_err(|_| RaidError::InvalidStoredState)?;
            if targets.insert(channel, None).is_some() {
                return Err(RaidError::InvalidStoredState.into());
            }
        }
        let limit = i64::try_from(targets.len())
            .map_err(|_| RaidError::InvalidStoredState)?
            .checked_add(1)
            .ok_or(RaidError::Overflow)?;
        let rows=sqlx::query("SELECT channel_id,payload FROM game_world_raid_channel_runs WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2 AND firing_ordinal=$3::text::numeric(20,0) LIMIT $4")
            .bind(key.world.as_bytes().as_slice()).bind(&key.raid).bind(payload.ordinal.to_string()).bind(limit).fetch_all(&mut **tx).await?;
        for row in rows {
            let channel = ChannelId::decode(&row.try_get::<Vec<u8>, _>("channel_id")?)
                .map_err(|_| RaidError::InvalidStoredState)?;
            let slot = targets
                .get_mut(&channel)
                .ok_or(RaidError::InvalidStoredState)?;
            *slot = Some(decode::<RunPayload>(&row.try_get::<Vec<u8>, _>("payload")?)?.decode()?);
        }
        Some(Firing {
            ordinal: payload.ordinal,
            id: payload.id,
            fired_at_ms: payload.fired_at_ms,
            reset_epoch: payload.reset_epoch,
            bundle_digest: payload.bundle_digest,
            close_at_ms: payload.close_at_ms,
            state: payload.state,
            targets,
        })
    } else {
        None
    };
    let snapshot = Snapshot {
        key: key.clone(),
        schedule,
        latest,
    };
    snapshot.validate()?;
    Ok(Some(snapshot))
}

/// Sealed transition CAS and exact replay, pending caller commit. PRIVATE data-only core;
/// deliberately not exported as a production writer until native fences/current producers exist.
pub(super) async fn apply(tx: &mut Transaction<'_, Postgres>, change: &Change) -> StoreResult<Cas> {
    change.after.validate()?;
    let key = &change.after.key;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:raid:'||encode($1::bytea,'hex')||':'||$2,0))")
        .bind(key.world.as_bytes().as_slice()).bind(&key.raid).execute(&mut **tx).await?;
    let current = load(tx, key).await?;
    if current.as_ref() == Some(&change.after) {
        return Ok(Cas::Replayed);
    }
    if current != change.before {
        return Ok(Cas::CompareFailed);
    }
    sqlx::query("INSERT INTO game_world_raid_schedules(world_id,raid_key,revision,payload) VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric(20,0),$4) ON CONFLICT(world_id,raid_key) DO UPDATE SET revision=EXCLUDED.revision,payload=EXCLUDED.payload")
        .bind(key.world.as_bytes().as_slice()).bind(&key.raid).bind(change.after.schedule.revision.to_string()).bind(encode(&change.after.schedule)?).execute(&mut **tx).await?;
    if let Some(firing) = &change.after.latest {
        sqlx::query("INSERT INTO game_world_raid_firings(world_id,raid_key,ordinal,firing_id,payload) VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric(20,0),$4,$5) ON CONFLICT(world_id,raid_key,ordinal) DO UPDATE SET payload=EXCLUDED.payload")
            .bind(key.world.as_bytes().as_slice()).bind(&key.raid).bind(firing.ordinal.to_string()).bind(firing.id.as_slice()).bind(encode(&FiringPayload::from(firing))?).execute(&mut **tx).await?;
        for (channel, run) in &firing.targets {
            if let Some(run) = run {
                sqlx::query("INSERT INTO game_world_raid_channel_runs(world_id,raid_key,firing_ordinal,channel_id,payload) VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric(20,0),$4,$5) ON CONFLICT(world_id,raid_key,firing_ordinal,channel_id) DO UPDATE SET payload=EXCLUDED.payload")
                    .bind(key.world.as_bytes().as_slice()).bind(&key.raid).bind(firing.ordinal.to_string()).bind(channel.as_bytes().as_slice()).bind(encode(&RunPayload::from(*run))?).execute(&mut **tx).await?;
            }
        }
    }
    Ok(Cas::Applied)
}
