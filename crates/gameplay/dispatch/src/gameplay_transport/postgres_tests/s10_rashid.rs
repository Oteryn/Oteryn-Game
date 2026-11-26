//! S10: real framed shop commands and durable payment after Rashid wanders one tile.
use super::TestResult;
use oteryn_gameplay_entities::rashid_schedule::destination_at;

const DAY_MS: i64 = 86_400_000;
const CLOCK_FILE: &str = "OTERYN_QUALIFICATION_CLOCK_FILE";

// The qualification offset is process-wide and never rewinds. Run both clock modes in fresh
// processes so this case cannot advance any other PostgreSQL scenario's owner clock.
fn with_world_clocks(name: &str, scenario: impl FnOnce() -> TestResult) -> TestResult {
    const PROBE: &str = "OTERYN_W05CLOCK_DISPATCH_PROBE";
    if std::env::var_os(PROBE).is_some() {
        #[cfg(feature = "qualification-clock")]
        if std::env::var_os(CLOCK_FILE).is_some() {
            let wall = crate::gameplay_transport::npc_trade_command::unix_ms_now();
            let world = i64::try_from(crate::world_clock::now_utc_ms().ok_or("World clock")?)?;
            assert_ne!(destination_at(world), destination_at(wall));
        }
        return scenario();
    }

    let (_, module) = module_path!().split_once("::").ok_or("test module")?;
    let case = format!("{module}::{name}");
    let mut child = std::process::Command::new(std::env::current_exe()?);
    child
        .args(["--exact", &case, "--nocapture"])
        .env(PROBE, "1");
    let status = child.env_remove(CLOCK_FILE).status()?;
    assert!(status.success(), "wall-clock dispatch: {status}");
    #[cfg(feature = "qualification-clock")]
    {
        use std::io::Write;
        use std::sync::atomic::{AtomicU64, Ordering};

        let now = crate::gameplay_transport::npc_trade_command::unix_ms_now();
        // Keep the weekdays distinct even when the wall clock crosses midnight during setup.
        let offset = 2 * DAY_MS + DAY_MS / 2 - now.rem_euclid(DAY_MS);
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let path = std::env::temp_dir().join(format!(
            "oteryn-w05clock-dispatch-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed),
        ));
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)?;
        let written = file.write_all(offset.to_string().as_bytes());
        drop(file);
        let status = written.and_then(|()| child.env(CLOCK_FILE, &path).status());
        let cleanup = std::fs::remove_file(path);
        let status = status?;
        cleanup?;
        assert!(status.success(), "qualification-clock dispatch: {status}");
    }
    Ok(())
}

#[test]
fn s10_rashid_wandering_postgres_shop_payment_and_replay() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    with_world_clocks(
        "s10_rashid_wandering_postgres_shop_payment_and_replay",
        shop_payment_and_replay,
    )
}

fn shop_payment_and_replay() -> TestResult {
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            super::npc_shop_fixtures::run_shop_scenario(true).map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "S10 Rashid PostgreSQL thread panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED S10 rashid_wandering_shop duration={:?}",
        started.elapsed(),
    );
    Ok(())
}

#[test]
fn s10_weekday_rollover_moves_live_rashid_on_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    with_world_clocks(
        "s10_weekday_rollover_moves_live_rashid_on_postgres",
        weekday_rollover,
    )
}

fn weekday_rollover() -> TestResult {
    use super::fixture::{Harness, debug, run};
    use super::{Dependencies, prepare_current_session};
    use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
    use crate::foundation::{MovementLocalPosition, NpcPlacementTable, NpcPlacements};
    use crate::gameplay_transport::npc_actor::{NpcWalkBook, NpcWalkState};
    use crate::gameplay_transport::npc_talk::{NpcTalkBook, NpcTalkEntry};
    use crate::gameplay_transport::npc_voice::NpcVoiceCycle;
    use oteryn_gameplay_entities::rashid_schedule::RASHID_NPC_KEY;
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| -> Result<(), String> {
            run(async |admin| {
                let h =
                    Harness::create_with_admission_seed(admin, "s10_rashid_rollover", false, false)
                        .await?;
                prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let deps = Dependencies::new()?;
                let owner = deps.owner(&h, &authority);
                deps.initialize(&owner).await;
                let session = deps.admitted.game_session_id;
                owner.states.chat.install_npc_talk(std::sync::Arc::new(
                    NpcTalkBook::default().with(RASHID_NPC_KEY, NpcTalkEntry::new("Rashid", None)),
                ));
                owner
                    .states
                    .chat
                    .admit(session, "Fixture Hero", owner.owner_now());
                owner.states.chat.set_npc_service(session);
                let catalogue = NpcDataCatalogue::load(
                    std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
                    std::ffi::OsStr::new("world"),
                    npc_catalogue_preproduction_limits(),
                )
                .map_err(debug)?;
                let model = catalogue.service_model_for_project().map_err(debug)?;
                let walks = NpcWalkBook::from_catalogue(&catalogue, &model).map_err(debug)?;
                let wall = crate::gameplay_transport::npc_trade_command::unix_ms_now();
                let now = i64::try_from(crate::world_clock::now_utc_ms().ok_or("World clock")?)?;
                let yesterday = destination_at(wall - DAY_MS).placement;
                let expected = destination_at(now).placement;
                let mut cycle = {
                    let mut runtime = deps.runtime.lock().await;
                    runtime
                        .initialize_pinned_test_position(
                            deps.actor,
                            MovementLocalPosition {
                                x: i32::from(yesterday.x) + 1,
                                y: i32::from(yesterday.y),
                                floor: -i16::from(yesterday.floor),
                            },
                        )
                        .map_err(debug)?;
                    runtime
                        .place_npcs(
                            &NpcPlacementTable {
                                npcs: vec![NpcPlacements {
                                    key: RASHID_NPC_KEY.into(),
                                    placements: vec![yesterday],
                                }],
                            },
                            |_| true,
                            true,
                        )
                        .map_err(debug)?;
                    NpcVoiceCycle::new(&runtime, &NpcTalkBook::default())
                };
                let npc = owner.runtime.lock().await.visible_entities().npcs[0];
                super::npc_paid_travel_fixtures::talk(
                    &owner,
                    &deps,
                    oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                        identity: npc.actor.placement_identity(),
                        generation: npc.generation,
                    },
                    1,
                    "hi",
                )
                .await?;
                assert_eq!(owner.states.chat.engaged(session), Some(npc.actor));
                owner.emit_npc_voices(&mut cycle, 1_000_000).await;
                assert_eq!(owner.states.chat.engaged(session), None);
                assert!(!owner.states.chat.take_walked_away(session).is_empty());
                {
                    let mut runtime = deps.runtime.lock().await;
                    let actual = runtime.visible_entities().npcs[0].position;
                    assert_eq!(
                        (actual.x, actual.y, actual.floor),
                        (
                            i32::from(expected.x),
                            i32::from(expected.y),
                            -i16::from(expected.floor)
                        ),
                        "the live voice cadence relocates Rashid to the World weekday",
                    );
                    assert!(
                        NpcWalkState::after_schedule(&mut runtime, npc.actor, &walks)
                            .map_err(debug)?
                            .is_some(),
                        "the walk owner accepts the relocated Rashid's weekday anchor",
                    );
                }
                drop(owner);
                drop(authority);
                drop(seal);
                h.cleanup().await
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "S10 weekday PostgreSQL thread panicked")??;
    eprintln!(
        "POSTGRES_CASE_COMPLETED S10 weekday_rollover duration={:?}",
        started.elapsed()
    );
    Ok(())
}
