//! WORLDCHG-Q portal decisions survive independent connections and epoch changes.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::world_event::changes;

#[test]
fn worldchg_postgres_portal_variants_reload_from_shared_owner() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        fixture::runtime().map_err(fixture::debug)?.block_on(async {
            let start = std::time::Instant::now();
            let h = fixture::Harness::create(admin,"worldchg_portals",false).await?;
            let result = exercise(&h).await;
            h.cleanup().await?;
            result?;
            assert!(start.elapsed().as_secs_f64() > 1.0,"real PostgreSQL execution required");
            println!("WORLDCHG-Q PG EXECUTED: all13 portal variants, independent connection reload, immutable epoch and stale-epoch refusal; elapsed={:?}",start.elapsed());
            Ok::<(),Box<dyn std::error::Error>>(())
        }).map_err(fixture::debug)
    })?.join().map_err(|_| "portal PG thread failed")?.map_err(Into::into)
}
async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let world = fixture::scope()?.world_id();
    let mut variants = std::collections::BTreeSet::new();
    let mut epochs = Vec::new();
    for epoch in 1..10000 {
        let day = changes::roll(world, epoch, 0)?;
        let mut added = false;
        if let Some(v) = day.fury_gates {
            added |= variants.insert(("fury", v));
        }
        if let Some(v) = day.nightmare_isles {
            added |= variants.insert(("nightmare", v));
        }
        if added {
            epochs.push((epoch, day));
        }
        if variants.len() == 13 {
            break;
        }
    }
    assert_eq!(variants.len(), 13);
    for (epoch, expected) in epochs {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp(0),'1970-01')")
            .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind([7u8;32].as_slice()).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_world_reset_active(world_id,reset_epoch) VALUES(encode($1,'hex')::uuid,$2::text::numeric) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=excluded.reset_epoch")
            .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).execute(&h.pool).await?;
        assert_eq!(h.root.world_changes(world, epoch).await?, Some(expected));
        let mut connection = h.pool.acquire().await?;
        let mut tx = sqlx::Connection::begin(&mut *connection).await?;
        assert_eq!(changes::load(&mut tx, world, epoch).await?, Some(expected));
        tx.rollback().await?;
        assert!(h.root.world_changes(world, epoch + 1).await?.is_none());
    }
    Ok(())
}
