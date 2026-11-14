use super::*;
use crate::play::{mailbox::lock, play_channel};
#[test]
fn skills_config_skills_window_shows_server_data_only() -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::skill_progress::{SkillProgress, SkillProgressSnapshot};
    let (link, mut commands, outbox) = play_channel();
    assert!(link.skills_config_snapshot().widget.rows.is_empty());
    outbox.skill_progress_initialize(true);
    assert!(link.skills_config_open());
    assert!(matches!(commands.try_recv()?, crate::combat_input::PlayCommand::Feature(command) if command.capability == 279 && command.kind == 495));
    assert!(link.skills_config_snapshot().widget.rows.is_empty());
    let snapshot = SkillProgressSnapshot { skills: [SkillProgress { level: 20, progress: 450, required: 1000, percent: 4500 }; 8] };
    outbox.skill_progress_result(&oteryn_session::GatedCommandResult {
        command_id: 1, server_sequence: 1, status: oteryn_session::CommandStatus::Accepted,
        payload: snapshot.encode().ok_or("encode")?,
    });
    let state = link.skills_config_snapshot();
    assert_eq!(state.widget, link.skills_widget());
    assert_eq!(state.widget.rows.len(), 8);
    assert_eq!(state.widget.rows[0].trained, 20);
    assert_eq!(state.widget.rows[0].progress, Some(4500));
    assert_eq!(state.widget.rows[0].loyalty, None);
    assert_eq!(state.widget.rows[0].current, None);
    Ok(())
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
    let (link, _commands, _outbox) = play_channel();
    assert!(link.skills_config_open());
    let state = link.skills_config_snapshot();
    assert!(link.skills_config_dispatch_bound(&state, SkillsConfigControl::Family(2)));
    assert!(!link.skills_config_dispatch_bound(&state, SkillsConfigControl::Visible(false)));
    let current = link.skills_config_snapshot();
    let (replacement, _replacement_commands, _replacement_outbox) = play_channel();
    assert!(replacement.skills_config_open());
    assert!(!replacement.skills_config_dispatch_bound(&current, SkillsConfigControl::Progress(false)));
    lock(&link.mailbox).quest_shutdown = true;
    assert!(!link.skills_config_dispatch_bound(&current, SkillsConfigControl::Progress(false)));
    assert_eq!(current, link.skills_config_snapshot());
}
