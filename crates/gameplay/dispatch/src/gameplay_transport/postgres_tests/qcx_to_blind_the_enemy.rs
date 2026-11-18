//! Eroth's mission and return use the production NPC dispatch and durable rewards.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use oteryn_gameplay_quests::quest::loader::load_embedded_quest_state;
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.to_blind_the_enemy_quest";
const TRACK: &str = "oteryn:quest-progress/quest/u8_1/to_blind_the_enemy/questline";

#[test]
fn eroth_mirror_and_once_only_reward() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx88-eroth", scenario);
    eprintln!("QCX88 Eroth duration={:?}", started.elapsed());
    result
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "qcx88_eroth", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("generation")?;
        let native = generation.native_gameplay().ok_or("native items")?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.quest_catalogue = Some(Arc::new(
            load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let mut admitted = deps.admitted;
        deps.initialize(&owner).await;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("quest fence")?
            .item_fence();
        admitted.item_fence = Some(fence);
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root.initialize_admission_equipment(
            &authority, &h.node, &fence, generation.identity().server_artifact_digest(),
            fresh.operation(),
        ).await.map_err(debug)?;
        h.root.grant_starter_items(
            &authority, &h.node, &fence, &NativeItemWeights(native),
            &crate::durability::starter_grant::StarterTemplate {
                definition: crate::durability::item_mint::TypedDefinitionRef {
                    family: "Item".into(),
                    production_key: "oteryn:item.tibia.i2854".into(),
                    revision_ref: "definition-r1".into(),
                },
                ..crate::durability::starter_grant::StarterTemplate::main()
            },
        ).await.map_err(debug)?;
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;
        let [eroth] = super::classic_quests::install(&owner, &admitted, ["oteryn:npc.eroth"]).await?;
        let talk = super::classic_quests::talk;
        let greeting = talk(&mut owner, &admitted, eroth, 1, "hi").await?;
        assert!(greeting.iter().any(|line| line == "I greet thee, outsider."));
        let offer = talk(&mut owner, &admitted, eroth, 2, "mission").await?;
        assert!(offer.iter().any(|line| line.contains("Are you willing to help?")));
        assert_eq!(quantity(&h, "oteryn:item.tibia.i3463").await?, 0);
        let confirmation = talk(&mut owner, &admitted, eroth, 3, "yes").await?;
        assert!(confirmation.iter().any(|line| line.contains("Take this mirror")));
        assert_eq!(quantity(&h, "oteryn:item.tibia.i3463").await?, 1);
        assert_eq!(super::classic_quests::progress(&owner, &admitted, &authority, TRACK).await?, 1);
        assert_eq!(quantity(&h, "oteryn:item.tibia.i3082").await?, 0);
        assert_eq!(quantity(&h, "oteryn:item.tibia.i3035").await?, 0);

        // Supply the persisted outcome of the held orb callback. The return itself
        // runs through Eroth and starts with zero treasure chest claims.
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
        let rows = sqlx::query("UPDATE game_character_quest_tracks SET value=2 WHERE track_key=$1 AND value=1")
            .bind(TRACK).execute(&mut *tx).await?.rows_affected();
        assert_eq!(rows, 1);
        tx.commit().await?;
        owner.refresh_quest_session(admitted.game_session_id, false).await;
        let reward = talk(&mut owner, &admitted, eroth, 4, "mission").await?;
        assert!(reward.iter().any(|line| line.contains("I heard the blow!")));
        assert_eq!(quantity(&h, "oteryn:item.tibia.i3082").await?, 1);
        assert_eq!(quantity(&h, "oteryn:item.tibia.i3035").await?, 10);
        assert_eq!(super::classic_quests::progress(&owner, &admitted, &authority, TRACK).await?, 3);
        let completed: bool = sqlx::query_scalar(
            "SELECT completed_character_revision IS NOT NULL FROM game_character_quest_states WHERE quest_key=$1",
        ).bind(QUEST).fetch_one(&h.pool).await?;
        assert!(completed);
        let repeated = talk(&mut owner, &admitted, eroth, 5, "mission").await?;
        assert!(!repeated.iter().any(|line| line.contains("I heard the blow!")));
        assert_eq!(quantity(&h, "oteryn:item.tibia.i3082").await?, 1);
        assert_eq!(quantity(&h, "oteryn:item.tibia.i3035").await?, 10);
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn quantity(h: &Harness, item: &str) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE lifecycle=1 AND definition_production_key=$1")
        .bind(item).fetch_one(&h.pool).await?)
}
