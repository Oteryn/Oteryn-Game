use super::{TestResult, fixture::id};
use crate::foundation::WorldId;

pub(super) async fn snapshot(
    pool: &sqlx::PgPool,
    world: WorldId,
    category: i32,
    character: &[u8; 16],
    own_position: i32,
) -> TestResult<u64> {
    let snapshot: i64 = sqlx::query_scalar("INSERT INTO game_world_highscore_snapshots(world_id,category,computed_at_ms,source_cut,row_count) VALUES(encode($1,'hex')::uuid,$2,100,'t143',102) RETURNING snapshot_id")
        .bind(world.as_bytes().as_slice()).bind(category).fetch_one(pool).await?;
    for filter in [0i16, 2] {
        for position in 1..=51 {
            let own = position == own_position;
            let other = id(100 + position as u8);
            sqlx::query("INSERT INTO game_world_highscore_rows(snapshot_id,filter,position,rank,character_id,name,vocation,level,value_primary,value_progress) VALUES($1,$2,$3,$4,encode($5,'hex')::uuid,$6,'knight',100,$7,0)")
                .bind(snapshot).bind(filter).bind(position).bind(if own {1500i64} else {i64::from(position)})
                .bind(if own {character.as_slice()} else {other.as_slice()})
                .bind(if own {"Own Knight"} else {"Other Knight"})
                .bind(if own {500i64} else {2000 - i64::from(position)}).execute(pool).await?;
        }
    }
    // Publication seals membership; populate every row before exposing the head.
    sqlx::query("INSERT INTO game_world_highscore_heads(world_id,category,current_snapshot_id) VALUES(encode($1,'hex')::uuid,$2,$3)")
        .bind(world.as_bytes().as_slice()).bind(category).bind(snapshot).execute(pool).await?;
    Ok(u64::try_from(snapshot)?)
}