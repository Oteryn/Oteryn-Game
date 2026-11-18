//! One place for each resizable window's size, minimum and content scale.
//!
//! The shared module layout state registers and persists these entries; until
//! then the entries live here and every window derives its sizes from them.
use bevy::{
    ecs::message::MessageCursor,
    input::mouse::AccumulatedMouseScroll,
    picking::{hover::HoverMap, pointer::PointerId},
    prelude::*,
    text::{FontSize, LineHeight},
    window::PrimaryWindow,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, io, path::Path};

mod menu;
mod resize;
mod stack;
pub(in crate::bevy_shell) use resize::ResizeCursor;
pub(in crate::bevy_shell) use stack::{OpenWindows, WindowLayer, WindowStack};

/// Sizes, zoom, resize grips, persistence, the focus stack and the window menu.
pub(in crate::bevy_shell) fn install(app: &mut App) {
    app.init_resource::<WindowLayouts>()
        .init_resource::<WindowStack>()
        .init_resource::<SavedBaseline>()
        .add_observer(stack::raise_on_press)
        .add_observer(menu::open)
        .add_observer(menu::dismiss)
        .add_systems(PreUpdate, load_layouts)
        .add_systems(Update, (zoom, menu::fit_open))
        .add_systems(Last, persist)
        .add_systems(
            PostUpdate,
            (
                (stack::adopt, project, stack::project)
                    .chain()
                    .before(bevy::ui::UiSystems::Layout),
                resize::anchor.after(bevy::ui::UiSystems::Layout),
            ),
        );
}

pub(in crate::bevy_shell) const MIN_ZOOM: f32 = 0.75;
pub(in crate::bevy_shell) const MAX_ZOOM: f32 = 2.;

/// The Global initial size (height `None` follows content) and the minimum, at 100%.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::bevy_shell) struct Geometry {
    pub width: f32,
    pub height: Option<f32>,
    pub min: Vec2,
}

const fn geometry(width: f32, height: Option<f32>, min: [f32; 2]) -> Geometry {
    Geometry {
        width,
        height,
        min: Vec2::new(min[0], min[1]),
    }
}

/// Ordered by window id.
pub(in crate::bevy_shell) const WINDOWS: &[(&str, Geometry)] = &[
    ("bestiary", geometry(940., Some(650.), [660., 460.])),
    ("bosstiary", geometry(650., None, [460., 300.])),
    ("chat", geometry(640., Some(200.), [320., 120.])),
    ("cyclopedia", geometry(700., Some(538.), [560., 400.])),
    ("cyclopedia_items", geometry(820., Some(620.), [580., 430.])),
    ("cyclopedia_map", geometry(700., None, [490., 300.])),
    ("depot", geometry(640., None, [450., 300.])),
    ("forge", geometry(760., None, [530., 300.])),
    ("gem_atelier", geometry(760., None, [530., 300.])),
    ("highscores", geometry(1050., Some(620.), [740., 430.])),
    ("houses", geometry(540., None, [380., 300.])),
    ("imbuing", geometry(760., None, [530., 300.])),
    ("item_details", geometry(482., None, [340., 300.])),
    ("journal", geometry(785., Some(620.), [550., 430.])),
    ("market", geometry(1000., Some(634.), [800., 480.])),
    ("minimap", geometry(176., None, [176., 130.])),
    (
        "module.analytics_selector",
        geometry(176., None, [176., 38.]),
    ),
    ("module.battle_list", geometry(176., None, [176., 38.])),
    ("module.containers", geometry(176., None, [176., 38.])),
    ("module.skills", geometry(176., None, [176., 38.])),
    ("module.vip", geometry(176., None, [176., 38.])),
    ("npc", geometry(600., None, [420., 350.])),
    ("options", geometry(685., Some(534.), [560., 420.])),
    ("outfit", geometry(760., None, [530., 300.])),
    ("prey", geometry(860., None, [600., 300.])),
    ("proficiency", geometry(1000., Some(600.), [700., 420.])),
    ("quick_loot", geometry(700., None, [490., 300.])),
    ("reward_wall", geometry(760., None, [530., 300.])),
    ("satellite_map", geometry(500., None, [350., 300.])),
    ("selection", geometry(745., None, [560., 300.])),
    ("stash", geometry(620., None, [430., 300.])),
    ("store", geometry(940., Some(620.), [660., 430.])),
    ("store_inbox", geometry(620., None, [430., 300.])),
    ("task_board", geometry(650., None, [460., 300.])),
    ("team_finder", geometry(720., None, [500., 300.])),
    ("wheel", geometry(780., None, [550., 300.])),
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::bevy_shell) struct Layout {
    pub geometry: Geometry,
    /// `None` until the user resizes a content-sized window.
    pub size: Option<Vec2>,
    pub zoom: f32,
    /// The user's move from the window's laid-out position, in UI pixels.
    pub offset: Vec2,
}

impl Layout {
    pub(in crate::bevy_shell) fn new(geometry: Geometry) -> Self {
        Self {
            geometry,
            size: geometry
                .height
                .map(|height| Vec2::new(geometry.width, height)),
            zoom: 1.,
            offset: Vec2::ZERO,
        }
    }
    pub(in crate::bevy_shell) fn scaled(&self, value: f32) -> f32 {
        value * self.zoom
    }
    pub(in crate::bevy_shell) fn min(&self) -> Vec2 {
        self.geometry.min * self.zoom
    }
    pub(in crate::bevy_shell) fn width(&self) -> f32 {
        self.size
            .map_or(self.scaled(self.geometry.width), |size| size.x)
    }
    /// A drag on an edge or corner: never below the minimum, never past the screen.
    pub(in crate::bevy_shell) fn resize(&mut self, requested: Vec2, viewport: Vec2) {
        self.size = Some(fit(requested, self.min(), viewport));
    }
    /// Ctrl+wheel or the window menu's Zoom; the size follows the content scale.
    pub(in crate::bevy_shell) fn set_zoom(&mut self, zoom: f32, viewport: Vec2) {
        let zoom = if zoom.is_finite() {
            zoom.clamp(MIN_ZOOM, MAX_ZOOM)
        } else {
            1.
        };
        if zoom == self.zoom {
            return;
        }
        let ratio = zoom / self.zoom;
        self.zoom = zoom;
        if let Some(size) = self.size {
            self.size = Some(fit(size * ratio, self.min(), viewport));
        }
    }
    pub(in crate::bevy_shell) fn reset_zoom(&mut self, viewport: Vec2) {
        self.set_zoom(1., viewport);
    }
    /// The screen resolution or the global UI scale changed.
    pub(in crate::bevy_shell) fn fit(&mut self, viewport: Vec2) {
        if let Some(size) = self.size {
            self.size = Some(fit(size, self.min(), viewport));
        }
    }
    /// The minimum never exceeds a known screen, so a window always fits on it.
    pub(in crate::bevy_shell) fn apply(&self, node: &mut Node, viewport: Option<Vec2>) {
        let viewport = viewport.map(|mut screen| {
            if self.size.is_none() {
                let inset = |value| match value {
                    Val::Percent(p) => screen.x * p / 100.,
                    Val::Px(x) => x,
                    _ => 0.,
                };
                screen.x = (screen.x - inset(node.left) - inset(node.right)).max(1.);
            }
            screen
        });
        let min = viewport.map_or(self.min(), |viewport| self.min().min(viewport));
        node.width = px(viewport
            .filter(|_| self.size.is_none())
            .map_or(self.width(), |screen| self.width().min(screen.x)));
        node.height = self.size.map_or(Val::Auto, |size| px(size.y));
        node.min_width = px(min.x);
        node.min_height = px(min.y);
    }
}

fn fit(requested: Vec2, min: Vec2, viewport: Vec2) -> Vec2 {
    let requested = if requested.is_finite() {
        requested
    } else {
        min
    };
    requested.max(min).min(viewport.max(Vec2::ONE))
}

/// The registry the shared layout state persists: one entry per window id.
#[derive(Resource, Clone, Debug, PartialEq)]
pub(in crate::bevy_shell) struct WindowLayouts(pub BTreeMap<&'static str, Layout>);

impl Default for WindowLayouts {
    fn default() -> Self {
        Self(
            WINDOWS
                .iter()
                .map(|(id, geometry)| (*id, Layout::new(*geometry)))
                .collect(),
        )
    }
}

/// What persists per window: size, zoom and position; the Global geometry is code.
#[derive(Serialize, Deserialize)]
struct Saved {
    size: Option<[f32; 2]>,
    zoom: f32,
    offset: [f32; 2],
}

impl Serialize for WindowLayouts {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let saved: BTreeMap<&str, Saved> = self
            .0
            .iter()
            .map(|(id, layout)| {
                (
                    *id,
                    Saved {
                        size: layout.size.map(|size| size.to_array()),
                        zoom: layout.zoom,
                        offset: layout.offset.to_array(),
                    },
                )
            })
            .collect();
        saved.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for WindowLayouts {
    /// Unknown windows are dropped and out-of-range values fall back, so a stale
    /// or hand-edited file can never place a window below its minimum.
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let saved = BTreeMap::<String, Saved>::deserialize(deserializer)?;
        let mut layouts = Self::default();
        for (id, layout) in &mut layouts.0 {
            let Some(saved) = saved.get(*id) else {
                continue;
            };
            if saved.zoom.is_finite() {
                layout.zoom = saved.zoom.clamp(MIN_ZOOM, MAX_ZOOM);
            }
            let offset = Vec2::from_array(saved.offset);
            if offset.is_finite() {
                layout.offset = offset;
            }
            if let Some(size) = saved.size.map(Vec2::from_array)
                && size.is_finite()
            {
                layout.size = Some(size.max(layout.min()));
            }
        }
        Ok(layouts)
    }
}

impl WindowLayouts {
    /// A missing file is the first start: every window at its Global size.
    pub(in crate::bevy_shell) fn load(path: &Path) -> io::Result<Self> {
        match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error)),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error),
        }
    }
    /// The window menu's Reset window layout: Global position, size and zoom, saved at once.
    pub(in crate::bevy_shell) fn reset(&mut self, path: Option<&Path>) -> io::Result<()> {
        *self = Self::default();
        path.map_or(Ok(()), |path| self.save(path))
    }
    /// Writes beside the target and renames, so a crash never leaves half a file.
    pub(in crate::bevy_shell) fn save(&self, path: &Path) -> io::Result<()> {
        Self::locked(path, || self.publish(path))
    }

    fn locked(path: &Path, write: impl FnOnce() -> io::Result<()>) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let lock = std::fs::File::options()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path.with_extension("json.lock"))?;
        lock.try_lock().map_err(io::Error::from)?;
        write()
    }

    fn publish(&self, path: &Path) -> io::Result<()> {
        let bytes = serde_json::to_vec_pretty(self)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        let staging = path.with_extension("json.tmp");
        std::fs::write(&staging, bytes)?;
        std::fs::rename(staging, path)
    }
}

/// The window menu's Reset window layout, for layout state kept outside `WindowLayouts`.
#[derive(Event, Clone, Copy, Debug)]
pub(in crate::bevy_shell) struct LayoutReset;

/// Marks a window root whose node follows its `WindowLayouts` entry.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::bevy_shell) struct WindowId(pub &'static str);

/// Base sizes at 100% for a node inside a window; they follow the window's zoom.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::bevy_shell) struct Scaled {
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub min_height: Option<f32>,
}

impl Scaled {
    pub(in crate::bevy_shell) fn square(side: f32) -> Self {
        Self {
            width: Some(side),
            height: Some(side),
            min_height: None,
        }
    }
    fn apply(&self, zoom: f32, node: &mut Node) {
        if let Some(width) = self.width {
            node.width = px(width * zoom);
        }
        if let Some(height) = self.height {
            node.height = px(height * zoom);
        }
        if let Some(height) = self.min_height {
            node.min_height = px(height * zoom);
        }
    }
}

/// The window root bundle at its initial size; `project` adds the screen-bounded minimum.
pub(in crate::bevy_shell) fn window(id: &'static str, mut node: Node) -> (WindowId, Node) {
    if let Some((_, geometry)) = WINDOWS.iter().find(|(candidate, _)| *candidate == id) {
        let layout = Layout::new(*geometry);
        node.width = px(layout.width());
        node.height = layout.size.map_or(Val::Auto, |size| px(size.y));
    }
    (WindowId(id), node)
}

/// Base font size and line height at 100% for text inside a window.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(in crate::bevy_shell) struct FontBase {
    size: f32,
    line: Option<f32>,
}

const ZOOM_STEP: f32 = 0.1;

/// The primary window's size in UI pixels: global UI scale and DPI already applied.
fn viewport(
    screens: &Query<&Window, With<PrimaryWindow>>,
    scale: Option<&UiScale>,
) -> Option<Vec2> {
    let factor = scale.map_or(1., |scale| scale.0);
    screens
        .single()
        .ok()
        .map(|screen| Vec2::new(screen.width(), screen.height()) / factor)
        .filter(|viewport| viewport.is_finite() && viewport.min_element() > 0.)
}

/// Ctrl+wheel over a window zooms its content; Ctrl+0 resets it to 100%.
#[allow(clippy::too_many_arguments)]
pub(in crate::bevy_shell) fn zoom(
    scroll: Option<Res<AccumulatedMouseScroll>>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
    hover: Option<Res<HoverMap>>,
    parents: Query<&ChildOf>,
    ids: Query<&WindowId>,
    screens: Query<&Window, With<PrimaryWindow>>,
    scale: Option<Res<UiScale>>,
    mut layouts: ResMut<WindowLayouts>,
) {
    let Some(keys) = keys else {
        return;
    };
    if !keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
        return;
    }
    let reset = keys.just_pressed(KeyCode::Digit0);
    let wheel = scroll.map_or(0., |scroll| scroll.delta.y);
    if !reset && wheel == 0. {
        return;
    }
    let Some(id) = hover
        .as_deref()
        .and_then(|hover| hover.get(&PointerId::Mouse))
        .into_iter()
        .flat_map(|hits| hits.keys())
        .find_map(|hit| {
            std::iter::once(*hit)
                .chain(parents.iter_ancestors(*hit))
                .find_map(|entity| ids.get(entity).ok())
        })
    else {
        return;
    };
    let Some(viewport) = viewport(&screens, scale.as_deref()) else {
        return;
    };
    let Some(current) = layouts.0.get(id.0).copied() else {
        return;
    };
    let mut next = current;
    if reset {
        next.reset_zoom(viewport);
    } else {
        let steps = (current.zoom / ZOOM_STEP).round() + wheel.signum();
        next.set_zoom(steps * ZOOM_STEP, viewport);
    }
    if next != current {
        layouts.0.insert(id.0, next);
    }
}

fn zoom_of(
    entity: Entity,
    parents: &Query<&ChildOf>,
    ids: &Query<&WindowId>,
    layouts: &WindowLayouts,
) -> Option<f32> {
    parents
        .iter_ancestors(entity)
        .find_map(|ancestor| ids.get(ancestor).ok())
        .and_then(|id| layouts.0.get(id.0))
        .map(|layout| layout.zoom)
}

// Window roots, zoomed nodes and zoomed text are disjoint projections of one registry.
type Roots<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        Ref<'static, WindowId>,
        &'static mut Node,
        Option<&'static mut UiTransform>,
    ),
    Without<Scaled>,
>;
type Fonts<'w, 's> = Query<
    'w,
    's,
    (
        Entity,
        &'static mut TextFont,
        Option<&'static mut LineHeight>,
        Option<&'static FontBase>,
    ),
>;

/// How long the layout must stay unchanged before it is written, so a drag writes once.
const SAVE_DELAY: std::time::Duration = std::time::Duration::from_secs(1);

#[derive(Resource, Default)]
struct SavedBaseline {
    layouts: Option<WindowLayouts>,
    reset_pending: bool,
}

fn load_layouts(
    owner: Option<NonSend<crate::bevy_shell::owner::HostOwner>>,
    mut layouts: ResMut<WindowLayouts>,
    mut loaded: ResMut<SavedBaseline>,
) {
    if loaded.layouts.is_some() {
        return;
    }
    let Some(path) = owner.and_then(|owner| owner.window_layouts_path()) else {
        return;
    };
    match WindowLayouts::load(&path) {
        Ok(saved) => *layouts = saved,
        Err(error) => eprintln!("Could not load window layouts: {error}"),
    }
    loaded.layouts = Some(layouts.clone());
}

#[derive(Default)]
struct ExitSave {
    reader: MessageCursor<AppExit>,
    deferred: Option<AppExit>,
}

impl ExitSave {
    fn retain(&mut self, exits: &mut Messages<AppExit>) {
        for exit in exits.drain() {
            if self.deferred.is_none()
                || (exit.is_error() && self.deferred.as_ref().is_some_and(AppExit::is_success))
            {
                self.deferred = Some(exit);
            }
        }
    }
}

/// Observe edits after the final resize correction, including on the exit frame.
fn persist(
    owner: Option<NonSend<crate::bevy_shell::owner::HostOwner>>,
    time: Res<Time<Real>>,
    layouts: Res<WindowLayouts>,
    mut loaded: ResMut<SavedBaseline>,
    mut pending: Local<Option<std::time::Duration>>,
    mut exits: ResMut<Messages<AppExit>>,
    mut exit_save: Local<ExitSave>,
) {
    let new_exit = exit_save.reader.read(&exits).count() > 0;
    let exiting = new_exit || exit_save.deferred.is_some();
    if exit_save.deferred.is_some() {
        exit_save.retain(&mut exits);
    }
    if let Some(baseline) = loaded.layouts.as_ref()
        && (loaded.reset_pending || baseline != &*layouts)
        && let Some(path) = owner.and_then(|owner| owner.window_layouts_path())
    {
        if layouts.is_changed() || pending.is_none() {
            *pending = Some(time.elapsed());
        }
        if !exiting
            && pending.is_some_and(|since| time.elapsed().saturating_sub(since) < SAVE_DELAY)
        {
            return;
        }
        let result = WindowLayouts::locked(&path, || {
            if loaded.reset_pending {
                return layouts.publish(&path);
            }
            let mut latest = match WindowLayouts::load(&path) {
                Err(error) if error.kind() == io::ErrorKind::InvalidData => baseline.clone(),
                result => result?,
            };
            for (&id, layout) in &layouts.0 {
                let (Some(before), Some(saved)) = (baseline.0.get(id), latest.0.get_mut(id)) else {
                    continue;
                };
                if layout.size != before.size {
                    saved.size = match (before.size, layout.size, saved.size) {
                        (Some(before), Some(edited), Some(latest)) => {
                            Some(Vec2::select(edited.cmpne(before), edited, latest))
                        }
                        _ => layout.size,
                    };
                }
                if layout.zoom != before.zoom {
                    saved.zoom = layout.zoom;
                }
                saved.offset = Vec2::select(
                    layout.offset.cmpne(before.offset),
                    layout.offset,
                    saved.offset,
                );
            }
            latest.publish(&path)
        });
        match result {
            Ok(()) => {
                loaded.layouts = Some(layouts.clone());
                loaded.reset_pending = false;
                *pending = None;
            }
            Err(error) => {
                *pending = Some(time.elapsed());
                if error.kind() == io::ErrorKind::WouldBlock {
                    if exiting {
                        // The runner checks AppExit after this schedule. Keep the request
                        // here until a later frame can finish the save.
                        exit_save.retain(&mut exits);
                        return;
                    }
                } else {
                    leprintln!("Could not save window layout: {error}", error = error);
                }
            }
        }
    } else {
        *pending = None;
    }
    if let Some(exit) = exit_save.deferred.take() {
        exits.write(exit);
    }
}

/// Clamps every window to the logical screen and projects sizes, position and zoom.
#[allow(clippy::too_many_arguments)]
pub(in crate::bevy_shell) fn project(
    mut commands: Commands,
    mut layouts: ResMut<WindowLayouts>,
    screens: Query<&Window, With<PrimaryWindow>>,
    scale: Option<Res<UiScale>>,
    mut seen: Local<Option<Vec2>>,
    mut roots: Roots,
    mut scaled: Query<(Entity, Ref<Scaled>, &mut Node), Without<WindowId>>,
    mut fonts: Fonts,
    parents: Query<&ChildOf>,
    ids: Query<&WindowId>,
) {
    let viewport = viewport(&screens, scale.as_deref());
    let screen_changed = viewport != *seen;
    *seen = viewport;
    let changed = layouts.is_changed();
    for (entity, id, mut node, transform) in &mut roots {
        let Some(layout) = layouts.0.get(id.0).copied() else {
            continue;
        };
        if id.is_added() {
            resize::spawn(&mut commands, entity);
        }
        if let Some(mut transform) = transform {
            if id.is_added() {
                transform.translation = Val2::px(layout.offset.x, layout.offset.y);
            } else if transform.is_changed()
                && let (Val::Px(x), Val::Px(y)) = (transform.translation.x, transform.translation.y)
                && Vec2::new(x, y) != layout.offset
                && let Some(entry) = layouts.0.get_mut(id.0)
            {
                entry.offset = Vec2::new(x, y);
            } else if changed && transform.translation != Val2::px(layout.offset.x, layout.offset.y)
            {
                transform.translation = Val2::px(layout.offset.x, layout.offset.y);
            }
        }
        if changed || screen_changed || id.is_added() {
            let mut projected = layout;
            if let Some(viewport) = viewport {
                projected.fit(viewport);
            }
            projected.apply(&mut node, viewport);
        }
    }
    for (entity, base, mut node) in &mut scaled {
        if changed || base.is_added() {
            let zoom = zoom_of(entity, &parents, &ids, &layouts).unwrap_or(1.);
            base.apply(zoom, &mut node);
        }
    }
    for (entity, mut font, line, base) in &mut fonts {
        let base = match base {
            Some(base) if changed => *base,
            // Text spawned outside a window (a docked region) takes its base when the region
            // moves into one, which marks the layouts changed.
            None if font.is_added() || changed => {
                let FontSize::Px(size) = font.font_size else {
                    continue;
                };
                if zoom_of(entity, &parents, &ids, &layouts).is_none() {
                    continue;
                }
                let base = FontBase {
                    size,
                    line: match line.as_deref() {
                        Some(LineHeight::Px(line)) => Some(*line),
                        _ => None,
                    },
                };
                commands.entity(entity).insert(base);
                base
            }
            _ => continue,
        };
        let zoom = zoom_of(entity, &parents, &ids, &layouts).unwrap_or(1.);
        if let (Some(mut line), Some(height)) = (line, base.line) {
            *line = LineHeight::Px(height * zoom);
        }
        font.font_size = FontSize::Px(base.size * zoom);
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
pub(in crate::bevy_shell) use tests::stacking::ui_layout as test_ui_layout;
