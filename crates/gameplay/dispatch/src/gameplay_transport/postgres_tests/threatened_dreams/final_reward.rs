//! Prior story milestones are fixtures; physical hand-ins and rewards use the real NPC owner.
use super::super::fixture::{Harness, debug, id, run};
use super::super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTransition,
};
use oteryn_protocol_oteryn::npc_service::NpcTalkIntent;
use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
use std::collections::BTreeMap;
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.threatened_dreams_quest";
const COUPLE:&str="oteryn:quest-progress/quest/u11_40/threatened_dreams/mission03_1";
const JOURNAL:&str="oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission03/unlikely_couple";
const NIGHTMARE:&str="oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/nightmare_intruders";
const CHARGES:[&str;3]=[
"oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/charged_moon_mirror",
"oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/charged_starlight_vial",
"oteryn:quest-progress/crystalserver/quest/u11_40/threatened_dreams/mission02/charged_sun_catcher"];
const VESSELS:[&str;3]=["oteryn:item.tibia.i25734","oteryn:item.tibia.i25732","oteryn:item.tibia.i25730"];
const FLOWER:&str="oteryn:item.tibia.i25782";
const HERB:&str="oteryn:item.tibia.i5953";
const BAG:&str="oteryn:item.tibia.i25780";
const SEED:&str="oteryn:quest-transition/fixture/threatened-final-prior-story";
const READY:&str="oteryn:quest-transition/fixture/threatened-final-prior-repairs";
#[test]
fn threatened_final_four_physical_contracts_are_atomic() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("THREATENED_FINAL POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20).spawn(|| {
        let started=std::time::Instant::now();
        match std::panic::catch_unwind(scenario) {
            Ok(result) => {
                eprintln!("THREATENED_FINAL POSTGRES_CASE_COMPLETED duration_ms={} success={}", started.elapsed().as_millis(), result.is_ok());
                result.map_err(|error|error.to_string())
            }
            Err(panic) => {
                eprintln!("THREATENED_FINAL POSTGRES_CASE_COMPLETED duration_ms={} success=false assertion_panicked=true",started.elapsed().as_millis());
                std::panic::resume_unwind(panic)
            }
        }
    })?.join().map_err(|_|"inventory scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "threatened_final", false, false)
            .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let (controller, pin) =
            super::super::death_ml_loss_composed::production_generation_with_pin(
                crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let room = pin.qualified_room().clone();
        let deps = Dependencies::with_content_and_capacity(
            pin.into_channel_pin(),
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
            8,
        )?;
        deps.runtime
            .lock()
            .await
            .initialize_pinned_test_position(
                deps.actor,
                crate::foundation::MovementLocalPosition {
                    x: 7,
                    y: 0,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        let source = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?;
        let mut tracks = BTreeMap::new();
        for key in [COUPLE,JOURNAL,NIGHTMARE,CHARGES[0],CHARGES[1],CHARGES[2]] {
            let track = source.catalogue().track(key).ok_or("source track")?.clone();
            tracks.insert(key.to_owned(), track);
        }
        let mut transitions = Vec::new();
        for key in [
            "oteryn:quest-reward/questpc3/threatened-final/taegen-flower",
            "oteryn:quest-reward/questpc3/threatened-final/taegen-herb",
            "oteryn:quest-reward/questpc3/threatened-final/maelyrra-held-lights",
            "oteryn:quest-reward/questpc3/threatened-final/maelyrra-blossom-bag",
        ] {
            if let Some(transition) = source.catalogue().transition(key) {
                for effect in &transition.effects {
                    tracks.entry(effect.track.clone()).or_insert(
                        source
                            .catalogue()
                            .track(&effect.track)
                            .ok_or("source effect track")?
                            .clone(),
                    );
                }
                transitions.push(transition.clone());
            }
        }
        // Only the required preceding story is a fixture. No reward, exchange
        // flag, item consumption or successful receipt is supplied here.
        transitions.push(QuestTransition {
world_change: None,
            key: SEED.into(),
            quest: QUEST.into(),
            effects: [(COUPLE,2),(JOURNAL,2),(NIGHTMARE,6),(CHARGES[0],5),(CHARGES[1],5),(CHARGES[2],5)]
                .into_iter()
                .map(|(track, value)| QuestEffect {
                    track: track.into(),
                    from: QuestComparison::Any,
                    effect: QuestEffectKind::Set(value),
                })
                .collect(),
            completes: false,
            experience: None,
        });
        // Isolated final-reward precondition only: actual fifteen-source world
        // repairs are a separate mechanism. This never grants a reward or NM8.
        transitions.push(QuestTransition {
world_change: None,key:READY.into(),quest:QUEST.into(),
            effects:CHARGES.into_iter().map(|track|QuestEffect {track:track.into(),
                from:QuestComparison::Between(5,5),effect:QuestEffectKind::Set(0)}).collect(),
            completes:false,experience:None});
        owner.quest_catalogue = Some(Arc::new(
            QuestStateCatalogue::new("content-1", tracks.into_values().collect(), transitions)
                .map_err(debug)?,
        ));
        deps.initialize(&owner).await;
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;
        super::super::item_look::seed_main_backpack(
            &h,
            &[
                (VESSELS[0],1),(VESSELS[1],1),
            ],
        )
        .await?;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "fence")?
            .ok_or("fence")?;
        let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
        let seeded = slot
            .commit_quest_transition(
                owner.root,
                owner.character,
                owner.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: SEED.into(),
                    cause: QuestCause::Command(CommandRef::new(
                        admitted.game_session_id,
                        CommandId::new(24000).map_err(debug)?,
                    )),
                },
                Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?),
            )
            .await
            .map_err(debug)?;
        assert!(matches!(seeded, QuestTransitionOutcome::Committed(_)));
        drop(slot);
        owner
            .refresh_quest_session(admitted.game_session_id, false)
            .await;
        let [taegen,maelyrra]=super::super::classic_quests::install(&owner,&admitted,
            ["oteryn:npc.taegen","oteryn:npc.maelyrra"]).await?;
        say(&mut owner,&admitted,taegen,26001,"hi").await?;
        say(&mut owner,&admitted,taegen,26002,"mission").await?;
        assert_eq!(super::super::classic_quests::progress(&owner,&admitted,&authority,COUPLE).await?,2);
        append_fixture_input(&h,FLOWER,90,3).await?;
        say(&mut owner,&admitted,taegen,26003,"mission please").await?;
        assert_eq!(quantity(&h,FLOWER).await?,1,"charge only exact authored words");
        say(&mut owner,&admitted,taegen,26004,"mission").await?;
        let flower=quantity(&h,FLOWER).await?==0
            && super::super::classic_quests::progress(&owner,&admitted,&authority,COUPLE).await?==3
            && super::super::classic_quests::progress(&owner,&admitted,&authority,JOURNAL).await?==4;
        say(&mut owner,&admitted,taegen,26005,"sun catcher").await?;
        say(&mut owner,&admitted,taegen,26006,"yes").await?;
        assert_eq!(quantity(&h,HERB).await?,0);
        append_fixture_input(&h,HERB,92,4).await?;
        say(&mut owner,&admitted,taegen,26007,"sun catcher").await?;
        say(&mut owner,&admitted,taegen,26008,"yes please").await?;
        assert_eq!(quantity(&h,HERB).await?,1);
        say(&mut owner,&admitted,taegen,26009,"sun catcher").await?;
        say(&mut owner,&admitted,taegen,26010,"yes").await?;
        let herb=quantity(&h,HERB).await?==0
            && super::super::classic_quests::progress(&owner,&admitted,&authority,COUPLE).await?==4;
        say(&mut owner,&admitted,taegen,26011,"bye").await?;
        say(&mut owner,&admitted,maelyrra,26012,"hi").await?;
        say(&mut owner,&admitted,maelyrra,26013,"mission").await?;
        say(&mut owner,&admitted,maelyrra,26014,"yes").await?;
        assert_eq!(super::super::classic_quests::progress(&owner,&admitted,&authority,NIGHTMARE).await?,6);
        for key in &VESSELS[..2] {assert_eq!(quantity(&h,key).await?,1,"carried evidence cannot be consumed");}
        append_fixture_input(&h,VESSELS[2],94,5).await?;
        say(&mut owner,&admitted,maelyrra,26015,"mission").await?;
        say(&mut owner,&admitted,maelyrra,26016,"yes please").await?;
        assert_eq!(super::super::classic_quests::progress(&owner,&admitted,&authority,NIGHTMARE).await?,6);
        say(&mut owner,&admitted,maelyrra,26017,"mission").await?;
        say(&mut owner,&admitted,maelyrra,26018,"yes").await?;
        let carried=super::super::classic_quests::progress(&owner,&admitted,&authority,NIGHTMARE).await?==7;
        for key in VESSELS {assert_eq!(quantity(&h,key).await?,1,"Maelyrra reads, never takes, charged vessels");}
        say(&mut owner,&admitted,maelyrra,26019,"mission").await?;
        say(&mut owner,&admitted,maelyrra,26020,"yes").await?;
        assert_eq!(quantity(&h,BAG).await?,0,"all three remaining charge counters must be exactly zero");
        let fence=owner.current_quest_fence(admitted.game_session_id).await.map_err(|()|"fence")?.ok_or("fence")?;
        let mut slot=owner.revision_sequencer.acquire(fence.character_id).await;
        let repaired=slot.commit_quest_transition(owner.root,owner.character,owner.holder,fence,
            QuestTransitionRequest {transition_key:READY.into(),cause:QuestCause::Command(CommandRef::new(admitted.game_session_id,CommandId::new(26021).map_err(debug)?))},
            Arc::clone(owner.quest_catalogue.as_ref().ok_or("catalogue")?)).await.map_err(debug)?;
        assert!(matches!(repaired,QuestTransitionOutcome::Committed(_)));
        drop(slot);owner.refresh_quest_session(admitted.game_session_id,false).await;
        say(&mut owner,&admitted,maelyrra,26022,"mission").await?;
        say(&mut owner,&admitted,maelyrra,26023,"yes").await?;
        say(&mut owner,&admitted,maelyrra,26024,"yes").await?;
        let bag=quantity(&h,BAG).await?==1
            && super::super::classic_quests::progress(&owner,&admitted,&authority,NIGHTMARE).await?==8;
        let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources WHERE transition_key LIKE 'oteryn:quest-reward/questpc3/threatened-final/%'").fetch_one(&h.pool).await?;
        eprintln!("THREATENED_FINAL POSTGRES_ASSERTION_REACHED flower={flower} herb={herb} carried={carried} bag={bag} receipts={receipts}");
        assert_eq!([flower,herb,carried,bag],[true,true,true,true]);assert_eq!(receipts,4);
        drop(owner);drop(authority);drop(seal);h.cleanup().await
    })
}

async fn quantity(h: &Harness, key: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key=$1 AND lifecycle=1").bind(key).fetch_one(&h.pool).await?)
}

async fn append_fixture_input(h: &Harness, key: &str, nonce: u8, ordinal: i32) -> TestResult {
    // Fixture inventory only, after proving real refusal. No quest receipt,
    // exchange success, output or progression is seeded.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,'definition-r1',1,1,encode($4,'hex')::uuid)")
        .bind(id(nonce).as_slice()).bind(id(42).as_slice()).bind(key).bind(id(nonce+1).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_item_container_entries(character_id,world_id,parent_item_instance_id,item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5::text::numeric,encode($6,'hex')::uuid)")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).bind(id(70).as_slice()).bind(id(nonce).as_slice()).bind(ordinal.to_string()).bind(id(nonce+1).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

async fn say(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    npc: EntityRef,
    command: u64,
    text: &str,
) -> TestResult<crate::gameplay_transport::npc_command::NpcTalkOutcome> {
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("clock")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    Ok(owner
        .npc_talk(
            admitted.runtime_actor.ok_or("actor")?,
            UseCommand {
                game_session_id: admitted.game_session_id,
                command_id: command,
                item_fence: Some(fence.item_fence()),
            },
            NpcTalkIntent {
                npc_actor: npc,
                text: text.into(),
            },
        )
        .await)
}
