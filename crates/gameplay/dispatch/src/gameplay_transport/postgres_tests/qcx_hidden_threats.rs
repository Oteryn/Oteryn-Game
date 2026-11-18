//! Global Hidden Threats dialogue and reward compatibility through NPC dispatch.
use super::classic_quests::{install, progress, talk};
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::content::native_gameplay::test_support::activated_with_loot_tables;
use crate::foundation::{ChannelContentPin, WorldId};
use oteryn_gameplay_quests::quest::loader::load_embedded_quest_state;
use std::sync::Arc;

#[test]
fn servant_accepts_walkthrough_wording() -> TestResult {
    corym_case("decreased resources", false)
}

#[test]
fn southwestern_worker_grants_small_amethyst() -> TestResult {
    corym_case("decreasing resources", true)
}

fn corym_case(prompt: &'static str, check_reward: bool) -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx-hidden-threats", move || {
        run(async move |admin| {
            reset();
            let h = Harness::create_with_admission_seed(admin, "qcx_hidden", true, false).await?;
            prepare_current_session(&h).await?;
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let world = WorldId::decode(&id(42)).map_err(debug)?;
            let (controller, pin) = activated_with_loot_tables(world, None);
            let channel_pin = ChannelContentPin::from_activation(
                world,
                pin.activation_sequence(),
                pin.identity().server_artifact_digest(),
                pin.identity().client_artifact_digest(),
                pin.frame_binding().digest(),
                pin.qualified_room().map_revision_digest(),
                (2, 0, 7),
            );
            let deps = Dependencies::with_content_and_capacity(
                channel_pin,
                crate::content::qualify_native_entry_room(world).map_err(debug)?,
                crate::spell::cast::v1_spell_book().map_err(debug)?,
                4,
            )?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(controller.active().ok_or("active content")?);
            owner.quest_catalogue = Some(Arc::new(
                load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .clone(),
            ));
            deps.initialize(&owner).await;
            deps.runtime
                .lock()
                .await
                .initialize_first_entry_position(deps.actor)
                .map_err(debug)?;
            let admitted = &deps.admitted;
            let [ratter, servant, worker] = install(
                &owner,
                admitted,
                [
                    "oteryn:npc.corym_ratter",
                    "oteryn:npc.corym_servant",
                    "oteryn:npc.corym_worker_5",
                ],
            )
            .await?;

            talk(&mut owner, admitted, ratter, 100, "hi").await?;
            talk(&mut owner, admitted, ratter, 101, "help").await?;
            talk(&mut owner, admitted, ratter, 102, "yes").await?;
            talk(&mut owner, admitted, ratter, 103, "bye").await?;
            talk(&mut owner, admitted, servant, 104, "hi").await?;
            let lines = talk(&mut owner, admitted, servant, 105, prompt).await?;
            assert!(
                lines.iter().any(|line| line.contains("{defy}")),
                "{lines:?}"
            );
            talk(&mut owner, admitted, servant, 106, "defy").await?;
            assert_eq!(
                progress(
                    &owner,
                    admitted,
                    &authority,
                    "oteryn:quest-progress/quest/u11_50/hidden_threats/servant_door"
                )
                .await?,
                1,
            );
            if check_reward {
                talk(&mut owner, admitted, servant, 107, "bye").await?;
                talk(&mut owner, admitted, worker, 108, "hi").await?;
                assert_eq!(super::questpc7_outfit_items::quantity(&h, 3033).await?, 1);
                assert_eq!(super::questpc7_outfit_items::quantity(&h, 3029).await?, 0);
                talk(&mut owner, admitted, worker, 109, "bye").await?;
                talk(&mut owner, admitted, worker, 110, "hi").await?;
                assert_eq!(super::questpc7_outfit_items::quantity(&h, 3033).await?, 1);
            }
            Ok(())
        })
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {prompt} {:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}
