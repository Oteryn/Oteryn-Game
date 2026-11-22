//! The Tibia Global status bar over the game window (statusBarEnabled, large style): health and
//! mana side by side with "cur/max" centred, then the level, a red XP bar with tick marks and
//! the XP Boost button, then the centred row of condition icons.
use super::{HostOwner, ModalGuard, Skins, source};
use crate::bevy_shell::ui::{Control, toolbar::open_toolbar};
use bevy::{prelude::*, ui_widgets::Activate};

/// The value text of the health (false) or mana (true) bar.
#[derive(Component)]
pub(super) struct BarText(bool);
#[derive(Component)]
pub(super) struct BarFill(bool);
#[derive(Component)]
pub(super) struct LevelText;
#[derive(Component)]
pub(super) struct XpFill;
#[derive(Component)]
pub(super) struct SelectedSkills;
#[derive(Component)]
pub(super) struct BarBody;

/// Line heights of the large style (about 27 px for the two lines).
const VITAL_HEIGHT: f32 = 14.;
const XP_LINE: f32 = 13.;
const XP_BAR: f32 = 5.;
const BOOST: Vec2 = Vec2::new(50., 13.);
/// The tick marks of the XP bar, one every tenth.
const TICKS: u8 = 9;

/// Spawns the status bar into `parent` and returns the entity the condition row goes into.
pub(super) fn setup(commands: &mut Commands, skins: &Skins, parent: Entity) -> Entity {
    let font = source::font(skins.font.clone(), true);
    let bar = commands
        .spawn((
            Name::new("Status bar"),
            BarBody,
            Node {
                width: percent(100.),
                flex_shrink: 0.,
                flex_direction: FlexDirection::Column,
                padding: UiRect::axes(px(4.), px(2.)),
                row_gap: px(1.),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(60, 60, 60)),
        ))
        .id();
    let vitals = commands
        .spawn((
            Name::new("Status bar vitals"),
            Node {
                width: percent(100.),
                height: px(VITAL_HEIGHT),
                ..default()
            },
        ))
        .id();
    for (mana, color) in [
        (false, Color::srgb_u8(0, 150, 0)),
        (true, Color::srgb_u8(0, 70, 180)),
    ] {
        let track = commands
            .spawn((
                Node {
                    width: percent(50.),
                    height: percent(100.),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    border: UiRect::all(px(1.)),
                    overflow: Overflow::clip(),
                    ..default()
                },
                BorderColor::all(Color::BLACK),
                BackgroundColor(Color::srgb_u8(30, 30, 30)),
            ))
            .id();
        let fill = commands
            .spawn((
                BarFill(mana),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(0.),
                    top: px(0.),
                    bottom: px(0.),
                    width: percent(0.),
                    ..default()
                },
                BackgroundColor(color),
                Pickable::IGNORE,
            ))
            .id();
        let text = commands
            .spawn((BarText(mana), source::text("—", font.clone())))
            .insert(TextColor(Color::WHITE))
            .id();
        commands.entity(track).add_children(&[fill, text]);
        commands.entity(vitals).add_child(track);
    }
    let experience = commands
        .spawn((
            Name::new("Status bar experience"),
            Node {
                width: percent(100.),
                height: px(XP_LINE),
                align_items: AlignItems::Center,
                column_gap: px(4.),
                ..default()
            },
        ))
        .id();
    let level = commands
        .spawn((LevelText, source::text("—", font.clone())))
        .insert(TextColor(Color::WHITE))
        .id();
    let track = commands
        .spawn((
            Node {
                flex_grow: 1.,
                height: px(XP_BAR),
                border: UiRect::all(px(1.)),
                ..default()
            },
            BorderColor::all(Color::BLACK),
            BackgroundColor(Color::srgb_u8(30, 30, 30)),
            Pickable::IGNORE,
        ))
        .id();
    let fill = commands
        .spawn((
            XpFill,
            Name::new("Status bar XP fill"),
            Node {
                width: percent(0.),
                height: percent(100.),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(200, 0, 0)),
            Pickable::IGNORE,
        ))
        .id();
    commands.entity(track).add_child(fill);
    for tick in 1..=TICKS {
        let mark = commands
            .spawn((
                Name::new("XP tick"),
                Node {
                    position_type: PositionType::Absolute,
                    left: percent(f32::from(tick) * 10.),
                    top: px(0.),
                    bottom: px(0.),
                    width: px(1.),
                    ..default()
                },
                BackgroundColor(Color::BLACK),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(track).add_child(mark);
    }
    let boost =
        super::super::art::spawn(commands, &skins.font, "status-xp-boost", "XP Boost", BOOST);
    commands
        .entity(boost)
        .insert(bevy::ui_widgets::Button)
        .observe(xp_boost);
    commands
        .entity(experience)
        .add_children(&[level, track, boost]);
    let conditions = commands
        .spawn(Node {
            width: percent(100.),
            justify_content: JustifyContent::Center,
            ..default()
        })
        .id();
    let skills = commands
        .spawn((
            SelectedSkills,
            source::text("", font),
            Node {
                display: Display::None,
                ..default()
            },
        ))
        .id();
    commands
        .entity(bar)
        .add_children(&[vitals, experience, skills, conditions]);
    commands.entity(parent).add_child(bar);
    conditions
}

/// XP Boost opens the store, where Tibia sells the boost.
fn xp_boost(event: On<Activate>, mut owner: NonSendMut<HostOwner>, modal: ModalGuard) {
    if modal.allows(event.entity) && owner.play.is_some() {
        open_toolbar(&mut owner, Control::Shortcut("store"));
    }
}

// The three node roles and two text roles are disjoint by their marker components.
#[allow(clippy::type_complexity)]
pub(super) fn project(
    owner: NonSend<HostOwner>,
    mut texts: Query<(&mut Text, Option<&BarText>), Or<(With<BarText>, With<LevelText>)>>,
    mut fills: Query<(&mut Node, Option<&BarFill>), Or<(With<BarFill>, With<XpFill>)>>,
) {
    let Some(play) = &owner.play else { return };
    let vitals = play.link.actor_vitals();
    let value = |mana: bool| {
        vitals.map(|v| {
            if mana {
                (v.mana, v.max_mana)
            } else {
                (v.health, v.max_health)
            }
        })
    };
    let level = play
        .link
        .skill_progress_panel()
        .complete
        .map(|full| full.level);
    let progress = play.link.skills_widget().level_progress;
    for (mut text, bar) in &mut texts {
        let next = match bar {
            Some(bar) => value(bar.0).map_or_else(
                || "—".into(),
                |(current, maximum)| format!("{current}/{maximum}"),
            ),
            None => level.map_or_else(|| "—".into(), |level| level.to_string()),
        };
        if text.0 != next {
            text.0 = next;
        }
    }
    for (mut node, bar) in &mut fills {
        let width = percent(match bar {
            Some(bar) => value(bar.0)
                .filter(|(_, maximum)| *maximum > 0)
                .map_or(0., |(current, maximum)| {
                    current.min(maximum) as f32 / maximum as f32 * 100.
                }),
            None => progress.map_or(0., |value| f32::from(value.min(10000)) / 100.),
        });
        if node.width != width {
            node.width = width;
        }
    }
}

pub(super) fn project_skills(
    owner: NonSend<HostOwner>,
    region: Option<Res<super::super::StatusBar>>,
    mut nodes: Query<(&mut Node, Has<BarBody>), Without<SelectedSkills>>,
    mut skills: Query<(&mut Node, &mut Text), With<SelectedSkills>>,
) {
    let prefs = &owner.settings.preferences;
    if let Some(region) = region
        && let Ok((mut node, _)) = nodes.get_mut(region.0)
    {
        node.display = if prefs.status_bar_visible {
            Display::Flex
        } else {
            Display::None
        };
    }
    for (mut body, is_bar) in &mut nodes {
        if is_bar {
            body.flex_direction = if prefs.status_bar_compact {
                FlexDirection::Row
            } else {
                FlexDirection::Column
            };
        }
    }
    let Some(play) = &owner.play else { return };
    let Ok((mut node, mut text)) = skills.single_mut() else {
        return;
    };
    let panel = play.link.skill_progress_panel();
    let snapshot = panel
        .complete
        .map(|complete| complete.trained)
        .or(panel.snapshot);
    let labels = oteryn_client_ui::skill_progress::SKILL_LABELS;
    let values: Vec<_> = snapshot
        .into_iter()
        .flat_map(|snapshot| {
            snapshot
                .skills
                .into_iter()
                .enumerate()
                .filter_map(|(index, skill)| {
                    let level = panel
                        .complete
                        .and_then(|full| full.current_levels)
                        .map_or(u32::from(skill.level), |levels| levels[index]);
                    prefs.status_bar_skills[index].then(|| format!("{} {level}", labels[index]))
                })
        })
        .collect();
    node.display = if values.is_empty() {
        Display::None
    } else {
        Display::Flex
    };
    let next = values.join("   ");
    if text.0 != next {
        text.0 = next;
    }
}
