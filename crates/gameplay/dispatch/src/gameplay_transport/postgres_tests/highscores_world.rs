//! T143: the composed authority reads persisted cross-World snapshots, never another actor.
use super::fixture::{Harness, debug, id};
use super::kill_reward_e2e::scenario_thread::run_async;
use super::{Dependencies, TestResult, maintain_ready, prepare_current_session};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::durability::highscore_snapshot::reply::HighscoresReply;
use crate::foundation::{FoundationProtocolError, GameSessionId, WorldId};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_protocol_oteryn::highscores::{self as wire, HighscoresQuery, HighscoresResult};

async fn snapshot(
    pool: &sqlx::PgPool,
    world: WorldId,
    category: i32,
    character: &[u8; 16],
    own_position: i32,
) -> TestResult<u64> {
    let mut tx = pool.begin().await?;
    let snapshot: i64 = sqlx::query_scalar("INSERT INTO game_world_highscore_snapshots(world_id,category,computed_at_ms,source_cut,row_count) VALUES(encode($1,'hex')::uuid,$2,100,'t143',102) RETURNING snapshot_id")
        .bind(world.as_bytes().as_slice()).bind(category).fetch_one(&mut *tx).await?;
    for filter in [0i16, 2] {
        for position in 1..=51 {
            let own = position == own_position;
            let other = id(100 + position as u8);
            sqlx::query("INSERT INTO game_world_highscore_rows(snapshot_id,filter,position,rank,character_id,name,vocation,level,value_primary,value_progress) VALUES($1,$2,$3,$4,encode($5,'hex')::uuid,$6,'knight',100,$7,0)")
                .bind(snapshot).bind(filter).bind(position).bind(if own {1500i64} else {i64::from(position)})
                .bind(if own {character.as_slice()} else {other.as_slice()})
                .bind(if own {"Own Knight"} else {"Other Knight"})
                .bind(if own {500i64} else {2000 - i64::from(position)}).execute(&mut *tx).await?;
        }
    }
    // 0129 freezes membership once a head publishes the snapshot, just as in
    // the production publisher: insert every row before publishing atomically.
    sqlx::query("INSERT INTO game_world_highscore_heads(world_id,category,current_snapshot_id) VALUES(encode($1,'hex')::uuid,$2,$3)")
        .bind(world.as_bytes().as_slice()).bind(category).bind(snapshot).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(u64::try_from(snapshot)?)
}
fn result(reply: HighscoresReply) -> TestResult<HighscoresResult> {
    let HighscoresReply::Result(bytes) = reply else {
        return Err(format!("expected page: {reply:?}").into());
    };
    wire::decode_highscores_result(&bytes).map_err(debug)
}
#[test]
fn highscores_world_reads_keep_session_authority_and_show_me_uses_filter_position() -> TestResult {
    run_async(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "t143_highscores_world", true, false)
            .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let dependencies = Dependencies::new()?;
        let owner = dependencies.owner(&h, &authority);
        let admitted = dependencies.admitted;
        let (current, _) = FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(admitted.game_session_id)
            .await?;
        let character = current.commit().character_id();
        let home = admitted.world_id;
        let foreign = WorldId::decode(&id(90)).map_err(debug)?;
        let home_snapshot = snapshot(&h.pool, home, 1, character.as_bytes(), 51).await?;
        snapshot(&h.pool, home, 2, character.as_bytes(), 51).await?;
        // Even a historical row bearing this CharacterId must not become a foreign own row.
        let foreign_snapshot = snapshot(&h.pool, foreign, 1, character.as_bytes(), 1).await?;
        let mut query = HighscoresQuery {
            world: None,
            details: true,
            category: 1,
            vocation: 1,
            page: 1,
            snapshot_id: 0,
        };
        maintain_ready(&h.root).await?;
        let first = result(
            owner
                .highscores(home, admitted.game_session_id, query)
                .await,
        )?;
        assert_eq!(
            (first.own_page, first.own_row.as_ref().map(|row| row.rank)),
            (Some(2), Some(1500))
        );
        assert_eq!(first.rows.len(), 50);
        assert!(first.has_more);
        query.page = first.own_page.ok_or("missing own page")?;
        query.snapshot_id = first.snapshot_id;
        maintain_ready(&h.root).await?;
        let own = result(
            owner
                .highscores(home, admitted.game_session_id, query)
                .await,
        )?;
        assert_eq!((own.page, own.snapshot_id), (2, home_snapshot));
        assert_eq!(own.rows[0].name, "Own Knight");
        assert!(!own.has_more);
        query.world = Some(wire::WorldId::decode(foreign.as_bytes()).map_err(debug)?);
        query.page = 1;
        query.snapshot_id = 0;
        maintain_ready(&h.root).await?;
        let other = result(
            owner
                .highscores(home, admitted.game_session_id, query)
                .await,
        )?;
        assert_eq!(other.snapshot_id, foreign_snapshot);
        assert_eq!(other.world, query.world);
        assert!(other.own_row.is_none() && other.own_page.is_none());
        query.world = None;
        query.snapshot_id = foreign_snapshot;
        maintain_ready(&h.root).await?;
        assert_eq!(
            owner
                .highscores(home, admitted.game_session_id, query)
                .await,
            HighscoresReply::Terminal(FoundationProtocolError::SnapshotExpired)
        );
        query.snapshot_id = 0;
        query.category = 2;
        query.vocation = 0;
        query.details = false;
        maintain_ready(&h.root).await?;
        let legacy = result(
            owner
                .highscores(home, admitted.game_session_id, query)
                .await,
        )?;
        assert_eq!(legacy.category, 2);
        assert!(legacy.world.is_none() && legacy.own_page.is_none());
        maintain_ready(&h.root).await?;
        assert_eq!(
            owner
                .highscores(foreign, admitted.game_session_id, query)
                .await,
            HighscoresReply::Rejected
        );
        maintain_ready(&h.root).await?;
        assert_eq!(
            owner
                .highscores(home, GameSessionId::decode(&id(99)).map_err(debug)?, query)
                .await,
            HighscoresReply::Rejected
        );
        maintain_ready(&h.root).await?;
        assert_eq!(
            crate::durability::highscore_snapshot::reply::answer_session_highscores(
                &h.root,
                foreign,
                foreign,
                admitted.game_session_id,
                query,
            )
            .await,
            HighscoresReply::Rejected
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
