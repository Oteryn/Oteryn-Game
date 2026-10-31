//! A real Task Shop purchase increases the existing fenced Wheel allocation budget.
use super::*;
use crate::durability::character_wheel::{
    WheelChangeFacts, WheelChangeRequest, WheelCommitOutcome, WheelIneligibility, WheelOccurrence,
    WheelRefusal, WheelRuleset, WheelSlots,
};
use crate::gameplay_transport::spell_premium_coordinator::{
    SpellPremiumCoordinator, purchase_fixture,
};
use oteryn_gameplay_progression::wheel_gem_data::WheelGemData;
use std::sync::Arc;

/// The earlier scenario already chose Monk through its build owner. Preserve
/// that accepted build and reach level 51 through an actual admitted XP award.
/// Only the pre-existing promotion eligibility fact is historical fixture input;
/// the additional Wheel point still has to come from the real Task Shop command.
async fn eligible_fixture(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &AdmittedSession,
) -> TestResult {
    let policy = progression_case::policy(owner, harness).await?;
    let threshold = policy.thresholds.iter().find(|row| row.level == 51)
        .ok_or("admitted level51 threshold")?.minimum_experience.get();
    progression_case::award_total(owner, harness, admitted, threshold, 51, 64_998).await?;
    let fence = owner.current_quest_fence(admitted.game_session_id).await
        .map_err(|()| "Wheel fixture fence")?.ok_or("Wheel fixture fence")?;
    let build = owner.root.read_character_build_state(owner.character, fence.character_id)
        .await.map_err(debug)?;
    assert_eq!(build.vocation(), "monk", "preserve the accepted build receipt");
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica").execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_promotions(character_id,purchase_occurrence_id) VALUES(encode($1,'hex')::uuid,game_character_uuid_v7()) ON CONFLICT(character_id) DO NOTHING")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn wheel_case(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &mut AdmittedSession,
) -> TestResult {
    eligible_fixture(owner, harness, admitted).await?;
    let data = WheelGemData::embedded().map_err(debug)?;
    let ruleset =
        Arc::new(WheelRuleset::from_catalogue(data.revision(), data.wheel()).map_err(debug)?);
    let center = data.wheel()["topology"]
        .as_array()
        .ok_or("Wheel topology")?
        .iter()
        .position(|entry| {
            entry["unlock_from_any_full_slot"]
                .as_array()
                .is_some_and(Vec::is_empty)
                && entry["minimum_available_points"]
                    .as_u64()
                    .is_some_and(|minimum| minimum <= 3)
        })
        .ok_or("an admitted center slot")?;
    let mut points = [0; 36];
    points[center] = 3;
    let request = WheelChangeRequest {
        occurrence: WheelOccurrence::from_bytes(id(248)).map_err(debug)?,
        expected_wheel_revision: 0,
        slots: WheelSlots::new(points).map_err(debug)?,
        vessels: None,
    };
    let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
    owner
        .root
        .latch_premium_activation(
            &activation.activation_id,
            activation.switch_over_us,
            activation.switch_over_us + 60_000_000,
        )
        .await
        .map_err(debug)?;
    let gate = SpellPremiumCoordinator::new(
        consumer,
        Some(clock.clone()),
        Some(activation.clone()),
        Some(harness.root.clone()),
    );
    let free = SpellPremiumCoordinator::new(
        Default::default(),
        Some(clock),
        Some(activation),
        Some(harness.root.clone()),
    );

    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence read")?
        .ok_or("current fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let before = slot
        .commit_wheel(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            request,
            Arc::clone(&ruleset),
            WheelChangeFacts { at_temple: false },
            &gate,
        )
        .await
        .map_err(debug)?;
    assert_eq!(
        before,
        WheelCommitOutcome::Refused(WheelRefusal::OverPoints),
        "level51 plus the first purchased point has a budget of two, not three"
    );
    drop(slot);

    let before = full_board(owner, admitted, 4, 0, "").await?;
    let purchase_id = admitted.continuity.next_command_id;
    let purchase = full_board(owner, admitted, 16, 23, "").await?;
    assert_eq!(
        (purchase.refusal, purchase.points),
        (0, before.points - 200)
    );
    let replay = serve_from(owner, admitted, purchase_id, &[full_request(16, 23, "")]).await?;
    assert_eq!(replay[0].1, CommandStatus::DuplicateReplay);
    assert_eq!(
        wire::full::decode_reply(&replay[0].2).map_err(debug)?,
        purchase
    );

    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|()| "fence read")?
        .ok_or("current fence")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let no_premium = slot
        .commit_wheel(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            request,
            Arc::clone(&ruleset),
            WheelChangeFacts { at_temple: false },
            &free,
        )
        .await
        .map_err(debug)?;
    assert_eq!(
        no_premium,
        WheelCommitOutcome::Refused(WheelRefusal::NotEligible(WheelIneligibility::NotPremium)),
        "a purchased point never replaces the existing Premium eligibility owner"
    );
    let committed = slot
        .commit_wheel(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            request,
            Arc::clone(&ruleset),
            WheelChangeFacts { at_temple: false },
            &gate,
        )
        .await
        .map_err(debug)?;
    let WheelCommitOutcome::Committed(committed) = committed else {
        return Err(format!("Wheel allocation: {committed:?}").into());
    };
    assert_eq!(committed.after.total(), 3);
    assert_eq!(committed.after_wheel_revision, 1);
    let replay = slot
        .commit_wheel(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            request,
            Arc::clone(&ruleset),
            WheelChangeFacts { at_temple: false },
            &gate,
        )
        .await
        .map_err(debug)?;
    assert_eq!(replay, WheelCommitOutcome::AlreadyCommitted(committed));
    drop(slot);
    let reloaded = owner
        .root
        .read_character_wheel(owner.character, fence.character_id, ruleset)
        .await
        .map_err(debug)?;
    assert_eq!((reloaded.wheel_revision, reloaded.slots.total()), (1, 3));
    // Accepted level/XP and the receipted Monk build remain intact for later cases.
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: purchased_promotion_increases_fenced_wheel_budget_once"
    );
    Ok(())
}
