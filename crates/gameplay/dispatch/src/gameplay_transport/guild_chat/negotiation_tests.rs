use super::*;
#[test]
fn guild_chat_requires_chat_and_resume_cannot_drop_or_widen_it() {
    let unsupported =
        SelectedCapabilities::select(PRODUCTION_OFFERED_CAPABILITIES, &[36]).expect("bounded");
    assert!(unsupported.as_slice().is_empty());
    let guild =
        SelectedCapabilities::select(PRODUCTION_OFFERED_CAPABILITIES, &[7, 36]).expect("bounded");
    assert_eq!(guild.as_slice(), [7, 36]);
    assert!(guild.resumable_with(&[7, 36, 1000]));
    assert!(!guild.resumable_with(&[7]));
    let old = SelectedCapabilities::select(PRODUCTION_OFFERED_CAPABILITIES, &[7]).expect("bounded");
    assert!(old.resumable_with(&[7, 36]));
    assert_eq!(old.as_slice(), [7]);
}
