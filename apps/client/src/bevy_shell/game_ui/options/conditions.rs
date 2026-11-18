//! Row 158: the HUD page's condition matrix; each condition shows or hides on the HUD above the
//! character and in the status bar independently, and Up / Down set the order both use.
use super::*;
use crate::play::ReferencePreferences;
use bevy::ui_widgets::Activate;

#[derive(Component)]
pub(super) struct Toggle {
    key: String,
    hud: bool,
}
#[derive(Component, Clone, Copy)]
pub(super) struct Shift {
    index: usize,
    down: bool,
}

fn visible(p: &ReferencePreferences, key: &str, hud: bool) -> bool {
    let hidden = if hud {
        &p.condition_hud_hidden
    } else {
        &p.condition_status_hidden
    };
    !hidden.iter().any(|value| value == key)
}

pub(super) fn build(
    commands: &mut Commands,
    parent: Entity,
    skins: &Skins,
    draft: &ReferencePreferences,
) {
    section::label(
        commands,
        parent,
        skins,
        tr!(
            "Conditions: choose which conditions show on the HUD and in the status bar, and their order."
        ),
    );
    let list = commands
        .spawn((
            bevy::ui_widgets::ScrollArea,
            Node {
                width: percent(100.),
                height: px(180.),
                flex_shrink: 0.,
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(list);
    for (index, key) in draft.ordered_condition_keys().into_iter().enumerate() {
        let bar = section::bar(commands, list);
        section::label(commands, bar, skins, &key);
        for (caption, down) in [("Up", false), ("Down", true)] {
            let button = section::action(
                commands,
                bar,
                skins,
                caption,
                60.,
                &format!("hud.conditions.order.{index}.{caption}"),
                false,
            );
            commands
                .entity(button)
                .insert(Shift { index, down })
                .observe(shift);
        }
        for (caption, hud, column) in [("HUD", true, "hud"), ("Status Bar", false, "status")] {
            let button = section::action(
                commands,
                bar,
                skins,
                caption,
                110.,
                &format!("hud.conditions.{column}.{key}"),
                visible(draft, &key, hud),
            );
            commands
                .entity(button)
                .insert(Toggle {
                    key: key.clone(),
                    hud,
                })
                .observe(toggle);
        }
    }
}

fn toggle(
    event: On<Activate>,
    toggles: Query<&Toggle>,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<OptionsState>,
    guard: ModalGuard,
) {
    if !section::editable(&owner, &state, &guard, event.entity, Page::Hud) {
        return;
    }
    let Ok(toggle) = toggles.get(event.entity) else {
        return;
    };
    let (key, hud) = (toggle.key.clone(), toggle.hud);
    owner.edit_preferences(|draft| {
        let shown = visible(draft, &key, hud);
        draft.set_condition_visible(&key, hud, !shown);
    });
    state.revision = state.revision.wrapping_add(1);
}

fn shift(
    event: On<Activate>,
    shifts: Query<&Shift>,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<OptionsState>,
    guard: ModalGuard,
) {
    if !section::editable(&owner, &state, &guard, event.entity, Page::Hud) {
        return;
    }
    let Ok(&Shift { index, down }) = shifts.get(event.entity) else {
        return;
    };
    owner.edit_preferences(|draft| {
        draft.move_special_condition(index, down);
    });
    state.revision = state.revision.wrapping_add(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_client_ui::condition_status::icon_key;
    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn press(app: &mut App, key: &str, hud: bool) {
        let button = app
            .world_mut()
            .spawn(Toggle {
                key: key.to_owned(),
                hud,
            })
            .observe(toggle)
            .id();
        app.world_mut().trigger(Activate { entity: button });
    }

    #[test]
    fn condition_matrix_hides_each_column_applies_and_reaches_the_status_row() -> TestResult {
        let fixture = tempfile::tempdir()?;
        let path = fixture.path().join("bevy-options-conditions.json");
        let result = (|| -> TestResult {
            let mut app = super::super::tests::app(Page::Hud, Some(path.clone()))?;
            let poison = icon_key(1);
            press(&mut app, poison, false);
            press(&mut app, "Soul War", true);
            press(&mut app, "Soul War", true);
            press(&mut app, "Rotten Blood", true);
            let owner = app.world().non_send::<HostOwner>();
            let draft = owner.preferences_draft().ok_or("draft")?;
            assert!(!visible(draft, poison, false));
            assert!(visible(draft, poison, true));
            assert!(visible(draft, "Soul War", true));
            assert!(!visible(draft, "Rotten Blood", true));
            assert!(
                owner
                    .settings
                    .preferences
                    .condition_status_hidden
                    .is_empty()
            );
            app.world_mut().non_send_mut::<HostOwner>().apply(true);
            let stored = crate::play::SettingsState::new(Some(path.clone()));
            assert_eq!(stored.preferences.condition_status_hidden, [poison]);
            assert_eq!(stored.preferences.condition_hud_hidden, ["Rotten Blood"]);
            // The status-bar column reaches the panel the Bevy status row filters by.
            let (link, _commands, _outbox) = crate::play::play_channel();
            stored.sync_local_preferences(&link);
            assert_eq!(link.condition_status_panel().hidden, [1]);
            Ok(())
        })();
        fixture.close()?;
        result
    }

    #[test]
    fn condition_rows_move_apply_and_reach_the_status_order() -> TestResult {
        let fixture = tempfile::tempdir()?;
        let path = fixture.path().join("bevy-options-condition-order.json");
        let result = (|| -> TestResult {
            let mut app = super::super::tests::app(Page::Hud, Some(path.clone()))?;
            let button = app
                .world_mut()
                .spawn(Shift {
                    index: 0,
                    down: true,
                })
                .observe(shift)
                .id();
            app.world_mut().trigger(Activate { entity: button });
            let owner = app.world().non_send::<HostOwner>();
            let draft = owner.preferences_draft().ok_or("draft")?;
            assert_eq!(
                draft.ordered_condition_keys()[..2],
                [icon_key(2), icon_key(1)]
            );
            app.world_mut().non_send_mut::<HostOwner>().apply(true);
            let stored = crate::play::SettingsState::new(Some(path.clone()));
            let (link, _commands, _outbox) = crate::play::play_channel();
            stored.sync_local_preferences(&link);
            assert_eq!(link.condition_status_panel().order[..2], [2, 1]);
            Ok(())
        })();
        fixture.close()?;
        result
    }
}
