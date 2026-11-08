//! NX-4: committed source voices through the production Channel cadence and bounded CHAT queue.
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::{NpcDataCatalogue, npc_catalogue_preproduction_limits};
use crate::foundation::{MovementFacing, NpcPlacement, NpcPlacementTable, NpcPlacements};
use crate::gameplay_transport::npc_talk::NpcTalkBook;
use crate::gameplay_transport::npc_voice::NpcVoiceCycle;
use oteryn_protocol_oteryn::chat::{ChatLine, ChatSpeechMode};
use std::sync::Arc;

#[test]
fn nx4_source_npc_voice_reaches_chat_once_per_channel_think_on_postgres() -> TestResult {
    source_voice_case("oteryn:npc.captain_bluebear", "Captain Bluebear", "Passages to Carlin, Ab'Dendriel")
}

pub(super) fn source_voice_case(npc_key: &'static str, npc_name: &'static str, prefix: &'static str) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("nx4-npc-voice", move || {
        run(async move |admin| {
            let h = Harness::create_with_admission_seed(admin, "nx4_npc_voice", false, false).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps = Dependencies::new()?;
            let owner = deps.owner(&h, &authority);
            let session = deps.admitted.game_session_id;
            let content = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../content"));
            let catalogue = NpcDataCatalogue::load(content, std::ffi::OsStr::new("world"), npc_catalogue_preproduction_limits()).map_err(debug)?;
            let model = catalogue.service_model_for_project().map_err(debug)?;
            let mut talk = NpcTalkBook::from_model(&model);
            talk.bind_source_voices(&catalogue, &model, |text| crate::chat::ChatText::parse(text).ok().map(|text| text.as_str().to_owned())).map_err(debug)?;
            assert!(talk.source_voices().contains_key(npc_key), "committed Captain voice is source-admitted");
            let source_lines = talk.source_voices().get(npc_key).ok_or("source voice missing")?
                .lines.iter().map(|line| line.text.clone()).collect::<Vec<_>>();
            owner.states.chat.install_npc_talk(Arc::new(talk));
            owner.states.chat.admit(session, "Fixture Hero", owner.owner_now());
            let npc = {
                let mut runtime = deps.runtime.lock().await;
                runtime.initialize_first_entry_position(deps.actor).map_err(debug)?;
                let at = runtime.read_actor_position(deps.actor).map_err(debug)?.position();
                let placed = runtime.place_npcs(&NpcPlacementTable { npcs: vec![NpcPlacements {
                    key: npc_key.into(), placements: vec![NpcPlacement {
                        x: u16::try_from(at.x + 1)?, y: u16::try_from(at.y)?, floor: -i8::try_from(at.floor)?, direction: MovementFacing::South,
                    }],
                }] }, |_| true, true).map_err(debug)?;
                placed.actors[0]
            };
            let mut cycle = {
                let runtime = deps.runtime.lock().await;
                NpcVoiceCycle::new(&runtime, owner.states.chat.npc_talk().ok_or("talk book")?)
            };
            let mut heard = None;
            for tick in 0..300_u64 {
                let now = tick * 1_000_000;
                owner.emit_npc_voices(&mut cycle, now).await;
                owner.emit_npc_voices(&mut cycle, now).await;
                if tick < 14 {
                    assert!(owner.states.chat.next(session).is_none(), "repeated owner calls cannot advance the 15-second cadence");
                    continue;
                }
                if let Some(line) = owner.states.chat.next(session) { heard = Some(line); break; }
            }
            let Some(ChatLine::Local { speaker, speaker_name, mode, text, .. }) = heard else { return Err("source voice never reached CHAT".into()); };
            assert_eq!(speaker.identity, npc.placement_identity());
            assert_eq!(speaker_name, npc_name);
            assert_eq!(mode, ChatSpeechMode::Say);
            assert!(text.starts_with(prefix));
            assert!(source_lines.contains(&text));
            assert!(owner.states.chat.next(session).is_none());
            owner.emit_npc_voices(&mut cycle, 1_000_000_000).await;
            assert!(owner.states.chat.next(session).is_none(), "late thinks coalesce instead of replaying missed voices");
            deps.runtime.lock().await.remove_dynamic_npc(npc_key).map_err(debug)?;
            for tick in 1001..1032_u64 {
                owner.emit_npc_voices(&mut cycle, tick * 1_000_000).await;
            }
            assert!(owner.states.chat.next(session).is_none(), "removed NPC generations cannot speak");
            // A forgotten session cannot receive automatic NPC speech.
            owner.states.chat.forget(session);
            owner.emit_npc_voices(&mut cycle, 301_000_000).await;
            assert!(owner.states.chat.next(session).is_none());
            Ok(())
        })
    })
}
