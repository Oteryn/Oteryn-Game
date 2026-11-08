//! Retained Crystal 201189f source death continuations must reach the production catalogue.
//! heart_of_destruction/creaturescripts_heart_boss_death.lua:26-138;
//! the_order_of_the_stag/court_warlock_fight.lua:323-329.
use oteryn_gameplay_quests::quest::loader::load_embedded_quest_state;

#[test]
fn source_boss_deaths_are_registered_in_the_production_quest_catalogue() -> Result<(), String> {
    let book = load_embedded_quest_state("srvenc-source-boss-deaths")
        .map_err(|error| format!("{error:?}"))?;
    for key in [
        "oteryn:quest-transition/heart-destruction/anomaly-room-death",
        "oteryn:quest-transition/heart-destruction/rupture-room-death",
        "oteryn:quest-transition/heart-destruction/realityquake-room-death",
        "oteryn:quest-transition/heart-destruction/eradicator-room-death",
        "oteryn:quest-transition/heart-destruction/outburst-room-death",
        "oteryn:quest-transition/heart-destruction/world-devourer-room-death",
        "oteryn:quest-transition/order-of-the-stag/court-warlock-room-death",
    ] {
        assert!(
            book.catalogue().transition(key).is_some(),
            "missing production source death binding: {key}"
        );
    }
    Ok(())
}

#[test]
fn source_room_credit_and_damage_map_credit_are_distinct_and_floor_exact() -> Result<(), String> {
    use oteryn_gameplay_quests::quest::source_deaths;
    let book =
        load_embedded_quest_state("srvenc-source-boundaries").map_err(|e| format!("{e:?}"))?;
    for rule in source_deaths::rules()? {
        assert_eq!(
            book.catalogue()
                .transition(&rule.transition)
                .map(|t| t.quest.as_str()),
            Some(rule.quest.as_str())
        );
        if let Some([x1, y1, x2, y2, z]) = rule.room {
            assert!(rule.recipient([x1, y1, z], false));
            assert!(rule.recipient([x2, y2, z], false));
            assert!(!rule.recipient([x1 - 1, y1, z], true));
            assert!(!rule.recipient([x1, y1, z + 1], true));
        } else {
            assert!(rule.recipient([0, 0, -7], true));
            assert!(!rule.recipient([0, 0, -7], false));
        }
    }
    assert!(!source_deaths::tracks_creature(
        "oteryn:creature.the_primal_menace"
    ));
    Ok(())
}

#[test]
fn selected_cake_counter_requires_its_previous_stage_and_stops_at_twenty() -> Result<(), String> {
    use oteryn_gameplay_quests::quest::source_deaths;
    use std::collections::BTreeMap;
    let rule = source_deaths::rules()?
        .iter()
        .find(|r| r.creature == "oteryn:creature.cake_golem")
        .ok_or("Cake binding")?;
    let start = "oteryn:quest-progress/chosen-source/a_piece_of_cake/s1";
    let count = "oteryn:quest-progress/chosen-source/a_piece_of_cake/s2";
    let mut tracks = BTreeMap::new();
    assert!(!rule.active(&tracks));
    tracks.insert(start.into(), 1);
    for value in 0..20 {
        tracks.insert(count.into(), value);
        assert!(rule.active(&tracks));
    }
    tracks.insert(count.into(), 20);
    assert!(!rule.active(&tracks));
    assert_eq!(
        rule.epoch_tracks.as_deref(),
        Some([start.to_string()].as_slice())
    );
    Ok(())
}
