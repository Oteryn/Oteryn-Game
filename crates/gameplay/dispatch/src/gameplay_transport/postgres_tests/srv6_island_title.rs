//! SRV6: the level-999 source STEP commits the visit that command 113 consumes.
use super::fixture::{Harness, debug};
use super::quest_world_callbacks::{local, wire};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::durability::character_authority::ReconciledCharacterAuthority;
use crate::durability::character_progression::{
    ExperienceAwardRequest, ExperienceCommitOutcome, ExperienceRewardOccurrence,
};
use crate::foundation::MovementLocalPosition;
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position};
use oteryn_protocol_oteryn::character_info::CharacterTitle;
use oteryn_protocol_oteryn::world_spatial::StepDirection;

const VISITED: &str = "oteryn:quest-progress/schrodingers_island/visited";

#[test]
fn srv6_robinson_crusoe_real_step_and_title_dispatch() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    super::kill_reward_e2e::scenario_thread::run("srv6-robinson-title", || {
        super::quest_world_callbacks::scenario(
            "srv6-schrodinger-island",
            Position {
                x: 32717,
                y: 32345,
                floor: -7,
            },
            false,
        )
    })
}

// The callback scenario retains the real content/runtime/Character owners and source tile.
pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    callback: &Callback,
    origin: MovementLocalPosition,
) -> TestResult {
    let started = std::time::Instant::now();
    let actor = admitted.runtime_actor.ok_or("actor")?;
    assert_eq!(callback.selector.action, Some(15998));
    assert_eq!(callback.minimum_level, Some(999));
    let title = CharacterTitle::ALL
        .into_iter()
        .find(|title| title.name() == "Robinson Crusoe")
        .ok_or("missing Robinson Crusoe")?;
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    assert!(
        owner
            .root
            .qualify_and_unlock_character_title(
                authority,
                owner.holder,
                fence.item_fence(),
                title,
                1_800_000_000_000,
                None,
                None
            )
            .await
            .is_err()
    );
    let east = oteryn_protocol_oteryn::world_spatial::encode_step_intent(StepDirection::East);
    owner.clock_origin -= std::time::Duration::from_secs(2);
    wire(owner, admitted, 1, 1, east.clone()).await?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        origin,
        "level refusal returns source fromPosition"
    );
    assert_eq!(visit_count(h).await?, 0);
    let binding = owner
        .progression_binding(admitted.game_session_id)
        .ok_or("progression")?;
    let xp = binding
        .policy
        .thresholds
        .iter()
        .find(|threshold| threshold.level == 999)
        .ok_or("level999 threshold")?
        .minimum_experience
        .get();
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    let current = owner
        .root
        .read_character_progression(authority, fence.character_id)
        .await
        .map_err(debug)?
        .ok_or("progression state")?;
    let amount = xp
        .checked_sub(current.total_experience.get())
        .ok_or("XP delta")?;
    let outcome = owner
        .revision_sequencer
        .acquire(fence.character_id)
        .await
        .commit_experience(
            owner.root,
            authority,
            owner.holder,
            fence,
            ExperienceAwardRequest {
                occurrence: ExperienceRewardOccurrence::from_bytes(super::fixture::id(105))
                    .map_err(debug)?,
                amount: oteryn_simulation_determinism::ExactI64::new(amount),
                context: binding.context.clone(),
                policy_revision: binding.policy_revision.clone(),
                reward_revision: binding.reward_revision.clone(),
                policy: binding.policy.clone(),
            },
            None,
        )
        .await
        .map_err(debug)?;
    assert!(matches!(outcome, ExperienceCommitOutcome::Committed(_)));
    // A qualifying level alone is not a visit and cannot unlock the title.
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence")?
        .ok_or("fence")?;
    assert!(
        owner
            .root
            .qualify_and_unlock_character_title(
                authority,
                owner.holder,
                fence.item_fence(),
                title,
                1_800_000_000_000,
                None,
                None
            )
            .await
            .is_err()
    );
    owner.clock_origin -= std::time::Duration::from_secs(2);
    let result = wire(owner, admitted, 2, 1, east.clone()).await?;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        local(callback.teleport.ok_or("teleport")?)
    );
    assert_eq!(visit_count(h).await?, 1);
    let replay = wire(owner, admitted, 2, 1, east).await?;
    assert_eq!(result, replay);
    assert_eq!(visit_count(h).await?, 1);
    let copy = owner
        .root
        .read_character_quest_state(authority, fence.character_id)
        .await
        .map_err(debug)?;
    assert_eq!(copy.tracks().get(VISITED), Some(&1));
    let offers = crate::gameplay_transport::character_info::offered(&[], true);
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offers, &[97])
            .ok_or("title capability")?;
    let command = admitted.continuity.next_command_id;
    let (status, payload) =
        super::cyclopedia::support::exchange(owner, admitted, command, 113, &[2, title.id()])
            .await?;
    assert_eq!(status, oteryn_protocol_oteryn::CommandStatus::Accepted);
    assert_eq!(payload, vec![2, title.id()]);
    let command = admitted.continuity.next_command_id;
    let selected =
        super::cyclopedia::support::exchange(owner, admitted, command, 113, &[3, title.id()])
            .await?;
    assert_eq!(selected.0, oteryn_protocol_oteryn::CommandStatus::Accepted);
    eprintln!(
        "SRV6_ROBINSON_POSTGRES_COMPLETED elapsed={:?}",
        started.elapsed()
    );
    Ok(())
}

async fn visit_count(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE transition_key='oteryn:quest-callback/srv6-schrodinger-island'")
        .fetch_one(&h.pool).await?)
}
