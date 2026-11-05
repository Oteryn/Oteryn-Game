mod clxc_projection;

use super::*;
use oteryn_gameplay_domain::domain::echo_warden::{
    CHILD_TTL_MS, EchoOutcome, EchoRaidPlan, SPAWN_STEP_MS, WARDEN_ADDS_MAX,
};

pub(super) fn world(
    world: WorldId,
    channel: ChannelId,
) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{
        bundle,
        sector::{Attrs, Item, Tile},
        writer::{Sector, write},
    };
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    let tiles = [2, 3]
        .into_iter()
        .map(|x| Tile {
            x,
            y: 11,
            flags: 0,
            house: 0,
            zones: vec![],
            items: vec![Item {
                palette: 0,
                depth: 0,
                attrs: Attrs::default(),
            }],
        })
        .collect();
    let bytes = write(
        &data.manifest,
        &[Sector {
            floor: -7,
            sx: 0,
            sy: 0,
            tiles,
        }],
        &Default::default(),
    )?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            bundle: pins,
            map_revision: crate::map::overlay::map_revision(&base),
            start: crate::map::overlay::TilePos {
                x: 2,
                y: 11,
                floor: -7,
            },
        },
        world,
        channel,
        |_| None,
    )
    .map_err(|error| debug(error).into())
}

pub(super) async fn verify(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    player: (ExactActorRef, GameSessionId),
    h: &Harness,
) -> TestResult {
    let started = std::time::Instant::now();
    let native = owner
        .active_generation
        .and_then(|active| active.native_gameplay())
        .ok_or("native")?;
    let race = native
        .reward_table()
        .row(CREATURE)
        .map_err(debug)?
        .race
        .clone()
        .ok_or("race")?;
    let character = owner
        .runtime
        .lock()
        .await
        .bound_attacker_lease(player.0, player.1)
        .map_err(debug)?
        .character_id();
    let character = crate::domain::CharacterId::from_bytes(*character.as_bytes()).map_err(debug)?;
    let scrolls_before = scrolls(h).await?;
    let catalysts_before = catalysts(h).await?;
    let receipts_before: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_character_echo_warden_kill_receipts")
            .fetch_one(&h.pool)
            .await?;
    {
        // Earlier reward/expiry cases intentionally leave trickle callbacks pending. Finish
        // those source slots and their individual lifetimes before counting a new family.
        let mut runtime = owner.runtime.lock().await;
        for step in 0..=u64::from(WARDEN_ADDS_MAX) {
            runtime
                .drive_echo_wardens(2_000_000 + step * SPAWN_STEP_MS, |_| true)
                .map_err(debug)?;
        }
        runtime
            .drive_echo_wardens(
                2_000_000 + u64::from(WARDEN_ADDS_MAX) * SPAWN_STEP_MS + CHILD_TTL_MS,
                |_| true,
            )
            .map_err(debug)?;
        assert!(runtime.visible_entities().creatures.is_empty());
    }
    for completed in [false, true] {
        if completed {
            complete_bestiary(owner, player.1, &race).await?;
        }
        let progress = owner
            .root
            .read_bestiary_progress(owner.character, character, race.key())
            .await
            .map_err(debug)?;
        assert_eq!(
            progress.is_some_and(|progress| progress.kill_count >= race.final_kill_threshold()),
            completed
        );
        let mut seen = [false; 3];
        let mut doubled_chance_seen = false;
        for attempt in 0..200_u64 {
            let now_ms = 3_000_000 + u64::from(completed) * 300_000_000 + attempt * 1_000_000;
            let mut walker = owner.clone();
            walker.clock_origin = std::time::Instant::now()
                .checked_sub(Duration::from_millis(now_ms))
                .ok_or("movement clock")?;
            let (away, _) = walker
                .ordinary_paced_step(
                    player.0,
                    player.1,
                    crate::gameplay_transport::StepDirection::East,
                    None,
                    None,
                )
                .await;
            assert_eq!(
                away.moved_to
                    .ok_or("step away from portal")?
                    .actor_position
                    .x,
                POSITION.x + 1
            );
            let (occurrence, plan) = {
                let mut runtime = owner.runtime.lock().await;
                let occurrence = portal(&mut runtime, player, now_ms, 1000 + attempt)?;
                let roll = crate::gameplay_transport::echo_warden::draw(
                    &runtime,
                    occurrence,
                    "ECHO_WARDEN_OUTCOME",
                    1,
                    100,
                )
                .ok_or("outcome draw")?;
                let family = crate::gameplay_transport::echo_warden::draw(
                    &runtime,
                    occurrence,
                    "ECHO_RAID_FAMILY",
                    0,
                    1,
                )
                .ok_or("family draw")?;
                let adds = crate::gameplay_transport::echo_warden::draw(
                    &runtime,
                    occurrence,
                    "ECHO_WARDEN_ADDS",
                    7,
                    12,
                )
                .ok_or("adds draw")?;
                doubled_chance_seen |= completed && (11..=20).contains(&roll);
                (
                    occurrence,
                    EchoRaidPlan::select(
                        u8::try_from(roll)?,
                        completed,
                        family == 1,
                        u8::try_from(adds)?,
                    )
                    .map_err(debug)?,
                )
            };
            // Drive the real committed movement caller, including the Bestiary read.
            walker.clock_origin = std::time::Instant::now()
                .checked_sub(Duration::from_millis(now_ms + 30_000))
                .ok_or("portal clock")?;
            if attempt == 0 {
                clxc_projection::portal_snapshot(&walker, player.0, true).await?;
            }
            let (entry, _) = walker
                .ordinary_paced_step(
                    player.0,
                    player.1,
                    crate::gameplay_transport::StepDirection::West,
                    None,
                    None,
                )
                .await;
            assert_eq!(
                entry.moved_to.ok_or("step onto portal")?.actor_position.x,
                POSITION.x
            );
            if attempt == 0 {
                clxc_projection::portal_snapshot(&walker, player.0, false).await?;
            }
            let mut runtime = owner.runtime.lock().await;
            assert!(
                !runtime
                    .echo_portals(now_ms + 30_000)
                    .any(|portal| portal.occurrence == occurrence)
            );
            for step in 1..=12 {
                runtime
                    .drive_echo_wardens(now_ms + 30_000 + step * 400, |_| true)
                    .map_err(debug)?;
            }
            let children = runtime.visible_entities().creatures;
            assert_eq!(children.len(), usize::from(plan.spawn_count()));
            let mut wardens = 0;
            for child in &children {
                let snapshot = runtime.companion_snapshot(child.actor).map_err(debug)?;
                wardens +=
                    usize::from(runtime.echo_variant(child.actor) == Some(EchoVariant::Warden));
                match plan.outcome() {
                    EchoOutcome::Regular => {
                        assert_eq!(snapshot.forge, None);
                        assert_eq!(runtime.echo_variant(child.actor), None);
                        assert_eq!(snapshot.maximum_health, 20);
                    }
                    EchoOutcome::Influenced => {
                        let kind = snapshot.forge.ok_or("influenced classification")?;
                        assert!((1..=5).contains(&kind.reward_stacks()));
                        assert_eq!(runtime.echo_variant(child.actor), None);
                        assert_eq!(
                            snapshot.maximum_health,
                            (20 * i64::from(kind.health_percent()) + 99) / 100
                        );
                    }
                    EchoOutcome::Warden => assert!(runtime.echo_variant(child.actor).is_some()),
                }
            }
            assert_eq!(wardens, usize::from(plan.outcome() == EchoOutcome::Warden));
            seen[match plan.outcome() {
                EchoOutcome::Regular => 0,
                EchoOutcome::Influenced => 1,
                EchoOutcome::Warden => 2,
            }] = true;
            // Every new family has a lifetime from its actual spawn, and expires without loot.
            runtime
                .drive_echo_wardens(now_ms + 640_000, |_| true)
                .map_err(debug)?;
            assert!(runtime.visible_entities().creatures.is_empty());
            drop(runtime);
            crate::gameplay_transport::echo_warden::step_in(
                owner,
                player.0,
                player.1,
                now_ms + 640_000,
            )
            .await;
            if seen == [true; 3] && (!completed || doubled_chance_seen) {
                break;
            }
        }
        assert_eq!(
            seen, [true; 3],
            "all outcome families must reach production activation"
        );
        assert!(
            !completed || doubled_chance_seen,
            "completed Bestiary must exercise the 11–20 draw interval"
        );
    }
    assert_eq!(scrolls(h).await?, scrolls_before);
    assert_eq!(catalysts(h).await?, catalysts_before);
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_character_echo_warden_kill_receipts"
        )
        .fetch_one(&h.pool)
        .await?,
        receipts_before
    );
    eprintln!(
        "SV16 ECHO_CURRENT POSTGRES_CASE_COMPLETED duration={:?}: all families, completed Bestiary doubling, child expiry, no expiry rewards",
        started.elapsed()
    );
    Ok(())
}

/// Complete the fixture's race through the real writer, one exact revision successor per kill.
async fn complete_bestiary(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
    race: &crate::domain::bestiary::BestiaryRace,
) -> TestResult {
    use crate::durability::bestiary_progress::{
        BestiaryKillOccurrence, BestiaryKillOutcome, BestiaryKillRequest,
    };
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "Bestiary fence")?
        .ok_or("Bestiary session")?;
    let state = owner
        .root
        .read_character_progression(owner.character, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("Bestiary progression")?;
    let before = owner
        .root
        .read_bestiary_progress(owner.character, fence.character_id, race.key())
        .await
        .map_err(debug)?
        .map_or(0, |progress| progress.kill_count);
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    for count in before..race.final_kill_threshold() {
        let mut occurrence = id(230);
        occurrence[12..].copy_from_slice(&count.to_be_bytes());
        // Each semantic write consumes its request, including the immutable race/context.
        let request = BestiaryKillRequest {
            occurrence: BestiaryKillOccurrence::from_bytes(occurrence).map_err(debug)?,
            race: race.clone(),
            context: state.context.clone(),
            policy_revision: state.policy_revision.clone(),
            reward_revision: state.reward_revision.clone(),
        };
        let result = slot
            .commit_bestiary(owner.root, owner.character, owner.holder, fence, request)
            .await
            .map_err(debug)?;
        assert!(matches!(result, BestiaryKillOutcome::Committed(_)));
    }
    Ok(())
}
