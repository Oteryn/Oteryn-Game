//! Actual NPC routing must select replies and mutations from carried custody.
use super::fixture::debug;
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTransition,
};
use std::sync::Arc;

const CAKE: &str =
    "oteryn:quest-progress/quest/u8_4/the_hidden_city_of_beregar/sweet_as_chocolate_cake";
const STAG: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_20/the_order_of_the_stag/questline";
const DEMERON: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_20/the_order_of_the_stag/demeron";
const HARLAN: &str =
    "oteryn:quest-progress/crystalserver/quest/u15_20/the_order_of_the_stag/harlan";

#[test]
fn npc_inventory_selects_real_missing_partial_and_complete_source_branches() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("catalogue-npc-inventory", || {
        super::quest_world_callbacks::scenario(
            "catalogue-npc-inventory",
            Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
            false,
        )
    })
}

async fn seed(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    transition: usize,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let outcome = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: format!(
                    "oteryn:quest-transition/npc-inventory-fixture/{transition}"
                ),
                cause: QuestCause::Command(CommandRef::new(
                    admitted.game_session_id,
                    CommandId::new(53000 + u64::try_from(transition)?).map_err(debug)?,
                )),
            },
            owner.quest_catalogue.as_ref().ok_or("catalogue")?.clone(),
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(outcome, QuestTransitionOutcome::Committed(_)),
        "fixture: {outcome:?}"
    );
    drop(slot);
    owner
        .refresh_quest_session(admitted.game_session_id, false)
        .await;
    Ok(())
}

fn topic(owner: &ComposedFreshAdmission<'_, '_, '_>, admitted: &AdmittedSession) -> Option<i32> {
    owner
        .states
        .npc_conversations
        .catalogue_topic_for_test(admitted.game_session_id)
}

async fn carry(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    item: &str,
    count: u16,
    id: u64,
) -> TestResult {
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    super::quest_world_callbacks::input_item(
        owner,
        fence.item_fence(),
        &Offering {
            item: item.into(),
            count,
            position: Position {
                x: 33294,
                y: 31677,
                floor: -15,
            },
        },
        id,
        true,
    )
    .await
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    authority: &ReconciledCharacterAuthority<'_, '_>,
) -> TestResult {
    // Legal fixture-only historical progress uses the real transition owner and
    // catalogue hashing. Source NPC programs and their binding markers remain intact.
    let mut catalogue = owner
        .quest_catalogue
        .as_ref()
        .ok_or("catalogue")?
        .as_ref()
        .clone();
    for (index, assignments) in [
        vec![(CAKE, 1)],
        vec![(STAG, 3), (HARLAN, 1)],
        vec![(STAG, 4)],
    ]
    .into_iter()
    .enumerate()
    {
        let quest = catalogue
            .track(assignments[0].0)
            .ok_or("track")?
            .quest
            .clone();
        catalogue = catalogue
            .with_fixture_transition(QuestTransition {
                key: format!("oteryn:quest-transition/npc-inventory-fixture/{index}"),
                quest,
                effects: assignments
                    .into_iter()
                    .map(|(track, value)| QuestEffect {
                        track: track.into(),
                        from: QuestComparison::Any,
                        effect: QuestEffectKind::Set(value),
                    })
                    .collect(),
                completes: false,
                experience: None,
                world_change: None,
            })
            .map_err(debug)?;
    }
    owner.quest_catalogue = Some(Arc::new(catalogue));
    seed(owner, admitted, 0).await?;
    seed(owner, admitted, 1).await?;
    let [bolfona, demeron, harlan] = super::classic_quests::install(
        owner,
        admitted,
        [
            "oteryn:npc.bolfona",
            "oteryn:npc.second_in_command_demeron",
            "oteryn:npc.lieutenant_harlan",
        ],
    )
    .await?;
    // The new classification seam rejects stale currentness and unknown source
    // ownership, and its output cannot be reused for another NPC or Item fence.
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let native = owner
        .active_generation
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native")?;
    let weights = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
        .with_wheel(owner.premium_coordinator);
    let command = CommandRef::new(
        admitted.game_session_id,
        CommandId::new(53010).map_err(debug)?,
    );
    let catalogue = owner.quest_catalogue.as_ref().ok_or("catalogue")?;
    let before = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    let snapshot = owner
        .root
        .read_current_npc_program_inventory(
            authority,
            owner.holder,
            fence,
            command,
            "oteryn:npc.second_in_command_demeron",
            catalogue,
            &weights,
        )
        .await
        .map_err(debug)?;
    assert!(
        snapshot
            .for_fence("oteryn:npc.second_in_command_demeron", fence.item_fence())
            .is_some()
    );
    assert!(
        snapshot
            .for_fence("oteryn:npc.bolfona", fence.item_fence())
            .is_none()
    );
    let mut stale = fence;
    stale.character_lease_generation += 1;
    assert!(
        snapshot
            .for_fence("oteryn:npc.second_in_command_demeron", stale.item_fence())
            .is_none()
    );
    assert!(
        owner
            .root
            .read_current_npc_program_inventory(
                authority,
                owner.holder,
                stale,
                command,
                "oteryn:npc.second_in_command_demeron",
                catalogue,
                &weights,
            )
            .await
            .is_err()
    );
    assert!(
        owner
            .root
            .read_current_npc_program_inventory(
                authority,
                owner.holder,
                fence,
                command,
                "oteryn:npc.unknown",
                catalogue,
                &weights,
            )
            .await
            .is_err()
    );
    let after = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(
        before.tracks(),
        after.tracks(),
        "classification and refusals never write progress"
    );
    use super::classic_quests::{progress, talk};
    talk(owner, admitted, bolfona, 53100, "hi").await?;
    let missing = talk(owner, admitted, bolfona, 53101, "chocolate cake").await?;
    assert!(!missing.iter().any(|line| line.contains("Is that for me?")));
    assert_ne!(topic(owner, admitted), Some(1));
    assert_eq!(progress(owner, admitted, authority, CAKE).await?, 1);
    carry(owner, admitted, "oteryn:item.tibia.i8019", 1, 5300).await?;
    let present = talk(owner, admitted, bolfona, 53102, "chocolate cake").await?;
    assert!(present.iter().any(|line| line.contains("Is that for me?")));
    assert_eq!(topic(owner, admitted), Some(1));

    // Obtain topic 3 through the real conversation, then independently advance
    // historical progress. No test setter fabricates the proof replay topic.
    talk(owner, admitted, demeron, 53110, "hi").await?;
    talk(owner, admitted, demeron, 53111, "tell").await?;
    talk(owner, admitted, demeron, 53112, "yes").await?;
    talk(owner, admitted, demeron, 53113, "yes").await?;
    assert_eq!(topic(owner, admitted), Some(3));
    seed(owner, admitted, 2).await?;
    let missing = talk(owner, admitted, demeron, 53114, "yes").await?;
    assert!(
        missing
            .iter()
            .any(|line| line.contains("Evidence? I see nothing"))
    );
    assert!(!missing.iter().any(|line| line.contains("A certificate of")));
    assert_eq!(topic(owner, admitted), Some(0));
    assert_eq!(progress(owner, admitted, authority, STAG).await?, 4);
    let missing = talk(owner, admitted, demeron, 53115, "spares").await?;
    assert!(
        missing
            .iter()
            .any(|line| line.contains("I need 30 salvaged iron parts"))
    );
    assert_ne!(topic(owner, admitted), Some(20));
    assert_eq!(progress(owner, admitted, authority, DEMERON).await?, 1);
    carry(owner, admitted, "oteryn:item.tibia.i52635", 29, 5301).await?;
    carry(owner, admitted, "oteryn:item.tibia.i52758", 20, 5302).await?;
    carry(owner, admitted, "oteryn:item.tibia.i52743", 10, 5303).await?;
    let partial = talk(owner, admitted, demeron, 53116, "spares").await?;
    assert!(
        !partial
            .iter()
            .any(|line| line.contains("Are those salvaged parts"))
    );
    assert_ne!(topic(owner, admitted), Some(20));
    assert_eq!(progress(owner, admitted, authority, DEMERON).await?, 1);
    carry(owner, admitted, "oteryn:item.tibia.i52635", 1, 5304).await?;
    let present = talk(owner, admitted, demeron, 53117, "spares").await?;
    assert!(
        present
            .iter()
            .any(|line| line.contains("Are those salvaged parts"))
    );
    assert_eq!(topic(owner, admitted), Some(20));

    talk(owner, admitted, harlan, 53120, "hi").await?;
    let missing = talk(owner, admitted, harlan, 53121, "help").await?;
    assert!(
        missing
            .iter()
            .any(|line| line.contains("Our supplies are running out"))
    );
    assert_ne!(topic(owner, admitted), Some(3));
    assert_eq!(progress(owner, admitted, authority, HARLAN).await?, 2);
    carry(owner, admitted, "oteryn:item.tibia.i52636", 19, 5305).await?;
    let partial = talk(owner, admitted, harlan, 53122, "help").await?;
    assert!(
        !partial
            .iter()
            .any(|line| line.contains("Is that tender venison?"))
    );
    assert_eq!(progress(owner, admitted, authority, HARLAN).await?, 2);
    carry(owner, admitted, "oteryn:item.tibia.i52636", 1, 5306).await?;
    let present = talk(owner, admitted, harlan, 53123, "help").await?;
    assert!(
        present
            .iter()
            .any(|line| line.contains("Is that tender venison?"))
    );
    assert_eq!(topic(owner, admitted), Some(3));
    Ok(())
}
