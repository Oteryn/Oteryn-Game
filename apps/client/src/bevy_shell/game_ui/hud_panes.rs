//! The four movable HUD panes of the local layout (Status, Combat, Conditions and rest,
//! Monk): shown or hidden, docked left, right or top or floating, ordered within a dock and
//! dragged by the title bar. Every change goes through `PlayView::hud_action` and is saved
//! with the preferences, so the panes come back where they were after a restart.
//! Behaviour of the removed native panes: b6305c1c^
//! apps/client/src/features/local-spell-action-bar/native.rs and native/events.rs.
use super::{
    super::{owner::HostOwner, ui::Skins},
    modal::ModalGuard,
};
use crate::play::{HudAction, HudDock, HudPanel};
use bevy::{
    picking::{
        events::{PointerDrag, PointerDragEnd, PointerDragStart},
        pointer::PointerButton,
    },
    prelude::*,
    ui::Checked,
    ui_widgets::Activate,
    window::PrimaryWindow,
};
use oteryn_client_ui_bevy as source;

#[derive(Component, Clone, Copy)]
struct Pane(HudPanel);
#[derive(Component, Clone, Copy)]
struct PaneText(HudPanel);
/// The drag handle of a pane; `origin` holds the pane position while a drag runs.
#[derive(Component, Clone, Copy)]
struct TitleBar {
    panel: HudPanel,
    origin: Option<Vec2>,
}
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum PaneCommand {
    Toggle(HudPanel),
    Earlier(HudPanel),
    Later(HudPanel),
    Dock(HudPanel, HudDock),
    Hide(HudPanel),
    Reset,
}

pub(super) fn install(app: &mut App) {
    app.add_systems(
        Startup,
        setup.after(super::setup).after(super::super::ui::setup),
    )
    .add_systems(Update, project)
    .add_observer(activate);
}

fn button(commands: &mut Commands, skins: &Skins, command: PaneCommand, label: &str) -> Entity {
    let width = 14. + 7. * label.chars().count() as f32;
    let entity = commands
        .spawn((command, source::button(skins.buttons.clone(), width)))
        .id();
    let caption = commands
        .spawn(source::button_caption(label, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(caption);
    entity
}

fn setup(mut commands: Commands, regions: Res<super::GameRegions>, skins: Res<Skins>) {
    let strip = commands
        .spawn(Node {
            width: percent(100.),
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(2.),
            row_gap: px(2.),
            margin: UiRect::vertical(px(2.)),
            ..default()
        })
        .id();
    for panel in HudPanel::ALL {
        let toggle = button(
            &mut commands,
            &skins,
            PaneCommand::Toggle(panel),
            panel.title(),
        );
        commands.entity(strip).add_child(toggle);
    }
    let reset = button(&mut commands, &skins, PaneCommand::Reset, "Reset panes");
    commands.entity(strip).add_child(reset);
    commands.entity(regions.vitals).add_child(strip);

    for panel in HudPanel::ALL {
        let pane = commands
            .spawn((
                Pane(panel),
                GlobalZIndex(20),
                source::frame(skins.frame.clone(), [4., 4., 4., 4.]),
                Node {
                    position_type: PositionType::Absolute,
                    display: Display::None,
                    padding: px(4.).all(),
                    row_gap: px(2.),
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.1, 0.1, 0.1, 0.9)),
            ))
            .id();
        let bar = commands
            .spawn((
                TitleBar {
                    panel,
                    origin: None,
                },
                Node {
                    width: percent(100.),
                    column_gap: px(2.),
                    align_items: AlignItems::Center,
                    flex_wrap: FlexWrap::Wrap,
                    ..default()
                },
            ))
            .observe(drag_start)
            .observe(drag)
            .observe(drag_end)
            .id();
        let title = commands
            .spawn(source::text(
                panel.title(),
                source::font(skins.font.clone(), false),
            ))
            .id();
        commands.entity(bar).add_child(title);
        for (command, label) in [
            (PaneCommand::Earlier(panel), "Up"),
            (PaneCommand::Later(panel), "Down"),
            (PaneCommand::Dock(panel, HudDock::Left), "Left"),
            (PaneCommand::Dock(panel, HudDock::Right), "Right"),
            (PaneCommand::Dock(panel, HudDock::Top), "Top"),
            (PaneCommand::Dock(panel, HudDock::Floating), "Float"),
            (PaneCommand::Hide(panel), "Hide"),
        ] {
            let entity = button(&mut commands, &skins, command, label);
            commands.entity(bar).add_child(entity);
        }
        let body = commands
            .spawn((
                PaneText(panel),
                source::text("", source::font(skins.font.clone(), false)),
            ))
            .id();
        commands.entity(pane).add_children(&[bar, body]);
    }
}

/// The window's logical size, the root the layout positions panes in.
fn root(windows: &Query<&Window, With<PrimaryWindow>>) -> [u16; 2] {
    windows.single().map_or([1024, 720], |window| {
        [window.width(), window.height()].map(|size| size.clamp(1., 4096.) as u16)
    })
}

/// Applies one layout edit to the play view and saves it with the preferences.
fn apply(owner: &mut HostOwner, action: HudAction, root: [u16; 2]) {
    let HostOwner { play, settings, .. } = owner;
    let Some(play) = play.as_mut() else {
        return;
    };
    if let Err(error) = play
        .view
        .hud_action(action, root)
        .and_then(|()| settings.capture(&mut play.view))
    {
        eprintln!("Could not save HUD layout: {error}");
    }
}

fn activate(
    event: On<Activate>,
    commands: Query<&PaneCommand>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(command) = commands.get(event.entity) else {
        return;
    };
    if !guard.allows(event.entity) {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let action = match *command {
        PaneCommand::Toggle(panel) => HudAction::Visible(
            panel,
            !play
                .view
                .preferences()
                .effective_hud_layout()
                .pane(panel)
                .visible,
        ),
        PaneCommand::Earlier(panel) => HudAction::Earlier(panel),
        PaneCommand::Later(panel) => HudAction::Later(panel),
        PaneCommand::Dock(panel, dock) => HudAction::Dock(panel, dock),
        PaneCommand::Hide(panel) => HudAction::Visible(panel, false),
        PaneCommand::Reset => HudAction::Reset,
    };
    apply(&mut owner, action, root(&windows));
}

fn drag_start(
    mut event: On<PointerDragStart>,
    mut bars: Query<&mut TitleBar>,
    panes: Query<(&Pane, &Node)>,
    guard: ModalGuard,
) {
    if event.button != PointerButton::Primary || !guard.allows(event.entity) {
        return;
    }
    let Ok(mut bar) = bars.get_mut(event.entity) else {
        return;
    };
    let panel = bar.panel;
    bar.origin = panes
        .iter()
        .find(|(pane, _)| pane.0 == panel)
        .and_then(|(_, node)| match (node.left, node.top) {
            (Val::Px(x), Val::Px(y)) => Some(Vec2::new(x, y)),
            _ => None,
        });
    event.propagate(false);
}

fn drag(
    mut event: On<PointerDrag>,
    bars: Query<&TitleBar>,
    mut panes: Query<(&Pane, &mut Node)>,
    scale: Res<UiScale>,
) {
    let Ok(bar) = bars.get(event.entity) else {
        return;
    };
    let Some(origin) = bar.origin else {
        return;
    };
    if event.button != PointerButton::Primary || !event.distance.is_finite() {
        return;
    }
    let at = (origin + event.distance / scale.0.max(0.01)).max(Vec2::ZERO);
    for (pane, mut node) in &mut panes {
        if pane.0 == bar.panel {
            node.left = px(at.x);
            node.top = px(at.y);
        }
    }
    event.propagate(false);
}

/// Hands the dropped bounds to the layout, which docks the pane by where it landed.
fn drag_end(
    event: On<PointerDragEnd>,
    mut bars: Query<&mut TitleBar>,
    panes: Query<(&Pane, &Node)>,
    mut owner: NonSendMut<HostOwner>,
    windows: Query<&Window, With<PrimaryWindow>>,
) {
    let Ok(mut bar) = bars.get_mut(event.entity) else {
        return;
    };
    if bar.origin.take().is_none() || event.button != PointerButton::Primary {
        return;
    }
    let panel = bar.panel;
    let Some((_, node)) = panes.iter().find(|(pane, _)| pane.0 == panel) else {
        return;
    };
    let (Val::Px(x), Val::Px(y), Val::Px(width), Val::Px(height)) =
        (node.left, node.top, node.width, node.height)
    else {
        return;
    };
    let bounds = [x, y, width, height].map(|value| value.clamp(0., 4096.) as u16);
    apply(
        &mut owner,
        HudAction::Dragged(panel, bounds),
        root(&windows),
    );
}

fn project(
    mut commands: Commands,
    (owner, playback): (
        NonSend<HostOwner>,
        Option<NonSend<crate::bevy_shell::audio::Playback>>,
    ),
    (windows, bars): (Query<&Window, With<PrimaryWindow>>, Query<&TitleBar>),
    mut panes: Query<(&Pane, &mut Node)>,
    mut texts: Query<(&PaneText, &mut Text)>,
    toggles: Query<(Entity, &PaneCommand, Has<Checked>)>,
) {
    let Some(play) = &owner.play else {
        for (_, mut node) in &mut panes {
            if node.display != Display::None {
                node.display = Display::None;
            }
        }
        return;
    };
    let preferences = play.view.preferences();
    let layout = preferences.effective_hud_layout();
    let root = root(&windows);
    for (pane, mut node) in &mut panes {
        if bars
            .iter()
            .any(|bar| bar.panel == pane.0 && bar.origin.is_some())
        {
            continue;
        }
        let Some([x, y, width, height]) = layout.rect(pane.0, root, preferences.interface_scale)
        else {
            if node.display != Display::None {
                node.display = Display::None;
            }
            continue;
        };
        let (left, top, width, height) = (
            px(f32::from(x)),
            px(f32::from(y)),
            px(f32::from(width)),
            px(f32::from(height)),
        );
        if node.display != Display::Flex
            || node.left != left
            || node.top != top
            || node.width != width
            || node.height != height
        {
            node.display = Display::Flex;
            node.left = left;
            node.top = top;
            node.width = width;
            node.height = height;
        }
    }
    for (pane, mut text) in &mut texts {
        if !layout.pane(pane.0).visible {
            continue;
        }
        let next = match pane.0 {
            HudPanel::Resources => {
                super::hud::status_line(play, playback.as_ref().and_then(|audio| audio.status()))
            }
            HudPanel::Combat => play.view.combat_status(),
            HudPanel::Conditions => play.view.hud_conditions_text(),
            HudPanel::Monk => play.view.hud_monk_text(),
        };
        if text.0 != next {
            text.0 = next;
        }
    }
    for (entity, command, checked) in &toggles {
        let PaneCommand::Toggle(panel) = command else {
            continue;
        };
        let visible = layout.pane(*panel).visible;
        if visible && !checked {
            commands.entity(entity).insert(Checked);
        } else if !visible && checked {
            commands.entity(entity).remove::<Checked>();
        }
    }
}

#[cfg(test)]
#[path = "hud_panes_tests.rs"]
mod tests;
