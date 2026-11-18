//! Bozo's source-backed entry through the NPC command owner and durable quest state.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
use std::sync::Arc;

const BOZO: &str = "oteryn:npc.bozo";
const QUEST: &str = "oteryn:quest.what_a_foolish_quest";
const JOIN_PROMPT: &str = "Do you wish to become a jester and join the fools guild?";

#[test]
fn bozo_join_requires_premium_and_two_confirmations() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("qcx_what_a_foolish", scenario);
    eprintln!(
        "POSTGRES_CASE_COMPLETED bozo_join duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

fn scenario() -> TestResult {
    run(async |admin| {
        super::reset();
        let h =
            Harness::create_with_admission_seed(admin, "qcx_what_a_foolish", false, false).await?;
        // The shared initialized fixture starts at level 50. Use level 1 to
        // exercise joining below the hat line's level requirement.
        sqlx::query(
            "INSERT INTO game_character_progression_state VALUES \
             (encode($1,'hex')::uuid,1,1,0,'profile-1','ruleset-1','content-1',\
              'simulation-1','evidence-1','declaration-1','policy-1','reward-1')",
        )
        .bind(id(41).as_slice())
        .execute(&h.pool)
        .await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(world, None);
        let active = controller.active().ok_or("active content")?;
        let channel_pin = crate::foundation::ChannelContentPin::from_activation(
            world,
            pin.activation_sequence(),
            pin.identity().server_artifact_digest(),
            pin.identity().client_artifact_digest(),
            pin.frame_binding().digest(),
            pin.qualified_room().map_revision_digest(),
            (32000, 32000, 7),
        );
        let deps = Dependencies::with_content(
            channel_pin,
            pin.qualified_room().clone(),
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(active);
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        deps.runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        deps.initialize(&owner).await;
        let catalogue =
            oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                .map_err(debug)?;
        owner.quest_catalogue = Some(Arc::new(catalogue.catalogue().clone()));
        owner.admit_progression_session(&mut admitted).await;
        owner.admit_quest_session(&admitted).await;
        let [bozo] = super::classic_quests::install(&owner, &admitted, [BOZO]).await?;
        let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
        h.root
            .latch_premium_activation(
                &activation.activation_id,
                activation.switch_over_us,
                activation.switch_over_us + 60_000_000,
            )
            .await
            .map_err(debug)?;
        let premium = SpellPremiumCoordinator::new(
            consumer,
            Some(clock),
            Some(activation),
            Some(h.root.clone()),
        );

        let receipt_count = || async {
            sqlx::query_scalar::<_, i64>(
                "SELECT count(*) FROM game_character_quest_receipts WHERE quest_key=$1",
            )
            .bind(QUEST)
            .fetch_one(&h.pool)
            .await
        };
        for (index, phrase) in [
            "hi", "join", "yes", "yes", "bye", "hi", "join", "yes", "no", "yes", "join", "yes",
            "yes", "join",
        ]
        .into_iter()
        .enumerate()
        {
            if index == 5 {
                owner.premium_coordinator = Some(&premium);
                owner.admit_premium(id(40), admitted.game_session_id);
                assert!(
                    owner
                        .session_premium_gate(admitted.game_session_id)
                        .await
                        .premium_current()
                );
            }
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(2))
                .ok_or("clock")?;
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("live session")?;
            let result = owner
                .npc_talk(
                    deps.actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: u64::try_from(index)? + 1,
                        item_fence: Some(fence.item_fence()),
                    },
                    NpcTalkIntent {
                        npc_actor: bozo,
                        text: phrase.into(),
                    },
                )
                .await;
            if index == 3 {
                assert_eq!(
                    result.disposition,
                    NpcIntentDisposition::Rejected,
                    "final confirmation requires Premium"
                );
            } else if !matches!(index, 9 | 13) {
                assert_eq!(result.disposition, NpcIntentDisposition::Ok, "{phrase}");
            }
            if index == 6 {
                assert_eq!(
                    result
                        .conversation
                        .as_ref()
                        .ok_or("Bozo join prompt")?
                        .lines,
                    [JOIN_PROMPT]
                );
            } else if index == 7 {
                let lines = &result
                    .conversation
                    .as_ref()
                    .ok_or("Bozo second confirmation")?
                    .lines;
                assert!(
                    lines
                        .last()
                        .is_some_and(|line| line.contains("Are you sure"))
                );
            }
            if index == 8 {
                assert_eq!(
                    result
                        .conversation
                        .as_ref()
                        .ok_or("Bozo cancellation")?
                        .lines,
                    ["Too bad, I'm convinced you have it in you."]
                );
            }
            assert_eq!(
                receipt_count().await?,
                i64::from(index >= 12),
                "{phrase} at {index}"
            );
        }
        let tracks: Vec<i64> = sqlx::query_scalar(
            "SELECT value FROM game_character_quest_tracks WHERE quest_key=$1 ORDER BY track_key",
        )
        .bind(QUEST)
        .fetch_all(&h.pool)
        .await?;
        assert_eq!(
            tracks,
            [1, 1],
            "joining starts the questline and first mission"
        );
        Ok(())
    })
}
