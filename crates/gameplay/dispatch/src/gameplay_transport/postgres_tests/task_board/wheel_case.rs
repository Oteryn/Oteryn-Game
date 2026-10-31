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

struct CharacterFixture {
    level: i64,
    experience: i64,
    build: Option<Value>,
    promotion: Option<Value>,
}

/// Supply historical eligibility facts only. The extra point itself is never seeded:
/// it must come from the Task Board command and its guarded receipt.
async fn eligible_fixture(harness: &Harness) -> TestResult<CharacterFixture> {
    let mut tx = harness.pool.begin().await?;
    let (level, experience): (i64, i64) = sqlx::query_as(
        "SELECT level,total_experience FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_one(&mut *tx).await?;
    let build = sqlx::query_scalar::<_, Value>(
        "SELECT to_jsonb(s) FROM game_character_build_state s WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_optional(&mut *tx).await?;
    let promotion = sqlx::query_scalar::<_, Value>(
        "SELECT to_jsonb(s) FROM game_character_promotions s WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).fetch_optional(&mut *tx).await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_progression_state SET level=51 WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_build_state(character_id,vocation,magic_level,mana_spent,fist_level,fist_tries,club_level,club_tries,sword_level,sword_tries,axe_level,axe_tries,distance_level,distance_tries,shielding_level,shielding_tries,fishing_level,fishing_tries,committed_character_revision,last_build_occurrence_id) SELECT character_id,'sorcerer',0,0,10,0,10,0,10,0,10,0,10,0,10,0,10,0,character_revision,game_character_uuid_v7() FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid ON CONFLICT(character_id) DO UPDATE SET vocation='sorcerer'")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_character_promotions(character_id,purchase_occurrence_id) VALUES(encode($1,'hex')::uuid,game_character_uuid_v7()) ON CONFLICT(character_id) DO NOTHING")
        .bind(id(41).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(CharacterFixture {
        level,
        experience,
        build,
        promotion,
    })
}

async fn restore_fixture(harness: &Harness, fixture: CharacterFixture) -> TestResult {
    let mut tx = harness.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("UPDATE game_character_progression_state SET level=$2,total_experience=$3 WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice()).bind(fixture.level).bind(fixture.experience).execute(&mut *tx).await?;
    sqlx::query("DELETE FROM game_character_build_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice())
        .execute(&mut *tx)
        .await?;
    if let Some(build) = fixture.build {
        sqlx::query("INSERT INTO game_character_build_state SELECT * FROM jsonb_populate_record(NULL::game_character_build_state,$1)")
            .bind(build).execute(&mut *tx).await?;
    }
    sqlx::query("DELETE FROM game_character_promotions WHERE character_id=encode($1,'hex')::uuid")
        .bind(id(41).as_slice())
        .execute(&mut *tx)
        .await?;
    if let Some(promotion) = fixture.promotion {
        sqlx::query("INSERT INTO game_character_promotions SELECT * FROM jsonb_populate_record(NULL::game_character_promotions,$1)")
            .bind(promotion).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

pub(super) async fn wheel_case(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    harness: &Harness,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let fixture = eligible_fixture(harness).await?;
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
    restore_fixture(harness, fixture).await?;
    println!(
        "<TASK_BOARD> POSTGRES_CASE_COMPLETED: purchased_promotion_increases_fenced_wheel_budget_once"
    );
    Ok(())
}
