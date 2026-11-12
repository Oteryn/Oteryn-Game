use super::{channel_text, protocol::*};
#[test]
fn gap_world_channel_list() {
    let mut state = None;
    for premium_only in [false, true] {
        apply(
            &mut state,
            Some(ServedChannel {
                world_id: [1; 16],
                channel_id: [2; 16],
                premium_only,
            }),
        );
        let text = state.map(channel_text).unwrap_or_default();
        assert!(text.contains("World: 01010101-0101-0101-0101-010101010101"));
        assert!(text.contains("Channel: 02020202-0202-0202-0202-020202020202"));
        assert!(text.ends_with(if premium_only {
            "Premium-only: true"
        } else {
            "Premium-only: false"
        }));
    }
    apply(&mut state, None);
    assert_eq!(state, None);
}
