#[test]
fn m38_hoaxette_requires_the_frozen_april_calendar_and_retires_only_its_actor() {
    let key = "oteryn:npc.hoaxette";
    let boot = timed_boot(&[key]);
    let mut runtime = channel_runtime_with(8);
    let unrelated = runtime
        .place_dynamic_npc(
            key,
            NpcPlacement {
                x: 32381,
                y: 32215,
                floor: -7,
                direction: MovementFacing::North,
            },
        )
        .expect("other occurrence");
    let mut timed = TimedNpcSpawns::from_boot(Some(&boot))
        .with_seasonal(Some(&boot), &[], |_| true)
        .with_calendar(Some((4, 30)));
    timed
        .reconcile(&mut runtime, 720)
        .expect("April source presence");
    assert_eq!(
        runtime.visible_entities().npcs.len(),
        2,
        "April must admit Hoaxette's own source occurrence"
    );
    let actor = runtime
        .visible_entities()
        .npcs
        .into_iter()
        .find(|npc| npc.position.x == 32380)
        .expect("source actor")
        .actor;
    timed
        .reconcile(&mut runtime, 0)
        .expect("world time cannot change the frozen month");
    assert_eq!(runtime.visible_entities().npcs.len(), 2);
    assert_eq!(runtime.npc_key(actor), Some(key));
    timed = timed.with_calendar(Some((5, 1)));
    timed.reconcile(&mut runtime, 720).expect("May retirement");
    assert_eq!(runtime.npc_key(actor), None);
    assert_eq!(runtime.npc_key(unrelated), Some(key));
}

#[test]
fn m38_hoaxette_refuses_absent_calendar_unadmitted_catalogue_and_bad_anchor() {
    let key = "oteryn:npc.hoaxette";
    let boot = timed_boot(&[key]);
    let no_npc = timed_boot(&[]);
    for (source, enterable, calendar) in [
        (Some(&boot), true, None),
        (Some(&boot), true, Some((3, 31))),
        (Some(&boot), true, Some((5, 1))),
        (Some(&boot), true, Some((4, 31))),
        (Some(&no_npc), true, Some((4, 1))),
        (None, true, Some((4, 1))),
        (Some(&boot), false, Some((4, 1))),
    ] {
        let mut runtime = channel_runtime_with(4);
        let mut timed = TimedNpcSpawns::from_boot(source)
            .with_seasonal(source, &[], |_| enterable)
            .with_calendar(calendar);
        timed
            .reconcile(&mut runtime, 720)
            .expect("closed source predicate");
        assert!(runtime.visible_entities().npcs.is_empty());
    }
}

#[test]
fn m38_hoaxette_return_defers_an_occupied_anchor_and_retries_next_tick() {
    let boot = timed_boot(&["oteryn:npc.hoaxette"]);
    let mut runtime = channel_runtime_with(8);
    let blocker = runtime
        .place_dynamic_npc(
            "oteryn:npc.rascalio",
            NpcPlacement {
                x: 32380,
                y: 32215,
                floor: -7,
                direction: MovementFacing::North,
            },
        )
        .expect("occupied source anchor");
    let mut timed = TimedNpcSpawns::from_boot(Some(&boot))
        .with_seasonal(Some(&boot), &[], |_| true)
        .with_calendar(Some((4, 1)));
    timed
        .reconcile(&mut runtime, 720)
        .expect("deferred return is not a tick failure");
    assert_eq!(runtime.npc_key(blocker), Some("oteryn:npc.rascalio"));
    assert_eq!(runtime.visible_entities().npcs.len(), 1);
    runtime
        .remove_dynamic_npc_actor(blocker)
        .expect("anchor becomes vacant");
    timed
        .reconcile(&mut runtime, 720)
        .expect("return on the next tick");
    let npcs = runtime.visible_entities().npcs;
    assert_eq!(npcs.len(), 1);
    assert_eq!(runtime.npc_key(npcs[0].actor), Some("oteryn:npc.hoaxette"));
    assert_eq!((npcs[0].position.x, npcs[0].position.y), (32380, 32215));
}

pub(crate) fn m38_hoaxette_postgres_case() -> crate::bestiary_postgres_harness::TestResult {
    use crate::bestiary_postgres_harness::{Harness, configured_admin, runtime};
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new().stack_size(64 << 20).spawn(move || {
        let run = || -> crate::bestiary_postgres_harness::TestResult {
        runtime()?.block_on(async {
            let h = Harness::create(admin, "m38_hoaxette", false).await?;
            let result: crate::bestiary_postgres_harness::TestResult = async {
                eprintln!("POSTGRES_STARTED NPCFIX_M38 Hoaxette immutable reset calendar");
                let key = "oteryn:npc.hoaxette";
                let boot = timed_boot(&[key]);
                let mut channel = channel_runtime_with(8);
                let world = channel.binding().runtime_scope().world_id();
                let mut timed = TimedNpcSpawns::from_boot(Some(&boot))
                    .with_seasonal(Some(&boot), &[], |_| true);
                let mut april_actor = None;
                for (epoch, month, day, present) in [(1u64, 3u8, 31u8, false), (2, 4, 1, true), (3, 4, 30, true), (4, 5, 1, false)] {
                    let timestamp = format!("2026-{month:02}-{day:02} 06:00:00+00");
                    let digest = vec![38u8; 32];
                    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,'m38-hoaxette',$3,$3,$3,'TIBIA_SERVER_SAVE_BERLIN_06',1,$4::text::timestamptz,$5)")
                        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(&digest)
                        .bind(timestamp).bind(format!("2026-{month:02}")).execute(&h.pool).await?;
                    let announcement = oteryn_gameplay_world_change::scheduled::announcement_for_epoch(epoch).unwrap_or_default();
                    sqlx::query("INSERT INTO game_world_event_epochs(world_id,reset_epoch,active,announcement) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4)")
                        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(!announcement.is_empty()).bind(announcement).execute(&h.pool).await?;
                    sqlx::query("INSERT INTO game_world_reset_active(world_id,reset_epoch) VALUES(encode($1,'hex')::uuid,$2::text::numeric) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=EXCLUDED.reset_epoch")
                        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).execute(&h.pool).await?;
                    let calendar = h.root.world_change_calendar(world, epoch).await?;
                    if calendar != (month, day) { return Err("immutable reset calendar mismatch".into()); }
                    if h.root.world_change_calendar(world, epoch).await? != calendar {
                        return Err("same epoch replay changed its calendar".into());
                    }
                    timed = timed.with_calendar(Some(calendar));
                    if epoch == 2 {
                        let blocker = channel.place_dynamic_npc("oteryn:npc.rascalio", NpcPlacement {
                            x: 32380, y: 32215, floor: -7, direction: MovementFacing::North,
                        }).map_err(|e| format!("{e:?}"))?;
                        timed.reconcile(&mut channel, 720).map_err(|e| format!("{e:?}"))?;
                        assert_eq!(channel.npc_key(blocker), Some("oteryn:npc.rascalio"));
                        assert_eq!(channel.visible_entities().npcs.len(), 1);
                        channel.remove_dynamic_npc_actor(blocker).map_err(|e| format!("{e:?}"))?;
                    }
                    timed.reconcile(&mut channel, 720).map_err(|e| format!("{e:?}"))?;
                    let npcs = channel.visible_entities().npcs;
                    if npcs.len() != usize::from(present) { return Err("Hoaxette presence does not match committed April calendar".into()); }
                    if present {
                        let npc = &npcs[0];
                        if (npc.position.x, npc.position.y, npc.position.floor) != (32380,32215,7) {
                            return Err("Hoaxette did not use her captured source anchor".into());
                        }
                        if let Some(previous) = april_actor
                            && previous != npc.actor
                        {
                            return Err("April epoch replay replaced the live NPC".into());
                        }
                        april_actor = Some(npc.actor);
                    }
                    if epoch > 1 && h.root.world_change_calendar(world, epoch-1).await.is_ok() {
                        return Err("stale epoch could supply an availability decision".into());
                    }
                }
                Ok(())
            }.await;
            h.cleanup().await?;
            result
        })
        };
        run().map_err(|error| error.to_string())
    })?.join().map_err(|_| "M38 Hoaxette PG thread panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED NPCFIX_M38 Hoaxette calendar duration={:?} passed={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}
