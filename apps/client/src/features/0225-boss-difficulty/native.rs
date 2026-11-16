//! Native boss difficulty controls and platform-independent form validation.
use oteryn_client_runtime::raw_feature::FeatureCommand;
use oteryn_client_ui::boss_difficulty::{BossDifficultyPanel, Intent, request};

pub(in crate::play) fn difficulty_levels(
    reply: Option<&oteryn_client_runtime::boss_difficulty::Reply>,
) -> std::ops::RangeInclusive<u16> {
    0..=match reply {
        Some(oteryn_client_runtime::boss_difficulty::Reply::Window {
            maximum_difficulty, ..
        }) => *maximum_difficulty,
        _ => 0,
    }
}

pub(in crate::play) fn enabled(panel: &BossDifficultyPanel) -> bool {
    panel.selected && !panel.pending
}

pub(in crate::play) fn submit(
    panel: &mut BossDifficultyPanel,
    intent: Intent,
    boss: &str,
    context: &str,
    difficulty: &str,
) -> Option<FeatureCommand> {
    if !enabled(panel) {
        return None;
    }
    if matches!(intent, Intent::Open) {
        panel.boss_key = boss.trim().to_owned();
    } else {
        let text = context.trim();
        let parsed = (|| {
            if text.len() != 32 || !text.is_ascii() {
                return None;
            }
            let mut id = [0; 16];
            for (slot, pair) in id.iter_mut().zip(text.as_bytes().as_chunks::<2>().0) {
                *slot = u8::from_str_radix(std::str::from_utf8(pair).ok()?, 16).ok()?;
            }
            (id != [0; 16]).then_some(id)
        })();
        let Some(id) = parsed else {
            panel.feedback = Some(tr!("Enter a nonzero context as 32 hex digits"));
            return None;
        };
        if !matches!(intent, Intent::Cancel) {
            let Some(value) = difficulty
                .trim()
                .parse::<u16>()
                .ok()
                .filter(|d| *d <= oteryn_client_runtime::boss_difficulty::MAX_DIFFICULTY)
            else {
                panel.feedback = Some(tr!("Enter a difficulty from 0 to 28"));
                return None;
            };
            panel.difficulty = value;
        }
        panel.context = Some(id);
    }
    request(panel, intent)
}


