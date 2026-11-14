use super::*;
use crate::play::{mailbox::lock, play_channel};
#[test]
fn skills_config_skills_window_shows_server_data_only() {
    let (link, _, _) = play_channel();
    let state = link.skills_config_snapshot();
    assert!(state.widget.rows.is_empty());
    assert_eq!(state.widget, link.skills_widget());
}
#[test]
fn skills_config_skills_actions_are_bound_and_confirmed() {
    let (link, mut commands, _) = play_channel();
    lock(&link.mailbox).loyalty_skills.selected = true;
    assert!(link.skills_config_open());
    assert!(matches!(commands.try_recv(), Ok(crate::combat_input::PlayCommand::LoyaltySkills(_))));
    let state = link.skills_config_snapshot();
    assert!(!link.skills_config_dispatch_bound(&state, SkillsConfigControl::Refresh));
    assert!(commands.try_recv().is_err());
    assert!(state.widget.rows.is_empty());
    assert!(link.skills_config_dispatch_bound(&state, SkillsConfigControl::Progress(false)));
    assert!(!link.skills_config_snapshot().layout.preferences.progress);
}
#[test]
fn skills_config_skills_rejected_when_session_replaced_or_target_changed() {
    let (link, _, _) = play_channel();
    assert!(link.skills_config_open());
    let state = link.skills_config_snapshot();
    assert!(link.skills_config_dispatch_bound(&state, SkillsConfigControl::Family(2)));
    assert!(!link.skills_config_dispatch_bound(&state, SkillsConfigControl::Visible(false)));
    let current = link.skills_config_snapshot();
    lock(&link.mailbox).quest_shutdown = true;
    assert!(!link.skills_config_dispatch_bound(&current, SkillsConfigControl::Progress(false)));
    assert_eq!(current, link.skills_config_snapshot());
}
