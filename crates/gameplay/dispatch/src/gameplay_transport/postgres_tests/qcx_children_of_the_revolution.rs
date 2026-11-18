//! TibiaWiki curid 41842/oldid 683807 and curid 41844/oldid 1160862.
use super::fixture::{Harness, debug, id, run};
use super::m39_bone_journey::{carried, stage};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use crate::gameplay_transport::item_ref_admission::NativeItemWeights;
use crate::gameplay_transport::spell_premium_coordinator::{SpellPremiumCoordinator, purchase_fixture};
use oteryn_gameplay_quests::quest::{loader, reward_callbacks};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use std::sync::Arc;

const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_54/children_of_the_revolution/questline";
const MISSION: &str = "oteryn:quest-progress/quest/u8_54/children_of_the_revolution/mission05";

#[test]
fn phantom_army_report_grants_one_crest_and_ten_thousand_xp_once() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("qcx57-phantom-army-report", scenario)?;
    eprintln!("POSTGRES_CASE_COMPLETED qcx57-phantom-army-report duration={:?}", started.elapsed());
    Ok(())
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "qcx57_phantom_army", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h.root.open_character_authority(&seal).await.map_err(debug)?;
        let deps = Dependencies::new()?;
        let (controller, activated) = super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id, "content-1",
        )?;
        let generation = controller.active().ok_or("generation")?;
        let native = generation.native_gameplay().ok_or("native")?;
        let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
        h.root.latch_premium_activation(
            &activation.activation_id, activation.switch_over_us, activation.switch_over_us + 60_000_000,
        ).await.map_err(debug)?;
        let premium = SpellPremiumCoordinator::new(consumer, Some(clock), Some(activation), Some(h.root.clone()));
        let current = FreshAdmissionStore::from_root(h.root.clone())
            .current_session_at(deps.admitted.game_session_id).await?.0;
        let fence = crate::gameplay_transport::item_fence_of(current).ok_or("item fence")?;
        sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
        sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice())
            .bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice())
            .bind(activated.frame_binding().digest().as_slice()).execute(&h.pool).await?;
        let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
        h.root.initialize_admission_equipment(
            &authority, &h.node, &fence, generation.identity().server_artifact_digest(), fresh.operation(),
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
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.premium_coordinator = Some(&premium);
        owner.quest_catalogue = Some(Arc::new(
            loader::load_embedded_quest_state("content-1").map_err(debug)?.catalogue().clone(),
        ));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = Some(fence);
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_premium(id(40), admitted.game_session_id);
        owner.admit_quest_session(&admitted).await;

        let [zalamon] = super::classic_quests::install(&owner, &admitted, ["oteryn:npc.zalamon"]).await?;
        let xp = || async {
            Ok::<i64, sqlx::Error>(sqlx::query_scalar("SELECT total_experience::bigint FROM game_character_progression_state")
                .fetch_one(&h.pool).await?)
        };
        let before = xp().await?;
        assert_eq!(carried(&h, 10199).await?, 0);
        let talk = super::qengine_rewards::talk;
        assert_eq!(talk(&mut owner, &admitted, zalamon, 1, "hi").await?, NpcIntentDisposition::Ok);
        talk(&mut owner, &admitted, zalamon, 2, "mission").await?;
        assert_eq!(xp().await?, before);
        assert_eq!(carried(&h, 10199).await?, 0);

        let callback = reward_callbacks::load()?.into_iter()
            .find(|callback| callback.key == "oteryn:quest-reward/l2-zalamon-phantom-army")
            .ok_or("Phantom Army report missing from the production catalogue")?;
        let mut survived = callback.clone();
        survived.storage[1].minimum = 2;
        super::qengine_rewards::seed(&mut owner, &admitted, &survived, 9000).await?;
        assert_eq!(stage(&h, QUESTLINE).await?, Some(20));
        assert_eq!(talk(&mut owner, &admitted, zalamon, 3, "mission").await?, NpcIntentDisposition::Ok);
        assert_eq!(xp().await?, before + 10_000);
        assert_eq!(carried(&h, 10199).await?, 1);
        assert_eq!(stage(&h, QUESTLINE).await?, Some(21));
        assert_eq!(stage(&h, MISSION).await?, Some(3));
        talk(&mut owner, &admitted, zalamon, 4, "mission").await?;
        assert_eq!(xp().await?, before + 10_000);
        assert_eq!(carried(&h, 10199).await?, 1);
        Ok(())
    })
}
