//! Real repository content -> native spawn compiler -> fenced durable activation.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::boss_encounter::{BossClockClaim, BossClockKey, spawned_bosses_for_epoch};
use oteryn_gameplay_content::content::{BossChance, BossPolicy};
use oteryn_world_bundle_compiler::{bundle, compile, source};

#[test]
fn boss_spawn_world_content_never_activates_encounter_bound_postgres() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .name("boss-spawn-world-content".into())
        .stack_size(64 << 20)
        .spawn(move || qualify(admin).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "boss world content thread failed")??;
    Ok(())
}

fn qualify(admin: String) -> fixture::TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    // Heoni's world point and Gaffir's encounter room; read the actual checked families.
    let project = source::project_regions(&root, &[(129, 124), (130, 127)])?;
    let (registry, palette) = source::registry(&root, &project)?;
    let (table, _) = compile::realize_spawns(
        &compile::Input {
            regions: &project.regions,
            palette: &palette,
            identity: bundle::Identity::default(),
            world: source::world_extent(&root)?,
            build_class: bundle::BuildClass::NonProduction,
            draft_areas: Vec::new(),
            families: &project.families,
        },
        &registry,
    )?;
    let held: serde_json::Value = serde_json::from_slice(&std::fs::read(
        root.join("content/world/spawns/held-bosses.json"),
    )?)?;
    let held = held["records"].as_array().ok_or("held records")?;
    for row in held {
        let boss = row["point"]["creature"].as_str().ok_or("held creature")?;
        assert_eq!(row["reason"], "encounter-bound");
        assert!(!table.creatures.iter().any(|creature| creature.key == boss));
    }
    assert!(table.creatures.iter().any(|creature| creature.key == "oteryn:creature.heoni"));
    fixture::runtime()?.block_on(async {
        let h = fixture::Harness::create(admin, "boss_world_content", false).await?;
        let result: fixture::TestResult = async {
            let seal = h.recovery.seal_current().map_err(fixture::debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(fixture::debug)?;
            let scope = fixture::scope()?;
            let world = scope.world_id();
            let channel = scope.channel_id().ok_or("channel")?;
            sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,0,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),'2026-11')")
                .bind(world.as_bytes().as_slice()).bind([1u8;32].as_slice()).execute(&h.pool).await?;
            sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,0)")
                .bind(world.as_bytes().as_slice()).execute(&h.pool).await?;
            let mut claimed = 0;
            for source in &table.sources {
                for (ordinal, point) in source.points.iter().enumerate() {
                    if point.boss_policy.is_none() { continue; }
                    let boss = &table.creatures[point.creature as usize].key;
                    // Force success only for this fixture's admitted world points. The
                    // exclusion assertion must hold even when every world roll wins.
                    let outcome = h.root.activate_boss_clock(
                        &authority, &h.node,
                        BossClockKey { world, channel, spawn:format!("{}:{ordinal}",source.key) },
                        boss.clone(),
                        crate::foundation::ScopeOwnershipGeneration::new(1)?,
                        BossPolicy::AtReset { chance:BossChance { numerator:1, denominator:1 } },
                    ).await?;
                    claimed += usize::from(matches!(outcome,BossClockClaim::Claimed(_)));
                }
            }
            assert!(claimed > 0, "positive control: real world boss activation");
            let mut tx = h.pool.begin().await?;
            let outcomes = spawned_bosses_for_epoch(&mut tx,world,0).await?;
            assert_eq!(outcomes.len(),claimed);
            for row in held {
                let boss = row["point"]["creature"].as_str().ok_or("boss")?;
                assert!(!outcomes.iter().any(|(key,_)| key == boss));
            }
            tx.rollback().await?;
            eprintln!("BOSS-SPAWN-1 PG EXECUTED: repository compiler -> durable world claims; {claimed} positive controls; {} encounter-bound points excluded", held.len());
            Ok(())
        }.await;
        h.cleanup().await?;
        result
    })
}
