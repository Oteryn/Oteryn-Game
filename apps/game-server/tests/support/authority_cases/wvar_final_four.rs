//! Real reset-owner proof for the final four population/layer rotations.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::world_event::changes;
use oteryn_gameplay_world_change::variants;

struct FinalFourGate;
impl crate::durability::npc_travel::NpcTravelGate for FinalFourGate {
    async fn travel_permitted(
        &self,
        _tx: &mut crate::durability::item_mint_audit::Type2Transaction<'_>,
        _account: [u8; 16],
    ) -> Result<bool, crate::durability::DurabilityError> {
        Ok(true)
    }
}

#[test]
fn wvar_final_four_postgres_stored_rotation() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new().stack_size(64 << 20).spawn(move || -> Result<(), String> {
        fixture::runtime().map_err(fixture::debug)?.block_on(async {
            let start = std::time::Instant::now();
            let h = fixture::Harness::create(admin, "wvar_final_four", true).await?;
            let result = exercise_final_four(&h).await;
            h.cleanup().await?;
            result?;
            println!("WVAR_FINAL_FOUR POSTGRES_CASE_COMPLETED variants=19 reset-owner,stored-rotation,second-reader,stale-epoch,spirit-independent-population,safe-return,complete-membership; duration={:?}", start.elapsed());
            Ok::<(), Box<dyn std::error::Error>>(())
        }).map_err(fixture::debug)
    })?.join().map_err(|_| "WVAR final-four PG thread failed")?.map_err(Into::into)
}

async fn exercise_final_four(h: &fixture::Harness) -> fixture::TestResult {
    let world = fixture::scope()?.world_id();
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let definitions = variants::definitions()?;
    let mut cases = Vec::new();
    for (key, count) in [
        ("chakoya_iceberg", 2),
        ("spirit_grounds", 12),
        ("hive_outpost", 2),
        ("poacher_caves", 3),
    ] {
        let definition = definitions
            .iter()
            .find(|d| d.key == key)
            .ok_or("missing final-four definition")?;
        assert_eq!(definition.variants.len(), count);
        for index in 0..count {
            let epoch = (1..100_000_u64)
                .find(|epoch| {
                    changes::roll(world, *epoch, 0).is_ok_and(|day| day.variant(key) == Some(index))
                })
                .ok_or("unreachable variant")?;
            cases.push((epoch, definition, index));
        }
    }
    assert_eq!(cases.len(), 19);
    cases.sort_by_key(|c| c.0);
    let mut spirit_pairs = std::collections::BTreeSet::new();
    for (epoch, definition, index) in cases {
        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'map-1',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,to_timestamp($4),'1970-01') ON CONFLICT DO NOTHING")
            .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind([7u8;32].as_slice()).bind((epoch * 86400) as f64).execute(&h.pool).await?;
        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2::text::numeric) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=excluded.reset_epoch")
            .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).execute(&h.pool).await?;
        let mut reset = h.pool.begin().await?;
        changes::record_reset_fixture(&mut reset, world, epoch).await?;
        reset.commit().await?;
        let day = h
            .root
            .world_changes(world, epoch)
            .await?
            .ok_or("missing committed decision")?;
        assert_eq!(day.variant(&definition.key), Some(index));
        let mut reader = h.pool.begin().await?;
        sqlx::query("SET LOCAL ROLE oteryn_game_runtime")
            .execute(&mut *reader)
            .await?;
        let replay = changes::load(&mut reader, world, epoch)
            .await?
            .ok_or("independent reader decision")?;
        assert_eq!(replay, day);
        assert!(
            changes::load(&mut reader, world, epoch + 1)
                .await?
                .is_none()
        );
        reader.rollback().await?;
        let selected = &definition.variants[index];
        assert!(replay.variants.active().any(|v| v.key == selected.key));
        for spawn in &selected.spawns {
            assert!(selected.tiles.iter().any(|t| t.position == spawn.position));
        }
        if definition.key == "spirit_grounds" {
            let entry = &selected.portals[0];
            let exit = &selected.portals[1];
            assert_eq!(
                replay
                    .variants
                    .passage(entry.position)
                    .map(|p| p.destination),
                Some(entry.destination)
            );
            assert_eq!(
                replay
                    .variants
                    .passage(exit.position)
                    .map(|p| p.destination),
                Some(exit.destination)
            );
            assert_ne!(exit.destination, entry.position);
            assert!(
                selected
                    .tiles
                    .iter()
                    .any(|t| t.position == entry.destination)
            );
            assert!(
                definition
                    .inactive_tiles
                    .iter()
                    .any(|t| t.position == exit.destination)
            );
            for other in &definition.variants {
                if other.portals[0].position != entry.position {
                    assert!(replay.variants.passage(other.portals[0].position).is_none());
                }
            }
            for portal in &selected.portals {
                assert_eq!(
                    h.root
                        .world_change_passage(
                            &authority,
                            &h.node,
                            fixture::fence(1)?,
                            epoch,
                            portal.position,
                            &FinalFourGate
                        )
                        .await
                        .map_err(fixture::debug)?,
                    Some(portal.destination)
                );
            }
            spirit_pairs.insert((entry.position, selected.spawns[0].creature.clone()));
        }
    }
    assert_eq!(spirit_pairs.len(), 12);
    Ok(())
}
