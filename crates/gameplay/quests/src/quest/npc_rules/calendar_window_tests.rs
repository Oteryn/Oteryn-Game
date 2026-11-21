use super::*;

fn catalog(start: [u8; 2], end: [u8; 2], within: bool) -> Result<NpcRuleCatalog, NpcRuleError> {
    let mut document: serde_json::Value =
        serde_json::from_str(include_str!("test-fixtures/hero_greeting.json"))
            .map_err(|e| NpcRuleError::Json(e.to_string()))?;
    document["rules"][0]["guards"] = serde_json::json!([{
        "kind": "calendar_window", "start_month": start[0], "start_day": start[1],
        "end_month": end[0], "end_day": end[1], "within": within
    }]);
    NpcRuleCatalog::from_json_documents(&[&document.to_string()])
}

fn eligible(catalog: &NpcRuleCatalog, epoch: i64) -> Result<bool, NpcRuleError> {
    let tracks = BTreeMap::new();
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    Ok(catalog
        .plan_storage(&NpcRuleInput {
            player_name: None,
            npc: "oteryn:npc.strange_pipe",
            trigger: NpcRuleTrigger::Greet,
            message: "hi",
            topic: 0,
            tracks: &tracks,
            inventory: &inventory,
            now_epoch_seconds: epoch,
            calendar_month: 8,
            premium_current: false,
            level: 100,
            vocation: "oteryn:vocation.knight",
            sex: 0,
            outfits: &outfits,
        })?
        .is_some())
}

#[test]
fn calendar_window_first_dragon_inclusive_utc_boundaries_and_inverse() -> Result<(), NpcRuleError> {
    let inside = catalog([1, 14], [2, 12], true)?;
    let outside = catalog([1, 14], [2, 12], false)?;
    // 2024-01-14, 2024-02-13, and the same window in 2025; use UTC epoch seconds.
    for (epoch, expected) in [
        (1_705_190_400 - 1, false),
        (1_705_190_400, true),
        (1_707_782_400 - 1, true),
        (1_707_782_400, false),
        (1_736_812_800, true),
        (1_739_404_800, false),
    ] {
        assert_eq!(eligible(&inside, epoch)?, expected, "epoch={epoch}");
        assert_eq!(
            eligible(&outside, epoch)?,
            !expected,
            "inverse epoch={epoch}"
        );
    }
    Ok(())
}

#[test]
fn calendar_window_wraps_year_and_handles_leap_day() -> Result<(), NpcRuleError> {
    let winter = catalog([12, 25], [1, 6], true)?;
    assert!(eligible(&winter, -1)?); // 1969-12-31, Euclidean epoch conversion.
    assert!(eligible(&winter, 0)?);
    assert!(!eligible(&winter, 1_705_190_400)?);
    let leap = catalog([2, 29], [2, 29], true)?;
    assert!(eligible(&leap, 1_709_164_800)?);
    assert!(!eligible(&leap, 1_709_251_200)?);
    assert!(!eligible(&leap, 1_677_628_800)?);
    Ok(())
}

#[test]
fn calendar_window_refuses_invalid_month_day_pairs() {
    for invalid in [[0, 1], [13, 1], [1, 0], [2, 30], [4, 31], [12, 32]] {
        assert!(catalog(invalid, [2, 12], true).is_err());
        assert!(catalog([1, 14], invalid, true).is_err());
    }
}

#[test]
fn calendar_window_tamoril_preserves_acceptance_resets_and_blocks_every_outside_branch()
-> Result<(), NpcRuleError> {
    let catalogue = NpcRuleCatalog::runtime()?;
    let program = catalogue
        .program("oteryn:quest.the_first_dragon_quest")
        .ok_or_else(|| NpcRuleError::Invalid("Tamoril programme".into()))?;
    let catalogue =
        NpcRuleCatalog::from_json_documents(&[&serde_json::to_string(&program.document)
            .map_err(|e| NpcRuleError::Json(e.to_string()))?])?;
    let tracks: BTreeMap<_, _> = program
        .document
        .tracks
        .iter()
        .map(|t| (t.key.clone(), 1))
        .collect();
    let inventory = NpcInventorySnapshot::default();
    let outfits = BTreeMap::new();
    let input = |epoch, topic| NpcRuleInput {
        player_name: None,
        npc: "oteryn:npc.tamoril",
        trigger: NpcRuleTrigger::Say,
        message: "yes",
        topic,
        tracks: &tracks,
        inventory: &inventory,
        now_epoch_seconds: epoch,
        calendar_month: 8,
        premium_current: false,
        level: 100,
        vocation: "oteryn:vocation.knight",
        sex: 0,
        outfits: &outfits,
    };
    let accepted = catalogue
        .plan_storage(&input(1_705_190_400, 6))?
        .ok_or_else(|| NpcRuleError::Invalid("inside acceptance".into()))?;
    assert_eq!(accepted.next_topic, 0);
    assert_eq!(accepted.track_changes.len(), 3);
    for suffix in ["dragon_counter", "gelidrazah_access", "secrets_counter"] {
        assert!(
            accepted
                .track_changes
                .iter()
                .any(|change| change.key.ends_with(suffix)
                    && change.before == 1
                    && change.after == 0)
        );
    }
    for topic in 0..=6 {
        let refused = catalogue
            .plan_storage(&input(1_707_782_400, topic))?
            .ok_or_else(|| NpcRuleError::Invalid("outside response".into()))?;
        assert!(refused.track_changes.is_empty());
        assert!(refused.effects.is_empty());
        assert!(
            refused
                .selected_rule_keys
                .iter()
                .all(|key| key.ends_with("outside-event"))
        );
    }
    // A first acceptance also initializes Questline and ChestCounter; the three
    // counters above remain unconditional, as in Crystal tamoril.lua:96-107.
    let initial: BTreeMap<_, _> = program
        .document
        .tracks
        .iter()
        .map(|t| (t.key.clone(), -1))
        .collect();
    let plan = catalogue
        .plan_storage(&NpcRuleInput {
            player_name: None,
            tracks: &initial,
            ..input(1_705_190_400, 6)
        })?
        .ok_or_else(|| NpcRuleError::Invalid("first acceptance".into()))?;
    assert_eq!(plan.track_changes.len(), 5);
    Ok(())
}

#[test]
fn calendar_window_current_year_projection_uses_utc_epoch() -> Result<(), NpcRuleError> {
    for (epoch, year, month, day) in [
        (-1, 1969, 12, 31),
        (0, 1970, 1, 1),
        (1_704_067_199, 2023, 12, 31),
        (1_704_067_200, 2024, 1, 1),
        (1_709_164_800, 2024, 2, 29),
        (1_735_689_600, 2025, 1, 1),
    ] {
        let date = NpcUtcCalendar::from_epoch(epoch)?;
        assert_eq!((date.year, date.month, date.day), (year, month, day));
    }
    assert!(NpcUtcCalendar::from_epoch(i64::MAX).is_err());
    assert!(NpcUtcCalendar::from_epoch(i64::MIN).is_err());
    Ok(())
}
