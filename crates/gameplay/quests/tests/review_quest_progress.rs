use oteryn_gameplay_quests::quest::{
    QuestStateCatalogue, loader, reward_callbacks, source_deaths, world_callbacks,
};
use std::collections::BTreeMap;

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
        ("vampire-hunt-dust", 7, false),
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
fn vampire_coffin_can_reopen_from_its_committed_stage() -> Result<(), String> {
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
            let mut recovered = tracks.clone();
            advance(catalogue, &mut recovered, &retry.key)?;
            assert_eq!(recovered, tracks, "retry must preserve the committed stage");
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
