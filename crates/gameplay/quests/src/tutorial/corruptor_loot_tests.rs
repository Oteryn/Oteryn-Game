use super::corruptor_items;

#[test]
fn tutorial_corruptor_rewards_use_source_items_and_base_vocations() {
    assert_eq!(corruptor_items("knight"), [3359, 3285, 3375]);
    assert_eq!(corruptor_items("royal_paladin"), [3571, 51760]);
    assert_eq!(corruptor_items("druid"), [3066, 7991]);
    assert_eq!(corruptor_items("master_sorcerer"), [3074, 7991]);
    assert_eq!(corruptor_items("monk"), [50195, 51119]);
    assert!(corruptor_items("none").is_empty());
}
