use super::BossDifficultyPanel;
use oteryn_client_runtime::boss_difficulty::Reply;
#[must_use]
pub fn panel_text(p: &BossDifficultyPanel) -> String {
    let state = match &p.reply {
        Some(Reply::Window {
            boss_key,
            content_revision,
            selected_difficulty,
            ..
        }) => format!(
            "Boss: {boss_key}\nRevision: {content_revision}\nSelected difficulty: {selected_difficulty}"
        ),
        Some(Reply::Closed { .. }) => "Difficulty window closed by server".into(),
        Some(Reply::Refused { reason, .. }) => format!("Server refusal: {reason:?}"),
        None => "Enter a boss id and open its server window".into(),
    };
    match p.feedback {
        Some(f) => format!("{state}\n{f}"),
        None => state,
    }
}
