//! Source-backed NPC state branches through the existing conversation and QuestState owners.
use super::classic_quests::{install, progress, talk};
use super::fixture::{Harness, debug, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use std::sync::Arc;

const FATHER: &str = "oteryn:quest-progress/quest/u8_6/afathers_burden/";
const ICE: &str = "oteryn:quest-progress/quest/u8_0/the_ice_islands/";
const KNOWLEDGE: &str = "oteryn:quest-progress/quest/u11_02/forgotten_knowledge/";

#[test]
fn npcfix_state_postgres_maris_beregar_clue_once() -> TestResult {
    threaded("maris_mistrock")
}

#[test]
fn npcfix_state_postgres_tereban_accepts_eleven_tracks_once() -> TestResult {
    threaded("tereban")
}
#[test]
fn npcfix_state_postgres_restless_soul_requires_spirit_language() -> TestResult {
    threaded("a_restless_soul")
}
#[test]
fn npcfix_state_postgres_chalice_requires_each_source_topic() -> TestResult {
    threaded("a_strange_chalice")
}

fn threaded(npc: &'static str) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(npc).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "NPC state dialogue thread panicked")??;
    Ok(())
}

fn scenario(npc: &str) -> TestResult {
    let started = std::time::Instant::now();
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "npcfix_state_dialogue", true, false)
            .await?;
        let result: TestResult = async {
            eprintln!("POSTGRES_STARTED NPCFIX state dialogue {npc}");
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let world = crate::foundation::WorldId::decode(&super::fixture::id(42)).map_err(debug)?;
            let activated = matches!(npc, "tereban" | "maris_mistrock").then(||
                crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None));
            let deps = if let Some((_, pin)) = activated.as_ref() {
                let channel_pin = crate::foundation::ChannelContentPin::from_activation(
                    world, pin.activation_sequence(), pin.identity().server_artifact_digest(),
                    pin.identity().client_artifact_digest(), pin.frame_binding().digest(),
                    pin.qualified_room().map_revision_digest(), (2, 0, 7));
                Dependencies::with_content_and_capacity(channel_pin,
                    crate::content::qualify_native_entry_room(world).map_err(debug)?,
                    crate::spell::cast::v1_spell_book().map_err(debug)?, 2)?
            } else { Dependencies::new()? };
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = activated.as_ref().and_then(|(controller, _)| controller.active());
            let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1").map_err(debug)?;
            let catalogue = if npc == "a_restless_soul" {
                let story = loaded.catalogue().transition("oteryn:quest-transition/npcfix/restless-soul/story")
                    .ok_or("production atomic restless-soul transition")?.clone();
                let tracks = story.effects.iter().map(|effect| loaded.catalogue().track(&effect.track)
                    .cloned().ok_or("real story track")).collect::<Result<Vec<_>,_>>()?;
                QuestStateCatalogue::new("content-1", tracks, vec![story.clone(), QuestTransition { world_change: None,
                    key: "oteryn:quest-transition/npcfix/fixture/spirit-language".into(), quest: story.quest,
                    effects: vec![QuestEffect { track: format!("{ICE}questline"), from: QuestComparison::Any, effect: QuestEffectKind::Set(37) }],
                    completes: false, experience: None,
                }]).map_err(debug)?
            } else { loaded.catalogue().clone() };
            owner.quest_catalogue = Some(Arc::new(catalogue));
            deps.initialize(&owner).await;
            owner.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
            let mut admitted = deps.admitted;
            admitted.first_entry = FirstEntryOutcome::Positioned;
            owner.admit_quest_session(&admitted).await;
            let key = format!("oteryn:npc.{npc}");
            let [actor] = install(&owner, &admitted, [key.as_str()]).await?;
            let greeting = talk(&mut owner, &admitted, actor, 100, "hi").await?;
            if npc == "a_restless_soul" {
                assert_eq!(greeting, ["Uhhhh..."]);
                assert!(owner.states.chat.engaged(admitted.game_session_id).is_none());
            }
            if npc == "maris_mistrock" {
                let target = "oteryn:quest-progress/quest/u8_4/the_hidden_city_of_beregar/way_to_beregar";
                assert_eq!(progress(&owner, &admitted, &authority, target).await?, 0);
                talk(&mut owner, &admitted, actor, 101, "mission").await?;
                assert_eq!(progress(&owner, &admitted, &authority, target).await?, 0);
                let clue = talk(&mut owner, &admitted, actor, 102, "mistrock").await?;
                assert_eq!(clue, [
                    "Do you smell this? It's the smell of fire... the fire of a forge. Many people searched this rock here for a hidden path, but they haven't found anything. ...",
                    "I'd search on Fenrock if I were you. Even though there's snow on the surface, it's still warm underground. There are often caves under fresh lava streams."
                ]);
                assert_eq!(progress(&owner, &admitted, &authority, target).await?, 1);
                let fence = owner.current_quest_fence(admitted.game_session_id).await
                    .map_err(|()| "fence")?.ok_or("fence")?;
                let replay = h.root.reconcile_npc_program(&authority, fence.character_id,
                    crate::durability::quest_state::npc_program::NpcProgramRequest {
                        command: CommandRef::new(admitted.game_session_id, CommandId::new(102).map_err(debug)?),
                        npc: key.clone(), trigger: oteryn_gameplay_quests::quest::npc_rules::NpcRuleTrigger::Say,
                        message: "mistrock".into(), topic: 0,
                        transition_key: "oteryn:quest-npc/the_hidden_city_of_beregar_quest/maris_mistrock/hidden-path".into(),
                    }).await.map_err(debug)?.ok_or("canonical Maris clue replay")?;
                assert_eq!(replay.reply, clue);
                let QuestTransitionOutcome::AlreadyCommitted(receipt) = replay.transition else {
                    return Err("canonical Maris replay receipt".into());
                };
                assert_eq!(receipt.changes.len(), 1);
                assert_eq!(talk(&mut owner, &admitted, actor, 103, "mistrock").await?, clue);
                assert_eq!(progress(&owner, &admitted, &authority, target).await?, 1);
                let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key='oteryn:quest.the_hidden_city_of_beregar_quest'")
                    .fetch_one(&h.pool).await?;
                assert_eq!(receipts, 1, "replay and repeat dialogue never duplicate the clue write");
                eprintln!("POSTGRES_CASE_COMPLETED NPCFIX maris_beregar_clue_once success=true duration_seconds={}", started.elapsed().as_secs_f64());
            } else if npc == "tereban" {
                talk(&mut owner, &admitted, actor, 101, "yes").await?;
                assert_eq!(progress(&owner,&admitted,&authority,&format!("{FATHER}status")).await?,0);
                talk(&mut owner, &admitted, actor, 102, "mission").await?;
                talk(&mut owner, &admitted, actor, 103, "no").await?;
                assert_eq!(progress(&owner,&admitted,&authority,&format!("{FATHER}status")).await?,0);
                talk(&mut owner, &admitted, actor, 104, "mission").await?;
                let lines = talk(&mut owner, &admitted, actor, 105, "yes").await?;
                assert!(lines.iter().any(|line| line.contains("I am relieved")));
                let fence = owner.current_quest_fence(admitted.game_session_id).await.map_err(|()| "fence")?.ok_or("fence")?;
                let replay = h.root.reconcile_npc_program(&authority, fence.character_id,
                    crate::durability::quest_state::npc_program::NpcProgramRequest {
                        command: CommandRef::new(admitted.game_session_id, CommandId::new(105).map_err(debug)?),
                        npc: key.clone(), trigger: oteryn_gameplay_quests::quest::npc_rules::NpcRuleTrigger::Say,
                        message: "yes".into(), topic: 1,
                        transition_key: "oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/mission-accept".into(),
                    }).await.map_err(debug)?.ok_or("canonical acceptance replay")?;
                assert_eq!(replay.reply, lines, "replay keeps the canonical response");
                let QuestTransitionOutcome::AlreadyCommitted(receipt) = replay.transition else { return Err("canonical replay receipt".into()); };
                assert_eq!(receipt.changes.len(), 11);
                let count: i64 = sqlx::query_scalar("SELECT cardinality(track_keys)::bigint FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-npc/a_father_s_burden_quest/tereban_functions/mission-accept'")
                    .fetch_one(&h.pool).await?;
                assert_eq!(count,11);
                let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key='oteryn:quest.a_father_s_burden_quest'").fetch_one(&h.pool).await?;
                assert_eq!(receipts,1,"one atomic acceptance, including replay");
                for track in ["status","quest_log","sinew","wood","cloth","silk","crystal","root","iron","scale"] {
                    assert_eq!(progress(&owner,&admitted,&authority,&format!("{FATHER}{track}")).await?,1);
                }
                assert_eq!(progress(&owner,&admitted,&authority,&format!("{FATHER}progress")).await?,0);
            } else {
                let (seed, target) = if npc == "a_restless_soul" {
                    ("oteryn:quest-transition/npcfix/fixture/spirit-language",format!("{ICE}questline"))
                } else {
                    ("oteryn:quest-transition/quest/u11_02/forgotten_knowledge/chalice/npc_1",format!("{KNOWLEDGE}access_lava_teleport"))
                };
                if npc == "a_restless_soul" {
                    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(2)).ok_or("clock")?;
                    let fence = owner.current_quest_fence(admitted.game_session_id).await
                        .map_err(|()| "fence")?.ok_or("fence")?;
                    let refused = owner.npc_talk(admitted.runtime_actor.ok_or("actor")?, UseCommand {
                        game_session_id: admitted.game_session_id, command_id: 101, item_fence: Some(fence.item_fence()),
                    }, NpcTalkIntent { npc_actor: actor, text: "story".into() }).await;
                    assert_eq!(refused.disposition, NpcIntentDisposition::NoConversation);
                    assert!(!refused.unknown);
                } else {
                    talk(&mut owner, &admitted, actor, 101, "learned").await?;
                }
                assert_eq!(progress(&owner,&admitted,&authority,&target).await?,0);
                if npc != "a_restless_soul" { talk(&mut owner, &admitted, actor, 102, "bye").await?; }
                let fence = owner.current_quest_fence(admitted.game_session_id).await.map_err(|()| "fence")?.ok_or("fence")?;
                let outcome = h.root.commit_character_quest_transition(&authority,&h.node,fence,QuestTransitionRequest {
                    transition_key: seed.into(), cause: QuestCause::Command(CommandRef::new(admitted.game_session_id,CommandId::new(103).map_err(debug)?)),
                },Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?)).await.map_err(debug)?;
                assert!(matches!(outcome,QuestTransitionOutcome::Committed(_)));
                owner.refresh_quest_session(admitted.game_session_id,false).await;
                let greeting = talk(&mut owner, &admitted, actor, 104, "hi").await?;
                if npc == "a_restless_soul" {
                    assert_eq!(greeting, ["Ahhhh! At last someone that can listen to my {story}!"]);
                    assert!(owner.states.chat.engaged(admitted.game_session_id).is_some());
                }
                if npc == "a_restless_soul" {
                    talk(&mut owner, &admitted, actor, 105, "story").await?;
                    for (name,value) in [("questline",38),("mission10",2),("mission11",1)] {
                        assert_eq!(progress(&owner,&admitted,&authority,&format!("{ICE}{name}")).await?,value);
                    }
                    let count: i64 = sqlx::query_scalar("SELECT cardinality(track_keys)::bigint FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-transition/npcfix/restless-soul/story'")
                        .fetch_one(&h.pool).await?;
                    assert_eq!(count,3);
                } else {
                    let named = talk(&mut owner,&admitted,actor,105,"name").await?;
                    assert_eq!(named.len(),1);
                    assert!(named[0].contains("UNDERCOVER"));
                    let job = talk(&mut owner,&admitted,actor,106,"job").await?;
                    assert_eq!(job.len(),1);
                    assert!(job[0].contains("research"));
                    for (n,word) in ["chalice","daughter","learned","story","disguised","thirsty"].into_iter().enumerate() {
                        talk(&mut owner,&admitted,actor,110+u64::try_from(n)?,word).await?;
                        assert_eq!(progress(&owner,&admitted,&authority,&target).await?,0);
                    }
                    talk(&mut owner,&admitted,actor,120,"learned").await?;
                    assert_eq!(progress(&owner,&admitted,&authority,&target).await?,1);
                }
            }
            Ok(())
        }.await;
        h.cleanup().await?;
        result
    })
}
