//! Command 6241 reaches both server-info owners and enforces capability selection.
use super::fixture::{Harness, debug};
use super::kill_reward_e2e_fixtures::scenario_thread::run_async;
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::{
    capabilities::{OfferedCapability, SelectedCapabilities},
    connection::FirstEntryOutcome,
};
use oteryn_protocol_oteryn::server_info::{Query, Reply};
use oteryn_session::CommandStatus;
#[test]
fn server_info_real_dispatch_queries_and_unselected_rejection() -> TestResult {
    run_async(async |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "server_info_dispatch", true, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let (controller, _) = super::death_ml_loss_composed::production_generation_with_pin(
            deps.admitted.world_id,
            "content-1",
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = controller.active();
        let lowered = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(debug)?;
        let journal =
            oteryn_gameplay_quests::quest::log::loader::load_embedded_quest_log(&lowered)?;
        owner.quest_catalogue = Some(std::sync::Arc::new(lowered.catalogue().clone()));
        owner.quest_log_content = Some(
            crate::gameplay_transport::quest_log::QuestLogLines::new(journal.content)
                .ok_or("journal bounds")?,
        );
        deps.initialize(&owner).await;
        let current =
            crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone())
                .current_session_at(deps.admitted.game_session_id)
                .await?
                .0;
        deps.runtime
            .lock()
            .await
            .bind_attacker_lease(
                deps.actor,
                deps.admitted.game_session_id,
                current.current_character_lease(),
            )
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.item_fence = crate::gameplay_transport::item_fence_of(current);
        owner.admit_quest_session(&admitted).await;
        let offers = [OfferedCapability {
            id: 500,
            requires: &[],
        }];
        let spells = owner
            .actor_spell_catalog_with_visibility(deps.actor, admitted.game_session_id, true)
            .await
            .ok_or("eligible spell catalogue")?;
        let spell = spells.first().ok_or("eligible spell")?.cast_index;
        let queries = [Query::Quests, Query::Spells(vec![spell])];
        let mut command = 1;
        for selected in [false, true] {
            admitted.continuity.selected_capabilities =
                SelectedCapabilities::select(&offers, if selected { &[500] } else { &[] })
                    .ok_or("selection")?;
            for query in &queries {
                let (status, payload) = super::cyclopedia_fixtures::support::exchange(
                    &owner,
                    &mut admitted,
                    command,
                    6241,
                    &query.encode().ok_or("query")?,
                )
                .await?;
                command += 1;
                if selected {
                    assert_eq!(status, CommandStatus::Accepted);
                    let reply = Reply::decode(&payload).ok_or("reply")?;
                    assert!(matches!(
                        (query, reply),
                        (Query::Quests, Reply::Quests(_)) | (Query::Spells(_), Reply::Spells(_))
                    ));
                } else {
                    assert_eq!(status, CommandStatus::Rejected);
                    assert!(payload.is_empty());
                }
            }
        }
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
