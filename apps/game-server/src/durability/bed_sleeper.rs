//! BED0 §5.2 private schema/read preparation, not a standalone sleep/wake writer.
//! No grants, production migration or runtime registration. Insert must eventually
//! share the actual logout-marker transaction; delete must share settlement (or
//! the next logout insert). These hooks and current property/grant authority are
//! not provided by this module. A row lock is not authority to wake its character.
//! The private owner-change fallback only follows the native ownership column.
//! CP must free disposition/specific-cause rows BEFORE that ownership UPDATE;
//! Times use native build committed_at Unix milliseconds.
//! This hook has no invented flag that can distinguish those operations.

use crate::content::LogicalCell;
use crate::domain::CharacterId;
use crate::foundation::WorldId;
use crate::house_bed::sleeper::{
    FreedCause, FreedSleeper, OccupiedVariant, StoredBedKey, StoredBedOccupancy, StoredSleeper,
};
use crate::house_scope::HouseId;
use sqlx::postgres::{PgConnection, PgRow};
use sqlx::{Postgres, Row, Transaction};
use std::num::NonZeroU64;

/// Delegated private design, qualified only in isolated migration-owner fixtures.
/// No independently executable insert/free/delete API is supplied. The guards
/// preserve row lifecycle and revision bookkeeping, not marker/grant provenance.
pub const PRIVATE_SLEEPER_SCHEMA_SQL: &str = r"
CREATE TABLE game_house_bed_revisions (
    world_id UUID NOT NULL,
    house_key TEXT NOT NULL,
    revision NUMERIC(20,0) NOT NULL CHECK (revision BETWEEN 1 AND 18446744073709551615),
    PRIMARY KEY(world_id,house_key),
    FOREIGN KEY(world_id,house_key) REFERENCES game_house_ownership(world_id,house_key)
);
CREATE TABLE game_house_bed_sleepers (
    world_id UUID NOT NULL,
    house_key TEXT NOT NULL,
    bed_x INTEGER NOT NULL, bed_y INTEGER NOT NULL, bed_z INTEGER NOT NULL,
    character_id UUID PRIMARY KEY REFERENCES game_character_roots(character_id),
    lease_generation NUMERIC(20,0) NOT NULL CHECK (lease_generation BETWEEN 1 AND 18446744073709551615),
    occupied_variant TEXT NOT NULL CHECK (occupied_variant IN ('male','female')),
    slept_at BIGINT NOT NULL CHECK (slept_at >= 0),
    freed_at BIGINT,
    freed_cause TEXT,
    FOREIGN KEY(world_id,house_key) REFERENCES game_house_ownership(world_id,house_key),
    CHECK ((freed_at IS NULL AND freed_cause IS NULL) OR
           (freed_at IS NOT NULL AND freed_cause IS NOT NULL AND freed_at >= slept_at
            AND freed_cause IN ('owner_wake','owner_change','disposition','bed_removed')))
);
CREATE UNIQUE INDEX game_house_bed_live_key
ON game_house_bed_sleepers(world_id,house_key,bed_x,bed_y,bed_z) WHERE freed_at IS NULL;
CREATE FUNCTION game_house_bed_lifecycle_guard() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.freed_at IS NOT NULL OR NEW.freed_cause IS NOT NULL OR NOT EXISTS
           (SELECT 1 FROM game_character_roots r WHERE r.character_id=NEW.character_id AND r.world_id=NEW.world_id) THEN
            RAISE EXCEPTION 'invalid initial sleeper binding' USING ERRCODE='23514';
        END IF;
    ELSIF OLD.freed_at IS NOT NULL OR NEW.freed_at IS NULL OR
          (to_jsonb(NEW)-'freed_at'-'freed_cause') IS DISTINCT FROM
          (to_jsonb(OLD)-'freed_at'-'freed_cause') THEN
        RAISE EXCEPTION 'sleeper permits only occupied to freed transition' USING ERRCODE='23514';
    END IF;
    RETURN NEW;
END $$;
CREATE TRIGGER game_house_bed_lifecycle BEFORE INSERT OR UPDATE ON game_house_bed_sleepers
FOR EACH ROW EXECUTE FUNCTION game_house_bed_lifecycle_guard();
CREATE FUNCTION game_house_bed_advance_revision() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE w UUID; h TEXT;
BEGIN
    IF TG_OP='DELETE' THEN w:=OLD.world_id; h:=OLD.house_key;
    ELSE w:=NEW.world_id; h:=NEW.house_key; END IF;
    INSERT INTO game_house_bed_revisions(world_id,house_key,revision) VALUES(w,h,1)
    ON CONFLICT(world_id,house_key) DO UPDATE SET revision=game_house_bed_revisions.revision+1;
    RETURN NULL;
END $$;
CREATE TRIGGER game_house_bed_revision AFTER INSERT OR UPDATE OR DELETE ON game_house_bed_sleepers
FOR EACH ROW EXECUTE FUNCTION game_house_bed_advance_revision();
-- Private native-table hook only. Specific disposition/owner_wake frees MUST
-- precede an ownership UPDATE when their cause applies; the first free is final.
CREATE FUNCTION game_house_bed_owner_changed() RETURNS trigger LANGUAGE plpgsql AS $$
DECLARE freed_clock BIGINT;
BEGIN
    PERFORM 1 FROM game_house_bed_revisions
    WHERE world_id=NEW.world_id AND house_key=NEW.house_key FOR UPDATE;
    IF NOT FOUND AND EXISTS (SELECT 1 FROM game_house_bed_sleepers
        WHERE world_id=NEW.world_id AND house_key=NEW.house_key) THEN
        RAISE EXCEPTION 'missing sleeper occupancy revision' USING ERRCODE='23514';
    END IF;
    freed_clock:=floor(extract(epoch FROM clock_timestamp())*1000)::bigint;
    UPDATE game_house_bed_sleepers SET freed_at=freed_clock,freed_cause='owner_change'
    WHERE world_id=NEW.world_id AND house_key=NEW.house_key AND freed_at IS NULL;
    RETURN NULL;
END $$;
CREATE TRIGGER game_house_bed_owner_change AFTER UPDATE OF owner_character_id ON game_house_ownership
FOR EACH ROW WHEN (OLD.owner_character_id IS DISTINCT FROM NEW.owner_character_id)
EXECUTE FUNCTION game_house_bed_owner_changed();
DO $$ DECLARE n TEXT; BEGIN
    FOREACH n IN ARRAY ARRAY['game_house_bed_lifecycle_guard','game_house_bed_advance_revision','game_house_bed_owner_changed'] LOOP
        EXECUTE format('ALTER FUNCTION %I() SET search_path = %I, pg_temp',n,current_schema());
        EXECUTE format('REVOKE ALL ON FUNCTION %I() FROM PUBLIC',n);
    END LOOP;
END $$;
REVOKE ALL ON game_house_bed_sleepers,game_house_bed_revisions FROM PUBLIC;
";

#[derive(Debug)]
pub enum SleeperReadError {
    InvalidStoredState,
    Storage(sqlx::Error),
}

impl From<sqlx::Error> for SleeperReadError {
    fn from(error: sqlx::Error) -> Self {
        Self::Storage(error)
    }
}

const READ_SLEEPER: &str = "SELECT uuid_send(s.world_id) AS world_identity,s.house_key, \
    s.bed_x,s.bed_y,s.bed_z,uuid_send(s.character_id) AS character_identity, \
    s.lease_generation::text AS lease,s.occupied_variant,s.slept_at,s.freed_at,s.freed_cause, \
    r.revision::text AS revision FROM game_house_bed_sleepers s \
    LEFT JOIN game_house_bed_revisions r USING(world_id,house_key) \
    WHERE s.character_id=encode($1,'hex')::uuid";

/// One statement snapshot, including its house's occupancy revision. Missing row
/// means no sleeper, not a missing logout or an authorization/award decision.
pub async fn read_sleeper(
    connection: &mut PgConnection,
    character: CharacterId,
) -> Result<Option<StoredSleeper>, SleeperReadError> {
    sqlx::query(READ_SLEEPER)
        .bind(character.as_bytes().as_slice())
        .fetch_optional(connection)
        .await?
        .map(|row| decode(&row, character))
        .transpose()
}

const READ_BED_OCCUPANCY: &str = "WITH wanted AS (SELECT encode($1,'hex')::uuid AS world_id, \
    $2::text AS house_key, $3::integer AS bed_x, $4::integer AS bed_y, $5::integer AS bed_z) \
    SELECT uuid_send(s.world_id) AS world_identity,s.house_key,s.bed_x,s.bed_y,s.bed_z, \
    uuid_send(s.character_id) AS character_identity,s.lease_generation::text AS lease, \
    s.occupied_variant,s.slept_at,s.freed_at,s.freed_cause,r.revision::text AS revision, \
    EXISTS(SELECT 1 FROM game_house_bed_sleepers live \
        WHERE live.world_id=w.world_id AND live.house_key=w.house_key \
        AND live.freed_at IS NULL) AS has_open_sleeper \
    FROM wanted w LEFT JOIN game_house_bed_revisions r \
        ON r.world_id=w.world_id AND r.house_key=w.house_key \
    LEFT JOIN game_house_bed_sleepers s \
        ON s.world_id=w.world_id AND s.house_key=w.house_key \
        AND s.bed_x=w.bed_x AND s.bed_y=w.bed_y AND s.bed_z=w.bed_z \
        AND s.freed_at IS NULL LIMIT 2";

/// Reads the exact key and its House-wide revision in one statement snapshot.
/// At most two rows are returned so a broken uniqueness constraint refuses
/// instead of silently choosing an occupant. The open-row check uses the
/// existing partial bed index and catches a missing House revision even when
/// another bed in that House is occupied.
///
/// This starts no transaction and takes no gameplay or mutation authority.
/// A None occupant is raw storage data, not permission to reserve or insert;
/// callers must still resolve the active bed and hold their actual native
/// runtime/property/access fences. No marker or settlement is inferred.
pub async fn read_bed_occupancy(
    connection: &mut PgConnection,
    bed: &StoredBedKey,
) -> Result<StoredBedOccupancy, SleeperReadError> {
    let rows = sqlx::query(READ_BED_OCCUPANCY)
        .bind(bed.house.world_id().as_bytes().as_slice())
        .bind(bed.house.house_key())
        .bind(bed.head.x)
        .bind(bed.head.y)
        .bind(bed.head.z)
        .fetch_all(connection)
        .await?;
    let [row] = rows.as_slice() else {
        return Err(SleeperReadError::InvalidStoredState);
    };
    let invalid = || SleeperReadError::InvalidStoredState;
    let occupancy_revision = row
        .try_get::<Option<String>, _>("revision")?
        .map(|value| {
            value
                .parse::<u64>()
                .ok()
                .and_then(NonZeroU64::new)
                .ok_or_else(invalid)
        })
        .transpose()?;
    if occupancy_revision.is_none() && row.try_get::<bool, _>("has_open_sleeper")? {
        return Err(invalid());
    }
    let sleeper = row
        .try_get::<Option<Vec<u8>>, _>("character_identity")?
        .map(|bytes| {
            let character = CharacterId::from_bytes(bytes.try_into().map_err(|_| invalid())?)
                .map_err(|_| invalid())?;
            let sleeper = decode(row, character)?;
            if &sleeper.bed != bed
                || sleeper.freed.is_some()
                || Some(sleeper.occupancy_revision) != occupancy_revision
            {
                return Err(invalid());
            }
            Ok(sleeper)
        })
        .transpose()?;
    Ok(StoredBedOccupancy {
        bed: bed.clone(),
        occupancy_revision,
        sleeper,
    })
}

/// Pins an existing row until the caller's transaction ends. This does not
/// settle, delete, wake, qualify current authority or resolve the offline marker.
/// Future mutation owners must acquire their actual property/revision locks in
/// canonical order before invoking this read; no independent transaction starts.
pub async fn read_sleeper_for_update(
    tx: &mut Transaction<'_, Postgres>,
    character: CharacterId,
) -> Result<Option<StoredSleeper>, SleeperReadError> {
    // Audited fixed SQL only: no identifier or grammar comes from the caller.
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "{READ_SLEEPER} FOR UPDATE OF s"
    )))
    .bind(character.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?
    .map(|row| decode(&row, character))
    .transpose()
}

fn decode(row: &PgRow, expected: CharacterId) -> Result<StoredSleeper, SleeperReadError> {
    let invalid = || SleeperReadError::InvalidStoredState;
    let world =
        WorldId::decode(&row.try_get::<Vec<u8>, _>("world_identity")?).map_err(|_| invalid())?;
    let house = HouseId::new(world, &row.try_get::<String, _>("house_key")?).ok_or_else(invalid)?;
    let character_bytes: [u8; 16] = row
        .try_get::<Vec<u8>, _>("character_identity")?
        .try_into()
        .map_err(|_| invalid())?;
    let character = CharacterId::from_bytes(character_bytes).map_err(|_| invalid())?;
    if character != expected {
        return Err(invalid());
    }
    let variant = match row.try_get::<String, _>("occupied_variant")?.as_str() {
        "male" => OccupiedVariant::Male,
        "female" => OccupiedVariant::Female,
        _ => return Err(invalid()),
    };
    let slept_at: i64 = row.try_get("slept_at")?;
    if slept_at < 0 {
        return Err(invalid());
    }
    let freed = match (
        row.try_get::<Option<i64>, _>("freed_at")?,
        row.try_get::<Option<String>, _>("freed_cause")?,
    ) {
        (None, None) => None,
        (Some(at), Some(cause)) if at >= slept_at => Some(FreedSleeper {
            at,
            cause: FreedCause::from_stored(&cause).ok_or_else(invalid)?,
        }),
        _ => return Err(invalid()),
    };
    let positive = |name| -> Result<NonZeroU64, SleeperReadError> {
        row.try_get::<Option<String>, _>(name)?
            .and_then(|value| value.parse::<u64>().ok())
            .and_then(NonZeroU64::new)
            .ok_or_else(invalid)
    };
    Ok(StoredSleeper {
        bed: StoredBedKey {
            house,
            head: LogicalCell {
                x: row.try_get("bed_x")?,
                y: row.try_get("bed_y")?,
                z: row.try_get("bed_z")?,
            },
        },
        character,
        lease_generation: positive("lease")?,
        variant,
        slept_at,
        freed,
        occupancy_revision: positive("revision")?,
    })
}
