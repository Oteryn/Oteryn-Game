use super::Panel;
// Crystal protocolgame.cpp:7626: public speech is presented as actual text.
pub fn panel_text(panel: &Panel) -> String {
    let mut text = panel.picture.status();
    text.push_str(if panel.picture.chat_enabled {
        tr!("\nRead-only cast · Enter: send cast chat · Escape: leave\n")
    } else {
        tr!("\nRead-only cast · Cast chat disabled · Escape: leave\n")
    });
    for line in panel.picture.speech.iter().rev().take(2).rev() {
        text.push_str(line);
        text.push('\n');
    }
    for (_, line) in panel.picture.chat.iter().rev().take(8).rev() {
        text.push_str(line);
        text.push('\n');
    }
    text.push_str(&panel.feedback);
    text
}
