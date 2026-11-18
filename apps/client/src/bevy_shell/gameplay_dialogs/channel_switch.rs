//! chsw (capability 512): the Change Channel window inside the gameplay dialog overlay.
use super::{Choice, HostOwner, Request, Skin, projection::button};
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
            checked: default(),
            unchecked: default(),
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
    #[test]
    fn channel_switch_cooldown_refresh_preserves_scroll_and_focus()
    -> Result<(), Box<dyn std::error::Error>> {
        use super::super::{Content, Overlay, project};
        use bevy::input_focus::{FocusCause, InputFocus};
        use oteryn_client_runtime::channel_switch::{SwitchLock, SwitchStatus};
        use oteryn_client_ui::channel_switch::{ChannelRow, ChannelSwitchPanel};

        let mut app = App::new();
        let mut host = HostOwner::new(None)?;
        let view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
        let (link, _commands, _outbox) = crate::play::play_channel();
        link.set_channel_switch_fixture(ChannelSwitchPanel {
            open: true,
            selected: true,
            current: Some([1; 16]),
            rows: vec![ChannelRow {
                channel: [2; 16],
                name: "Destination".into(),
                pvp: None,
                ping_ms: None,
                online: None,
                full: false,
            }],
            status: Some(SwitchStatus {
                lock: SwitchLock::Cooldown,
                cooldown_left_ms: 1_500,
            }),
            status_at_ms: crate::play::feature_0512_channel_switch::view::now_ms(),
            ..default()
        });
        host.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
        host.dialogs.open(host.generation(), Content::ChannelSwitch);
        app.insert_non_send(host)
            .init_resource::<InputFocus>()
            .insert_resource(Skin {
                buttons: source::ButtonImages {
                    up: default(),
                    down: default(),
                },
                frame: default(),
                font: default(),
                checked: default(),
                unchecked: default(),
            })
            .add_systems(Update, project);
        app.world_mut()
            .spawn((Overlay, Node::default(), BackgroundColor::default()));
        app.update();

        let (row, first_text) = {
            let world = app.world_mut();
            world
                .query::<(Entity, &TableRow, &Text, &TextColor)>()
                .iter(world)
                .find(|(_, row, _, _)| row.0 == 1)
                .map(|(entity, _, text, color)| {
                    assert_eq!(color.0, source::DISABLED_TEXT_COLOR);
                    (entity, text.0.clone())
                })
                .ok_or("destination row")?
        };
        let window = app
            .world()
            .get::<ChildOf>(row)
            .ok_or("scroll area")?
            .parent();
        let offset = Vec2::new(0., 37.);
        app.world_mut()
            .entity_mut(window)
            .insert(ScrollPosition(offset));
        let focus = {
            let world = app.world_mut();
            world
                .query::<(Entity, &super::super::Reply)>()
                .iter(world)
                .find(|(_, reply)| matches!(reply.choice, Choice::SwitchRow(_)))
                .map(|(entity, _)| entity)
                .ok_or("destination button")?
        };
        app.world_mut()
            .resource_mut::<InputFocus>()
            .set(focus, FocusCause::Navigated);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            app.update();
            let text = app
                .world()
                .get::<Text>(row)
                .ok_or("cooldown replaced the row")?;
            assert_eq!(app.world().resource::<InputFocus>().get(), Some(focus));
            assert_eq!(
                app.world()
                    .get::<ScrollPosition>(window)
                    .map(|scroll| scroll.0),
                Some(offset)
            );
            if text.0.ends_with("Available") {
                assert_ne!(text.0, first_text);
                assert_eq!(
                    app.world().get::<TextColor>(row).map(|color| color.0),
                    Some(source::TEXT_COLOR)
                );
                break;
            }
            if std::time::Instant::now() >= deadline {
                return Err("cooldown did not refresh its destination row".into());
            }
            std::thread::yield_now();
        }
        Ok(())
    }
}
