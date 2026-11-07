//! Real source rental -> world cadence/native actors -> actual31159 arrival owner.
use super::*;
use crate::durability::horse_rental::rules;
use crate::foundation::MovementLocalPosition;
pub(super) async fn exercise(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> TestResult {
    let started = std::time::Instant::now();
    let expires: i64 =
        sqlx::query_scalar("SELECT max(expires_seconds) FROM game_horse_rental_receipts")
            .fetch_one(&h.pool)
            .await?;
    super::expiry::clock(h, expires + 1).await?;
    let mut triggered = false;
    for _ in 0..64 {
        let next = admitted.continuity.next_command_id;
        // The confirmation belongs to this same real hi/rent connection;
        // closing the stream after rent correctly clears NPC focus.
        let yes = next.checked_add(2).ok_or("rental command range")?;
        let wanted:bool=sqlx::query_scalar("SELECT game_horse_station_draw($1,sha256(uuid_send(encode($2,'hex')::uuid)||uuid_send(encode($3,'hex')::uuid)||uuid_send(encode($4,'hex')::uuid)||convert_to($5,'UTF8')),10)<=3")
            .bind(i32::from(rules::RNG_ESCAPE)).bind(id(42).as_slice()).bind(id(41).as_slice()).bind(admitted.game_session_id.as_bytes().as_slice()).bind(yes.to_string()).fetch_one(&h.pool).await?;
        super::super::grizzly_npc::wire::talk(
            owner,
            admitted,
            npc,
            &[
                (next, "hi"),
                (next + 1, "rent"),
                (yes, if wanted { "yes" } else { "no" }),
            ],
        )
        .await?;
        if wanted {
            triggered = true;
            break;
        }
    }
    assert!(
        triggered,
        "real source confirmation must trigger the finite escape"
    );
    let active: (i64, i64, bool) = sqlx::query_as(
        "SELECT started_seconds,expires_seconds,ended FROM game_horse_station_worlds",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(active.1 - active.0, 86400);
    assert!(!active.2);
    let fees: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_item_fee_burns WHERE cause_kind=473")
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(fees, 3);
    assert!(
        !owner
            .root
            .horse_station_rent_available(owner.character, owner.world_id)
            .await?
    );
    let stop = oteryn_foundation::CancellationToken::new();
    let poll = async {
        tokio::time::timeout(std::time::Duration::from_secs(10), async {
            loop {
                let runtime = owner.runtime.lock().await;
                let count = runtime
                    .positioned_actor_census()
                    .map_err(debug)?
                    .into_iter()
                    .filter(|(actor, _, session)| {
                        session.is_none()
                            && rules::WAVE_CREATURES.iter().any(|k| {
                                runtime.matches_live_creature_identity(*actor, k.as_bytes())
                            })
                    })
                    .count();
                drop(runtime);
                if count > 0 {
                    break Ok::<_, Box<dyn std::error::Error>>(count);
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
    };
    let run = owner.run_horse_station(&stop);
    let finish = async {
        let answer = poll.await;
        stop.cancel();
        answer
    };
    // Existing test runtime intentionally has no Tokio macros feature.
    let mut run = std::pin::pin!(run);
    let mut finish = std::pin::pin!(finish);
    let mut run_end = None;
    let mut population = None;
    let count = std::future::poll_fn(|cx| {
        use std::future::Future;
        if run_end.is_none()
            && let std::task::Poll::Ready(value) = run.as_mut().poll(cx)
        {
            run_end = Some(value);
        }
        if population.is_none()
            && let std::task::Poll::Ready(value) = finish.as_mut().poll(cx)
        {
            population = Some(value);
        }
        if run_end.is_some()
            && let Some(value) = population.take()
        {
            return std::task::Poll::Ready(value);
        }
        std::task::Poll::Pending
    })
    .await;
    assert!(count?? > 0, "actual source wave must admit native horses");
    assert_eq!(
        rules::RAID_AREAS.len() * 3 * rules::WAVE_CREATURES.len(),
        24
    );
    let permit = owner
        .spell_lane_permit()
        .await
        .ok_or("current Horse lane")?;
    let mut ended = false;
    for _ in 0..256 {
        let map = owner.bundle_map().ok_or("source horse map")?;
        let at = MovementLocalPosition {
            x: 32446,
            y: 32230,
            floor: 7,
        };
        let source_tile = crate::map::view::compose_tile(
            &map.overlay,
            &map.facts.pinned(),
            crate::map::overlay::TilePos {
                x: 32446,
                y: 32230,
                floor: -7,
            },
        )
        .map_err(debug)?
        .ok_or("actual source stable catch tile")?;
        assert!(
            source_tile
                .entries
                .iter()
                .any(|entry| entry.facts.appearance_id == 31159),
            "arrival fixture uses the original actual31159 source tile, not adjacent unmarked grass"
        );
        let actor = {
            let mut runtime = owner.runtime.lock().await;
            let doors = owner.door.try_lock().map_err(debug)?;
            let closed = crate::map::floor::BundleDoors(map.facts.doors());
            runtime
                .spawn_arena_opponent("oteryn:creature.horse", at, |r, p| {
                    Some(
                        crate::movement::relocate::creature_cell_admissible(
                            owner.movement_cells,
                            &closed,
                            p,
                        ) && crate::movement::relocate::creature_cell_admissible(
                            owner.movement_cells,
                            doors.blocking_cells(),
                            p,
                        ) && r.native_summon_cell_free(p),
                    )
                })
                .map_err(debug)?
        };
        assert!(
            owner.extract_arrived_station_horses(&permit).await,
            "real masterless Horse on source31159 must settle before next AI turn"
        );
        assert!(
            owner
                .runtime
                .lock()
                .await
                .companion_snapshot(actor)
                .is_err(),
            "source arrival removes Horse regardless of one-in30 outcome"
        );
        ended = sqlx::query_scalar("SELECT ended FROM game_horse_station_worlds")
            .fetch_one(&h.pool)
            .await?;
        if ended {
            break;
        }
    }
    assert!(
        ended,
        "finite deterministic source catches must restore station availability"
    );
    assert!(
        owner
            .root
            .horse_station_rent_available(owner.character, owner.world_id)
            .await?
    );
    let catches: i64 = sqlx::query_scalar("SELECT count(*) FROM game_horse_station_catches")
        .fetch_one(&h.pool)
        .await?;
    assert!(catches > 0);
    eprintln!(
        "SRV7_HORSE_WORLD_NATIVE_EXECUTED elapsed={:.3}s actual_rental_trigger=true source_wave_attempts=24 actual_31159_catches={catches} no_sql_world_seed=true",
        started.elapsed().as_secs_f64()
    );
    Ok(())
}
