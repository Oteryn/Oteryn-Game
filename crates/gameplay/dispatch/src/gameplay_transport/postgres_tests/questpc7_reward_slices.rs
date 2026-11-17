//! Source reward slices only: these do not certify their access or outfit programmes.
use super::{TestResult, quest_catalogue_chests};
use std::collections::BTreeSet;

fn rewards(claims: &[&str]) -> TestResult {
    let expected: BTreeSet<_> = claims.iter().copied().collect();
    assert_eq!(expected.len(), claims.len(), "duplicate requested reward");
    let selected: Vec<_> = quest_catalogue_chests::cases()?
        .into_iter()
        .filter(|case| expected.contains(case.claim.as_str()))
        .collect();
    assert_eq!(
        selected
            .iter()
            .map(|case| case.claim.as_str())
            .collect::<BTreeSet<_>>(),
        expected,
        "every requested reward must reach its production source binding"
    );
    quest_catalogue_chests::execute(selected)
}

#[test]
fn questpc7_story_rewards_use_replay_and_refusal() -> TestResult {
    rewards(&[
        "oteryn:reward-claim.halls_of_hope.reward1",
        "oteryn:reward-claim.halls_of_hope.reward2",
        "oteryn:reward-claim.halls_of_hope.reward3",
        "oteryn:reward-claim.halls_of_hope.reward4",
        "oteryn:reward-claim.halls_of_hope.reward5",
        "oteryn:reward-claim.kv.ghostsilver_lantern_quest",
        "oteryn:reward-claim.kv.imbuing_crystal_quest",
        "oteryn:reward-claim.kv.painting_of_agirl_quest",
        "oteryn:reward-claim.quest.u10_90.ferumbras_ascension.stone_coffins_bones",
        "oteryn:reward-claim.quest.u10_90.ferumbras_ascension.the_rite_of_eternal_night",
        "oteryn:reward-claim.quest.u10_90.ferumbras_ascension.vampire_teeth",
        "oteryn:reward-claim.quest.u11_50.hidden_threats.rewards.key_fragment01",
        "oteryn:reward-claim.quest.u11_50.hidden_threats.rewards.key_fragment02",
        "oteryn:reward-claim.quest.u11_50.hidden_threats.rewards.metal_file",
        "oteryn:reward-claim.quest.u7_8.hunter_outfits.elane_crossbow",
    ])
}

#[test]
fn questpc7_formorgar_books_use_replay_and_refusal() -> TestResult {
    rewards(&[
        "oteryn:reward-claim.quest.u8_0.the_ice_islands.formorgar_glacier_chest",
        "oteryn:reward-claim.quest.u8_0.the_ice_islands.formorgar_glacier_skeleton",
    ])
}

#[test]
fn questpc7_giant_smithhammer_rewards_use_replay_and_refusal() -> TestResult {
    let cases = quest_catalogue_chests::cases()?;
    for (suffix, item, quantity, x, unique) in [
        ("gold_coin", "oteryn:item.tibia.i3031", 100, 32778, 6079),
        // PR2770 Q5/A retains this accepted native substitute over donor item3208.
        ("hammer", "oteryn:item.tibia.i12510", 1, 32776, 6078),
        ("talon", "oteryn:item.tibia.i3034", 1, 32774, 6077),
    ] {
        let key =
            format!("oteryn:reward-claim.quest.u7_6.explorer_society.giant_smith_hammer.{suffix}");
        let placements: Vec<_> = cases.iter().filter(|case| case.claim == key).collect();
        assert_eq!(placements.len(), 1, "one exact source chest for {suffix}");
        let case = placements[0];
        assert_eq!(case.root, item);
        assert_eq!(case.quantity, quantity);
        assert_eq!(case.position, (x, 32253, -8));
        assert_eq!(case.unique, unique);
        assert_eq!(case.appearance, 2472);
        assert!(case.contents.is_empty());
        assert!(case.servable);
    }
    rewards(&[
        "oteryn:reward-claim.quest.u7_6.explorer_society.giant_smith_hammer.gold_coin",
        "oteryn:reward-claim.quest.u7_6.explorer_society.giant_smith_hammer.hammer",
        "oteryn:reward-claim.quest.u7_6.explorer_society.giant_smith_hammer.talon",
    ])
}
