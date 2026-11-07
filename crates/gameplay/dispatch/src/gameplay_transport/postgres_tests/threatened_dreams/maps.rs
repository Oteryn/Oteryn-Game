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
const FAIRY: &str = "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission04/tooth_fairy";
const COLLECTION: &str = "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission04/teeth_collection";
const TREE: &str = "oteryn:quest-progress/quest/u11_40/threatened_dreams/mission04/tired_tree";
const SEED: &str = "oteryn:quest-transition/fixture/threatened-map-prior-story";
const TEETH: [&str;6] = ["oteryn:item.tibia.i10196", "oteryn:item.tibia.i22649", "oteryn:item.tibia.i9685", "oteryn:item.tibia.i5893", "oteryn:item.tibia.i10275", "oteryn:item.tibia.i22052"];
const MILK: &str = "oteryn:item.tibia.i25303";
#[test]
fn threatened_map_four_source_exchanges_are_atomic() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        eprintln!("THREATENED_MAP POSTGRES_SKIPPED no_admin_url");
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20).spawn(|| {
        let started=std::time::Instant::now();
        match std::panic::catch_unwind(scenario) {
            Ok(result) => {
                eprintln!("THREATENED_MAP POSTGRES_CASE_COMPLETED duration_ms={} success={}", started.elapsed().as_millis(), result.is_ok());
                result.map_err(|error|error.to_string())
            }
            Err(panic) => {
                eprintln!("THREATENED_MAP POSTGRES_CASE_COMPLETED duration_ms={} success=false assertion_panicked=true",started.elapsed().as_millis());
                std::panic::resume_unwind(panic)
            }
        }
    })?.join().map_err(|_|"inventory scenario panicked")??;
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "threatened_map", false, false)
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
        for key in [FAIRY, COLLECTION, TREE] {
            let track = source.catalogue().track(key).ok_or("source track")?.clone();
            tracks.insert(key.to_owned(), track);
        }
        let mut transitions = Vec::new();
        for key in [
            "oteryn:quest-transition/threatened-dreams/tooth-collection-offer",
            "oteryn:quest-transition/threatened-dreams/tree-start",
            "oteryn:quest-reward/questpc3/threatened-maps/tooth-gifts",
            "oteryn:quest-reward/questpc3/threatened-maps/tooth-map",
            "oteryn:quest-reward/questpc3/threatened-maps/tooth-collection",
            "oteryn:quest-reward/questpc3/threatened-maps/tree-map",
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
            key: SEED.into(),
            quest: QUEST.into(),
            effects: [(FAIRY, 0), (COLLECTION, 0), (TREE, 0)]
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
        owner.quest_catalogue = Some(Arc::new(
            QuestStateCatalogue::new("content-1", tracks.into_values().collect(), transitions)
                .map_err(debug)?,
        ));
        {
            let runtime = deps.runtime.lock().await;
            let mut states = owner.states.spell_states.lock().await;
            assert!(states.initialize(&runtime,deps.actor,deps.admitted.game_session_id,
                crate::spell::cast::CharacterCastFacts {level:50,vocation:crate::spell::Vocation::Monk, ..crate::gameplay_transport::actor_spell::tests::FACTS},
                (3,7_000_000),owner.owner_now()).is_some());
        }
        let mut admitted = deps.admitted;
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;
        // Prior character level only, before the first quest receipt. The native
        // admission initializes a new character at level1; an actor cache alone
        // cannot satisfy a source level30 requirement in the durable owner.
        let prior_revision:String=sqlx::query_scalar("SELECT character_revision::text FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(prior_revision,"1","stage prior level before the earned receipt chain");
        fixture_level(&h,50).await?;
        super::super::item_look::seed_main_backpack(
            &h,
            &[
                (MILK, 2),
                (TEETH[0], 1), (TEETH[1], 1), (TEETH[2], 1),
                (TEETH[3], 1), (TEETH[4], 1),
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
        let [fairy, tree] = super::super::classic_quests::install(&owner, &admitted,
            ["oteryn:npc.tooth_fairy", "oteryn:npc.tired_tree"]).await?;
        say(&mut owner, &admitted, fairy, 25001, "hi").await?;
        // A stale higher actor cache may offer, but locked current progression
        // must refuse before granting presents or advancing the quest.
        let original_level:i64=sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        fixture_level(&h,29).await?;
        say(&mut owner,&admitted,fairy,24990,"mission").await?;
        say(&mut owner,&admitted,fairy,24991,"yes").await?;
        assert_eq!(quantity(&h,"oteryn:item.tibia.i25302").await?,0);
        assert_eq!(super::super::classic_quests::progress(&owner,&admitted,&authority,FAIRY).await?,0);
        fixture_level(&h,original_level).await?;
        say(&mut owner, &admitted, fairy, 25002, "mission").await?;
        say(&mut owner, &admitted, fairy, 25003, "yes please").await?;
        assert_eq!(quantity(&h, "oteryn:item.tibia.i25302").await?,0);
        say(&mut owner, &admitted, fairy, 25004, "yes").await?;
        let gifts=quantity(&h,"oteryn:item.tibia.i25302").await?==3;
        say(&mut owner, &admitted, fairy, 25005, "yes").await?;
        assert!(quantity(&h,"oteryn:item.tibia.i25302").await?<=3);
        say(&mut owner, &admitted, fairy, 25006, "mission").await?;
        assert_eq!(quantity(&h,MILK).await?,2,"short milk hand-in must preserve inputs");
        assert_eq!(quantity(&h,"oteryn:item.tibia.i24943").await?,0);
        append_fixture_input(&h,MILK,90,7).await?;
        say(&mut owner, &admitted, fairy, 25007, "mission").await?;
        let milk=quantity(&h,MILK).await?==0 && quantity(&h,"oteryn:item.tibia.i24943").await?==1;
        say(&mut owner, &admitted, fairy, 25008, "cause").await?;
        say(&mut owner, &admitted, fairy, 25009, "yes").await?;
        for key in &TEETH[..5] {assert_eq!(quantity(&h,key).await?,1,"missing species cannot consume {key}");}
        assert_eq!(quantity(&h,"oteryn:item.tibia.i3026").await?,0);
        append_fixture_input(&h,TEETH[5],92,8).await?;
        say(&mut owner, &admitted, fairy, 25010, "cause").await?;
        say(&mut owner, &admitted, fairy, 25011, "yes").await?;
        let mut teeth_left=0;
        for key in TEETH {teeth_left+=quantity(&h,key).await?;}
        let collection=teeth_left==0 && quantity(&h,"oteryn:item.tibia.i3026").await?==6;
        say(&mut owner, &admitted, fairy, 25012, "bye").await?;
        say(&mut owner, &admitted, tree, 25013, "hi").await?;
        say(&mut owner, &admitted, tree, 25014, "seeds of life").await?;
        assert_eq!(quantity(&h,"oteryn:item.tibia.i24944").await?,0,"story needs the source prompt");
        say(&mut owner, &admitted, tree, 25015, "mission").await?;
        say(&mut owner, &admitted, tree, 25016, "yes").await?;
        say(&mut owner, &admitted, tree, 25017, "mission").await?;
        say(&mut owner, &admitted, tree, 25018, "yes").await?;
        say(&mut owner, &admitted, tree, 25019, "seeds of life").await?;
        say(&mut owner, &admitted, tree, 25020, "seeds of life").await?;
        let map=quantity(&h,"oteryn:item.tibia.i24944").await?==1;
        let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_reward_sources WHERE transition_key LIKE 'oteryn:quest-reward/questpc3/threatened-maps/%'").fetch_one(&h.pool).await?;
        eprintln!("THREATENED_MAP POSTGRES_ASSERTION_REACHED gifts={gifts} milk={milk} collection={collection} map={map} receipts={receipts}");
        assert_eq!([gifts,milk,collection,map],[true,true,true,true]);
        assert_eq!(receipts,4);
        let achievements:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/toothfairy_assistant'").fetch_one(&h.pool).await?;
        assert_eq!(achievements,1,"the six-species hand-in grants the account achievement atomically");
        assert_eq!(super::super::classic_quests::progress(&owner,&admitted,&authority,FAIRY).await?,2);
        assert_eq!(super::super::classic_quests::progress(&owner,&admitted,&authority,COLLECTION).await?,2);
        assert_eq!(super::super::classic_quests::progress(&owner,&admitted,&authority,TREE).await?,2);
        drop(owner);drop(authority);drop(seal);h.cleanup().await
    })
}

async fn fixture_level(h:&Harness,level:i64)->TestResult {
    let mut tx=h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("UPDATE game_character_progression_state SET level=$1 WHERE character_id=encode($2,'hex')::uuid").bind(level).bind(id(41).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
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
