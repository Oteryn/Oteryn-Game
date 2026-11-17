//! chsw (capability 512): the Change Channel window inside the gameplay dialog overlay.
use super::{Choice, HostOwner, Request, Skin, button};
use bevy::prelude::*;
use oteryn_client_ui_bevy as source;

impl HostOwner {
    /// The window stays up for a row, a confirmation or a step back; Cancel closes it.
    pub(super) fn choose_channel_switch(&mut self, request: Request, choice: Choice) -> bool {
        let Some(play) = self.play.as_ref() else {
            return false;
        };
        let keep = match choice {
            Choice::SwitchRow(channel) => {
                play.link.channel_switch_select(channel);
                true
            }
            Choice::SwitchConfirm => play.link.channel_switch_confirm(),
            Choice::SwitchBack => {
                play.link.channel_switch_back();
                true
            }
            _ => {
                play.link.channel_switch_close();
                return matches!(choice, Choice::Cancel);
            }
        };
        self.dialogs.pending = Some(request);
        keep
    }
}
pub(super) type SwitchView = (oteryn_client_ui::channel_switch::ChannelSwitchPanel, i64);

#[derive(Component)]
pub(super) struct TableRow(pub usize);

fn row_color(greyed: bool) -> TextColor {
    TextColor(if greyed {
        source::DISABLED_TEXT_COLOR
    } else {
        source::TEXT_COLOR
    })
}

pub(super) fn refresh_rows(
    rows: &mut Query<(&TableRow, &mut Text, &mut TextColor)>,
    panel: &oteryn_client_ui::channel_switch::ChannelSwitchPanel,
    now_ms: i64,
) {
    let projection = oteryn_client_ui::channel_switch::panel_rows(panel, now_ms);
    for (TableRow(index), mut text, mut color) in rows.iter_mut() {
        if let Some((cells, greyed)) = projection.get(*index) {
            text.0 = cells.join(" | ");
            *color = row_color(*greyed);
        }
    }
}

fn dialog_text(commands: &mut Commands, skin: &Skin, parent: Entity, text: &str) -> Entity {
    let text = commands
        .spawn(source::text(text, source::font(skin.font.clone(), false)))
        .insert(Node {
            width: percent(100.),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(text);
    text
}

/// chsw: the error dialog, the confirmation (with the Open PvP warning), the wait notice, or
/// the Channel table with one button per Channel.
pub(super) fn channel_switch_body(
    commands: &mut Commands,
    skin: &Skin,
    window: Entity,
    id: u64,
    panel: &oteryn_client_ui::channel_switch::ChannelSwitchPanel,
    now_ms: i64,
) {
    use oteryn_client_ui::channel_switch as model;
    if let Some(error) = &panel.error {
        dialog_text(commands, skin, window, error);
        button(commands, skin, window, id, "Ok", Choice::SwitchBack);
    } else if panel.switching.is_some() {
        dialog_text(commands, skin, window, model::wait_text());
    } else if let Some(text) = model::confirm_text(panel) {
        dialog_text(commands, skin, window, &text);
        button(commands, skin, window, id, "Ok", Choice::SwitchConfirm);
    } else {
        // Each Channel: its columns (Channel | Type | Ping | Load | Status), then its button. A
        // greyed row still answers a click with the error of what blocks it.
        let rows = model::panel_rows(panel, now_ms);
        for (index, (cells, greyed)) in rows.iter().enumerate() {
            let text = dialog_text(commands, skin, window, &cells.join(" | "));
            commands
                .entity(text)
                .insert((TableRow(index), row_color(*greyed)));
            if let Some(row) = index.checked_sub(1).and_then(|row| panel.rows.get(row)) {
                button(
                    commands,
                    skin,
                    window,
                    id,
                    &row.name,
                    Choice::SwitchRow(row.channel),
                );
            }
        }
        if panel.pending {
            dialog_text(commands, skin, window, "Updating…");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_channel_row_renders_disabled_text() {
        let mut world = World::new();
        let window = world.spawn_empty().id();
        let skin = Skin {
            buttons: source::ButtonImages {
                up: default(),
                down: default(),
            },
            frame: default(),
            font: default(),
        };
        let panel = oteryn_client_ui::channel_switch::ChannelSwitchPanel {
            current: Some([1; 16]),
            rows: vec![oteryn_client_ui::channel_switch::ChannelRow {
                channel: [1; 16],
                name: "A".into(),
                pvp: None,
                ping_ms: None,
                online: None,
                full: false,
            }],
            ..default()
        };
        channel_switch_body(&mut world.commands(), &skin, window, 1, &panel, 0);
        world.flush();
        assert!(
            world
                .query::<(&Text, &TextColor)>()
                .iter(&world)
                .any(|(text, color)| text.0.contains("Current")
                    && color.0 == source::DISABLED_TEXT_COLOR)
        );
    }
}
