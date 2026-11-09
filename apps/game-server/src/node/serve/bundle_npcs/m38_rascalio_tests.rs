/// The production timer must retain other roles of the same NPC identity.
pub(crate) fn m38_rascalio_postgres_case() -> crate::bestiary_postgres_harness::TestResult {
    use crate::bestiary_postgres_harness::{Harness, configured_admin, runtime};
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
        let run = || -> crate::bestiary_postgres_harness::TestResult {
            runtime()?.block_on(async {
                let h = Harness::create(admin, "m38_rascalio", false).await?;
                let result: crate::bestiary_postgres_harness::TestResult = async {
                    eprintln!("POSTGRES_STARTED NPCFIX_M38 Rascalio committed event presence");
                    let seal = h.recovery.seal_current().map_err(|e| format!("{e:?}"))?;
                    let authority = h.root.open_character_authority(&seal).await.map_err(|e| format!("{e:?}"))?;
                    let key = "oteryn:npc.rascalio";
                    let boot = timed_boot(&[key]);
                    let mut channel = channel_runtime_with(8);
                    let world = channel.binding().runtime_scope().world_id();
                    let other = channel.place_dynamic_npc(key, NpcPlacement {
                        x: 32302, y: 32538, floor: -7, direction: MovementFacing::North,
                    }).map_err(|e| format!("{e:?}"))?;
                    let mut timed = TimedNpcSpawns::from_boot(Some(&boot))
                        .with_seasonal(Some(&boot), &[], |_| true);
                    for (epoch, enabled) in [(1u64, false), (2, true), (3, false)] {
                        let stamp = format!("2026-05-{epoch:02} 06:00:00+00");
                        let digest = [38u8;32];
                        sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'m38-rascalio',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,$4::text::timestamptz,'2026-05')")
                            .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(digest.as_slice()).bind(stamp).execute(&h.pool).await?;
                        let announcement = oteryn_gameplay_world_change::scheduled::announcement_for_epoch(epoch).unwrap_or_default();
                        sqlx::query("INSERT INTO game_world_event_epochs(world_id,reset_epoch,active,announcement) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4)")
                            .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(!announcement.is_empty()).bind(announcement).execute(&h.pool).await?;
                        sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,$2::text::numeric) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=EXCLUDED.reset_epoch")
                            .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).execute(&h.pool).await?;
                        h.root.world_change_calendar(world, epoch).await?;
                        h.root.bind_world_change_definition(&authority, world, epoch,
                            oteryn_gameplay_world_change::rules::Definition {
                                key: event_contacts::EVENT.into(),
                                schedule: oteryn_gameplay_world_change::rules::Schedule::AtReset {
                                    chance_permille: if enabled { 1000 } else { 0 },
                                }, counter: None,
                                sources: vec!["https://tibiopedia.pl/npcs/Rascalio".into()],
                            }).await?;
                        assert_eq!(h.root.world_change_active(world, event_contacts::EVENT).await?, enabled);
                        timed.refresh_event_contacts(&h.root, world).await?;
                        timed.reconcile(&mut channel, 720).map_err(|e| format!("{e:?}"))?;
                        let npcs = channel.visible_entities().npcs;
                        assert_eq!(npcs.len(), 1 + usize::from(enabled));
                        assert_eq!(channel.npc_key(other), Some(key));
                        if enabled {
                            let actor = npcs.iter().find(|n| n.actor != other).ok_or("event actor")?.actor;
                            for _ in 0..3 {
                                timed.refresh_event_contacts(&h.root, world).await?;
                                timed.reconcile(&mut channel, 720).map_err(|e| format!("{e:?}"))?;
                            }
                            assert_eq!(channel.npc_key(actor), Some(key));
                            assert_eq!(channel.visible_entities().npcs.len(), 2);
                        }
                    }
                    Ok(())
                }.await;
                h.cleanup().await?;
                result
            })
        };
        run().map_err(|e| e.to_string())
    })?.join().map_err(|_| "Rascalio PG thread panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED NPCFIX_M38_RASCALIO duration={:?} passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
