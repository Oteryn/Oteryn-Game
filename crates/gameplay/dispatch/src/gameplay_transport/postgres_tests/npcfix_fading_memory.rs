//! Owned NPC exchanges use the real dialogue, current lease and durable item owners.
use super::classic_quests::{install, talk};
use super::fixture::{Harness, debug, run};
use super::item_look::seed_main_backpack;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::durability::quest_state::{QuestCause, QuestTransitionRequest};
use crate::foundation::{ChannelContentPin, CommandId, CommandRef};
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use std::sync::Arc;

#[test]
fn npcfix_reward_postgres_fading_memory_story_and_database_cooldown() -> TestResult {
    threaded(
        "a_fading_memory",
        "oteryn:quest-reward/npcfix-ghost-tear",
        None,
        8746,
    )
}

fn threaded(npc: &'static str, key: &'static str, input: Option<u16>, output: u16) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(npc, key, input, output).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "NPC reward thread panicked")??;
    Ok(())
}

fn scenario(npc: &str, key: &str, input: Option<u16>, output: u16) -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "npcfix_reward", true, false).await?;
        let result: TestResult = async {
            eprintln!("POSTGRES_STARTED NPCFIX reward {npc}");
            let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1").map_err(debug)?;
            let actual = loaded.catalogue().transition(key).ok_or("source-backed production reward callback missing")?.clone();
            let tracks = actual.effects.iter().map(|effect| loaded.catalogue().track(&effect.track)
                .cloned().ok_or("production reward track")).collect::<Result<Vec<_>,_>>()?;
            let setup = "oteryn:quest-transition/npcfix-fixture/reward-ready";
            let lapse = "oteryn:quest-transition/npcfix-fixture/cooldown-elapsed";
            let mut transitions = vec![actual.clone(), QuestTransition {
                key: setup.into(), quest: actual.quest.clone(), completes: false, experience: None,
                effects: tracks.iter().map(|track| QuestEffect { track: track.key.clone(),
                    from: QuestComparison::Any, effect: QuestEffectKind::Set(if track.key.ends_with("/mission05") { 2 } else { track.initial }) }).collect(),
            }];
            if npc == "a_fading_memory" {
                let clock = tracks.iter().find(|track| track.key.ends_with("/ghost_tear_cooldown")).ok_or("source cooldown track")?;
                transitions.push(QuestTransition { key: lapse.into(), quest: actual.quest.clone(),
                    effects: vec![QuestEffect { track: clock.key.clone(), from: QuestComparison::Ge(72_000), effect: QuestEffectKind::Add(-72_000) }],
                    completes: false, experience: None });
            }
            let catalogue = Arc::new(QuestStateCatalogue::new("content-1", tracks, transitions).map_err(debug)?);
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let world = crate::foundation::WorldId::decode(&super::fixture::id(42)).map_err(debug)?;
            let (controller, pin) = activated_with_loot_tables(world, None);
            let active = controller.active().ok_or("active content")?;
            let channel_pin = ChannelContentPin::from_activation(world, pin.activation_sequence(),
                pin.identity().server_artifact_digest(), pin.identity().client_artifact_digest(),
                pin.frame_binding().digest(), pin.qualified_room().map_revision_digest(), (2,0,7));
            let deps = Dependencies::with_content_and_capacity(channel_pin,
                crate::content::qualify_native_entry_room(world).map_err(debug)?,
                crate::spell::cast::v1_spell_book().map_err(debug)?, 2)?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(active);
            owner.quest_catalogue = Some(Arc::clone(&catalogue));
            deps.initialize(&owner).await;
            owner.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
            let mut admitted = deps.admitted;
            admitted.first_entry = FirstEntryOutcome::Positioned;
            let commit = async |transition: &str, ordinal: u64| -> TestResult {
                let fence = owner.current_quest_fence(admitted.game_session_id).await.map_err(|()| "fence")?.ok_or("fence")?;
                let result = h.root.commit_character_quest_transition(&authority, &h.node, fence,
                    QuestTransitionRequest { transition_key: transition.into(), cause: QuestCause::Command(CommandRef::new(admitted.game_session_id, CommandId::new(ordinal).map_err(debug)?)) },
                    Arc::clone(&catalogue)).await.map_err(debug)?;
                assert!(matches!(result, crate::durability::quest_state::QuestTransitionOutcome::Committed(_)));
                Ok(())
            };
            commit(setup, 1).await?;
            if let Some(item) = input { let item_key = format!("oteryn:item.tibia.i{item}");
                seed_main_backpack(&h, &[(item_key.as_str(), 1), (item_key.as_str(), 1)]).await?; }
            else { seed_main_backpack(&h, &[]).await?; }
            owner.admit_quest_session(&admitted).await;
            let [actor] = install(&owner, &admitted, [format!("oteryn:npc.{npc}").as_str()]).await?;
            talk(&mut owner, &admitted, actor, 2, "hi").await?;
            let wrong = match npc { "al_dee" => "yes please", "a_wandering_soul" => "blood crystals", _ => "awaken" };
            talk(&mut owner, &admitted, actor, 3, wrong).await?;
            assert_eq!(count(&h, output).await?, 0, "non-matching input grants nothing");
            if let Some(item) = input { assert_eq!(count(&h, item).await?, 2, "non-match never charges"); }
            let words: &[&str] = match npc {
                "al_dee" => &["hi", "pick", "yes"],
                "a_wandering_soul" => &["hi", "blood crystal"],
                _ => &["hi", "kala", "diary", "yes", "vampire", "free soul", "awaken"],
            };
            let mut ordinal = 10;
            for word in words {
                if *word == "awaken" {
                    owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(2)).ok_or("clock")?;
                    let fence = owner.current_quest_fence(admitted.game_session_id).await.map_err(|()| "fence")?.ok_or("fence")?;
                    let _near_miss = owner.npc_talk(deps.actor, UseCommand { game_session_id: admitted.game_session_id, command_id: ordinal, item_fence: Some(fence.item_fence()) }, NpcTalkIntent { npc_actor: actor, text: "awaken please".into() }).await;
                    assert_eq!(count(&h, output).await?, 0, "a ready topic still requires the exact grant keyword");
                    ordinal += 1;
                }
                talk(&mut owner, &admitted, actor, ordinal, word).await?;
                ordinal += 1;
            }
            assert_eq!(count(&h, output).await?, 1, "one source reward in player inventory");
            if let Some(item) = input {
                assert_eq!(count(&h, item).await?, 1, "exactly one input consumed");
                let repeated = if npc == "al_dee" { &["pick", "yes"][..] } else { &["blood crystal"][..] };
                for (offset, word) in repeated.iter().enumerate() { talk(&mut owner, &admitted, actor, 30 + u64::try_from(offset)?, word).await?; }
                assert_eq!(count(&h, output).await?, 2, "fresh command earns a second costed reward");
                assert_eq!(count(&h, item).await?, 0);
            } else {
                owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(100_000)).ok_or("clock")?;
                for (offset, word) in words.iter().enumerate().take(words.len()-1) { talk(&mut owner, &admitted, actor, 30 + u64::try_from(offset)?, word).await?; }
                let fence = owner.current_quest_fence(admitted.game_session_id).await.map_err(|()| "fence")?.ok_or("fence")?;
                owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(2)).ok_or("clock")?;
                let refused = owner.npc_talk(deps.actor, UseCommand { game_session_id: admitted.game_session_id, command_id: 40, item_fence: Some(fence.item_fence()) }, NpcTalkIntent { npc_actor: actor, text: "awaken".into() }).await;
                assert_eq!(refused.disposition, NpcIntentDisposition::Rejected, "twenty real database hours must elapse");
                assert_eq!(count(&h, output).await?, 1);
                // The typed fixture transition advances only the source clock fact.
                let fence = owner.current_quest_fence(admitted.game_session_id).await.map_err(|()| "fence")?.ok_or("fence")?;
                let advanced = h.root.commit_character_quest_transition(&authority, &h.node, fence, QuestTransitionRequest {
                    transition_key: lapse.into(), cause: QuestCause::Command(CommandRef::new(admitted.game_session_id, CommandId::new(41).map_err(debug)?)) }, Arc::clone(&catalogue)).await.map_err(debug)?;
                assert!(matches!(advanced, crate::durability::quest_state::QuestTransitionOutcome::Committed(_)));
                owner.refresh_quest_session(admitted.game_session_id, false).await;
                for (offset, word) in words.iter().enumerate() { talk(&mut owner, &admitted, actor, 50 + u64::try_from(offset)?, word).await?; }
                assert_eq!(count(&h, output).await?, 2, "elapsed source clock permits a second legitimate reward");
                let fence = owner.current_quest_fence(admitted.game_session_id).await.map_err(|()| "fence")?.ok_or("fence")?;
                let again = h.root.commit_character_quest_transition(&authority, &h.node, fence, QuestTransitionRequest {
                    transition_key: lapse.into(), cause: QuestCause::Command(CommandRef::new(admitted.game_session_id, CommandId::new(70).map_err(debug)?)) }, Arc::clone(&catalogue)).await.map_err(debug)?;
                assert!(matches!(again, crate::durability::quest_state::QuestTransitionOutcome::Committed(_)));
                owner.refresh_quest_session(admitted.game_session_id, false).await;
                let clock_track = actual.effects.iter().find(|effect| effect.effect == QuestEffectKind::SetNow).ok_or("clock effect")?.track.as_str();
                let stamp_before: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(clock_track).fetch_one(&h.pool).await?;
                sqlx::raw_sql("CREATE FUNCTION npcfix_bad_reward_clock() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.plan=jsonb_set(NEW.plan,'{cooldown,seconds}','0'::jsonb); RETURN NEW; END $$; CREATE TRIGGER npcfix_bad_reward_clock BEFORE INSERT ON game_quest_reward_sources FOR EACH ROW EXECUTE FUNCTION npcfix_bad_reward_clock()")
                    .execute(&h.pool).await?;
                for (offset, word) in words.iter().enumerate().take(words.len()-1) { talk(&mut owner, &admitted, actor, 80 + u64::try_from(offset)?, word).await?; }
                owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(2)).ok_or("clock")?;
                let current = owner.current_quest_fence(admitted.game_session_id).await.map_err(|()| "fence")?.ok_or("fence")?;
                let forged = owner.npc_talk(deps.actor, UseCommand { game_session_id: admitted.game_session_id, command_id: 86, item_fence: Some(current.item_fence()) }, NpcTalkIntent { npc_actor: actor, text: "awaken".into() }).await;
                assert_eq!(forged.disposition, NpcIntentDisposition::Rejected, "the database must reject an unproved free repeat even after valid elapsed storage");
                assert_eq!(count(&h, output).await?, 2, "deferred proof refusal rolls back the minted item");
                let stamp_after: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(clock_track).fetch_one(&h.pool).await?;
                assert_eq!(stamp_after, stamp_before, "deferred refusal rolls back the source clock");
                sqlx::raw_sql("DROP TRIGGER npcfix_bad_reward_clock ON game_quest_reward_sources; DROP FUNCTION npcfix_bad_reward_clock()")
                    .execute(&h.pool).await?;
                assert_eq!(count(&h, output).await?, 2);
            }
            if npc == "a_wandering_soul" {
                let earned: i64 = sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/ghostwhisperer'").fetch_one(&h.pool).await?;
                assert_eq!(earned, 1, "source achievement is idempotent across paid exchanges");
            }
            Ok(())
        }.await;
        h.cleanup().await?;
        result
    })
}

async fn count(h: &Harness, item: u16) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.lifecycle=1 AND i.definition_production_key=$1")
        .bind(format!("oteryn:item.tibia.i{item}")).fetch_one(&h.pool).await?)
}
