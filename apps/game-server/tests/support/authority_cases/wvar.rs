//! Focused real-PG proof of the reused daily decision and current passage consumer.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::world_event::changes;
use oteryn_gameplay_world_change::variants;

struct Premium(bool);
impl crate::durability::npc_travel::NpcTravelGate for Premium {
    async fn travel_permitted(
        &self,
        _tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
        _account: [u8; 16],
    ) -> Result<bool, crate::durability::DurabilityError> {
        Ok(self.0)
    }
}

#[test]
fn wvar_postgres_stored_variants_and_current_passages() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        fixture::runtime().map_err(fixture::debug)?.block_on(async {
            let start = std::time::Instant::now();
            let h = fixture::Harness::create(admin, "wvar", true).await?;
            let result = exercise(&h).await;
            h.cleanup().await?;
            result?;
            println!("WVAR POSTGRES_CASE_COMPLETED batch1 stored-selection,current-passage,all-pairs,premium,promotion,level,stale-session,reset; duration={:?}", start.elapsed());
            Ok::<(), Box<dyn std::error::Error>>(())
        }).map_err(fixture::debug)
    })?.join().map_err(|_| "wvar PG thread failed")?.map_err(Into::into)
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let world = fixture::scope()?.world_id();
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let definitions = variants::definitions()?;
    // Each variant is reached through a real deterministic server-save decision;
    // no test-edited activation JSON or chance override grants admission.
    let mut cases = Vec::new();
    for d in definitions {
        for (index, v) in d.variants.iter().enumerate() {
            let epoch = (1..100_000_u64)
                .find(|epoch| {
                    changes::roll(world, *epoch, 0)
                        .is_ok_and(|day| day.variant(&d.key) == Some(index))
                })
                .ok_or("variant epoch")?;
            cases.push((epoch, d, index, v));
        }
    }
    cases.sort_by_key(|c| c.0);
    for (epoch, d, index, v) in cases {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp($4),'1970-01') ON CONFLICT DO NOTHING")
                .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind([7u8;32].as_slice()).bind((epoch * 86400) as f64).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2::text::numeric) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=excluded.reset_epoch")
                .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).execute(&h.pool).await?;
        // Fixture of the reset-finalization write; the exact unmodified
        // production selection is serialized, not a hand-built activation.
        sqlx::query("INSERT INTO game_world_change_days(world_id,reset_epoch,decision) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3) ON CONFLICT DO NOTHING")
                .bind(world.as_bytes().as_slice()).bind(epoch.to_string())
                .bind(serde_json::to_value(changes::roll(world,epoch,(epoch * 86400000) as i64)?)?).execute(&h.pool).await?;
        let day = h
            .root
            .world_changes(world, epoch)
            .await?
            .ok_or("selected day")?;
        assert_eq!(day.variant(&d.key), Some(index));
        assert_eq!(h.root.world_changes(world, epoch).await?, Some(day));
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *tx)
            .await?;
        assert_eq!(changes::load(&mut tx, world, epoch).await?, Some(day));
        assert!(changes::load(&mut tx, world, epoch + 1).await?.is_none());
        tx.rollback().await?;
        for portal in &v.portals {
            let actual = h
                .root
                .world_change_passage(
                    &authority,
                    &h.node,
                    fixture::fence(1)?,
                    epoch,
                    portal.position,
                    &Premium(true),
                )
                .await
                .map_err(fixture::debug)?;
            assert_eq!(
                actual,
                if portal.promoted || portal.min_level > 50 {
                    None
                } else {
                    Some(portal.destination)
                }
            );
            let actual = h
                .root
                .world_change_passage(
                    &authority,
                    &h.node,
                    fixture::fence(1)?,
                    epoch,
                    portal.position,
                    &Premium(false),
                )
                .await
                .map_err(fixture::debug)?;
            assert_eq!(
                actual,
                if portal.premium || portal.promoted || portal.min_level > 50 {
                    None
                } else {
                    Some(portal.destination)
                }
            );
            assert_eq!(
                h.root
                    .world_change_passage(
                        &authority,
                        &h.node,
                        fixture::fence(1)?,
                        epoch + 1,
                        portal.position,
                        &Premium(true)
                    )
                    .await
                    .map_err(fixture::debug)?,
                None
            );
            let mut stale = fixture::fence(1)?;
            stale.character_lease_generation += 1;
            assert!(
                h.root
                    .world_change_passage(
                        &authority,
                        &h.node,
                        stale,
                        epoch,
                        portal.position,
                        &Premium(true)
                    )
                    .await
                    .is_err()
            );
        }
    }
    // Promotion is acquired through its existing purchase owner. Only the test
    // Character's starting level is raised; no entitlement or promotion row is forged.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_progression_state SET level=60 WHERE character_id=encode($1,'hex')::uuid")
        .bind(fixture::id(fixture::CHARACTER).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    crate::character_promotion_fixture::purchase(h, 1).await?;
    let fury = definitions
        .iter()
        .find(|d| d.key == "fury_gates")
        .ok_or("fury definition")?;
    let mut next_epoch = 100_000_u64;
    for (index, variant) in fury.variants.iter().enumerate() {
        let epoch = (next_epoch..next_epoch + 100_000)
            .find(|epoch| {
                changes::roll(world, *epoch, 0)
                    .is_ok_and(|day| day.variant("fury_gates") == Some(index))
            })
            .ok_or("positive fury epoch")?;
        next_epoch = epoch + 1;
        seed_epoch(h, epoch).await?;
        for portal in &variant.portals {
            assert_eq!(
                h.root
                    .world_change_passage(
                        &authority,
                        &h.node,
                        fixture::fence(2)?,
                        epoch,
                        portal.position,
                        &Premium(true)
                    )
                    .await
                    .map_err(fixture::debug)?,
                Some(portal.destination)
            );
            assert_eq!(
                h.root
                    .world_change_passage(
                        &authority,
                        &h.node,
                        fixture::fence(2)?,
                        epoch,
                        portal.position,
                        &Premium(false)
                    )
                    .await
                    .map_err(fixture::debug)?,
                if portal.premium {
                    None
                } else {
                    Some(portal.destination)
                }
            );
        }
    }
    Ok(())
}

async fn seed_epoch(h: &fixture::Harness, epoch: u64) -> fixture::TestResult {
    let world = fixture::scope()?.world_id();
    // Give each test probe its own monotonically advancing reset-history date.
    let timestamp = 4_000_000_000_i64 + i64::try_from(epoch)? * 86_400;
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp($4),'1970-01') ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind([7u8;32].as_slice()).bind(timestamp as f64).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2::text::numeric) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=excluded.reset_epoch")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_change_days(world_id,reset_epoch,decision) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3) ON CONFLICT DO NOTHING")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(serde_json::to_value(changes::roll(world,epoch,timestamp*1000)?)?).execute(&h.pool).await?;
    Ok(())
}
