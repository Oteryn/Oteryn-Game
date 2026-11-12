use super::{ForgeMarker, label, protocol::replace};

#[test]
fn pushed_forge_markers_replace_and_remove_creature_labels() {
    let mut marker = None;
    replace(&mut marker, true, Some(3));
    assert_eq!(marker, Some(ForgeMarker::Influenced(3)));
    assert_eq!(label("Rat", marker), "[Influenced 3] Rat");
    replace(&mut marker, true, Some(15));
    assert_eq!(label("Rat", marker), "[Fiendish] Rat");
    replace(&mut marker, true, None);
    assert_eq!(label("Rat", marker), "Rat");
    replace(&mut marker, false, Some(15));
    assert_eq!(marker, None);
    replace(&mut marker, true, Some(6));
    assert_eq!(marker, None);
}
