//! World-reset window through the existing shrine, details and claim commands.
use super::daily_reward_fixtures::{epoch, results, serve};
use super::*;
use crate::gameplay_transport::capabilities::{
    PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities,
};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_foundation::scheduled_events::{Schedule, test_clock};
use oteryn_protocol_oteryn::daily_reward as wire;

#[test]
fn double_daily_rewards_dispatch_doubles_preview_and_atomic_claims_only_during_event() -> TestResult
{
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("double-daily-rewards", scenario)
}

fn scenario() -> TestResult {
    let schedule = Schedule::parse(
        r#"{"events":[{"name":"Double Daily Rewards","startdate":"10/02/2026",
            "starthour":"04:00","enddate":"10/06/2026","endhour":"03:59:59",
            "ingame":{"double_daily_rewards":true}}]}"#,
    )
    .map_err(debug)?;
    let _clock = test_clock::set(schedule, 0);
    run(async |admin| {
        let harness =
            Harness::create_with_admission_seed(admin, "double_daily_rewards", false, false)
                .await?;
        prepare_current_session(&harness).await?;
        let seal = harness.recovery.seal_current().map_err(debug)?;
        let authority = harness
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let world = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
        let (generation, pin) =
            super::death_ml_loss_composed_fixtures::production_generation_with_pin(
                world,
                "content-1",
            )?;
        let room = pin.qualified_room().clone();
        let (channel_pin, _, _, _) = pin.into_channel_parts();
        let dependencies = Dependencies::with_content(
            channel_pin,
            room,
            crate::spell::cast::v1_spell_book().map_err(debug)?,
        )?;
        let (consumer, clock, activation, _) =
            crate::gameplay_transport::spell_premium_coordinator::purchase_fixture::current(id(40));
        harness
            .root
            .latch_premium_activation(
                &activation.activation_id,
                activation.switch_over_us,
                activation.switch_over_us + 60_000_000,
            )
            .await
            .map_err(debug)?;
        let coordinator =
            crate::gameplay_transport::spell_premium_coordinator::SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(harness.root.clone()),
            );
        let mut owner = dependencies.owner(&harness, &authority);
        owner.premium_coordinator = Some(&coordinator);
        owner.active_generation = generation.active();
        dependencies.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(dependencies.actor)
            .map_err(debug)?;
        let current = crate::gameplay_transport::character_gameplay_fence::current(
            &owner,
            dependencies.admitted.game_session_id,
            false,
        )
        .await
        .ok_or("current fence")?;
        let mut admitted = dependencies.admitted;
        admitted.item_fence = Some(current.item_fence());
        admitted.first_entry = FirstEntryOutcome::Positioned;
        assert!(
            owner.daily_reward_available(),
            "the shipped native profile supports DAILY"
        );
        let offer = crate::gameplay_transport::daily_reward::offered(
            PRODUCTION_OFFERED_CAPABILITIES,
            owner.daily_reward_available(),
        );
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(&offer, &[1, wire::CAPABILITY_DAILY_REWARD_V2])
                .ok_or("selection")?;
        assert!(
            admitted
                .continuity
                .selected_capabilities
                .contains(wire::CAPABILITY_DAILY_REWARD_V2)
        );
        epoch(&harness, 1).await?;
        let shrine = oteryn_protocol_oteryn::world_object::encode_use_intent(
            &oteryn_protocol_oteryn::world_object::WorldObjectTarget {
                placement: b"oteryn:placement/daily-reward-shrine".to_vec(),
                expected_revision: 0,
            },
        )
        .map_err(debug)?;
        double_rewards(&harness, &owner, &mut admitted, &shrine).await?;
        drop(owner);
        drop(authority);
        drop(seal);
        harness.cleanup().await
    })
}

async fn double_rewards(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    shrine: &[u8],
) -> TestResult {
    for (day, expected) in [
        (1, [4, 0, 0, 0, 0, 1]),
        (2, [0, 4, 0, 0, 0, 2]),
        (3, [0, 0, 2, 0, 0, 2]),
        (4, [0, 0, 0, 25, 60, 2]),
        (5, [12, 0, 0, 0, 0, 2]),
        (6, [0, 2, 0, 0, 0, 1]),
    ] {
        if day > 1 {
            epoch(h, day).await?;
        }
        let (end, output) = serve(
            owner,
            admitted,
            &[
                (2, shrine),
                (wire::details::COMMAND_TYPE, &[]),
                (wire::COMMAND_TYPE_DAILY_REWARD_CLAIM, &[]),
                (wire::COMMAND_TYPE_DAILY_REWARD_CLAIM, &[]),
            ],
        )
        .await?;
        let values = results(&output)?;
        assert_eq!(values.len(), 4, "{end}");
        assert_eq!(values[2].2.get(1), Some(&1), "{values:?}");
        assert_eq!(values[3].2, [8, 2]);
        let details = wire::details::Details::decode(&values[1].2)?;
        let tier = details.days[usize::try_from(day - 1)?].premium;
        assert_eq!(
            [
                tier.cosmetic_tokens,
                tier.prey_wildcards,
                tier.temple_scrolls,
                tier.xp_percent,
                tier.xp_minutes,
                tier.bounty_tokens
            ],
            expected
        );
        let (lane_day, streak, spec): (i16, i32, serde_json::Value) = sqlx::query_as(
            "SELECT lane_day,streak_after,reward_spec FROM game_character_daily_reward_claims
             ORDER BY reset_epoch DESC LIMIT 1",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(lane_day, i16::try_from(day)?);
        assert_eq!(streak, i32::try_from(day)?);
        assert_eq!(spec, serde_json::json!(expected));
    }
    let totals: (i64, i64, i64, i64, i64) = sqlx::query_as(
        "SELECT (SELECT balance FROM game_character_cosmetic_tokens),
                (SELECT balance::bigint FROM game_account_prey_wildcards),
                (SELECT count(*) FROM game_item_bindings WHERE cause='DAILY_REWARD'),
                (SELECT count(*) FROM game_spell_item_temporal_schedules p
                 JOIN game_item_bindings b USING(item_instance_id) WHERE b.cause='DAILY_REWARD'),
                (SELECT balance FROM game_character_bounty_reroll_tokens)",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(totals, (16, 6, 2, 2, 10));
    let duration: i64 = sqlx::query_scalar(
        "SELECT extract(epoch FROM effect_expires_at-effect_started_at)::bigint
         FROM game_character_daily_reward_lines WHERE kind='XP_BOOST'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(duration, 3_600);
    Ok(())
}
