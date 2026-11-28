use oteryn_gameplay_quests::quest::{
    QuestRefusal, QuestStateCatalogue, loader, reward_callbacks, source_deaths, world_callbacks,
};
use std::collections::BTreeMap;

const STORKUS_DUST: &str =
    "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/storkus_vampiredust";
const STORKUS_DUST_REPORT: &str = "oteryn:quest-reward/inquisition-vampire-hunt-dust/already-paid";

fn advance(
    catalogue: &QuestStateCatalogue,
    tracks: &mut BTreeMap<String, i64>,
    binding: &str,
) -> Result<(), String> {
    let transition = catalogue
        .transition(binding)
        .ok_or_else(|| format!("missing live transition {binding}"))?;
    let changes = catalogue
        .evaluate(transition, tracks, 100)
        .map_err(|error| format!("{binding}: {}", error.code()))?;
    tracks.extend(
        changes
            .into_iter()
            .map(|change| (change.track, change.after)),
    );
    Ok(())
}

#[test]
fn nomad_death_and_both_contacts_supply_mission_four_progress() -> Result<(), String> {
    let loaded =
        loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let root = "oteryn:quest-progress/quest/u8_2/the_thieves_guild_quest/";
    let track = format!("{root}mission04");
    let callbacks = reward_callbacks::offered_for(catalogue)?;
    let mut tracks = BTreeMap::from([(format!("{root}questline"), 4), (track.clone(), 1)]);
    let reward = |name: &str| {
        let key = format!("oteryn:quest-reward/thieves-guild-{name}");
        callbacks.iter().find(|c| c.key == key).ok_or(key)
    };
    for (name, stage) in [("snake-eye-referral", 2), ("ahmet-nomad", 3)] {
        advance(catalogue, &mut tracks, &reward(name)?.key)?;
        assert_eq!(tracks[&track], stage);
    }
    let rule = source_deaths::rules()?
        .iter()
        .find(|rule| {
            rule.creature == "oteryn:creature.nomad"
                && rule.effects.iter().any(|effect| effect.track == track)
        })
        .ok_or("Nomad death binding")?;
    assert!(rule.active(&tracks));
    advance(catalogue, &mut tracks, &rule.transition)?;
    assert_eq!(tracks[&track], 4);
    assert!(!rule.active(&tracks));
    for (name, stage) in [
        ("ahmet-documents", 5),
        ("percybald-disguise", 6),
        ("nurik-painting", 7),
        ("report-04", 8),
    ] {
        advance(catalogue, &mut tracks, &reward(name)?.key)?;
        assert_eq!(tracks[&track], stage);
    }
    assert_eq!(tracks[&format!("{root}questline")], 5);
    Ok(())
}

#[test]
fn storkus_qualification_can_be_earned_through_registered_writers() -> Result<(), String> {
    let loaded =
        loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let track = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline";
    let npc = reward_callbacks::offered_for(catalogue)?;
    let world = world_callbacks::load_for(catalogue)?;
    let mut tracks = BTreeMap::from([(track.to_owned(), 3)]);
    for (name, stage, use_item) in [
        ("eclipse-start", 4, false),
        ("eclipse-brew-32657-31927", 5, true),
        ("eclipse-report", 6, false),
        ("vampire-hunt-referral", 6, false),
        ("vampire-hunt-dust/accept", 6, false),
        ("vampire-hunt-dust/collect", 6, false),
        ("vampire-hunt-dust/already-paid", 7, false),
        ("vampire-hunt-count", 8, false),
        ("vampire-hunt-coffin", 9, true),
        ("vampire-hunt-ring", 10, false),
        ("vampire-hunt-report", 11, false),
    ] {
        let family = if use_item {
            "quest-callback"
        } else {
            "quest-reward"
        };
        let key = format!("oteryn:{family}/inquisition-{name}");
        assert!(
            if use_item {
                world.iter().any(|callback| callback.key == key)
            } else {
                npc.iter().any(|callback| callback.key == key)
            },
            "{key} has no live owner"
        );
        advance(catalogue, &mut tracks, &key)?;
        assert_eq!(tracks[track], stage, "{key}");
    }
    let first_rank = npc
        .iter()
        .find(|row| row.key == "oteryn:quest-reward/vampire-hunter-rank-1")
        .ok_or("Storkus rank one")?;
    assert!(first_rank.requires.iter().any(|gate| gate.track == track));
    assert!(
        first_rank
            .requires
            .iter()
            .all(|gate| gate.holds(catalogue, &tracks))
    );
    Ok(())
}

#[test]
fn white_raven_accepts_published_catacomb_access_saves() -> Result<(), String> {
    let loaded =
        loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let quest = "oteryn:quest.the_white_raven_monastery_quest";
    let mut hash = [0_u8; 32];
    let baseline = include_str!("fixtures/white_raven_catacombs_predecessor.txt").trim();
    for (slot, at) in hash.iter_mut().zip((0..baseline.len()).step_by(2)) {
        *slot = u8::from_str_radix(&baseline[at..at + 2], 16).map_err(|error| error.to_string())?;
    }
    assert_ne!(loaded.catalogue().definition_hash(quest), Some(hash));
    assert!(
        oteryn_gameplay_quests::quest::npc_rules::legacy_defaults(loaded.catalogue(), quest, &hash)
            .is_some(),
        "published Catacombs access save has no definition predecessor"
    );
    hash[0] ^= 1;
    assert!(
        oteryn_gameplay_quests::quest::npc_rules::legacy_defaults(loaded.catalogue(), quest, &hash)
            .is_none()
    );
    Ok(())
}

#[test]
fn ordinary_vampire_coffins_refuse_after_the_hunt_starts() -> Result<(), String> {
    let loaded =
        loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let callbacks = world_callbacks::load_for(catalogue)?;
    let coffins: Vec<_> = callbacks
        .iter()
        .filter(|callback| {
            callback
                .key
                .starts_with("oteryn:quest-callback/inquisition-vampire-hunt-coffin")
        })
        .collect();
    assert_eq!(coffins.len(), 2);
    for first in &coffins {
        let mut tracks = first
            .storage
            .iter()
            .map(|guard| (guard.track.clone(), guard.minimum))
            .collect();
        advance(catalogue, &mut tracks, &first.key)?;
        for retry in &coffins {
            assert!(retry.effects.encounter.is_some());
            let transition = catalogue
                .transition(&retry.key)
                .ok_or("coffin retry transition")?;
            assert_eq!(
                catalogue.evaluate(transition, &tracks, 100),
                Err(oteryn_gameplay_quests::quest::QuestRefusal::StageMismatch),
            );
        }
        let mut finished = tracks;
        advance(
            catalogue,
            &mut finished,
            "oteryn:quest-reward/inquisition-vampire-hunt-ring",
        )?;
        let transition = catalogue
            .transition(&first.key)
            .ok_or("coffin transition")?;
        assert_eq!(
            catalogue.evaluate(transition, &finished, 100),
            Err(oteryn_gameplay_quests::quest::QuestRefusal::StageMismatch),
        );
    }
    Ok(())
}

#[test]
fn nomad_death_binding_rejects_damage_without_party_evidence() -> Result<(), String> {
    let loaded =
        loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let rule = source_deaths::rules()?
        .iter()
        .find(|rule| rule.creature == "oteryn:creature.nomad")
        .ok_or("Nomad death binding")?;
    assert!(loaded.catalogue().transition(&rule.transition).is_some());
    assert!(
        !rule.recipient([100, 100, -7], true),
        "Nomad credit was granted from damage alone, with no party witness",
    );
    Ok(())
}

#[test]
fn all_donor_nomads_are_admitted_by_the_source_death_loader() -> Result<(), String> {
    let loaded =
        loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let key =
        "oteryn:quest-transition/quest/u8_2/the_thieves_guild_quest/mission04/creature_event_1";
    assert!(loaded.catalogue().transition(key).is_some());
    for creature in ["nomad", "nomad_female", "nomad_blue"] {
        let creature = format!("oteryn:creature.{creature}");
        assert!(source_deaths::tracks_creature(&creature), "{creature}");
    }
    Ok(())
}

#[test]
fn storkus_deposit_reply_agrees_with_the_mission_report_gate() -> Result<(), String> {
    let loaded =
        loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let callbacks = reward_callbacks::offered_for(loaded.catalogue())?;
    let report = callbacks
        .iter()
        .find(|callback| callback.key == STORKUS_DUST_REPORT)
        .ok_or("Storkus dust report")?;
    let threshold = report
        .storage
        .iter()
        .find(|guard| guard.track == STORKUS_DUST)
        .ok_or("Storkus report counter gate")?
        .minimum;
    let nearly_paid = callbacks
        .iter()
        .find(|callback| {
            callback.key == "oteryn:quest-reward/inquisition-vampire-hunt-dust/deposit-1-19"
        })
        .ok_or("Storkus 19-dust deposit")?;
    assert_eq!(
        nearly_paid.reply,
        ["Ye've brought me 19 vampire dusts. Ye' need to bring 1 more."]
    );

    for callback in callbacks
        .iter()
        .filter(|callback| callback.npc == "oteryn:npc.storkus" && callback.topic == 1)
    {
        let [payment] = callback.take.as_slice() else {
            return Err(format!("{} needs exactly one dust payment", callback.key));
        };
        let saved = callback
            .storage
            .iter()
            .find(|guard| guard.track == STORKUS_DUST)
            .ok_or_else(|| format!("{} does not write the dust counter", callback.key))?
            .set;
        let remainder = threshold - saved;
        let instruction = if remainder == 0 {
            "Ask me for a mission to continue your quest.".into()
        } else {
            format!("Ye' need to bring {remainder} more.")
        };
        assert_eq!(
            callback.reply,
            [format!(
                "Ye've brought me {} vampire dusts. {instruction}",
                payment.count
            )],
            "{}",
            callback.key
        );
    }
    Ok(())
}

#[test]
fn storkus_accepts_partial_dust_and_finishes_existing_partial_saves() -> Result<(), String> {
    let loaded =
        loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let catalogue = loaded.catalogue();
    let callbacks = reward_callbacks::offered_for(catalogue)?;
    let questline = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/questline";
    let mission = "oteryn:quest-progress/quest/u8_2/the_inquisition_quest/mission03";
    let item = "oteryn:item.tibia.i5905";
    let accept = "oteryn:quest-reward/inquisition-vampire-hunt-dust/accept";
    let report = catalogue
        .transition(STORKUS_DUST_REPORT)
        .ok_or("Storkus dust report transition")?;
    for (saved, carried, consumed, after, reported_stage) in [
        (0, 7, 7, 8, 6),
        (8, 3, 3, 11, 6),
        (8, 17, 13, 21, 7),
        (20, 4, 1, 21, 7),
        (0, 22, 20, 21, 7),
    ] {
        let mut tracks = BTreeMap::from([
            (questline.into(), 6),
            (mission.into(), 1),
            (STORKUS_DUST.into(), saved),
        ]);
        if saved == 0 {
            advance(catalogue, &mut tracks, accept)?;
            assert_eq!(
                (tracks[STORKUS_DUST], tracks[questline], tracks[mission]),
                (1, 6, 1)
            );
        }
        assert_eq!(
            catalogue.evaluate(report, &tracks, 100),
            Err(QuestRefusal::StageMismatch)
        );
        let inventory = BTreeMap::from([(item.into(), carried)]);
        let mut candidates = callbacks.iter().filter(|callback| {
            callback.npc == "oteryn:npc.storkus"
                && callback.topic == 1
                && callback.keywords.iter().any(|word| word == "yes")
                && callback.storage.iter().all(|guard| {
                    tracks
                        .get(&guard.track)
                        .is_some_and(|value| (guard.minimum..=guard.maximum).contains(value))
                })
                && callback
                    .state_ops
                    .iter()
                    .all(|op| op.advisory_holds(catalogue, &tracks))
                && callback
                    .inventory
                    .iter()
                    .all(|guard| guard.holds(&inventory))
        });
        let selected = candidates
            .next()
            .ok_or_else(|| format!("no loaded deposit for {saved}+{carried}"))?;
        assert!(
            candidates.next().is_none(),
            "ambiguous deposit for {saved}+{carried}"
        );
        let [payment] = selected.take.as_slice() else {
            return Err(format!("{} needs exactly one dust payment", selected.key));
        };
        assert_eq!((payment.item.as_str(), payment.count), (item, consumed));
        advance(catalogue, &mut tracks, &selected.key)?;
        assert_eq!(
            (tracks[STORKUS_DUST], tracks[questline], tracks[mission]),
            (after, 6, 1)
        );
        if reported_stage == 7 {
            advance(catalogue, &mut tracks, STORKUS_DUST_REPORT)?;
            assert_eq!(
                (tracks[STORKUS_DUST], tracks[questline], tracks[mission]),
                (after, reported_stage, 2)
            );
        } else {
            assert_eq!(
                catalogue.evaluate(report, &tracks, 100),
                Err(QuestRefusal::StageMismatch)
            );
        }
    }
    Ok(())
}

#[test]
fn both_live_coffin_selectors_have_a_weakened_count_offering_plan() -> Result<(), String> {
    let loaded =
        loader::load_embedded_quest_state("content-1").map_err(|error| format!("{error:?}"))?;
    let callbacks = world_callbacks::load_for(loaded.catalogue())?;
    for suffix in ["", "-side"] {
        let key = format!("oteryn:quest-callback/inquisition-vampire-hunt-coffin{suffix}");
        let ordinary = callbacks.iter().find(|row| row.key == key).ok_or(&key)?;
        let weakened = callbacks
            .iter()
            .find(|row| {
                row.selector == ordinary.selector
                    && row.effects.encounter.as_ref().is_some_and(|encounter| {
                        encounter
                            .monsters
                            .iter()
                            .any(|monster| monster.creature == "oteryn:creature.the_weakened_count")
                    })
            })
            .ok_or_else(|| format!("no live weakened variant for {key}"))?;
        let mut recovery = ordinary.storage.clone();
        let questline = recovery
            .iter_mut()
            .find(|guard| guard.track == world_callbacks::VAMPIRE_COFFIN_QUESTLINE)
            .ok_or("coffin questline guard")?;
        questline.maximum = questline.set;
        assert_eq!(weakened.storage, recovery);
        assert_eq!(weakened.offerings.len(), 3);
        assert!(
            weakened.offerings.iter().all(|offering| {
                offering.item == "oteryn:item.tibia.i3083" && offering.count == 1
            })
        );
    }
    Ok(())
}
