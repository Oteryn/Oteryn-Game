use crate::play::feature_0199_cyclopedia_character::view::native as native_controls;
use oteryn_client_ui::cyclopedia_character::CyclopediaCharacterPanel;

#[test]
fn native_badge_target_and_refresh_do_not_query_viewer_badges() -> Result<(), &'static str> {
    let mut p = CyclopediaCharacterPanel {
        selected: true,
        ..Default::default()
    };
    let target = "00112233-4455-6677-8899-aabbccddeeff";
    let cmd =
        native_controls::submit(&mut p, 111, ["ignored", "ignored", target]).ok_or("badges")?;
    assert_eq!(cmd.kind, 388);
    assert_eq!(cmd.payload[0], 129);
    assert_eq!(cmd.payload.len(), 17);
    assert_eq!(cmd.payload[1..], p.target.ok_or("target")?);
    assert!(p.badges_open);
    p.pending = false;
    let refresh =
        native_controls::submit(&mut p, 113, ["ignored", "ignored", target]).ok_or("refresh")?;
    assert_eq!(refresh, cmd);
    assert!(p.badges_open);
    p.pending = false;
    assert!(native_controls::submit(&mut p, 111, ["0", "20", "wrong"]).is_none());
    assert_eq!(
        p.target,
        Some(cmd.payload[1..].try_into().map_err(|_| "target")?)
    );
    Ok(())
}
