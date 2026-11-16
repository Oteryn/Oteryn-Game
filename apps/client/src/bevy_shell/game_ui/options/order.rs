//! The HUD page orders the skill rows; the Skills window shows the saved order.
use super::*;
use bevy::ui_widgets::Activate;
use oteryn_client_ui::skill_progress::SKILL_LABELS;

#[derive(Component, Clone, Copy)]
pub(super) struct Move {
    index: usize,
    down: bool,
}

/// The skill families in display order.
fn families(draft: &crate::play::ReferencePreferences) -> Vec<usize> {
    if draft.skill_order.is_empty() {
        (0..SKILL_LABELS.len()).collect()
    } else {
        draft
            .skill_order
            .iter()
            .map(|&id| usize::from(id))
            .collect()
    }
}

pub(super) fn build(
    commands: &mut Commands,
    parent: Entity,
    skins: &Skins,
    draft: &crate::play::ReferencePreferences,
) {
    section::label(commands, parent, skins, tr!("Skill Order"));
    for (index, family) in families(draft).into_iter().enumerate() {
        let bar = section::bar(commands, parent);
        let name = SKILL_LABELS.get(family).copied().unwrap_or_default();
        section::label(commands, bar, skins, name);
        for (caption, down) in [(tr!("Up"), false), (tr!("Down"), true)] {
            let button = section::action(
                commands,
                bar,
                skins,
                caption,
                60.,
                &format!("hud.skill_order.{index}.{caption}"),
                false,
            );
            commands
                .entity(button)
                .insert(Move { index, down })
                .observe(shift);
        }
    }
}

fn shift(
    event: On<Activate>,
    moves: Query<&Move>,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<OptionsState>,
    guard: ModalGuard,
) {
    if !section::editable(&owner, &state, &guard, event.entity, Page::Hud) {
        return;
    }
    let Ok(&Move { index, down }) = moves.get(event.entity) else {
        return;
    };
    owner.edit_preferences(|draft| {
        draft.move_status_entry(true, index, down);
    });
    state.revision = state.revision.wrapping_add(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn skill_rows_move_apply_and_reach_the_skills_window_order() -> TestResult {
        let path = std::env::temp_dir().join(format!(
            "bevy-options-skill-order-{}.json",
            std::process::id()
        ));
        let result = (|| -> TestResult {
            let mut app = super::super::tests::app(Page::Hud, Some(path.clone()))?;
            let button = app
                .world_mut()
                .spawn(Move {
                    index: 0,
                    down: true,
                })
                .observe(shift)
                .id();
            app.world_mut().trigger(Activate { entity: button });
            let owner = app.world().non_send::<HostOwner>();
            let draft = owner.preferences_draft().ok_or("draft")?;
            assert_eq!(families(draft)[..3], [1, 0, 2]);
            assert!(owner.settings.preferences.skill_order.is_empty());
            app.world_mut().non_send_mut::<HostOwner>().apply(true);
            let stored = crate::play::SettingsState::new(Some(path.clone()));
            assert_eq!(stored.preferences.skill_order[..3], [1, 0, 2]);
            // The saved order reaches the panel the Skills window lists from.
            let (link, _commands, _outbox) = crate::play::play_channel();
            stored.sync_local_preferences(&link);
            assert_eq!(
                link.skill_progress_panel().ordered_families()[..3],
                [1, 0, 2]
            );
            Ok(())
        })();
        let _ = std::fs::remove_file(&path);
        result
    }
}
