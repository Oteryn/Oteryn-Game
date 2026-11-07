use super::*;

pub(super) async fn verify(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    player: (ExactActorRef, GameSessionId),
    h: &Harness,
) -> TestResult {
    let started = std::time::Instant::now();
    let before = scrolls(h).await?;
    let catalysts_before = catalysts(h).await?;
    {
        let mut runtime = owner.runtime.lock().await;
        let warden = raid(&mut runtime, player, 1_000_000, 100)?;
        runtime
            .drive_echo_wardens(1_629_999, |_| true)
            .map_err(debug)?;
        assert!(runtime.companion_snapshot(warden).is_ok());
        runtime
            .drive_echo_wardens(1_630_000, |_| true)
            .map_err(debug)?;
        assert!(
            runtime.companion_snapshot(warden).is_err(),
            "live Warden expires without a kill"
        );
    }
    drain(owner, player.1).await?;
    assert_eq!(scrolls(h).await?, before, "expiry cannot mint a scroll");
    assert_eq!(
        catalysts(h).await?,
        catalysts_before,
        "expiry cannot mint a catalyst"
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            "SELECT count(*) FROM game_character_echo_warden_kill_receipts"
        )
        .fetch_one(&h.pool)
        .await?,
        4,
        "expiry cannot credit a Warden kill"
    );
    eprintln!(
        "ECHO_EXPIRY POSTGRES_CASE_COMPLETED duration={:?}: live Warden expiry, unchanged scroll/catalyst custody and kill receipts",
        started.elapsed()
    );
    Ok(())
}
