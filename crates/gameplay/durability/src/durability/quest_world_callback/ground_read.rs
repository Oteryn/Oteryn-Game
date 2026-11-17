//! Read-only evidence from the existing Ground custody owner, held until commit.
use oteryn_gameplay_quests::quest::world_callbacks::{
    Position,
    ground::{GroundStackObserved, GroundStackRequirement, MAX_GROUND_STACKS},
};
use sqlx::{Acquire, Row};

pub(crate) struct Context<'a> {
    pub world: &'a [u8; 16],
    pub channel: &'a [u8; 16],
    pub generation: u64,
    pub map: &'a str,
    pub content: &'a str,
    pub placement: &'a [u8],
}

pub(crate) async fn matches(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    context: &Context<'_>,
    position: Position,
    required: &GroundStackRequirement,
) -> Result<bool, sqlx::Error> {
    if !required.valid() || context.generation == 0 || !(-15..=0).contains(&position.floor) {
        return Ok(false);
    }
    // Same channel namespace as 0034's INSERT/DELETE owner, including empty tiles:
    // row locks alone cannot prevent an additional Ground stack appearing.
    let mut key = context.world.to_vec();
    key.extend_from_slice(context.channel);
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended(encode($1,'hex'),33))")
        .bind(key)
        .execute(&mut **tx)
        .await?;
    let mut spatial = i32::from(position.x).to_be_bytes().to_vec();
    spatial.extend_from_slice(&i32::from(position.y).to_be_bytes());
    spatial.extend_from_slice(&i16::from(position.floor).to_be_bytes());
    // Do not pre-filter definitions, quantities, or lifecycle: an extra or
    // invalid current Ground instance must not disappear from exact-stack evidence.
    // Ground has no runtime UPDATE grant; retain its owner lock and lock Item rows.
    // Ground mutations can already hold an Item row before asking for key 33.
    // Do not wait in the opposite order. A savepoint makes a busy-row refusal
    // leave the caller's quest transaction usable and retains its Ground lock.
    let mut probe = tx.begin().await?;
    let read = sqlx::query(
        "SELECT i.item_instance_id, i.lifecycle, i.definition_family, \
                i.definition_production_key, i.definition_revision_ref, i.quantity, g.stack_ordinal \
         FROM game_item_ground_locations g JOIN game_item_instances i USING(item_instance_id,world_id) \
         WHERE g.world_id=encode($1,'hex')::uuid AND g.channel_id=encode($2,'hex')::uuid \
           AND g.scope_kind=1 AND g.instance_id IS NULL \
           AND g.runtime_scope_ownership_generation=$3::text::numeric \
           AND g.spatial_position=$4 AND g.map_revision=$5 AND g.content_revision=$6 \
           AND g.native_room_placement_context=$7 \
         ORDER BY g.stack_ordinal NULLS FIRST, i.item_instance_id LIMIT $8 FOR SHARE OF i NOWAIT",
    )
    .bind(context.world.as_slice()).bind(context.channel.as_slice())
    .bind(context.generation.to_string()).bind(spatial).bind(context.map)
    .bind(context.content).bind(context.placement).bind((MAX_GROUND_STACKS + 1) as i64)
    .fetch_all(&mut *probe).await;
    let rows = match read {
        Ok(rows) => {
            probe.commit().await?;
            rows
        }
        Err(error) => {
            let busy = error
                .as_database_error()
                .and_then(|e| e.code())
                .is_some_and(|code| code == "55P03");
            probe.rollback().await?;
            return if busy { Ok(false) } else { Err(error) };
        }
    };
    if rows.len() != required.stacks.len() {
        return Ok(false);
    }
    let mut observed = Vec::with_capacity(rows.len());
    for row in &rows {
        if row.try_get::<i16, _>("lifecycle")? != 1
            || row.try_get::<&str, _>("definition_family")? != "Item"
        {
            return Ok(false);
        }
        let quantity: i64 = row.try_get("quantity")?;
        let ordinal: Option<i64> = row.try_get("stack_ordinal")?;
        let Ok(quantity) = u64::try_from(quantity) else {
            return Ok(false);
        };
        observed.push(GroundStackObserved {
            instance: *row
                .try_get::<sqlx::types::Uuid, _>("item_instance_id")?
                .as_bytes(),
            item: row.try_get("definition_production_key")?,
            revision: row.try_get("definition_revision_ref")?,
            quantity,
            ordinal: ordinal.and_then(|value| u64::try_from(value).ok()),
        });
    }
    Ok(required.matches(&observed))
}
