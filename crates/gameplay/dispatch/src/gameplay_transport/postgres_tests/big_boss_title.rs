//! P12a: the Character Info command follows the current World Boss Points snapshot.
use super::cyclopedia_fixtures::support::exchange;
use super::fixture::{Harness, id};
use super::kill_reward_e2e_fixtures::scenario_thread::run_async;
use super::{Dependencies, TestResult, debug, prepare_current_session, reset};
use crate::durability::highscore_snapshot::BOSS_POINTS_CATEGORY;
use crate::gameplay_transport::ComposedFreshAdmission;
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{AdmittedSession, FirstEntryOutcome};
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::character_info::{
    CAPABILITY_CHARACTER_INFO_V1, COMMAND_TYPE_CHARACTER_INFO,
};

const BIG_BOSS_ID: u8 = 95;

#[test]
fn big_boss_title_follows_committed_world_rank_postgres() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    run_async(async |admin| {
        let started = std::time::Instant::now();
        reset();
        let h = Harness::create_with_admission_seed(admin, "big_boss_title", true, false).await?;
        eprintln!("POSTGRES_STARTED big_boss_title_follows_committed_world_rank_postgres");
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        let offers = crate::gameplay_transport::character_info::offered(&[], true);
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offers, &[CAPABILITY_CHARACTER_INFO_V1])
                .ok_or("character info capability")?;

        let world = *admitted.world_id.as_bytes();
        let character = *admitted
            .item_fence
            .ok_or("item fence")?
            .character_id
            .as_bytes();
        let read_self = [0, 0, 0, 1];
        let select = [3, BIG_BOSS_ID];

        // The foreign projection deliberately contains the active character, so omitting the
        // World predicate would qualify before its own World has a published snapshot.
        publish_fixture(&h.pool, id(90), character, id(91), 1, 100).await?;
        assert_eq!(
            title_command(&owner, &mut admitted, &read_self).await?.0,
            CommandStatus::Accepted,
        );
        assert!(
            !earned(&h.pool, character).await?,
            "another World cannot qualify"
        );

        publish_fixture(&h.pool, world, id(91), character, 2, 100).await?;
        assert_eq!(
            title_command(&owner, &mut admitted, &read_self).await?.0,
            CommandStatus::Accepted,
        );
        assert!(
            !earned(&h.pool, character).await?,
            "rank two cannot qualify"
        );

        publish_fixture(&h.pool, world, character, id(91), 3, 10).await?;
        assert_eq!(
            title_command(&owner, &mut admitted, &read_self).await?.0,
            CommandStatus::Accepted,
        );
        assert!(
            !earned(&h.pool, character).await?,
            "Crystal requires more than 10 published Boss Points"
        );

        publish_fixture(&h.pool, world, character, id(91), 4, 11).await?;
        assert_eq!(
            title_command(&owner, &mut admitted, &read_self).await?.0,
            CommandStatus::Accepted,
        );
        assert!(
            earned(&h.pool, character).await?,
            "committed rank one earns Big Boss"
        );
        let (status, payload) = title_command(&owner, &mut admitted, &select).await?;
        assert_eq!(status, CommandStatus::Accepted);
        assert_eq!(payload, [3, 1, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(
            selection(&h.pool, character).await?,
            (Some(i16::from(BIG_BOSS_ID)), 1),
        );

        // The earning snapshot is retained as the previous head. Only the current head counts.
        publish_fixture(&h.pool, world, id(91), character, 5, 100).await?;
        assert_eq!(
            title_command(&owner, &mut admitted, &read_self).await?.0,
            CommandStatus::Accepted,
        );
        assert!(
            !earned(&h.pool, character).await?,
            "losing rank one revokes Big Boss"
        );
        assert_eq!(selection(&h.pool, character).await?, (None, 2));
        assert_eq!(
            title_command(&owner, &mut admitted, &select).await?.0,
            CommandStatus::Rejected,
        );

        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await?;
        eprintln!(
            "big_boss_title_follows_committed_world_rank_postgres POSTGRES_CASE_COMPLETED elapsed={:?}",
            started.elapsed()
        );
        Ok(())
    })
}

async fn title_command(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    let command = admitted.continuity.next_command_id;
    exchange(
        owner,
        admitted,
        command,
        COMMAND_TYPE_CHARACTER_INFO,
        payload,
    )
    .await
}

async fn earned(pool: &sqlx::PgPool, character: [u8; 16]) -> TestResult<bool> {
    Ok(sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM game_character_titles WHERE character_id=encode($1,'hex')::uuid AND title=$2)",
    )
    .bind(character.as_slice())
    .bind(i16::from(BIG_BOSS_ID))
    .fetch_one(pool)
    .await?)
}

async fn selection(pool: &sqlx::PgPool, character: [u8; 16]) -> TestResult<(Option<i16>, i64)> {
    Ok(sqlx::query_as(
        "SELECT title,revision FROM game_character_title_selection WHERE character_id=encode($1,'hex')::uuid",
    )
    .bind(character.as_slice())
    .fetch_one(pool)
    .await?)
}

async fn publish_fixture(
    pool: &sqlx::PgPool,
    world: [u8; 16],
    first: [u8; 16],
    second: [u8; 16],
    computed_at_ms: i64,
    winning_points: i64,
) -> TestResult {
    let mut tx = pool.begin().await?;
    let snapshot: i64 = sqlx::query_scalar(
        "INSERT INTO game_world_highscore_snapshots(world_id,category,computed_at_ms,source_cut,row_count)          VALUES(encode($1,'hex')::uuid,$2,$3,pg_current_snapshot()::text,2) RETURNING snapshot_id",
    )
    .bind(world.as_slice())
    .bind(i32::from(BOSS_POINTS_CATEGORY))
    .bind(computed_at_ms)
    .fetch_one(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_world_highscore_rows(snapshot_id,filter,position,rank,character_id,name,vocation,level,value_primary,value_progress)          VALUES($1,0,1,1,encode($2,'hex')::uuid,'First Knight','knight',100,$4,0),                ($1,0,2,2,encode($3,'hex')::uuid,'Second Knight','knight',100,$4::bigint/2,0)",
    )
    .bind(snapshot)
    .bind(first.as_slice())
    .bind(second.as_slice())
    .bind(winning_points)
    .execute(&mut *tx)
    .await?;
    sqlx::query(
        "INSERT INTO game_world_highscore_heads(world_id,category,current_snapshot_id)          VALUES(encode($1,'hex')::uuid,$2,$3) ON CONFLICT (world_id,category) DO UPDATE          SET previous_snapshot_id=game_world_highscore_heads.current_snapshot_id,current_snapshot_id=EXCLUDED.current_snapshot_id",
    )
    .bind(world.as_slice())
    .bind(i32::from(BOSS_POINTS_CATEGORY))
    .bind(snapshot)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(())
}
