use super::BossDifficultyPanel;
use oteryn_client_runtime::boss_difficulty::Reply;
#[must_use]
pub fn panel_text(p: &BossDifficultyPanel) -> String {
    let state = match &p.reply {
        Some(Reply::Window {
            boss_key,
            content_revision,
            selected_difficulty,
            unlocked_max: 0,
            ..
        }) => lfmt!(
            "Boss: {boss_key}\nRevision: {content_revision}\nSelected difficulty: {selected_difficulty}",
            boss_key = boss_key,
            content_revision = content_revision,
            selected_difficulty = selected_difficulty
        ),
        Some(Reply::Window {
            boss_key,
            content_revision,
            selected_difficulty,
            personal_highest,
            unlocked_max,
            maximum_difficulty,
            ..
        }) => lfmt!(
            "Boss: {boss_key}\nRevision: {content_revision}\nSelected difficulty: {selected_difficulty}\nPersonal highest: {personal_highest}\nUnlocked difficulty: {unlocked_max} / {maximum_difficulty}",
            boss_key = boss_key,
            content_revision = content_revision,
            selected_difficulty = selected_difficulty,
            personal_highest = personal_highest,
            unlocked_max = unlocked_max,
            maximum_difficulty = maximum_difficulty
        ),
        Some(Reply::Closed { .. }) => tr!("Difficulty window closed by server").into(),
        Some(Reply::Refused { reason, .. }) => lfmt!("Server refusal: {reason:?}", reason = reason),
        None => tr!("Enter a boss id and open its server window").into(),
    };
    let state = match &p.pity {
        Some(pity) => format!(
            "{state}\n{}",
            crate::boss_difficulty_pity::tracker_text(pity)
        ),
        None => state,
    };
    match p.feedback {
        Some(f) => format!("{state}\n{f}"),
        None => state,
    }
}
