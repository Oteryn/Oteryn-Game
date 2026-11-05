use super::Panel;
pub fn request(panel: &mut Panel, kind: u32) -> bool {
    panel.begin(kind)
}
