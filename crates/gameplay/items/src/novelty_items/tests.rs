#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    reason = "test fixtures require the pinned catalogue and explicitly reject unexpected random draws"
)]
use super::*;

fn rule(id: u32) -> &'static NoveltyRule {
    rule_for_key(&format!("oteryn:item.tibia.i{id}")).expect("captured source rule")
}

#[test]
fn source_catalogue_closes_fifteen_supported_items() {
    let rules = rules().expect("valid captured source");
    assert_eq!(rules.len(), 10);
    assert_eq!(rules.iter().map(|r| r.item_ids.len()).sum::<usize>(), 15);
    for wrong in [
        "",
        "oteryn:item.tibia.i0",
        "oteryn:item.tibia.i6573",
        "oteryn:item.tibia.i6579",
    ] {
        assert!(!is_novelty(wrong));
    }
    assert!(select(rule(6572), 6573, |_, _| Some(0)).is_none());
}

#[test]
fn every_costume_retains_source_appearance_and_fifty_minute_duration() {
    for id in [653, 654, 655, 24949] {
        let rule = rule(id);
        for (index, expected) in rule.costumes.iter().enumerate() {
            let selected = select(rule, id, |domain, upper| {
                assert_eq!(domain, 192);
                assert_eq!(upper as usize, rule.costumes.len());
                Some(index as u64)
            })
            .unwrap();
            assert!(selected.consume_source);
            assert_eq!(selected.output, None);
            let costume = selected.effects.costume.unwrap();
            assert_eq!(costume.monster, expected.monster);
            assert_eq!(costume.look_type, expected.look_type);
            assert_eq!(costume.duration_ms, 3_000_000);
            assert_eq!(selected.effects.achievement_progress[0].target, 100);
        }
        assert!(select(rule, id, |_, upper| Some(upper)).is_none());
    }
}

#[test]
fn dice_all_faces_use_exact_source_output_and_reject_unbounded_draw() {
    for current in 5792..=5797 {
        for roll in 0..6 {
            let selected = select(rule(current), current, |domain, upper| {
                assert_eq!((domain, upper), (193, 6));
                Some(roll)
            })
            .unwrap();
            assert_eq!(selected.output, Some(5792 + roll as u32));
            assert_eq!(selected.effects.dice_value, Some(roll as u32 + 1));
            assert_eq!(selected.effects.cues[0].effect_id, 27);
        }
        assert!(select(rule(current), current, |_, upper| Some(upper)).is_none());
    }
}

#[test]
fn voodoo_exact_five_percent_boundary_never_consumes_the_doll() {
    for roll in 0..100 {
        let selected = select(rule(3002), 3002, |domain, upper| {
            assert_eq!((domain, upper), (194, 100));
            Some(roll)
        })
        .unwrap();
        assert!(!selected.consume_source);
        assert!(selected.effects.require_player_target);
        assert!(selected.effects.private_speech);
        assert_eq!(selected.effects.cues.len(), usize::from(roll < 5));
        assert_eq!(selected.effects.achievement.is_some(), roll < 5);
        assert_eq!(
            selected.effects.speech,
            if roll < 5 {
                rule(3002).success_text.clone()
            } else {
                rule(3002).failure_text.clone()
            }
        );
    }
    assert!(select(rule(3002), 3002, |_, _| Some(100)).is_none());
}

#[test]
fn trumpet_keeps_four_second_decay_and_hat_requires_the_head_slot() {
    let trumpet = select(rule(6572), 6572, |_, _| panic!("certain source transform")).unwrap();
    assert_eq!(trumpet.output, Some(6573));
    assert_eq!(
        trumpet.effects.timed_output,
        Some(TimedOutput {
            duration_ms: 4000,
            decay_to: 6572
        })
    );
    assert_eq!(trumpet.effects.speech.as_deref(), Some("TOOOOOOT!"));
    let hat = select(rule(6578), 6578, |_, _| panic!("certain source effect")).unwrap();
    assert!(!hat.consume_source);
    assert!(hat.effects.require_head);
    assert_eq!(hat.effects.cues[0].effect_id, 28);
    assert_eq!(hat.effects.achievement_progress[0].target, 200);
}

#[test]
fn explosive_present_and_carried_rocket_have_distinct_source_effects() {
    let present = select(rule(906), 906, |_, _| panic!("certain source effect")).unwrap();
    assert!(present.consume_source);
    assert_eq!(present.effects.damage, None);
    assert_eq!(
        present.effects.achievement.as_deref(),
        Some("oteryn:achievement/joke_s_on_you")
    );
    let rocket = select(rule(6576), 6576, |_, _| panic!("certain carried effect")).unwrap();
    assert!(rocket.consume_source);
    assert_eq!(rocket.effects.damage, Some(10));
    assert_eq!(
        rocket
            .effects
            .cues
            .iter()
            .map(|c| c.effect_id)
            .collect::<Vec<_>>(),
        [7, 16]
    );
    assert_eq!(rocket.effects.achievement_progress[0].target, 250);
}

#[test]
fn stored_effects_round_trip_and_deterministic_command_draws_replay_exactly() {
    for rule in rules().unwrap() {
        let id = rule.item_ids[0];
        let draw = |domain, upper| reward_bag_bounded_draw([1; 16], 33, [2; 16], domain, upper);
        let first = select(rule, id, draw).unwrap();
        assert_eq!(first, select(rule, id, draw).unwrap());
        let json = serde_json::to_value(&first.effects).unwrap();
        assert_eq!(
            serde_json::from_value::<NoveltyEffects>(json).unwrap(),
            first.effects
        );
    }
}

#[test]
fn every_source_achievement_is_an_actual_native_catalogue_key() {
    let catalogue: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../content/achievements/achievements-00000-00499.json"
    ))
    .expect("native Achievement catalogue");
    let keys = catalogue["records"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|record| record["identity"]["key"].as_str())
        .collect::<std::collections::BTreeSet<_>>();
    for rule in rules().unwrap() {
        for key in rule.achievement.iter().map(String::as_str).chain(
            rule.achievement_progress
                .iter()
                .chain(rule.secondary_achievement_progress.iter())
                .map(|progress| progress.key.as_str()),
        ) {
            assert!(
                keys.contains(key),
                "source label must resolve to an existing key: {key}"
            );
        }
    }
    assert!(keys.contains("oteryn:achievement/number_of_the_beast"));
}

#[test]
fn ground_rockets_use_exact_three_colour_draw_without_pocket_damage() {
    for roll in 0..3 {
        let ground = GroundEffect {
            x: 100,
            y: 101,
            floor: 7,
        };
        let selected = select_ground(rule(6576), 6576, ground.clone(), |domain, upper| {
            assert_eq!((domain, upper), (195, 3));
            Some(roll)
        })
        .unwrap();
        assert_eq!(selected.effects.ground, Some(ground));
        assert_eq!(selected.effects.cues.len(), 1);
        assert_eq!(selected.effects.cues[0].effect_id, 29 + roll as u16);
        assert!(selected.effects.damage.is_none());
        assert!(selected.effects.speech.is_none());
        assert_eq!(selected.effects.achievement_progress.len(), 1);
        assert_eq!(
            selected.effects.achievement_progress[0].key,
            "oteryn:achievement/fireworks_in_the_sky"
        );
    }
    assert!(
        select_ground(
            rule(6576),
            6576,
            GroundEffect {
                x: 0,
                y: 0,
                floor: 7
            },
            |_, _| Some(3)
        )
        .is_none()
    );
}

#[test]
fn every_balloon_has_exactly_one_source_authored_reverse_mapping() {
    let rules = house_transforms().unwrap();
    assert_eq!(rules.len(), 76);
    for rule in rules {
        let reverse =
            house_transform_for_key(&format!("oteryn:item.tibia.i{}", rule.target)).unwrap();
        assert_eq!(reverse.target, rule.item_id);
        assert!(is_ground_action(&format!(
            "oteryn:item.tibia.i{}",
            rule.item_id
        )));
    }
    assert_eq!(
        house_transform_for_key("oteryn:item.tibia.i37471")
            .unwrap()
            .target,
        37414
    );
    assert_eq!(
        house_transform_for_key("oteryn:item.tibia.i39692")
            .unwrap()
            .target,
        39680
    );
    assert!(!is_ground_action("oteryn:item.tibia.i6578"));
}
