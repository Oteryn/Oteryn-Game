use super::*;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn admitted() -> Result<RottenBloodState, RottenBloodRefusal> {
    RottenBloodState::from_stored(4, 0, 0, false, false, None)
}

fn chain() -> Result<RottenBloodState, RottenBloodRefusal> {
    RottenBloodBoss::CHILDREN
        .into_iter()
        .try_fold(admitted()?, |state, boss| {
            state.defeat_child(boss, state.regular_taints(), true)
        })
}

#[test]
fn access_uses_two_tears_after_mission_and_never_grants_from_missing_resources() -> TestResult {
    assert_eq!(
        RottenBloodState::default().begin_mission(249),
        Err(RottenBloodRefusal::LevelTooLow)
    );
    let state = RottenBloodState::default().begin_mission(250)?;
    assert_eq!(
        state.begin_mission(250),
        Err(RottenBloodRefusal::WrongStage)
    );
    assert_eq!(
        state.sacrifice(0),
        Err(RottenBloodRefusal::MissingBloodyTear)
    );
    let first = state.sacrifice(2)?;
    assert_eq!((first.state.access_stage(), first.tears_consumed), (2, 0));
    let second = first.state.sacrifice(2)?;
    assert_eq!((second.state.access_stage(), second.tears_consumed), (3, 1));
    let third = second.state.sacrifice(1)?;
    assert_eq!((third.state.access_stage(), third.tears_consumed), (4, 1));
    assert!(third.state.access());
    assert_eq!(
        third.state.sacrifice(1),
        Err(RottenBloodRefusal::AlreadyHasAccess)
    );
    assert_eq!(RottenBloodState::default().sacrifice(1)?.tears_consumed, 1);
    Ok(())
}

#[test]
fn paid_access_obeys_donor_price_and_stage_and_does_not_consume_tears() -> TestResult {
    let state = RottenBloodState::default();
    assert_eq!(
        state.purchase_access(ACCESS_PRICE - 1),
        Err(RottenBloodRefusal::InsufficientGold)
    );
    let paid = state.purchase_access(ACCESS_PRICE)?;
    assert!(paid.state.access());
    assert_eq!((paid.gold_consumed, paid.tears_consumed), (5_000_000, 0));
    assert_eq!(
        paid.state.purchase_access(ACCESS_PRICE),
        Err(RottenBloodRefusal::WrongStage)
    );
    Ok(())
}

#[test]
fn entry_checks_level_access_and_four_distinct_children() -> TestResult {
    assert_eq!(
        admitted()?.validate_entry(249, None),
        Err(RottenBloodRefusal::LevelTooLow)
    );
    assert_eq!(
        RottenBloodState::default().validate_entry(250, None),
        Err(RottenBloodRefusal::AccessMissing)
    );
    assert_eq!(
        admitted()?.validate_entry(250, Some(RottenBloodBoss::Bakragore)),
        Err(RottenBloodRefusal::ChainIncomplete)
    );
    assert!(
        chain()?
            .validate_entry(250, Some(RottenBloodBoss::Bakragore))
            .is_ok()
    );
    assert!(
        admitted()?
            .validate_entry(250, Some(RottenBloodBoss::Murcion))
            .is_ok()
    );
    Ok(())
}

#[test]
fn child_gain_requires_damage_party_minimum_and_distinct_boss() -> TestResult {
    let boss = RottenBloodBoss::Ichgahal;
    assert_eq!(
        admitted()?.defeat_child(boss, 0, false),
        Err(RottenBloodRefusal::NoDamageContribution)
    );
    assert_eq!(
        admitted()?.defeat_child(boss, 1, true),
        Err(RottenBloodRefusal::InvalidPartyMinimum)
    );
    assert_eq!(
        admitted()?.defeat_child(boss, 5, true),
        Err(RottenBloodRefusal::InvalidPartyMinimum)
    );
    assert_eq!(
        RottenBloodState::default().defeat_child(boss, 0, true),
        Err(RottenBloodRefusal::AccessMissing)
    );
    assert_eq!(
        admitted()?.defeat_child(RottenBloodBoss::Bakragore, 0, true),
        Err(RottenBloodRefusal::NotChildBoss)
    );
    let first = admitted()?.defeat_child(boss, 0, true)?;
    assert_eq!(
        first.defeat_child(boss, 1, true),
        Err(RottenBloodRefusal::AlreadyHoldsChild)
    );
    assert_eq!(
        first.defeat_child(RottenBloodBoss::Murcion, 0, true),
        Err(RottenBloodRefusal::PartyMinimumMismatch)
    );
    assert_eq!(first.regular_taints(), 1);
    assert!(first.holds_child(boss));
    assert!(!first.holds_child(RottenBloodBoss::Bakragore));
    assert_eq!(chain()?.child_flags(), 15);
    let at_limit = RottenBloodState::from_stored(4, 4, 0, false, false, None)?;
    assert_eq!(
        at_limit.defeat_child(boss, 4, true),
        Err(RottenBloodRefusal::TaintLimit)
    );
    Ok(())
}

#[test]
fn final_reward_once_regular_reset_final_taint_survives_repeated_cycles() -> TestResult {
    assert_eq!(
        RottenBloodState::default().defeat_final(true),
        Err(RottenBloodRefusal::AccessMissing)
    );
    assert_eq!(
        admitted()?.defeat_final(true),
        Err(RottenBloodRefusal::ChainIncomplete)
    );
    assert_eq!(
        chain()?.defeat_final(false),
        Err(RottenBloodRefusal::NoDamageContribution)
    );
    let first = chain()?.defeat_final(true)?;
    assert!(first.grant_base_outfit);
    assert!(first.state.final_taint());
    assert!(first.state.completed());
    assert_eq!(
        (first.state.regular_taints(), first.state.child_flags()),
        (0, 0)
    );
    let next_chain = RottenBloodBoss::CHILDREN
        .into_iter()
        .try_fold(first.state, |state, boss| {
            state.defeat_child(boss, state.regular_taints(), true)
        })?;
    assert!(!next_chain.defeat_final(true)?.grant_base_outfit);
    Ok(())
}

#[test]
fn first_cleanse_immediate_subsequent_clock_from_success_and_modes_are_distinct() -> TestResult {
    let state = RottenBloodState::from_stored(4, 2, 3, true, true, None)?;
    let cleaned = state.cleanse(CleanseMode::Regular, 10)?;
    assert!(cleaned.final_taint());
    assert_eq!((cleaned.child_flags(), cleaned.regular_taints()), (0, 0));
    assert_eq!(cleaned.last_cleanse_at(), Some(10));
    assert_eq!(
        cleaned.cleanse(CleanseMode::Regular, 11),
        Err(RottenBloodRefusal::NothingToCleanse)
    );
    assert_eq!(
        cleaned.cleanse(CleanseMode::All, 10 + CLEANSE_COOLDOWN_SECONDS - 1),
        Err(RottenBloodRefusal::CleanseCooldown {
            available_at: 10 + CLEANSE_COOLDOWN_SECONDS
        })
    );
    let all = cleaned.cleanse(CleanseMode::All, 10 + CLEANSE_COOLDOWN_SECONDS)?;
    assert!(!all.final_taint());
    assert!(all.completed());
    assert_eq!(
        all.cleanse(CleanseMode::All, u64::MAX),
        Err(RottenBloodRefusal::NothingToCleanse)
    );
    Ok(())
}

#[test]
fn stored_state_and_timestamp_bounds_are_checked() -> TestResult {
    for (stage, taints, flags) in [(6, 0, 0), (4, 5, 0), (4, 0, 16)] {
        assert_eq!(
            RottenBloodState::from_stored(stage, taints, flags, false, false, None),
            Err(RottenBloodRefusal::InvalidState)
        );
    }
    let state = RottenBloodState::from_stored(4, 1, 1, false, false, Some(u64::MAX))?;
    assert_eq!(
        state.cleanse(CleanseMode::Regular, u64::MAX),
        Err(RottenBloodRefusal::InvalidTimestamp)
    );
    let mut encoded = serde_json::to_value(admitted()?)?;
    encoded["regular_taints"] = 255.into();
    assert!(serde_json::from_value::<RottenBloodState>(encoded).is_err());
    Ok(())
}

#[test]
fn failed_final_clears_shared_selected_pair_and_preserves_final_taint() -> TestResult {
    for taints in 0..=4 {
        let state = RottenBloodState::from_stored(4, taints, 15, true, true, None)?;
        let failed = state.fail_final(0b0101)?;
        assert_eq!(failed.regular_taints(), taints.saturating_sub(2));
        assert!(failed.final_taint());
        assert_eq!(failed.child_flags(), 0b1010);
    }
    for flags in [0, 1, 7, 15, 16, 17] {
        assert_eq!(
            chain()?.fail_final(flags),
            Err(RottenBloodRefusal::InvalidFailureSelection)
        );
    }
    assert_eq!(
        admitted()?.fail_final(3),
        Err(RottenBloodRefusal::InvalidFailureSelection)
    );
    Ok(())
}

#[test]
fn creature_key_lookup_excludes_echoes_and_unrelated_deaths() -> TestResult {
    for boss in RottenBloodBoss::CHILDREN
        .into_iter()
        .chain([RottenBloodBoss::Bakragore])
    {
        assert_eq!(
            RottenBloodBoss::from_creature_key(boss.creature_key()),
            Some(boss)
        );
    }
    assert_eq!(RottenBloodBoss::from_creature_key("echo_of_ichgahal"), None);
    assert_eq!(RottenBloodBoss::from_creature_key("rat"), None);
    Ok(())
}

#[test]
fn catalogue_mission_and_altar_keep_existing_quest_receipt_owner() -> TestResult {
    use super::super::{QuestRefusal, loader, world_callbacks};
    let loaded =
        loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let mission = catalogue.transition(START).ok_or("mission transition")?;
    let changes = catalogue
        .evaluate(mission, &Default::default(), 0)
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(changes.len(), 2);
    assert!(
        changes
            .iter()
            .all(|change| change.before == 0 && change.after == 1)
    );
    let altar = catalogue.transition(SACRIFICE).ok_or("altar transition")?;
    for stage in 0..=3 {
        let tracks = std::collections::BTreeMap::from([(ACCESS_TRACK.to_owned(), stage)]);
        let change = catalogue
            .evaluate(altar, &tracks, 0)
            .map_err(|error| format!("{error:?}"))?;
        assert_eq!((change[0].before, change[0].after), (stage, stage + 1));
    }
    assert_eq!(
        catalogue.evaluate(
            altar,
            &std::collections::BTreeMap::from([(ACCESS_TRACK.into(), 4)]),
            0
        ),
        Err(QuestRefusal::StageMismatch)
    );
    let callback = world_callbacks::load()?
        .into_iter()
        .find(|c| c.key == SACRIFICE)
        .ok_or("altar callback")?;
    assert_eq!(callback.selector.appearance, Some(43891));
    assert_eq!(
        callback.item_on_target.ok_or("altar item input")?.item,
        "oteryn:item.tibia.i32594"
    );
    assert_eq!(
        loaded.source_track(ACCESS_TRACK),
        Some("kv:rotten-blood-quest:access")
    );
    let acknowledge = catalogue
        .transition(ACKNOWLEDGE_CHAIN)
        .ok_or("chain acknowledgement")?;
    let change = catalogue
        .evaluate(
            acknowledge,
            &std::collections::BTreeMap::from([(ACCESS_TRACK.into(), 4)]),
            0,
        )
        .map_err(|error| format!("{error:?}"))?;
    assert_eq!(change.len(), 1);
    assert_eq!((change[0].before, change[0].after), (4, 5));
    for wrong in [0, 1, 2, 3, 5] {
        assert_eq!(
            catalogue.evaluate(
                acknowledge,
                &std::collections::BTreeMap::from([(ACCESS_TRACK.into(), wrong)]),
                0
            ),
            Err(QuestRefusal::StageMismatch)
        );
    }
    Ok(())
}
