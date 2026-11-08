//! The full-manifest node must admit its World epoch before world admission warm-up.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::character_cyclopedia::CyclopediaWorldBinding;
use crate::durability::spell_owner_commit::SpellLanePermit;

#[test]
fn node_boot_world_epoch_prepares_admission() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime().map_err(|e| e.to_string())?.block_on(async {
                let h = fixture::Harness::create(admin, "node_boot_epoch", false).await?;
                let result: fixture::TestResult = async {
                    let scope = fixture::scope()?;
                    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
                    let authority = h.root.open_character_authority(&seal).await.map_err(fixture::debug)?;
                    let permit = SpellLanePermit::of_fresh_scope(scope).await.map_err(fixture::debug)?;
                    let binding = CyclopediaWorldBinding {
                        scope, generation: 1, bundle: [7; 32], epoch: 0, area: 1, legacy_area: 1,
                    };
                    assert!(matches!(
                        h.root.prepare_cyclopedia_epoch(&authority, &h.node, &permit, binding).await,
                        Err(crate::durability::DurabilityError::Unavailable)
                    ));
                    let (world, channel): (String, String) = sqlx::query_as(
                        "SELECT world_id::text,channel_id::text FROM game_runtime_scope_assignments WHERE scope_kind=1",
                    ).fetch_one(&h.pool).await?;
                    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,$1::uuid,$2::uuid,4)")
                        .bind(&world).bind(&channel).execute(&h.pool).await?;
                    sqlx::query("SELECT game_content_record_activation($1::uuid,$2::uuid,1,NULL,decode(repeat('01',32),'hex'),decode(repeat('02',32),'hex'),decode(repeat('03',32),'hex'))")
                        .bind(&world).bind(&channel).execute(&h.pool).await?;
                    // Execute the exact setup SQL used by the node-boot shell harness.
                    let setup = include_str!("../../../../../tools/qualification/node_boot/world_epoch.sql");
                    let digest = "07".repeat(32);
                    for expected in [true, false] {
                        let initialized: bool = sqlx::query_scalar(setup)
                            .bind(&world).bind(&channel).bind(&digest).fetch_one(&h.pool).await?;
                        assert_eq!(initialized, expected);
                    }
                    let pins: (Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>, String) = sqlx::query_as(
                        "SELECT target_digest,server_digest,client_digest,frame_digest,map_revision FROM game_world_reset_epochs WHERE world_id=$1::uuid AND reset_epoch=0",
                    ).bind(&world).fetch_one(&h.pool).await?;
                    assert_eq!(pins, (vec![7;32],vec![1;32],vec![2;32],vec![3;32],format!("sha256:{digest}")));
                    let conflict = sqlx::query_scalar::<_, bool>(setup)
                        .bind(&world).bind(&channel).bind("08".repeat(32)).fetch_one(&h.pool).await;
                    assert_eq!(conflict.err().and_then(|e| e.as_database_error().and_then(|e| e.code()).map(|c| c.into_owned())).as_deref(), Some("23514"));
                    h.root.await_ready().await?;
                    // This is the same mandatory warm-up that fresh world admissions run.
                    assert!(!h.root.prepare_cyclopedia_epoch(&authority, &h.node, &permit, binding).await?);
                    Ok(())
                }.await;
                h.cleanup().await?;
                result
            }).map_err(|e| e.to_string())
        })?
        .join().map_err(|_| "node boot epoch thread failed")??;
    Ok(())
}
