use super::corruptor_items;

#[test]
fn tutorial_corruptor_rewards_use_source_items_and_base_vocations() {
    assert_eq!(corruptor_items("knight"), [3359, 3285, 3375]);
    assert!(corruptor_items("royal_paladin").is_empty());
    assert!(corruptor_items("druid").is_empty());
    assert!(corruptor_items("master_sorcerer").is_empty());
    assert!(corruptor_items("monk").is_empty());
    assert!(corruptor_items("none").is_empty());
}
