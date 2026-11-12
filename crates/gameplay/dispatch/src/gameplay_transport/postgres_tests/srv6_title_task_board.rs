//! SRV6: existing Task Board points qualify titles through command 113, never client proof.
use super::cyclopedia::support::exchange;
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::character_info::CharacterTitle;

#[test]
fn srv6_title_task_board_real_dispatch_boundaries_reload_and_fence() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new().stack_size(256 << 20).spawn(|| {
        run(async |admin| {
            let began = std::time::Instant::now();
            reset();
            let h = Harness::create_with_admission_seed(admin, "srv6_title_tasks", true, false).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
            let deps = Dependencies::new()?;
            deps.runtime.lock().await.initialize_first_entry_position(deps.actor).map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.quest_catalogue=Some(source_catalogue()?);
            deps.initialize(&owner).await;
            let current = crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id).await?.0;
            let mut admitted = deps.admitted;
            admitted.first_entry = FirstEntryOutcome::Positioned;
            admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
            let offers = crate::gameplay_transport::character_info::offered(&[], true);
            admitted.continuity.selected_capabilities = SelectedCapabilities::select(&offers, &[97]).ok_or("capability")?;
            for (name, points) in [("Aspiring Huntsman",160_000_i64),("Competent Beastslayer",320_000),("Feared Bountyhunter",430_000)] {
                let title = CharacterTitle::ALL.into_iter().find(|title| title.name()==name).ok_or_else(|| format!("missing title {name}"))?;
                for (balance, expected) in [(points-1,CommandStatus::Rejected),(points,CommandStatus::Accepted)] {
                    // Seed only the existing owner fact; the dispatch must qualify and write the title.
                    commit_source(&owner,admitted.game_session_id,&format!("points-{balance}")).await?;
                    let command=admitted.continuity.next_command_id;
                    assert_eq!(exchange(&owner,&mut admitted,command,113,&[2,title.id()]).await?.0,expected,"{name} at {balance}");
                }
                let command=admitted.continuity.next_command_id;
                assert_eq!(exchange(&owner,&mut admitted,command,113,&[3,title.id()]).await?.0,CommandStatus::Accepted);
                let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_character_titles WHERE character_id=encode($1,'hex')::uuid AND title=$2")
                    .bind(id(41).as_slice()).bind(i16::from(title.id())).fetch_one(&h.pool).await?;
                assert_eq!(count,1);
            }
            let title=CharacterTitle::ALL.into_iter().find(|title|title.name()=="Chompmeister").ok_or("missing Chompmeister")?;
            for dish in 2..=16 {
                commit_source(&owner,admitted.game_session_id,&format!("dish-{dish}")).await?;
                let command=admitted.continuity.next_command_id;
                let expected=if dish==16 {CommandStatus::Accepted} else {CommandStatus::Rejected};
                assert_eq!(exchange(&owner,&mut admitted,command,113,&[2,title.id()]).await?.0,expected);
            }
            let fence=admitted.item_fence.ok_or("fence")?;
            let reloaded=h.root.read_and_requalify_character_titles(&authority,&h.node,fence,fence.character_id,1_800_000_000_000,None, None).await.map_err(debug)?;
            assert!(reloaded.earned.iter().any(|(title,_)|title.name()=="Feared Bountyhunter"));
            // WORLD-SRC owns the canonical producer. Seed only its documented projection
            // shape here; qualification and durable title writes use the production owner.
            let table:oteryn_world_bundle::subarea::Table=serde_json::from_value(serde_json::json!({
                "default_town":null,"client_version":"15.30","candidate_sha256":"fixture",
                "masks":[],"runs":[],"areas":[
                    {"id":1,"key":"one","name":"One","parent":null,"source_record_sha256":"fixture","town_id":null,"hometown":false,"temple":null},
                    {"id":2,"key":"two","name":"Two","parent":null,"source_record_sha256":"fixture","town_id":null,"hometown":false,"temple":null},
                    {"id":11,"key":"one.a","name":"One A","parent":1,"source_record_sha256":"fixture","town_id":null,"hometown":false,"temple":null},
                    {"id":12,"key":"one.b","name":"One B","parent":1,"source_record_sha256":"fixture","town_id":null,"hometown":false,"temple":null},
                    {"id":21,"key":"two.a","name":"Two A","parent":2,"source_record_sha256":"fixture","town_id":null,"hometown":false,"temple":null}
                ]
            }))?;
            let title=CharacterTitle::ALL.into_iter().find(|title|title.name()=="Dedicated Entrepreneur").ok_or("missing exploration title")?;
            for (progress,qualifies) in [
                (serde_json::json!({"discovered":true,"exploration":null}),false),
                (serde_json::json!({"legacy":{"discovered":false,"exploration":null},"subareas":{"11":{"discovered":true},"999":{"discovered":true}}}),false),
                (serde_json::json!({"legacy":{"discovered":false,"exploration":null},"subareas":{"11":{"discovered":true},"12":{"discovered":true}}}),true)
            ] {
                let mut tx=h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_character_cyclopedia(character_id,bundle_digest,catalogue_digest,progress,occurrence) VALUES(encode($1,'hex')::uuid,$2,$5,$3,encode($4,'hex')::uuid) ON CONFLICT(character_id) DO UPDATE SET progress=EXCLUDED.progress")
                    .bind(id(41).as_slice()).bind([7_u8;32].as_slice()).bind(progress).bind(id(91).as_slice()).bind(crate::domain::cyclopedia::BasicArea::content_digest().as_slice()).execute(&mut *tx).await?;
                tx.commit().await?;
                let result=h.root.qualify_and_unlock_character_title(&authority,&h.node,fence,title,1_800_000_000_000,None,Some(&table)).await;
                assert_eq!(result.is_ok(),qualifies,"only complete canonical main areas count");
            }
            {
                let mut tx=h.pool.begin().await?;
                sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
                sqlx::query("UPDATE game_character_cyclopedia SET catalogue_digest=$2 WHERE character_id=encode($1,'hex')::uuid")
                    .bind(id(41).as_slice()).bind([8_u8;32].as_slice()).execute(&mut *tx).await?;
                tx.commit().await?;
                assert!(h.root.qualify_and_unlock_character_title(&authority,&h.node,fence,title,1_800_000_000_000,None,Some(&table)).await.is_err(),"unadmitted stored catalogue cannot supply discovery facts");
            }
            let mut stale=fence;
            stale.connection_generation=crate::foundation::ConnectionGeneration::new(2).map_err(debug)?;
            assert!(h.root.qualify_and_unlock_character_title(&authority,&h.node,stale,title,1_800_000_000_000,None,Some(&table)).await.is_err());
            let revoked=h.root.read_and_requalify_character_titles(&authority,&h.node,fence,fence.character_id,1_800_000_000_000,None,None).await.map_err(debug)?;
            assert!(!revoked.earned.iter().any(|(earned,_)|*earned==title),"absent current catalogue cannot retain a conditional title");
            eprintln!("SRV6_TITLE_TASKS_POSTGRES_COMPLETED elapsed={:?}",began.elapsed());
            drop(owner);drop(authority);drop(seal);
            h.cleanup().await
        }).map_err(|error|error.to_string())
    })?.join().map_err(|_|"title Task Board thread panicked")?.map_err(Into::into)
}

fn source_catalogue()
-> TestResult<std::sync::Arc<oteryn_gameplay_quests::quest::QuestStateCatalogue>> {
    use oteryn_gameplay_quests::quest::{
        QuestComparison, QuestEffect, QuestEffectKind, QuestStateCatalogue, QuestTrack,
        QuestTransition,
    };
    let points = oteryn_gameplay_quests::quest::task_board::POINTS;
    let task_quest = oteryn_gameplay_quests::quest::task_board::QUEST;
    let hot = "oteryn:quest.hot_cuisine_quest";
    let mut tracks = vec![QuestTrack {
        key: points.into(),
        quest: task_quest.into(),
        initial: 0,
        min: 0,
        max: i64::MAX,
    }];
    let mut transitions = Vec::new();
    for balance in [159_999, 160_000, 319_999, 320_000, 429_999, 430_000] {
        transitions.push(QuestTransition {
            world_change: None,
            key: format!("oteryn:quest-transition/srv6-points-{balance}"),
            quest: task_quest.into(),
            effects: vec![QuestEffect {
                track: points.into(),
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(balance),
            }],
            completes: false,
            experience: None,
        });
    }
    for dish in 2..=16 {
        let key = format!("oteryn:quest-progress/quest/u8_5/hot_cuisine_quest/kv/cookcount-{dish}");
        tracks.push(QuestTrack {
            key: key.clone(),
            quest: hot.into(),
            initial: 0,
            min: 0,
            max: i64::MAX,
        });
        transitions.push(QuestTransition {
            world_change: None,
            key: format!("oteryn:quest-transition/srv6-dish-{dish}"),
            quest: hot.into(),
            effects: vec![QuestEffect {
                track: key,
                from: QuestComparison::Any,
                effect: QuestEffectKind::Set(1),
            }],
            completes: false,
            experience: None,
        });
    }
    Ok(std::sync::Arc::new(
        QuestStateCatalogue::new("content-1", tracks, transitions).map_err(debug)?,
    ))
}

async fn commit_source(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    session: crate::foundation::GameSessionId,
    suffix: &str,
) -> TestResult {
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use crate::foundation::{CommandId, CommandRef};
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|_| "source fence")?
        .ok_or("source session")?;
    let catalogue = owner.quest_catalogue.as_ref().ok_or("source catalogue")?;
    let key = format!("oteryn:quest-transition/srv6-{suffix}");
    let ordinal = suffix.bytes().fold(0_u64, |value, byte| {
        value.wrapping_mul(31).wrapping_add(u64::from(byte))
    });
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let result = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: key,
                cause: QuestCause::Command(CommandRef::new(
                    session,
                    CommandId::new(1000 + ordinal % 1_000_000).map_err(debug)?,
                )),
            },
            std::sync::Arc::clone(catalogue),
        )
        .await
        .map_err(debug)?;
    assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
    Ok(())
}
