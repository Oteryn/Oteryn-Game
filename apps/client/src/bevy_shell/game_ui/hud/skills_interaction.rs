//! Session estimates use observed replies, never an assumed server XP curve.
use super::*;
use bevy::{
    picking::{events::PointerClick, pointer::PointerButton},
    window::PrimaryWindow,
};
use std::time::Duration;

#[derive(Clone, Copy, Component)]
enum Control {
    Skill(u8),
    Progress,
    Reset,
    Defaults,
}
#[derive(Component)]
struct Menu;
#[derive(Component)]
struct Tooltip;
type SkillRows = Or<(With<Slot>, With<Header>)>;
#[derive(Default, Resource)]
struct State {
    generation: Option<u64>,
    menu: bool,
    baseline: Option<(Duration, u64)>,
    level: Option<(Duration, u32, u16)>,
    latest: Option<(u64, u32, u16)>,
    last_gain: Duration,
    refresh: Option<Duration>,
    hover: Option<(Entity, Duration)>,
}
impl State {
    fn observe(&mut self, now: Duration, xp: u64, level: u32, percent: u16) {
        if self.latest.is_none_or(|last| xp < last.0)
            || now.saturating_sub(self.last_gain) >= Duration::from_secs(900)
        {
            self.baseline = Some((now, xp));
            self.level = Some((now, level, percent));
            self.last_gain = now;
        }
        if self.latest.is_some_and(|last| xp > last.0) {
            self.last_gain = now;
        }
        if self
            .latest
            .is_some_and(|last| level != last.1 || percent < last.2)
        {
            self.level = Some((now, level, percent));
        }
        self.latest = Some((xp, level, percent));
    }
    fn reset(&mut self, now: Duration) {
        if let Some((xp, level, percent)) = self.latest {
            self.baseline = Some((now, xp));
            self.level = Some((now, level, percent));
        }
        self.last_gain = now;
    }
    fn estimate(&self, now: Duration) -> String {
        let Some((xp, _, percent)) = self.latest else {
            return "XP estimates unavailable".into();
        };
        let Some((start, initial)) = self.baseline else {
            return String::new();
        };
        let seconds = now.saturating_sub(start).as_secs_f64();
        let gained = xp.saturating_sub(initial);
        let rate = if seconds > 0. {
            (gained as f64 * 3600. / seconds) as u64
        } else {
            0
        };
        let mut text = format!("Session XP: {gained}\nXP/hour: {rate}");
        if seconds < 900. {
            text.push_str(" (preliminary; 15 minutes recommended)");
        }
        let eta = self.level.and_then(|(start, _, initial)| {
            let delta = percent.checked_sub(initial).filter(|delta| *delta > 0)?;
            let seconds = now.saturating_sub(start).as_secs_f64();
            (seconds > 0.).then(|| {
                (f64::from(10000 - percent.min(10000)) * seconds / f64::from(delta)).ceil() as u64
            })
        });
        text.push_str(&eta.map_or_else(
            || "\nETA: collecting level progress".into(),
            |s| {
                format!(
                    "\nEstimated next level: {}h {}m",
                    s.div_ceil(60) / 60,
                    s.div_ceil(60) % 60
                )
            },
        ));
        text.push_str("\nXP to next level: unavailable (server threshold not supplied)");
        text
    }
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .init_resource::<UiScale>()
        .add_systems(Startup, setup.after(super::setup))
        .add_systems(Update, (hover, project).chain().after(super::project))
        .add_observer(activate)
        .add_observer(context_menu);
}
fn control(commands: &mut Commands, skins: &Skins, parent: Entity, action: Control, name: &str) {
    let button = commands
        .spawn((
            Name::new(name.to_owned()),
            action,
            source::button(skins.buttons.clone(), 180.),
        ))
        .id();
    let text = commands
        .spawn(source::button_caption(name, skins.font.clone()))
        .id();
    commands.entity(button).add_child(text);
    commands.entity(parent).add_child(button);
}
fn setup(
    mut commands: Commands,
    regions: Res<super::super::super::GameRegions>,
    skins: Res<Skins>,
) {
    // SkillsWidget.qml: the settings open from the widget's right-click menu.
    let menu = commands
        .spawn((
            Menu,
            Node {
                display: Display::None,
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    for (family, name) in oteryn_client_ui::skill_progress::SKILL_LABELS
        .iter()
        .enumerate()
    {
        control(
            &mut commands,
            &skins,
            menu,
            Control::Skill(family as u8),
            &format!("Toggle {name}"),
        );
    }
    for (action, name) in [
        (Control::Progress, "Show/hide all bars"),
        (Control::Reset, "Reset XP counter"),
        (Control::Defaults, "Restore skill visibility"),
    ] {
        control(&mut commands, &skins, menu, action, name);
    }
    commands.entity(regions.skills).add_child(menu);
    commands
        .spawn((
            Name::new("Skills tooltip"),
            Tooltip,
            source::text("", source::font(skins.font.clone(), false)),
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                padding: UiRect::all(px(6.)),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(25, 25, 25)),
            GlobalZIndex(120),
        ))
        .insert(TextLayout::default());
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
    time: Res<Time<Real>>,
    mut state: ResMut<State>,
) {
    if !guard.allows(event.entity)
        || owner.modal_active()
        || owner.play.is_none()
        || state.generation != Some(owner.generation())
    {
        return;
    }
    let Ok(control) = controls.get(event.entity) else {
        return;
    };
    match control {
        Control::Reset => state.reset(time.elapsed()),
        _ => {
            let mut prefs = owner.settings.preferences.clone();
            match control {
                Control::Skill(family) => {
                    if prefs.skills_layout.order.contains(family) {
                        prefs.skills_layout.order.retain(|id| id != family);
                    } else {
                        prefs.skills_layout.order.push(*family);
                    }
                }
                Control::Progress => prefs.skills_layout.progress = !prefs.skills_layout.progress,
                Control::Defaults => prefs.skills_layout = default(),
                _ => {}
            }
            owner.settings.apply(prefs);
            let HostOwner { settings, play, .. } = &mut *owner;
            if let Some(play) = play {
                settings.sync_skills_layout(&play.link);
            }
        }
    }
}
fn context_menu(
    event: On<PointerClick>,
    rows: Query<(), SkillRows>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
    mut state: ResMut<State>,
) {
    if event.button == PointerButton::Secondary
        && rows.contains(event.entity)
        && guard.allows(event.entity)
        && !owner.modal_active()
        && owner.play.is_some()
    {
        state.menu = !state.menu;
    }
}
fn hover(
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
    time: Res<Time<Real>>,
    hovered: Query<(Entity, &Hovered), SkillRows>,
    mut state: ResMut<State>,
) {
    let target = hovered
        .iter()
        .find(|(entity, hover)| hover.get() && guard.allows(*entity));
    state.hover = target
        .filter(|_| !owner.modal_active() && owner.settings.preferences.tooltips_enabled)
        .map(|(entity, _)| match state.hover {
            Some((previous, since)) if previous == entity => (entity, since),
            _ => (entity, time.elapsed()),
        });
}
// Disjoint node queries separate the menu and floating tooltip.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn project(
    owner: NonSend<HostOwner>,
    time: Res<Time<Real>>,
    scale: Res<UiScale>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut state: ResMut<State>,
    slots: Query<&Slot>,
    mut menus: Query<&mut Node, (With<Menu>, Without<Tooltip>)>,
    mut tooltips: Query<(&mut Node, &mut Text), (With<Tooltip>, Without<Menu>)>,
) {
    let now = time.elapsed();
    let Ok((mut node, mut text)) = tooltips.single_mut() else {
        return;
    };
    node.display = Display::None;
    let Some(play) = &owner.play else {
        *state = default();
        return;
    };
    if state.generation != Some(owner.generation()) {
        *state = State {
            generation: Some(owner.generation()),
            ..default()
        };
    }
    for mut node in &mut menus {
        node.display = if state.menu {
            Display::Flex
        } else {
            Display::None
        };
    }
    let panel = play.link.skill_progress_panel();
    if let Some(full) = panel.complete {
        state.observe(now, full.experience, full.level, full.level_percent);
    }
    // Queries serialize through the existing pending flag, as in the native skills HUD.
    if !panel.pending
        && panel.selected
        && state
            .refresh
            .is_none_or(|last| now.saturating_sub(last) >= Duration::from_secs(1))
    {
        play.link.skill_progress_request();
        state.refresh = Some(now);
    }
    let Some((entity, since)) = state.hover else {
        return;
    };
    let slot = slots.get(entity).ok();
    let widget = play.link.skills_widget();
    text.0 = if let Some(slot) = slot {
        let Some(row) = widget.rows.get(slot.0) else {
            return;
        };
        let name = oteryn_client_ui::skill_progress::SKILL_LABELS[usize::from(row.family)];
        let progress = row.progress.map_or_else(
            || "Progress unavailable".into(),
            |p| format!("{}.{:02}% complete", p / 100, p % 100),
        );
        let counts = row.counts.map_or_else(
            || "Training counts unavailable".into(),
            |(current, required)| {
                if required == 0 {
                    "No next level".into()
                } else {
                    format!(
                        "{current}/{required}\n{} training points to next level",
                        required.saturating_sub(current)
                    )
                }
            },
        );
        format!("{name}\n{progress}\n{counts}")
    } else {
        format!(
            "{}\n{}",
            widget.header.replace("\r\n", "\n"),
            state.estimate(now)
        )
    };
    if now.saturating_sub(since)
        < Duration::from_millis(u64::from(owner.settings.preferences.tooltip_delay_ms))
    {
        return;
    }
    if let Ok(window) = windows.single()
        && window.focused
        && let Some(pointer) = window.cursor_position()
    {
        let size = Vec2::new(window.width(), window.height()) / scale.0;
        let width = (size.x - 8.).clamp(1., 330.);
        let height = (text.0.lines().count() as f32 * 13. + 12.).min(size.y);
        let position = (pointer / scale.0 + Vec2::new(12., 20.))
            .min((size - Vec2::new(width, height)).max(Vec2::ZERO));
        node.left = px(position.x);
        node.top = px(position.y);
        node.width = px(width);
        node.max_height = px(size.y);
        node.overflow = Overflow::clip();
        node.display = Display::Flex;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn xp_estimates_reset_on_idle_loss_level_change_and_manual_reset() {
        let mut state = State::default();
        state.observe(Duration::ZERO, 1000, 10, 2000);
        state.observe(Duration::from_secs(60), 1600, 10, 3000);
        let text = state.estimate(Duration::from_secs(60));
        assert!(text.contains("Session XP: 600"));
        assert!(text.contains("XP/hour: 36000"));
        assert!(text.contains("Estimated next level: 0h 7m"));
        state.observe(Duration::from_secs(120), 2000, 11, 0);
        assert!(
            state
                .estimate(Duration::from_secs(120))
                .contains("collecting level progress")
        );
        state.reset(Duration::from_secs(120));
        assert!(
            state
                .estimate(Duration::from_secs(120))
                .contains("Session XP: 0")
        );
        state.observe(Duration::from_secs(180), 2600, 11, 1000);
        state.observe(Duration::from_secs(1080), 2600, 11, 1000);
        assert!(
            state
                .estimate(Duration::from_secs(1080))
                .contains("XP/hour: 0")
        );
        state.observe(Duration::from_secs(1140), 3200, 11, 2000);
        state.observe(Duration::from_secs(1150), 2500, 10, 9000);
        assert!(
            state
                .estimate(Duration::from_secs(1150))
                .contains("Session XP: 0")
        );
        assert!(
            state
                .estimate(Duration::from_secs(1150))
                .contains("collecting level progress")
        );
    }
}
