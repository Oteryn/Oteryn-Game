//! Tibia Global sidebar widget chrome (SidebarWidget.qml): a 15 px title bar with the title, a
//! 12x12 minimise and close button and drag-to-move, a 4 px border, a 19 px minimised height
//! and a bottom edge that resizes the content. Widgets live in the widget pane of a sidebar
//! column; the 9x27 arrows beside the game window add and remove the optional sidebars.
use super::{
    super::{
        owner::HostOwner,
        ui::{
            Skins,
            window_layout::{Scaled, WindowId, WindowLayouts},
        },
    },
    art,
    modal::ModalGuard,
};
use bevy::{
    picking::{
        events::{PointerCancel, PointerDrag, PointerDragEnd, PointerDragStart},
        pointer::{PointerButton, PointerId},
    },
    prelude::*,
    ui_widgets::{Activate, Button},
};
use oteryn_client_ui_bevy as source;

pub(super) mod context_menu;
mod dock;
pub(in crate::bevy_shell) mod float;

/// SidebarWidget.qml geometry.
pub(super) const HEADER: f32 = 15.;
pub(super) const BORDER: f32 = 4.;
pub(super) const MINIMISED: f32 = 19.;
pub(super) const MIN_CONTENT: f32 = 38.;
const MAX_CONTENT: f32 = 600.;
/// The add and remove sidebar arrows beside the game window.
pub(super) const ARROW: Vec2 = Vec2::new(9., 27.);

/// A sidebar column (left, second right or right).
#[derive(Component)]
pub(super) struct Column;
/// The dockable widget pane of a column.
#[derive(Component)]
pub(super) struct WidgetPane;
/// The fill pane that closes every widget pane.
#[derive(Component)]
pub(super) struct FillPane;

/// The dockable widgets with chrome.
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::bevy_shell) enum Widget {
    BattleList,
    Containers,
    Skills,
    Vip,
    AnalyticsSelector,
}
impl Widget {
    pub(super) const ALL: [Self; 5] = [
        Self::BattleList,
        Self::Containers,
        Self::Skills,
        Self::Vip,
        Self::AnalyticsSelector,
    ];
    pub(super) fn title(self) -> &'static str {
        match self {
            Self::BattleList => "Battle List",
            Self::Containers => "Containers",
            Self::Skills => "Skills",
            Self::Vip => "VIP",
            Self::AnalyticsSelector => "Analytics Selector",
        }
    }
    /// The initial content height of the QML widget; `None` sizes to the content.
    pub(super) fn content_height(self) -> Option<f32> {
        match self {
            Self::BattleList => Some(75.),
            Self::Skills | Self::Vip => Some(200.),
            Self::Containers | Self::AnalyticsSelector => None,
        }
    }
    /// Tibia sidebars.json opens battleList, container, skills and vip; analysers stay closed.
    fn open_by_default(self) -> bool {
        !matches!(self, Self::AnalyticsSelector)
    }
    /// The settings panel id that also decides whether the widget is shown.
    fn panel(self) -> Option<&'static str> {
        match self {
            Self::BattleList => Some("battle"),
            Self::Vip => Some("vip"),
            Self::Containers => Some("inventory"),
            Self::Skills | Self::AnalyticsSelector => None,
        }
    }
}
/// The column a widget is docked in.
#[derive(Clone, Copy, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Dock {
    Left,
    Extra,
    Right,
}
/// The side of the game window an arrow pair sits on: left adds the left sidebar, right
/// adds the second right sidebar.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Side {
    Left,
    Right,
}
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) struct Entry {
    pub widget: Widget,
    pub dock: Dock,
    pub open: bool,
    pub minimised: bool,
    pub height: Option<f32>,
    /// Floating in its own window instead of its dock.
    pub floating: bool,
}
/// The open set, order, dock, minimised state and content height of every widget, and the
/// optional sidebars.
#[derive(Resource, Clone, Debug)]
pub(in crate::bevy_shell) struct SidebarWidgets {
    pub(super) entries: Vec<Entry>,
    pub(super) left: bool,
    pub(super) extra: bool,
    /// The regions (minimap, chat) floating in their own window.
    pub(super) floating: Vec<&'static str>,
    saved_revision: Option<std::sync::Arc<[u8]>>,
}
impl SidebarWidgets {
    pub(super) fn entry(&self, widget: Widget) -> Option<&Entry> {
        self.entries.iter().find(|entry| entry.widget == widget)
    }
    fn entry_mut(&mut self, widget: Widget) -> Option<&mut Entry> {
        self.entries.iter_mut().find(|entry| entry.widget == widget)
    }
    /// Opens a closed widget (restoring it if minimised) or closes an open one.
    pub(in crate::bevy_shell) fn toggle(&mut self, widget: Widget) -> bool {
        let Some(entry) = self.entry_mut(widget) else {
            return false;
        };
        entry.open = !entry.open || entry.minimised;
        entry.minimised = false;
        entry.open
    }
    pub(in crate::bevy_shell) fn set_open(&mut self, widget: Widget, open: bool) {
        if let Some(entry) = self.entry_mut(widget) {
            entry.open = open;
            entry.minimised &= open;
        }
    }
    /// Moves `widget` into `dock` before `before` (or to the end of the dock).
    pub(super) fn place(&mut self, widget: Widget, dock: Dock, before: Option<Widget>) {
        let Some(index) = self.entries.iter().position(|entry| entry.widget == widget) else {
            return;
        };
        let mut entry = self.entries.remove(index);
        entry.dock = dock;
        entry.floating = false;
        let at = before
            .and_then(|before| self.entries.iter().position(|entry| entry.widget == before))
            .or_else(|| {
                self.entries
                    .iter()
                    .rposition(|entry| entry.dock == dock)
                    .map(|index| index + 1)
            })
            .unwrap_or(self.entries.len());
        self.entries.insert(at, entry);
    }
    /// Shows or hides an optional sidebar; the widgets of a removed sidebar go back to the
    /// right sidebar.
    pub(super) fn set_sidebar(&mut self, side: Side, shown: bool) {
        let dock = match side {
            Side::Left => {
                self.left = shown;
                Dock::Left
            }
            Side::Right => {
                self.extra = shown;
                Dock::Extra
            }
        };
        if !shown {
            for entry in &mut self.entries {
                if entry.dock == dock {
                    entry.dock = Dock::Right;
                }
            }
        }
    }
}

/// The outer node of a widget.
#[derive(Component, Clone, Copy)]
pub(super) struct WidgetFrame(pub Widget);
#[derive(Component, Clone, Copy)]
struct Body(Widget);
#[derive(Component, Clone, Copy)]
struct Handle(Widget);
#[derive(Component, Clone, Copy)]
struct TitleBar {
    widget: Widget,
    pointer: Option<PointerId>,
}
#[derive(Component, Clone, Copy)]
struct Resize {
    widget: Widget,
    gesture: Option<(PointerId, f32)>,
}
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum TitleButton {
    Float(Widget),
    Minimise(Widget),
    Close(Widget),
}
#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
pub(super) struct SidebarArrow {
    side: Side,
    add: bool,
}

pub(super) fn install(app: &mut App) {
    art::install(app);
    app.add_message::<bevy::input::keyboard::KeyboardInput>()
        .init_resource::<SidebarWidgets>()
        .add_systems(Last, dock::persist)
        .add_systems(
            Update,
            (
                context_menu::layout,
                (project, float::project).chain(),
                float::escape,
                super::super::ui::toolbar::grid::project,
            ),
        )
        .add_observer(activate)
        .add_observer(dock::reset)
        .add_observer(float::toggle)
        .add_observer(cancel);
}

/// Wraps `content` in the chrome of `widget`; returns the outer node.
pub(super) fn frame(
    commands: &mut Commands,
    skins: &Skins,
    widget: Widget,
    content: Entity,
) -> Entity {
    let outer = commands
        .spawn((
            WidgetFrame(widget),
            Name::new(widget.title()),
            WindowId(widget.window_id()),
            Scaled::default(),
            source::frame(skins.frame.clone(), [BORDER; 4]),
            Node {
                width: percent(100.),
                flex_shrink: 0.,
                flex_direction: FlexDirection::Column,
                padding: UiRect::horizontal(px(BORDER)),
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .id();
    let bar = commands
        .spawn((
            TitleBar {
                widget,
                pointer: None,
            },
            Node {
                width: percent(100.),
                height: px(HEADER),
                flex_shrink: 0.,
                column_gap: px(2.),
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .observe(move_start)
        .observe(move_end)
        .id();
    let title = commands
        .spawn((
            source::text(widget.title(), source::font(skins.font.clone(), false)),
            Node {
                flex_grow: 1.,
                min_width: px(0.),
                overflow: Overflow::clip_x(),
                ..default()
            },
        ))
        .id();
    commands.entity(bar).add_child(title);
    for (button, key) in [
        (TitleButton::Float(widget), "widget-float"),
        (TitleButton::Minimise(widget), "widget-minimise"),
        (TitleButton::Close(widget), "widget-close"),
    ] {
        let slot = art::fixed(commands, &skins.font, key, Vec2::splat(12.));
        commands.entity(slot).insert((button, Button));
        commands.entity(bar).add_child(slot);
    }
    let body = commands
        .spawn((
            Body(widget),
            Node {
                width: percent(100.),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                height: widget.content_height().map_or(Val::Auto, px),
                ..default()
            },
            bevy::ui_widgets::ScrollArea,
        ))
        .id();
    commands.entity(body).add_child(content);
    let handle = commands
        .spawn((
            Handle(widget),
            Resize {
                widget,
                gesture: None,
            },
            Name::new("Resize widget"),
            Node {
                width: percent(100.),
                height: px(BORDER),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .observe(resize_start)
        .observe(resize)
        .observe(resize_end)
        .id();
    commands.entity(outer).add_children(&[bar, body, handle]);
    outer
}

/// The arrow pair on one side of the game window.
pub(super) fn arrows(commands: &mut Commands, skins: &Skins, side: Side) -> Entity {
    let strip = commands
        .spawn(Node {
            width: px(ARROW.x),
            height: percent(100.),
            flex_shrink: 0.,
            flex_direction: FlexDirection::Column,
            row_gap: px(2.),
            ..default()
        })
        .id();
    for add in [true, false] {
        let key = if add == (side == Side::Left) {
            "sidebar-add"
        } else {
            "sidebar-remove"
        };
        let slot = art::fixed(commands, &skins.font, key, ARROW);
        commands
            .entity(slot)
            .insert((SidebarArrow { side, add }, Button));
        commands.entity(strip).add_child(slot);
    }
    strip
}

/// Whether `widget` is shown: its own open flag and, for battle, VIP and containers, the
/// settings panel that the control buttons toggle.
fn shown(owner: &HostOwner, state: &SidebarWidgets, widget: Widget) -> bool {
    let Some(entry) = state.entry(widget) else {
        return false;
    };
    entry.open
        && widget.panel().is_none_or(|id| {
            owner.settings.preferences.panel_visible(id)
                || id == "vip"
                    && owner
                        .play
                        .as_ref()
                        .is_some_and(|play| play.link.vip_is_open())
        })
}

fn activate(
    event: On<Activate>,
    buttons: Query<&TitleButton>,
    arrows: Query<&SidebarArrow>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<SidebarWidgets>,
) {
    if !guard.allows(event.entity) || owner.modal_active() {
        return;
    }
    if let Ok(arrow) = arrows.get(event.entity) {
        state.set_sidebar(arrow.side, arrow.add);
        return;
    }
    let Ok(button) = buttons.get(event.entity) else {
        return;
    };
    match *button {
        TitleButton::Float(widget) => {
            if let Some(entry) = state.entry_mut(widget) {
                entry.floating = !entry.floating;
                entry.minimised = false;
            }
        }
        TitleButton::Minimise(widget) => {
            if let Some(entry) = state.entry_mut(widget) {
                entry.minimised = !entry.minimised;
            }
        }
        TitleButton::Close(widget) => close(&mut owner, &mut state, widget),
    }
}
/// Battle and VIP close through the same owner the control buttons use, so the VIP
/// subscription ends and the setting is saved.
fn close(owner: &mut HostOwner, state: &mut SidebarWidgets, widget: Widget) {
    if let Some(id @ ("battle" | "vip")) = widget.panel() {
        if owner.settings.preferences.panel_visible(id)
            || owner
                .play
                .as_ref()
                .is_some_and(|play| id == "vip" && play.link.vip_is_open())
        {
            super::super::ui::toolbar::open_toolbar(owner, super::super::ui::Control::Panel(id));
        }
    } else {
        state.set_open(widget, false);
    }
}

mod drag;
use drag::{cancel, move_end, move_start, resize, resize_end, resize_start};

/// Applies the widget state: open and minimised widgets, content heights, the order of every
/// widget pane and the optional sidebars.
// Disjoint node roles of one widget are updated together from one state snapshot.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    state: Res<SidebarWidgets>,
    layouts: Option<Res<WindowLayouts>>,
    sidebar: Option<Res<super::Sidebar>>,
    frames: Query<(Entity, &WidgetFrame)>,
    mut nodes: ParamSet<(
        Query<(&WidgetFrame, &mut Node)>,
        Query<(&Body, &mut Node)>,
        Query<(&Handle, &mut Node)>,
        Query<&mut Node, With<Column>>,
    )>,
    children: Query<&Children>,
    fills: Query<(), With<FillPane>>,
) {
    for (frame, mut node) in &mut nodes.p0() {
        let display = if shown(&owner, &state, frame.0) {
            Display::Flex
        } else {
            Display::None
        };
        let minimised = state.entry(frame.0).is_some_and(|entry| entry.minimised);
        let height = if minimised { px(MINIMISED) } else { Val::Auto };
        let grow = if state.entry(frame.0).is_some_and(|entry| entry.floating) {
            1.
        } else {
            0.
        };
        if node.display != display || node.height != height || node.flex_grow != grow {
            node.display = display;
            node.height = height;
            node.flex_grow = grow;
        }
    }
    for (body, mut node) in &mut nodes.p1() {
        let Some(entry) = state.entry(body.0) else {
            continue;
        };
        let display = if entry.minimised {
            Display::None
        } else {
            Display::Flex
        };
        let zoom = dock::zoom(layouts.as_deref(), body.0);
        let fills = entry.floating
            && layouts
                .as_deref()
                .and_then(|layouts| layouts.0.get(body.0.window_id()))
                .is_some_and(|layout| layout.size.is_some());
        let (height, grow) = if fills {
            (Val::Auto, 1.)
        } else {
            (
                entry.height.map_or(Val::Auto, |height| px(height * zoom)),
                0.,
            )
        };
        if node.display != display || node.height != height || node.flex_grow != grow {
            node.display = display;
            node.height = height;
            node.flex_grow = grow;
        }
    }
    for (handle, mut node) in &mut nodes.p2() {
        let display = if state.entry(handle.0).is_some_and(|entry| entry.minimised) {
            Display::None
        } else {
            Display::Flex
        };
        if node.display != display {
            node.display = display;
        }
    }
    let Some(sidebar) = sidebar else {
        return;
    };
    let mut columns = nodes.p3();
    for (column, shown) in [(sidebar.left, state.left), (sidebar.extra, state.extra)] {
        if let Ok(mut node) = columns.get_mut(column) {
            let display = if shown { Display::Flex } else { Display::None };
            if node.display != display {
                node.display = display;
            }
        }
    }
    // `replace_children` skips the relationship hook for added children, so a widget moving
    // between panes leaves its old pane first; otherwise the old pane's replace detaches it.
    let mut replaced = Vec::with_capacity(3);
    for (pane, dock) in [
        (sidebar.left_widgets, Dock::Left),
        (sidebar.extra_widgets, Dock::Extra),
        (sidebar.right_widgets, Dock::Right),
    ] {
        let current: Vec<Entity> = children
            .get(pane)
            .map(|children| children.iter().collect())
            .unwrap_or_default();
        let mut desired: Vec<Entity> = state
            .entries
            .iter()
            .filter(|entry| entry.dock == dock && !entry.floating)
            .filter_map(|entry| {
                frames
                    .iter()
                    .find(|(_, frame)| frame.0 == entry.widget)
                    .map(|(entity, _)| entity)
            })
            .collect();
        desired.extend(
            current
                .iter()
                .filter(|entity| !frames.contains(**entity) && !fills.contains(**entity)),
        );
        desired.extend(current.iter().filter(|entity| fills.contains(**entity)));
        if desired != current {
            for arriving in desired.iter().filter(|entity| !current.contains(entity)) {
                commands.entity(*arriving).remove::<ChildOf>();
            }
            replaced.push((pane, desired));
        }
    }
    for (pane, desired) in replaced {
        commands.entity(pane).replace_children(&desired);
    }
}

#[cfg(test)]
#[path = "hud_panes_context_menu_tests.rs"]
mod context_menu_tests;
#[cfg(test)]
#[path = "hud_panes_tests.rs"]
pub(super) mod tests;
