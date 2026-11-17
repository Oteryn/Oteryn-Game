use super::Panel;
pub fn panel_text(panel: &Panel) -> String {
    use std::fmt::Write;
    let mut text = String::from(tr!(
        "Live cast chat — viewers receive their copies after the cast delay.\n"
    ));
    let _ = lwriteln!(text, "Viewers: {}", panel.subjects.len());
    for handle in &panel.subjects {
        for byte in handle {
            let _ = write!(text, "{byte:02x}");
        }
        text.push('\n');
    }
    for line in panel.lines.iter().rev().take(12).rev() {
        for byte in line.handle {
            let _ = write!(text, "{byte:02x}");
        }
        text.push(' ');
        text.push_str(&line.text);
        text.push('\n');
    }
    text.push_str(&panel.feedback);
    text
}
