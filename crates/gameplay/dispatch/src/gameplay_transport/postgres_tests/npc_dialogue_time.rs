//! The production NPC talk and local speech routes render the authored time reply.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::world_clock::{DEFAULT_WORLD_CLOCK_EPOCH_UTC_MS, TIBIAN_MINUTE_MS, WorldClockSource};
use oteryn_protocol_oteryn::chat::{ChatDisposition, ChatIntent, ChatLine, ChatSpeechMode};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;

const FRODO: &str = "oteryn:npc.frodo";

#[test]
fn npc_dialogue_renders_world_time_in_targeted_talk() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_dialogue_case(admin, true)
}

#[test]
fn npc_dialogue_renders_world_time_in_local_say() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_dialogue_case(admin, false)
}

fn run_dialogue_case(admin: String, targeted: bool) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || run(async |_| scenario(admin, targeted).await).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "NPC dialogue time scenario panicked")??;
    Ok(())
}

async fn scenario(admin: String, targeted: bool) -> TestResult {
    let tag = if targeted {
        "npc_time_talk"
    } else {
        "npc_time_say"
    };
    let harness = Harness::create_with_admission_seed(admin, tag, true, false).await?;
    prepare_current_session(&harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let deps = Dependencies::new()?;
    let owner = deps.owner(&harness, &authority);
    let session = deps.admitted.game_session_id;
    let catalogue = NpcDataCatalogue::load(
        std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content")),
        std::ffi::OsStr::new("world"),
        npc_catalogue_preproduction_limits(),
    )
    .map_err(debug)?;
    let book = NpcTalkBook::from_model(&catalogue.service_model_for_project().map_err(debug)?);
    owner
        .states
        .chat
        .admit(session, "Fixture Hero", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    let npc = {
        let mut runtime = owner.runtime.lock().await;
        runtime
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let spot = runtime
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position();
        runtime
            .place_npcs(
                &NpcPlacementTable {
                    npcs: vec![NpcPlacements {
                        key: FRODO.into(),
                        placements: vec![NpcPlacement {
                            x: u16::try_from(spot.x + 1)?,
                            y: u16::try_from(spot.y)?,
                            floor: -i8::try_from(spot.floor)?,
                            direction: MovementFacing::South,
                        }],
                    }],
                },
                |key| book.display_name(key).is_some(),
                true,
            )
            .map_err(debug)?;
        let visible = runtime.visible_entities();
        let placed = visible.npcs.first().ok_or("Frodo was not placed")?;
        EntityRef {
            identity: placed.actor.placement_identity(),
            generation: placed.generation,
        }
    };
    owner
        .states
        .chat
        .install_npc_talk(std::sync::Arc::new(book));

    let talk = async |command_id, text: &str| {
        owner
            .npc_talk(
                deps.actor,
                UseCommand {
                    game_session_id: session,
                    command_id,
                    item_fence: None,
                },
                NpcTalkIntent {
                    npc_actor: npc,
                    text: text.into(),
                },
            )
            .await
    };
    let say = async |command_id, text: &str| {
        owner
            .chat_intent(
                deps.actor,
                session,
                command_id,
                ChatIntent::Say {
                    mode: ChatSpeechMode::Say,
                    text: text.into(),
                },
            )
            .await
    };
    if targeted {
        let greeting = talk(1, "hi").await;
        assert_eq!(greeting.disposition, NpcIntentDisposition::Ok);
        assert!(greeting.conversation.and_then(|state| state.open).is_some());
    } else {
        assert_eq!(say(1, "hi").await.disposition, ChatDisposition::Ok);
        assert!(owner.states.chat.engaged(session).is_some());
        owner.states.chat.drain(session);
    }

    let started = owner
        .world_clock_utc_ms()
        .ok_or("World clock before time keyword")?;
    let lines = if targeted {
        let outcome = talk(2, "time").await;
        assert_eq!(outcome.disposition, NpcIntentDisposition::Ok);
        outcome
            .conversation
            .map_or_else(Vec::new, |state| state.lines)
    } else {
        assert_eq!(say(2, "time").await.disposition, ChatDisposition::Ok);
        owner
            .states
            .chat
            .drain(session)
            .into_iter()
            .filter_map(|line| match line {
                ChatLine::Local {
                    speaker_name, text, ..
                } if speaker_name == "Frodo" => Some(text),
                _ => None,
            })
            .collect()
    };
    let finished = owner
        .world_clock_utc_ms()
        .ok_or("World clock after time keyword")?;
    let clock = WorldClockSource::new(DEFAULT_WORLD_CLOCK_EPOCH_UTC_MS);
    let first = clock
        .absolute_elapsed_ms(started)
        .ok_or("World clock start")?
        / TIBIAN_MINUTE_MS;
    let last = clock
        .absolute_elapsed_ms(finished)
        .ok_or("World clock finish")?
        / TIBIAN_MINUTE_MS;
    assert_eq!(lines.len(), 1, "Frodo's authored time reply: {lines:?}");
    assert!(
        (first..=last).any(|tick| {
            let instant = DEFAULT_WORLD_CLOCK_EPOCH_UTC_MS + tick * TIBIAN_MINUTE_MS;
            clock.tibian_minute(instant).is_some_and(|minute| {
                lines[0] == format!("It is exactly {:02}:{:02}.", minute / 60, minute % 60)
            })
        }),
        "Frodo should answer with the current World clock: {lines:?}"
    );

    drop(owner);
    drop(authority);
    drop(seal);
    harness.cleanup().await?;
    eprintln!("POSTGRES_CASE_COMPLETED {tag}");
    Ok(())
}
