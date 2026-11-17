use super::{QuestComparison, QuestEffectKind};

const QUESTS: [(&str, usize); 5] = [
    ("circle_room_quest", 2),
    ("crystal_wand_quest", 2),
    ("demona_ring_quest", 2),
    ("double_hero_quest", 2),
    ("griffin_shield_quest", 3),
];
#[test]
fn engclaim_m35_production_completion_requires_every_personal_claim() -> Result<(), String> {
    let loaded =
        super::loader::load_embedded_quest_state("content-1").map_err(|e| format!("{e:?}"))?;
    let catalogue = loaded.catalogue();
    for (slug, count) in QUESTS {
        let quest = format!("oteryn:quest.{slug}");
        let tracks = catalogue.completion_tracks(&quest).collect::<Vec<_>>();
        assert_eq!(
            tracks.len(),
            count,
            "{quest} must require every independent once claim"
        );
        let mut stored = std::collections::BTreeMap::new();
        for (index, track) in tracks.iter().rev().enumerate() {
            let marker = track
                .strip_prefix("oteryn:quest-progress/")
                .and_then(|s| s.strip_suffix("/claimed"))
                .ok_or("member track")?;
            let key = format!("oteryn:quest-transition/{marker}/chest");
            let transition = catalogue
                .transition(&key)
                .ok_or("native completion member")?;
            assert_eq!(transition.effects[0].from, QuestComparison::Eq(0));
            assert!(matches!(
                transition.effects[0].effect,
                QuestEffectKind::SetWithBinding { value: 1, .. }
            ));
            let changes = catalogue
                .evaluate(transition, &stored, 0)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(
                catalogue.completes_after_claim_sets(transition, &stored, &changes),
                index + 1 == count
            );
            for change in changes {
                stored.insert(change.track, change.after);
            }
        }
    }
    Ok(())
}
#[test]
fn engclaim_m35_exact_mint_authority_cannot_be_stripped_or_renamed() -> Result<(), String> {
    let mut checked = 0;
    for transition in super::completion_sets::mint_transitions()? {
        if !QUESTS
            .iter()
            .any(|(slug, _)| transition.quest == format!("oteryn:quest.{slug}"))
        {
            continue;
        }
        super::admission_binding::validate(&transition, false)?;
        let claim = super::completion_sets::required_mint_claim(&transition.key)?
            .ok_or("mint authority")?;
        assert_eq!(
            super::completion_sets::mint_transition_for_claim(claim)?,
            Some(transition.key.as_str())
        );
        let mut stripped = transition.clone();
        stripped.effects[0].effect = QuestEffectKind::Set(1);
        assert!(super::admission_binding::validate(&stripped, false).is_err());
        stripped.key.push_str("-shadow");
        assert!(super::admission_binding::validate(&stripped, false).is_err());
        checked += 1;
    }
    assert_eq!(checked, 11);
    assert!(
        super::completion_sets::required_mint_claim(
            "oteryn:quest-transition/kv/bear_chest1/chest"
        )?
        .is_none()
    );
    Ok(())
}
/// TibiaWiki Skull of Ratha Quest (rev 756266/749158): both Witch Hill bags and
/// the witch-house basement chest are one quest; completion needs all three.
#[test]
fn skull_of_ratha_completes_only_after_both_bags_and_the_basement_chest() -> Result<(), String> {
    let loaded =
        super::loader::load_embedded_quest_state("content-1").map_err(|e| format!("{e:?}"))?;
    let catalogue = loaded.catalogue();
    let quest = "oteryn:quest.skull_of_ratha_quest";
    let markers = [
        "quest/u7_6/explorer_society/skull_of_ratha/bag2",
        "kv/witch_house_quest",
        "quest/u7_6/explorer_society/skull_of_ratha/bag1",
    ];
    assert_eq!(catalogue.completion_tracks(quest).count(), markers.len());
    let mut stored = std::collections::BTreeMap::new();
    for (index, marker) in markers.iter().enumerate() {
        let transition = catalogue
            .transition(&format!("oteryn:quest-transition/{marker}/chest"))
            .ok_or("Skull of Ratha chest member")?;
        assert_eq!(transition.quest, quest);
        assert_eq!(transition.effects[0].effect, QuestEffectKind::Set(1));
        let changes = catalogue
            .evaluate(transition, &stored, 0)
            .map_err(|e| format!("{e:?}"))?;
        assert_eq!(
            catalogue.completes_after_claim_sets(transition, &stored, &changes),
            index + 1 == markers.len()
        );
        for change in changes {
            stored.insert(change.track, change.after);
        }
    }
    Ok(())
}

/// TibiaWiki Sanctuary of the Lizard God Quest (rev 1198612): the Legion Helmet
/// chest behind the sanctuary teleport is the quest's reward and closes it.
#[test]
fn sanctuary_of_the_lizard_god_completes_on_the_legion_helmet_chest() -> Result<(), String> {
    let loaded =
        super::loader::load_embedded_quest_state("content-1").map_err(|e| format!("{e:?}"))?;
    let catalogue = loaded.catalogue();
    let quest = "oteryn:quest.sanctuary_of_the_lizard_god";
    assert_eq!(catalogue.completion_tracks(quest).count(), 1);
    let transition = catalogue
        .transition("oteryn:quest-transition/kv/legiondawnport/chest")
        .ok_or("Legion Helmet chest")?;
    assert_eq!(transition.quest, quest);
    assert_eq!(transition.effects[0].effect, QuestEffectKind::Set(1));
    let stored = std::collections::BTreeMap::new();
    let changes = catalogue
        .evaluate(transition, &stored, 0)
        .map_err(|e| format!("{e:?}"))?;
    assert!(catalogue.completes_after_claim_sets(transition, &stored, &changes));
    Ok(())
}
