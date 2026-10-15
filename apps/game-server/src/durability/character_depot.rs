//! DEPOT0 §§3,6,8: raw custody/read/capacity preparation, not a TRANSFER writer.
//! The proposed SQL is never installed here. CP must compose native placement/removal,
//! exclusivity, receipt/audit and current session/locker authority before value can move.

use crate::domain::premium::PremiumGate;
use crate::domain::{CharacterId, ItemInstanceId, WorldId};
use crate::durability::DurabilityError;
use sqlx::{PgConnection, Row};
use std::num::NonZeroU64;

pub(crate) const BOXES: u8 = 17;
pub(crate) const FREE_ENTRIES: u32 = 2_000;
pub(crate) const HARD_ENTRIES: u32 = 15_000;
pub(crate) const PAGE_ENTRIES: usize = 32;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DepotBox(u8);
impl DepotBox {
    pub(crate) fn new(value: u8) -> Option<Self> {
        (1..=BOXES).contains(&value).then_some(Self(value))
    }
    pub(crate) const fn get(self) -> u8 {
        self.0
    }
}

/// Character+World, deliberately without a Channel, town, locker or session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DepotOwner {
    pub(crate) character: CharacterId,
    pub(crate) world: WorldId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DepotEntry {
    pub(crate) item: ItemInstanceId,
    pub(crate) ordinal: NonZeroU64,
}

/// Raw count/Premium classification only. This grants neither ownership nor commit authority.
/// Withdrawals do not call this insertion guard, even after Premium expires.
pub(crate) fn insertion_fits(entries: u32, premium: PremiumGate) -> bool {
    entries
        < if premium.premium_current() {
            HARD_ENTRIES
        } else {
            FREE_ENTRIES
        }
}

pub(crate) fn next_ordinal(highest: u64) -> Option<NonZeroU64> {
    highest.checked_add(1).and_then(NonZeroU64::new)
}

/// Fixed 17 counters; aggregation never materializes the entries themselves.
/// Not an access boundary: the caller must own the current private view/read authorization.
pub(crate) async fn read_summary(
    connection: &mut PgConnection,
    owner: DepotOwner,
) -> Result<[u32; BOXES as usize], DurabilityError> {
    let rows = sqlx::query(
        "SELECT box, count(*) AS entries FROM game_item_character_depot_locations \
         WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid \
         GROUP BY box ORDER BY box LIMIT $3",
    )
    .bind(owner.character.as_bytes().as_slice())
    .bind(owner.world.as_bytes().as_slice())
    .bind(i64::from(BOXES) + 1)
    .fetch_all(connection)
    .await?;
    let mut counts = [0; BOXES as usize];
    let mut total = 0_u32;
    for row in rows {
        let raw: i16 = row.try_get("box")?;
        let box_id = u8::try_from(raw)
            .ok()
            .and_then(DepotBox::new)
            .ok_or(DurabilityError::InvalidStoredState)?;
        let count = u32::try_from(row.try_get::<i64, _>("entries")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        total = total
            .checked_add(count)
            .ok_or(DurabilityError::InvalidStoredState)?;
        if total > HARD_ENTRIES {
            return Err(DurabilityError::InvalidStoredState);
        }
        counts[usize::from(box_id.get() - 1)] = count;
    }
    Ok(counts)
}

/// One descending ordinal page, at most 32 rows. The internal cursor is not a wire page ID.
/// No authorization, open-view state or relocation is reconstructed from this persisted data.
pub(crate) async fn read_box_page(
    connection: &mut PgConnection,
    owner: DepotOwner,
    box_id: DepotBox,
    before: Option<NonZeroU64>,
) -> Result<Vec<DepotEntry>, DurabilityError> {
    let rows = sqlx::query(
        "SELECT uuid_send(item_instance_id) AS item, ordinal::text AS ordinal \
         FROM game_item_character_depot_locations \
         WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid \
           AND box=$3 AND ($4::text IS NULL OR ordinal < $4::text::numeric(20,0)) \
         ORDER BY game_item_character_depot_locations.ordinal DESC LIMIT 32",
    )
    .bind(owner.character.as_bytes().as_slice())
    .bind(owner.world.as_bytes().as_slice())
    .bind(i16::from(box_id.get()))
    .bind(before.map(|v| v.get().to_string()))
    .fetch_all(connection)
    .await?;
    let mut result = Vec::with_capacity(PAGE_ENTRIES);
    for row in rows {
        let bytes: Vec<u8> = row.try_get("item")?;
        let bytes = bytes
            .try_into()
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let item =
            ItemInstanceId::from_bytes(bytes).map_err(|_| DurabilityError::InvalidStoredState)?;
        let text: String = row.try_get("ordinal")?;
        let ordinal = text
            .parse::<u64>()
            .ok()
            .and_then(NonZeroU64::new)
            .ok_or(DurabilityError::InvalidStoredState)?;
        if result
            .last()
            .is_some_and(|previous: &DepotEntry| previous.ordinal <= ordinal)
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        result.push(DepotEntry { item, ordinal });
    }
    Ok(result)
}

/// Private proposed schema, exercised only in isolated qualification. This is deliberately
/// insufficient to authorize native item placement: CP must add every DEPOT0 §6 shared proof.
/// No runtime grants, migration number or install path is supplied.
pub(crate) const PROPOSED_SCHEMA: &str = r#"
CREATE TABLE game_item_character_depot_locations (
 item_instance_id UUID PRIMARY KEY,
 world_id UUID NOT NULL,
 character_id UUID NOT NULL REFERENCES game_character_roots(character_id) ON DELETE RESTRICT,
 box SMALLINT NOT NULL CHECK(box BETWEEN 1 AND 17),
 ordinal NUMERIC(20,0) NOT NULL CHECK(ordinal BETWEEN 1 AND 18446744073709551615),
 UNIQUE(character_id,box,ordinal),
 FOREIGN KEY(item_instance_id,world_id) REFERENCES game_item_instances(item_instance_id,world_id)
);
CREATE FUNCTION game_depot_character_world() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 IF NOT EXISTS(SELECT 1 FROM game_character_roots
               WHERE character_id=NEW.character_id AND world_id=NEW.world_id) THEN
  RAISE EXCEPTION 'depot Character and item must share World' USING ERRCODE='23514';
 END IF;
 RETURN NEW;
END $$;
CREATE TRIGGER game_depot_character_world BEFORE INSERT ON game_item_character_depot_locations
 FOR EACH ROW EXECUTE FUNCTION game_depot_character_world();
CREATE TRIGGER game_depot_immutable BEFORE UPDATE ON game_item_character_depot_locations
 FOR EACH ROW EXECUTE FUNCTION game_item_immutable();
CREATE TRIGGER game_depot_no_truncate BEFORE TRUNCATE ON game_item_character_depot_locations
 FOR EACH STATEMENT EXECUTE FUNCTION game_item_reject_truncate();
CREATE FUNCTION game_depot_capacity() RETURNS trigger LANGUAGE plpgsql AS $$
BEGIN
 PERFORM 1 FROM game_character_roots WHERE character_id=NEW.character_id FOR UPDATE;
 IF (SELECT count(*) FROM game_item_character_depot_locations
     WHERE character_id=NEW.character_id) > 15000 THEN
  RAISE EXCEPTION 'depot hard entry ceiling' USING ERRCODE='23514';
 END IF;
 RETURN NULL;
END $$;
CREATE CONSTRAINT TRIGGER game_depot_capacity AFTER INSERT ON game_item_character_depot_locations
 DEFERRABLE INITIALLY DEFERRED FOR EACH ROW EXECUTE FUNCTION game_depot_capacity();
DO $$ BEGIN
 EXECUTE format('ALTER FUNCTION game_depot_character_world() SET search_path = %I, pg_temp',current_schema());
 EXECUTE format('ALTER FUNCTION game_depot_capacity() SET search_path = %I, pg_temp',current_schema());
END $$;
REVOKE ALL ON game_item_character_depot_locations FROM PUBLIC;
REVOKE ALL ON FUNCTION game_depot_character_world(),game_depot_capacity() FROM PUBLIC;
"#;

#[cfg(test)]
#[path = "character_depot_tests.rs"]
mod tests;
