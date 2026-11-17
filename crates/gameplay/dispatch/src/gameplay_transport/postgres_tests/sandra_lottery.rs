//! Donor odds and boundary draws shared with the durable NPC exchange.
use oteryn_gameplay_quests::quest::npc_rules::random_outcome::SANDRA_LOTTERY;

#[test]
fn sandra_lottery_odds_equal_crystal_201189f8() {
    assert_eq!(SANDRA_LOTTERY.total_weight(), Some(98));
    assert_eq!(SANDRA_LOTTERY.outcomes[0].weight, 1);
    assert_eq!(SANDRA_LOTTERY.outcomes[1].weight, 97);
    assert_eq!(SANDRA_LOTTERY.input, "oteryn:item.tibia.i5957");
    assert_eq!(SANDRA_LOTTERY.draw(0), Some(Some("oteryn:item.tibia.i5958")));
    for draw in 1..98 {
        assert_eq!(SANDRA_LOTTERY.draw(draw), Some(None));
    }
    assert_eq!(SANDRA_LOTTERY.draw(98), None);
}
