use super::*;

pub(super) const MODE: u8 = 5;

#[test]
fn daily_reward_promotion_dispatch_grants_sixty_minutes_once() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_mode(true, MODE).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "daily reward promotion PostgreSQL scenario panicked")?
        .map_err(Into::into)
}

pub(super) async fn claim_promoted_day_seven(
    h: &Harness,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let shrine = oteryn_protocol_oteryn::world_object::encode_use_intent(
        &oteryn_protocol_oteryn::world_object::WorldObjectTarget {
            placement: b"oteryn:placement/daily-reward-shrine".to_vec(),
            expected_revision: 0,
        },
    )
    .map_err(debug)?;
    for epoch in 1..=7 {
        reset(h, epoch, "2025-02-23").await?;
        let (_, output) = serve(
            owner,
            admitted,
            &[(2, &shrine), (wire::COMMAND_TYPE_DAILY_REWARD_CLAIM, &[])],
        )
        .await?;
        assert_eq!(
            results(&output)?.last().and_then(|row| row.2.get(1)),
            Some(&1)
        );
        if epoch == 6 {
            assert_eq!(displayed_minutes(owner, admitted).await?, 30);
        }
    }

    let promoted_grants: Vec<(i32, i32, i32, i16)> = sqlx::query_as(
        "SELECT (c.reward_spec->>4)::integer,l.quantity, \
         (extract(epoch FROM l.effect_expires_at-l.effect_started_at)/60)::integer,l.xp_boost_percent \
         FROM game_character_daily_reward_lines l JOIN game_character_daily_reward_claims c \
         ON c.transaction_id=l.claim_id WHERE c.lane_day=7 AND l.kind='XP_BOOST'",
    )
    .fetch_all(&h.pool)
    .await?;
    assert_eq!(promoted_grants, [(60, 60, 60, 50)]);
    let claim_command = admitted.continuity.next_command_id - 1;
    let (_, output) = serve_from(
        owner,
        admitted,
        claim_command,
        &[(wire::COMMAND_TYPE_DAILY_REWARD_CLAIM, &[])],
    )
    .await?;
    assert_eq!(
        results(&output)?.first().map(|row| row.1),
        Some(crate::foundation::CommandStatus::DuplicateReplay)
    );

    assert_eq!(displayed_minutes(owner, admitted).await?, 60);

    reset(h, 8, "2025-03-25").await?; // April 1, after the archived March window.
    assert_eq!(displayed_minutes(owner, admitted).await?, 30);
    let grants: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_character_daily_reward_lines WHERE kind='XP_BOOST'",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(grants, 2);

    // Only the fixture clock moves. Keep the Store line active at 31 minutes
    // so the existing real-kill check proves the daily line still adds 50%.
    sqlx::query("INSERT INTO game_character_store_xp_boosts(operation_id,character_id,world_id,starts_at,expires_at,percent) SELECT game_character_uuid_v7(),encode($1,'hex')::uuid,encode($2,'hex')::uuid,at,at+interval '45 minutes',50 FROM (SELECT max(effect_started_at) AS at FROM game_character_daily_reward_lines WHERE kind='XP_BOOST') starts")
        .bind(id(41).as_slice()).bind(id(42).as_slice()).execute(&h.pool).await?;
    let fence = admitted.item_fence.ok_or("current item fence")?;
    super::effects::kills_before_and_at_expiry(h, authority, true, Some(fence)).await
}

async fn reset(h: &Harness, epoch: i32, first_date: &str) -> TestResult {
    sqlx::query(
        "INSERT INTO game_world_reset_history(world_id,reset_epoch,reset_day,reset_at_unix_ms,committed_at_unix_ms) \
         SELECT encode($1,'hex')::uuid,$2,d-DATE '1970-01-01',at_ms,at_ms \
         FROM (SELECT d,extract(epoch FROM ((d+TIME '06:00') AT TIME ZONE 'Europe/Berlin'))::bigint*1000 AS at_ms \
         FROM (SELECT $3::text::date+($2::integer-1) AS d) dates) instants",
    )
    .bind(id(42).as_slice())
    .bind(epoch)
    .bind(first_date)
    .execute(&h.pool)
    .await?;
    Ok(())
}

async fn displayed_minutes(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<u8> {
    let (_, output) = serve(
        owner,
        admitted,
        &[
            (wire::details::COMMAND_TYPE, &[]),
            (wire::details::COMMAND_TYPE_WITH_RESET, &[]),
        ],
    )
    .await?;
    let mut minutes = Vec::new();
    for (_, status, bytes) in results(&output)? {
        assert_eq!(status, crate::foundation::CommandStatus::Accepted);
        let details = wire::details::Details::decode(&bytes)?;
        assert_eq!(details.days[6].free.xp_minutes, 30);
        assert_eq!(details.days[3].premium.xp_minutes, 30);
        minutes.push(details.days[6].premium.xp_minutes);
    }
    let [legacy, current] = minutes.as_slice() else {
        return Err("expected both reward details replies".into());
    };
    assert_eq!(legacy, current);
    Ok(*current)
}
