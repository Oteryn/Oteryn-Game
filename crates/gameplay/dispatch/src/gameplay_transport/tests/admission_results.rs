/// Runs `test` on a current-thread runtime whose clock is paused, so backoffs auto-advance.
fn run_paused(test: impl std::future::Future<Output = ()>) {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .expect("runtime")
        .block_on(test);
}

/// A busy durability holder fails a pass at once with nothing written. Fresh admission
/// retries only that transient refusal; rejections and classified refusals are never
/// retried. The backoff carries the per-attempt jitter.
#[test]
fn fresh_admission_retries_only_transient_noncommits() {
    run_paused(async {
        let session = GameSessionId::decode(&uuid_v7(0x40)).expect("session");
        let budget = AdmissionRetryBudget::new(&session);
        assert!(!budget.retries_unavailable(AdmissionRefusal::Rejected).await);
        assert!(
            !budget
                .retries_unavailable(AdmissionRefusal::Classified(
                    FoundationProtocolError::AdmissionGrantReplayed
                ))
                .await
        );
        assert!(
            !budget
                .retries_unavailable(AdmissionRefusal::PremiumRequired)
                .await
        );
        assert!(budget.retries_unavailable(AdmissionRefusal::Unavailable).await);
        let low = GameSessionId::decode(&uuid_v7(0x40)).expect("low session");
        let high = GameSessionId::decode(&uuid_v7(0x7f)).expect("high session");
        assert_eq!(fresh_admission_backoff(&low), RECONCILE_BACKOFF);
        assert_eq!(
            fresh_admission_backoff(&high),
            RECONCILE_BACKOFF + Duration::from_millis(63)
        );
    });
}

/// One admission step against a holder that background passes keep busy for `busy_for`:
/// it succeeds once the holder is free, and otherwise asks the attempt's budget.
async fn admit_against_busy_holder(busy_for: Duration) -> Result<Duration, AdmissionRefusal> {
    let session = GameSessionId::decode(&uuid_v7(0x7f)).expect("session");
    let start = tokio::time::Instant::now();
    let budget = AdmissionRetryBudget::new(&session);
    loop {
        if start.elapsed() >= busy_for {
            return Ok(start.elapsed());
        }
        if !budget
            .retries_unavailable(AdmissionRefusal::Unavailable)
            .await
        {
            return Err(AdmissionRefusal::Unavailable);
        }
    }
}

/// DEFLAKE-2: a holder busy for longer than the former three rounds (two backoffs, at most
/// 526 ms) no longer refuses the attempt; one still busy at the end of the root recovery
/// window does, before the window ends.
#[test]
fn admission_survives_a_busy_holder_within_the_root_recovery_window() {
    run_paused(async {
        let session = GameSessionId::decode(&uuid_v7(0x7f)).expect("session");
        let former_bound = fresh_admission_backoff(&session) * (FRESH_ADMISSION_ROUNDS - 1);
        for busy_for in [
            former_bound + Duration::from_millis(1),
            Duration::from_secs(2),
            Duration::from_secs(4),
        ] {
            let admitted = admit_against_busy_holder(busy_for).await;
            assert!(
                matches!(admitted, Ok(at) if at >= busy_for && at < crate::durability::ROOT_RECOVERY_WINDOW),
                "busy for {busy_for:?}: {admitted:?}"
            );
        }
        let start = tokio::time::Instant::now();
        assert_eq!(
            admit_against_busy_holder(crate::durability::ROOT_RECOVERY_WINDOW).await,
            Err(AdmissionRefusal::Unavailable)
        );
        assert!(start.elapsed() < crate::durability::ROOT_RECOVERY_WINDOW);
    });
}

/// DEFLAKE-2: a round left stale by concurrent owner evidence keeps its bound of
/// `FRESH_ADMISSION_ROUNDS` rounds, however much of the window is left, and busy-holder
/// retries do not spend it.
#[test]
fn stale_rounds_keep_their_round_bound() {
    run_paused(async {
        let session = GameSessionId::decode(&uuid_v7(0x40)).expect("session");
        let mut budget = AdmissionRetryBudget::new(&session);
        for _ in 0..4 {
            assert!(budget.retries_unavailable(AdmissionRefusal::Unavailable).await);
        }
        for _ in 1..FRESH_ADMISSION_ROUNDS {
            assert!(budget.retries_stale().await);
        }
        assert!(!budget.retries_stale().await);
        assert!(budget.retries_unavailable(AdmissionRefusal::Unavailable).await);
    });
}

/// DEFLAKE-2: a silent admission or resume refusal leaves one closed-schema operator line
/// naming its kind alone; a refusal that answers the client with a frame leaves none.
#[test]
fn silent_admission_refusals_name_their_kind_alone() {
    use connection::admission_refused_event;
    assert_eq!(
        admission_refused_event(AdmissionRefusal::Unavailable),
        Some("event=admission_refused kind=unavailable")
    );
    for refusal in [AdmissionRefusal::Rejected, AdmissionRefusal::PremiumRequired] {
        assert_eq!(
            admission_refused_event(refusal),
            Some("event=admission_refused kind=rejected")
        );
    }
    for refusal in [
        AdmissionRefusal::RecoveryCapabilityRequired,
        AdmissionRefusal::Classified(FoundationProtocolError::AdmissionGrantReplayed),
    ] {
        assert_eq!(admission_refused_event(refusal), None);
    }
}

/// N8 (ARCH-LOGIN-FIRST-PACKETS-V1 §1.2): each fresh verifier refusal carries its FND-04A
/// row; a recovery error stays frameless.
#[test]
fn fresh_grant_refusals_carry_their_n8_code() {
    use Fnd04ConsumerError as F;
    let rows = [
        (F::FreshMalformed, 1100),
        (F::FreshAuthenticationFailed, 1101),
        (F::FreshBindingMismatch, 1102),
        (F::FreshNotYetValid, 1103),
        (F::FreshExpired, 1104),
        (F::FreshSecurityStateRevoked, 1107),
        (F::FreshSecurityEvidenceStale, 1108),
        (F::FreshRouteStale, 1109),
        (F::FreshRuntimeStale, 1110),
        (F::FreshWorldStale, 1111),
        (F::FreshRevisionUnsupported, 1112),
        (F::FreshAccountCharacterConflict, 1113),
    ];
    for (error, code) in rows {
        assert!(
            matches!(
                fresh_grant_refusal(error),
                AdmissionRefusal::Classified(refusal) if refusal.code() == code
            ),
            "{error:?}"
        );
    }
    assert_eq!(
        fresh_grant_refusal(F::RecoveryExpired),
        AdmissionRefusal::Rejected
    );
}

/// ACH-NOTIFY-1: the earned delta takes its name from the catalogue and its watermark from
/// the fact keys read in the granting transaction; a key the catalogue lacks sends none.
#[test]
fn achievement_earned_names_the_grant_and_sums_the_account_facts() -> Result<(), Box<dyn Error>>
{
    use crate::achievement_catalogue::AchievementCatalogue;
    use crate::durability::reward_claim_mint::GrantedAchievementNotice;
    use oteryn_protocol_oteryn::achievement_notices::AchievementWatermark;
    let record = |slug: &str, points: u32| {
        format!(
            r#"{{"identity":{{"family":"Achievement","key":"oteryn:achievement/{slug}","revision":"1"}},"name":"N {slug}","description":"D","grade":1,"points":{points},"secret":false}}"#
        )
    };
    let shard = format!(
        r#"{{"family":"Achievement","records":[{},{}]}}"#,
        record("first", 3),
        record("second", 5),
    );
    let catalogue =
        AchievementCatalogue::from_shards(&[&shard]).map_err(|error| format!("{error:?}"))?;
    let notice = |key: &str, keys: &[&str]| GrantedAchievementNotice {
        achievement_key: format!("oteryn:achievement/{key}"),
        account_fact_keys: keys
            .iter()
            .map(|key| format!("oteryn:achievement/{key}"))
            .collect(),
    };
    let earned = achievement_earned(&catalogue, &notice("second", &["first", "second"]))
        .ok_or("earned")?;
    assert_eq!(earned.key, "oteryn:achievement/second");
    assert_eq!(earned.name, "N second");
    assert_eq!(
        earned.watermark,
        AchievementWatermark {
            fact_count: 2,
            total_points: 8,
        }
    );
    assert!(achievement_earned(&catalogue, &notice("absent", &["absent"])).is_none());
    assert!(achievement_earned(&catalogue, &notice("first", &["first", "absent"])).is_none());

    // ACH-NOTIFY-2: the three states. An underivable `Granted` and an `Unknown` read are both
    // `Unknown` with one operator event each; nothing granted is `NoneEarned` and logs nothing.
    use crate::durability::reward_claim_mint::GrantNotice;
    let mut events = Vec::new();
    let mut log = |line: &str| events.push(line.to_owned());
    assert_eq!(
        earned_notice(&catalogue, GrantNotice::None, &mut log),
        connection::EarnedNotice::NoneEarned
    );
    assert_eq!(
        earned_notice(
            &catalogue,
            GrantNotice::Granted(notice("second", &["first", "second"])),
            &mut log
        ),
        connection::EarnedNotice::Earned(earned)
    );
    for unknown in [
        GrantNotice::Granted(notice("first", &["first", "absent"])),
        GrantNotice::Unknown,
    ] {
        assert_eq!(
            earned_notice(&catalogue, unknown, &mut log),
            connection::EarnedNotice::Unknown
        );
    }
    assert_eq!(events.len(), 2);
    assert!(
        events
            .iter()
            .all(|line| line.starts_with("event=achievement_notice_unknown level=error"))
    );
    Ok(())
}

/// Display contract §3.3, §4.3: a valid page encodes; a row over its byte bounds fails closed
/// with `PAYLOAD_LIMIT_EXCEEDED`; an unknown key with `ACCOUNT_DATA_INTEGRITY` and one operator
/// event (owner decision 2026-09-30); a malformed row is a plain `REJECTED`. Nothing is
/// truncated or skipped, and only the integrity fault logs.
#[test]
fn account_achievements_reply_encodes_or_fails_closed() -> Result<(), Box<dyn Error>> {
    use crate::achievement_catalogue::AchievementCatalogue;
    use crate::durability::account_achievement::EarnedAchievement;
    use oteryn_protocol_oteryn::account_achievements::decode_account_achievements_result;
    let record = |slug: &str, name: &str, grade: u32| {
        format!(
            r#"{{"identity":{{"family":"Achievement","key":"oteryn:achievement/{slug}","revision":"1"}},"name":"{name}","description":"D","grade":{grade},"points":2,"secret":false}}"#
        )
    };
    let shard = format!(
        r#"{{"family":"Achievement","records":[{},{},{}]}}"#,
        record("fits", "Fits", 1),
        record("long", &"n".repeat(65), 1),
        record("gradeless", "Gradeless", 0),
    );
    let catalogue =
        AchievementCatalogue::from_shards(&[&shard]).map_err(|error| format!("{error:?}"))?;
    let facts = |slugs: &[&str]| -> Vec<EarnedAchievement> {
        slugs
            .iter()
            .map(|slug| EarnedAchievement {
                achievement_key: format!("oteryn:achievement/{slug}"),
                earned_at_unix_ms: 7,
            })
            .collect()
    };
    let request = |page| AccountAchievementsRequest {
        account_id: [0x11; 16],
        page,
        game_session_id: [0x22; 16],
        command_id: 9,
    };
    let reply = |catalogue: &AchievementCatalogue, slugs: &[&str], page| {
        let mut events = Vec::new();
        let reply =
            account_achievements_reply(catalogue, &facts(slugs), &request(page), &mut |line| {
                events.push(line.to_owned())
            });
        (reply, events)
    };
    let (AccountAchievementsReply::Page(payload), events) = reply(&catalogue, &["fits"], 0)
    else {
        return Err("a valid page encodes".into());
    };
    let page =
        decode_account_achievements_result(&payload).map_err(|error| format!("{error:?}"))?;
    assert_eq!(
        (page.total_points, page.fact_count, page.rows.len()),
        (2, 1, 1)
    );
    assert!(events.is_empty());
    let integrity_event = crate::achievement_catalogue::integrity_fault_event(
        &request(0),
        "oteryn:achievement/absent",
    );
    for (slugs, expected, logged) in [
        (
            &["fits", "long"][..],
            AccountAchievementsReply::Terminal(FoundationProtocolError::PayloadLimitExceeded),
            &[][..],
        ),
        (
            &["fits", "gradeless"][..],
            AccountAchievementsReply::Rejected,
            &[][..],
        ),
        (
            &["fits", "absent"][..],
            AccountAchievementsReply::Terminal(FoundationProtocolError::AccountDataIntegrity),
            &[integrity_event.as_str()][..],
        ),
    ] {
        assert_eq!(
            reply(&catalogue, slugs, 0),
            (
                expected,
                logged.iter().map(|line| (*line).to_owned()).collect()
            ),
            "{slugs:?}"
        );
    }
    // Only the reply that would carry the over-bound row fails; its grade puts it on page 1.
    let many: Vec<String> = (0..70).map(|index| format!("k{index:02}")).collect();
    let records: Vec<String> = many.iter().map(|slug| record(slug, "A", 1)).collect();
    let shard = format!(
        r#"{{"family":"Achievement","records":[{},{}]}}"#,
        records.join(","),
        record("long", &"n".repeat(65), 4),
    );
    let catalogue =
        AchievementCatalogue::from_shards(&[&shard]).map_err(|error| format!("{error:?}"))?;
    let mut slugs: Vec<&str> = many.iter().map(String::as_str).collect();
    slugs.push("long");
    assert!(matches!(
        reply(&catalogue, &slugs, 0).0,
        AccountAchievementsReply::Page(_)
    ));
    assert_eq!(
        reply(&catalogue, &slugs, 1).0,
        AccountAchievementsReply::Terminal(FoundationProtocolError::PayloadLimitExceeded)
    );
    Ok(())
}

// USE-WIRE-V1 reach (#162 5868482467). SEAM_EVIDENCE: gameplay_transport/mod.rs
// use_object_reachable unit coverage (a genuine TOO_FAR case cannot be reached through real
// movement in the qualified native entry room; see `use_object_reachable`'s own doc comment).
