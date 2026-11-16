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
    ecs::system::SystemParam,
    picking::{
        events::{PointerCancel, PointerDrag, PointerDragEnd, PointerDragStart},
        pointer::{PointerButton, PointerId},
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
/// The drag handle of a pane; `gesture` holds the running drag.
#[derive(Component, Clone, Copy)]
struct TitleBar {
    panel: HudPanel,
    gesture: Option<Gesture>,
}
/// A title-bar drag: the pointer, the pane position it started from and the coordinate
/// space it started in. Any change of that space ends the drag without saving it.
#[derive(Clone, Copy, PartialEq, Debug)]
struct Gesture {
    pointer: PointerId,
    origin: Vec2,
    context: Context,
}
#[derive(Clone, Copy, PartialEq, Debug)]
struct Context {
    window: Vec2,
    dpi: f32,
    scale: f32,
    header: f32,
}

/// The one coordinate space of the panes. The layout works in window pixels below the
/// header (vitals and pane-toggle strip) with the interface scale applied; pane nodes are
/// in UI units, which Bevy multiplies by `UiScale` (the same interface scale) when it
/// renders them, and start below the header.
#[derive(SystemParam)]
struct Space<'w, 's> {
    windows: Query<'w, 's, &'static Window, With<PrimaryWindow>>,
    scale: Res<'w, UiScale>,
    regions: Res<'w, super::GameRegions>,
    computed: Query<'w, 's, &'static ComputedNode>,
}
impl Space<'_, '_> {
    fn scale(&self) -> f32 {
        if self.scale.0.is_finite() && self.scale.0 > 0. {
            self.scale.0
        } else {
            1.
        }
    }
    /// The bottom of the header in UI units; the panes never cover it.
    fn header(&self) -> f32 {
        self.computed
            .get(self.regions.vitals)
            .map(|node| node.size.y * node.inverse_scale_factor)
            .ok()
            .filter(|height| height.is_finite())
            .map_or(0., |height| height.max(0.))
    }
    /// The area the layout positions panes in: the window below the header, in window pixels.
    fn root(&self) -> [u16; 2] {
        let header = self.header() * self.scale();
        self.windows.single().map_or([1024, 720], |window| {
            [window.width(), window.height() - header].map(|size| size.clamp(1., 4096.) as u16)
        })
    }
    /// The space a drag runs in; `None` without a focused window.
    fn context(&self) -> Option<Context> {
        let window = self.windows.single().ok().filter(|window| window.focused)?;
        Some(Context {
            window: window.size(),
            dpi: window.scale_factor(),
            scale: self.scale(),
            header: self.header(),
        })
    }
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
    .add_systems(Update, (cancel_stale, project).chain())
    .add_observer(activate)
    .add_observer(cancel);
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
                    gesture: None,
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
    space: Space,
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
    apply(&mut owner, action, space.root());
}

fn drag_start(
    mut event: On<PointerDragStart>,
    mut bars: Query<&mut TitleBar>,
    panes: Query<(&Pane, &Node)>,
    guard: ModalGuard,
    space: Space,
) {
    if event.button != PointerButton::Primary || !guard.allows(event.entity) {
        return;
    }
    let Ok(mut bar) = bars.get_mut(event.entity) else {
        return;
    };
    if bar.gesture.is_some() {
        return;
    }
    let Some(context) = space.context() else {
        return;
    };
    let panel = bar.panel;
    bar.gesture = panes
        .iter()
        .find(|(pane, _)| pane.0 == panel)
        .and_then(|(_, node)| match (node.left, node.top) {
            (Val::Px(x), Val::Px(y)) => Some(Vec2::new(x, y)),
            _ => None,
        })
        .map(|origin| Gesture {
            pointer: event.pointer.id,
            origin,
            context,
        });
    event.propagate(false);
}

/// Whether a running drag may still move or drop its pane: same pointer, same coordinate
/// space, a focused window and no modal above the bar.
fn live(
    gesture: &Gesture,
    pointer: PointerId,
    space: &Space,
    guard: &ModalGuard,
    bar: Entity,
) -> bool {
    gesture.pointer == pointer && space.context() == Some(gesture.context) && guard.allows(bar)
}

fn drag(
    mut event: On<PointerDrag>,
    mut bars: Query<&mut TitleBar>,
    mut ui: ParamSet<(ModalGuard, Query<(&Pane, &mut Node)>)>,
    space: Space,
) {
    let Ok(mut bar) = bars.get_mut(event.entity) else {
        return;
    };
    let Some(gesture) = bar.gesture else {
        return;
    };
    if !live(&gesture, event.pointer.id, &space, &ui.p0(), event.entity) {
        bar.gesture = None;
        return;
    }
    if event.button != PointerButton::Primary || !event.distance.is_finite() {
        return;
    }
    let at = (gesture.origin + event.distance / gesture.context.scale)
        .max(Vec2::new(0., gesture.context.header));
    for (pane, mut node) in &mut ui.p1() {
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
    (guard, space): (ModalGuard, Space),
) {
    let Ok(mut bar) = bars.get_mut(event.entity) else {
        return;
    };
    let Some(gesture) = bar.gesture else {
        return;
    };
    if gesture.pointer != event.pointer.id {
        return;
    }
    bar.gesture = None;
    if event.button != PointerButton::Primary
        || !live(&gesture, event.pointer.id, &space, &guard, event.entity)
    {
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
    let Context { scale, header, .. } = gesture.context;
    let bounds =
        [x, y - header, width, height].map(|value| (value * scale).round().clamp(0., 4096.) as u16);
    apply(&mut owner, HudAction::Dragged(panel, bounds), space.root());
}

/// A cancelled pointer (a lifted touch, a lost capture) ends its drag without saving it.
fn cancel(event: On<PointerCancel>, mut bars: Query<&mut TitleBar>) {
    for mut bar in &mut bars {
        if bar
            .gesture
            .is_some_and(|gesture| gesture.pointer == event.pointer.id)
        {
            bar.gesture = None;
        }
    }
}

/// Ends a drag whose window, scale, header or modal state changed since it started; the
/// projection then puts the pane back where the layout keeps it.
fn cancel_stale(mut bars: Query<(Entity, &mut TitleBar)>, guard: ModalGuard, space: Space) {
    let context = space.context();
    for (entity, mut bar) in &mut bars {
        if bar
            .gesture
            .is_some_and(|gesture| context != Some(gesture.context) || !guard.allows(entity))
        {
            bar.gesture = None;
        }
    }
}

fn project(
    mut commands: Commands,
    (owner, playback): (
        NonSend<HostOwner>,
        Option<NonSend<crate::bevy_shell::audio::Playback>>,
    ),
    (space, bars): (Space, Query<&TitleBar>),
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
    let (root, scale, header) = (space.root(), space.scale(), space.header());
    for (pane, mut node) in &mut panes {
        if bars
            .iter()
            .any(|bar| bar.panel == pane.0 && bar.gesture.is_some())
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
            px(f32::from(x) / scale),
            px(header + f32::from(y) / scale),
            px(f32::from(width) / scale),
            px(f32::from(height) / scale),
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
