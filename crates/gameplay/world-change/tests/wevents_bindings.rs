use oteryn_gameplay_world_change::{
    binding::Binding,
    timed::{EventKind, Input},
};

#[test]
fn wevents_essence_costs_use_tentacle_pieces_for_each_incarnation() {
    for (member, count) in (0_u8..=4).zip([2, 5, 8, 12, 16]) {
        let binding = Binding {
            event: EventKind::DevovorgasEssence,
            action: Input::Enter(member),
            minimum_level: 0,
            maximum_level: None,
        };
        assert!(binding.valid());
        assert_eq!(binding.cost(), Some((11666, count)));
    }
    let invalid = Binding {
        event: EventKind::DevovorgasEssence,
        action: Input::Enter(5),
        minimum_level: 0,
        maximum_level: None,
    };
    assert!(!invalid.valid());
}
