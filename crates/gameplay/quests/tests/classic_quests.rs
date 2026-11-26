use oteryn_gameplay_quests::quest::{QuestRefusal, loader, reward_callbacks, world_callbacks};
use std::collections::BTreeMap;

const CRYSTAL: &str = "oteryn:quest-progress/quest/u10_70/the_cursed_crystal/";

#[test]
fn cursed_crystal_world_rewards_require_both_bones_and_an_expired_clock() -> Result<(), String> {
    let loaded =
        loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let callbacks = world_callbacks::load_for(catalogue)?;
    let reward = |suffix: &str| {
        let key = format!("oteryn:quest-callback/cursed-crystal/{suffix}");
        let callback = callbacks
            .iter()
            .find(|row| row.key == key)
            .ok_or("callback")?;
        let transition = catalogue.transition(&key).ok_or("world transition")?;
        Ok::<_, &str>((callback, transition))
    };
    let mut tracks = BTreeMap::from([(format!("{CRYSTAL}questline"), 0)]);
    let (blood, draw_blood) = reward("blood")?;
    assert_eq!(blood.selector.action, Some(40002));
    assert_eq!(
        blood
            .effects
            .item_reward
            .as_ref()
            .ok_or("blood reward")?
            .item,
        "oteryn:item.tibia.i21504"
    );
    for (source, track, item) in [
        ("paper", "sheet_of_paper", "2820"),
        ("bell", "small_crystal_bell", "21554"),
    ] {
        assert_eq!(
            catalogue.evaluate(draw_blood, &tracks, 100),
            Err(QuestRefusal::StageMismatch)
        );
        let (callback, transition) = reward(source)?;
        assert_eq!(callback.selector.action, Some(40001));
        let reward = callback
            .effects
            .item_reward
            .as_ref()
            .ok_or("bones reward")?;
        assert_eq!(reward.item, format!("oteryn:item.tibia.i{item}"));
        assert_eq!(reward.count, 1);
        if source == "paper" {
            assert!(
                reward
                    .text
                    .as_ref()
                    .and_then(|text| text.root())
                    .is_some_and(|text| text.contains("bell"))
            );
        }
        let changes = catalogue
            .evaluate(transition, &tracks, 100)
            .map_err(|error| error.code())?;
        assert!(
            changes
                .iter()
                .any(|change| change.track == format!("{CRYSTAL}{track}") && change.after == 1)
        );
        tracks.extend(
            changes
                .into_iter()
                .map(|change| (change.track, change.after)),
        );
        assert_eq!(
            catalogue.evaluate(transition, &tracks, 100),
            Err(QuestRefusal::StageMismatch)
        );
    }
    tracks.insert(format!("{CRYSTAL}time"), 100);
    assert_eq!(
        catalogue.evaluate(draw_blood, &tracks, 100),
        Err(QuestRefusal::StageMismatch)
    );
    let changes = catalogue
        .evaluate(draw_blood, &tracks, 101)
        .map_err(|error| error.code())?;
    tracks.extend(
        changes
            .into_iter()
            .map(|change| (change.track, change.after)),
    );
    assert_eq!(tracks.get(&format!("{CRYSTAL}oneeyedjoe")), Some(&1));
    assert_eq!(
        catalogue.evaluate(draw_blood, &tracks, 101),
        Err(QuestRefusal::StageMismatch)
    );
    Ok(())
}

#[test]
fn cursed_crystal_shrine_advances_only_the_started_mission() -> Result<(), String> {
    let loaded =
        loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let transition = catalogue
        .transition("oteryn:quest-callback/cursed-crystal/inscription")
        .ok_or("inscription")?;
    let track = format!("{CRYSTAL}questline");
    for (before, after) in [(-1, -1), (0, 1), (1, 1), (2, 2), (3, 3), (4, 4)] {
        let changes = catalogue
            .evaluate(transition, &BTreeMap::from([(track.clone(), before)]), 100)
            .map_err(|error| error.code())?;
        assert_eq!(changes[0].after, after);
    }
    Ok(())
}

#[test]
fn eroth_report_binds_the_four_real_caches_and_both_rewards() -> Result<(), String> {
    let loaded =
        loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let callbacks = reward_callbacks::offered_for(catalogue)?;
    let report = callbacks
        .iter()
        .find(|row| row.key == "oteryn:quest-reward/elvenbane-mirror-report")
        .ok_or("Eroth report")?;
    let claims = ["bp1", "bp2", "dwarven_shield", "morning_star"]
        .map(|suffix| format!("oteryn:reward-claim.quest.u8_1.to_blind_the_enemy.{suffix}"));
    assert_eq!(report.required_claims, claims);
    let reward = report.give.as_ref().ok_or("report reward")?;
    assert_eq!(
        (reward.item.as_str(), reward.count),
        ("oteryn:item.tibia.i3082", 1)
    );
    assert_eq!(reward.additional.len(), 1);
    assert_eq!(
        (
            reward.additional[0].item.as_str(),
            reward.additional[0].count
        ),
        ("oteryn:item.tibia.i3035", 10)
    );
    let completion = catalogue
        .transition(&report.key)
        .ok_or("published completion")?;
    assert!(completion.completes);
    let mut progress = BTreeMap::from([(
        "oteryn:quest-progress/quest/u8_1/to_blind_the_enemy/questline".into(),
        1,
    )]);
    assert_eq!(
        catalogue.evaluate(completion, &progress, 0),
        Err(QuestRefusal::StageMismatch)
    );
    let mirror = world_callbacks::load_for(catalogue)?
        .into_iter()
        .find(|callback| {
            callback.quest == report.quest
                && callback.trigger == world_callbacks::Trigger::Use
                && callback.selector.action == Some(4001)
                && callback.selector.appearance == Some(132)
                && callback.item_on_target.as_ref().is_some_and(|item| {
                    item.item == "oteryn:item.tibia.i3463"
                        && item.selected_instance
                        && item.target_remove
                })
        })
        .ok_or("Eroth's mirror has no registered USE callback")?;
    assert_eq!(mirror.effects.reset_after_ms, Some(60_000));
    let changes = catalogue
        .evaluate(
            catalogue
                .transition(&mirror.key)
                .ok_or("mirror transition")?,
            &progress,
            0,
        )
        .map_err(|error| error.code())?;
    progress.extend(
        changes
            .into_iter()
            .map(|change| (change.track, change.after)),
    );
    let changes = catalogue
        .evaluate(completion, &progress, 0)
        .map_err(|error| error.code())?;
    assert!(changes.iter().any(|change| change.after == 3));
    Ok(())
}

#[test]
fn cursed_crystal_acceptance_retains_previously_mixed_ointment_progress() -> Result<(), String> {
    let loaded =
        loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let transition = catalogue
        .transition("oteryn:quest-reward/cursed-crystal-start")
        .ok_or("acceptance")?;
    let track = format!("{CRYSTAL}questline");
    let changes = catalogue
        .evaluate(transition, &BTreeMap::from([(track.clone(), 2)]), 100)
        .map_err(|error| error.code())?;
    assert!(
        changes
            .iter()
            .any(|change| change.track == track && change.after == 2)
    );
    assert!(
        changes
            .iter()
            .any(|change| change.track == format!("{CRYSTAL}oneeyedjoe") && change.after == 0)
    );
    Ok(())
}
