//! The global Mission 2 transcript allows buying syrup before starting the quest.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::foundation::{ChannelContentPin, WorldId};
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_protocol_oteryn::npc_service::NpcIntentDisposition;
use std::sync::Arc;

#[test]
fn padreia_sells_one_syrup_for_fifty_gold_before_quest_start() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("ape-city-syrup", scenario);
    eprintln!(
        "POSTGRES_CASE_COMPLETED ape-city-syrup duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

fn scenario() -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "ape_city_syrup", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, activated) = activated_with_loot_tables(world, None);
        let pin = ChannelContentPin::from_activation(
            world,
            activated.activation_sequence(),
            activated.identity().server_artifact_digest(),
            activated.identity().client_artifact_digest(),
            activated.frame_binding().digest(),
            activated.qualified_room().map_revision_digest(),
            (2, 0, 7),
        );
        let deps = Dependencies::with_content(
            pin,
            crate::content::qualify_native_entry_room(world).map_err(debug)?,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(controller.active().ok_or("active content")?);
        owner.quest_catalogue = Some(Arc::new(
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")?
                .catalogue()
                .clone(),
        ));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(debug)?
            .ok_or("session fence")?;
        admitted.item_fence = Some(fence.item_fence());
        owner.admit_quest_session(&admitted).await;
        deps.initialize(&owner).await;
        super::item_look::seed_main_backpack(&h, &[("oteryn:item.tibia.i3031", 100)]).await?;
        let [npc] =
            super::classic_quests::install(&owner, &admitted, ["oteryn:npc.padreia"]).await?;

        // Two purchases exercise the same dialogue with no quest state seeded.
        for (i, word) in ["hi", "cough syrup", "yes", "cough syrup", "yes"]
            .into_iter()
            .enumerate()
        {
            assert_eq!(
                super::qengine_rewards::talk(
                    &mut owner,
                    &mut admitted,
                    npc,
                    100 + u64::try_from(i)?,
                    word
                )
                .await?,
                NpcIntentDisposition::Ok,
                "Padreia should accept {word}"
            );
        }
        let purchases: (i64, i64) = sqlx::query_as(
            "SELECT count(*), coalesce(sum(fee_gold_units),0)::bigint FROM game_item_fee_burns WHERE cause_kind=18",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(purchases, (2, 100));
        let syrup: (i64, i64) = sqlx::query_as(
            "SELECT count(*), coalesce(sum(i.quantity),0)::bigint FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i4828'",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(syrup, (2, 2), "one bottle per purchase");
        Ok(())
    })
}
