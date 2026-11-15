//! Compact blessing ownership over the retained 0199 self-only query.
use super::*;
use bevy::{picking::hover::Hovered, ui::Pressed, window::PrimaryWindow};
use std::time::Duration;

// Storage keys and names from rulesets/character/blessings/catalogue.json.
const BLESSINGS: [(&str, &str, &str); 8] = [
    ("the_spiritual_shielding", "SS", "The Spiritual Shielding"),
    ("the_embrace_of_tibia", "ET", "The Embrace of Tibia"),
    ("the_fire_of_the_suns", "FS", "The Fire of the Suns"),
    ("the_spark_of_the_phoenix", "SP", "The Spark of the Phoenix"),
    ("the_wisdom_of_solitude", "WS", "The Wisdom of Solitude"),
    ("blood_of_the_mountain", "BM", "Blood of the Mountain"),
    ("heart_of_the_mountain", "HM", "Heart of the Mountain"),
    ("twist_of_fate", "TF", "Twist of Fate"),
];
#[derive(Component)]
struct Panel(Entity);
#[derive(Component)]
struct Tooltip;
#[derive(Component)]
struct Details;
#[derive(Component)]
struct ToggleDetails;
#[derive(Default, Resource)]
struct Hover(bool);
#[derive(Default)]
struct State {
    generation: Option<u64>,
    snapshot: Option<Vec<String>>,
    last_refresh: Option<Duration>,
    hover_since: Option<Duration>,
}

/// Mount the panel and its projection together from the HUD's single spawn call.
pub(super) fn spawn(app: &mut App) {
    app.init_resource::<Hover>()
        .init_resource::<UiScale>()
        .add_systems(Startup, setup.after(super::setup))
        .add_systems(Update, project_details.after(super::project))
        .add_systems(
            Update,
            (toggle_details, hover, project)
                .chain()
                .after(super::project),
        );
}
fn setup(mut commands: Commands, regions: Res<super::super::GameRegions>, skins: Res<Skins>) {
    let icons = commands
        .spawn(Node {
            width: percent(100.),
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(6.),
            row_gap: px(2.),
            ..default()
        })
        .id();
    let root = commands
        .spawn((
            Name::new("Blessings status"),
            Panel(icons),
            Hovered::default(),
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                padding: UiRect::all(px(4.)),
                flex_direction: FlexDirection::Column,
                display: Display::None,
                ..default()
            },
        ))
        .id();
    let title = commands
        .spawn(source::text(
            "Blessings",
            source::font(skins.font.clone(), false),
        ))
        .id();
    let toggle = commands
        .spawn((
            bevy::ui_widgets::Button,
            ToggleDetails,
            source::text(
                "Ankh — blessing details",
                source::font(skins.font.clone(), false),
            ),
        ))
        .id();
    let details = commands
        .spawn((
            Details,
            GlobalZIndex(110),
            BackgroundColor(Color::srgb_u8(25, 25, 25)),
            source::text("", source::font(skins.font.clone(), false)),
            Node {
                display: Display::None,
                position_type: PositionType::Absolute,
                width: px(420.),
                max_height: px(480.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands
        .entity(root)
        .add_children(&[title, toggle, icons, details]);
    commands.entity(regions.equipment).add_child(root);
    commands
        .spawn((
            Name::new("Blessings tooltip"),
            Tooltip,
            source::text("", source::font(skins.font.clone(), false)),
            Node {
                position_type: PositionType::Absolute,
                display: Display::None,
                padding: UiRect::all(px(6.)),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(25, 25, 25)),
            GlobalZIndex(120),
        ))
        .insert(TextLayout::default());
}
fn toggle_details(
    mut commands: Commands,
    buttons: Query<Entity, (With<ToggleDetails>, Added<Pressed>)>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    details: Query<(Entity, &Node), With<Details>>,
) {
    if !owner.modal_active() && buttons.iter().any(|entity| guard.allows(entity)) {
        for (entity, current) in &details {
            let mut node = current.clone();
            node.display = if node.display == Display::None {
                Display::Flex
            } else {
                Display::None
            };
            commands.entity(entity).insert(node);
        }
    }
}
fn details_text(keys: &[String]) -> String {
    let mut text = String::from("Blessing ownership\n");
    for (key, _, name) in BLESSINGS {
        let owned = keys
            .iter()
            .any(|value| value == &format!("oteryn:blessing.{key}"));
        text.push_str(&format!(
            "{name}: {}\n",
            if owned { "owned" } else { "missing" }
        ));
    }
    text.push_str("Death penalty details\nRegular blessings reduce experience and skill loss. Promotion also reduces these losses.\nFive or more regular blessings protect equipment and containers; red or black skulls override this protection.\nTwist of Fate protects regular blessings on qualifying PvP deaths, rather than adding regular death-loss protection.\nExact next-death losses, promotion and world modifiers are not supplied by the server.");
    text
}
fn project_details(
    owner: NonSend<HostOwner>,
    mut details: Query<(&mut Text, &mut Node), With<Details>>,
) {
    for (mut text, mut node) in &mut details {
        if let Some(snapshot) = owner
            .play
            .as_ref()
            .and_then(|play| play.link.blessings_panel().snapshot)
        {
            text.0 = details_text(&snapshot.blessings);
        } else {
            text.0 = "Waiting for blessing ownership from the server.".into();
            node.display = Display::None;
        }
    }
}
fn hover(
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
    panels: Query<(Entity, &Hovered), With<Panel>>,
    mut hover: ResMut<Hover>,
) {
    hover.0 = !owner.modal_active()
        && panels
            .iter()
            .any(|(entity, hovered)| hovered.get() && guard.allows(entity));
}
// The independent queries keep panel nodes and the floating tooltip disjoint.
#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    time: Res<Time<Real>>,
    hover: Res<Hover>,
    scale: Res<UiScale>,
    windows: Query<&Window, With<PrimaryWindow>>,
    mut panels: Query<(&Panel, &mut Node), Without<Tooltip>>,
    mut tooltips: Query<(&mut Node, &mut Text), (With<Tooltip>, Without<Panel>)>,
    mut state: Local<State>,
) {
    let Ok((panel, mut root)) = panels.single_mut() else {
        return;
    };
    let Ok((mut tooltip, mut text)) = tooltips.single_mut() else {
        return;
    };
    tooltip.display = Display::None;
    let Some((play, character)) = owner.play.as_ref().and_then(|play| {
        let character = play.link.cyclopedia_character_panel();
        character.selected.then_some((play, character))
    }) else {
        root.display = Display::None;
        if state.generation.is_some() {
            commands.entity(panel.0).despawn_children();
        }
        text.0.clear();
        *state = State::default();
        return;
    };
    root.display = Display::Flex;
    let now = time.elapsed();
    let new_session = state.generation != Some(owner.generation());
    if new_session {
        *state = State {
            generation: Some(owner.generation()),
            ..default()
        };
    }
    // Ownership has no pushed wire delta. Reuse the existing query, bounded to one
    // refresh per five seconds; request_blessings also serializes character queries.
    if state
        .last_refresh
        .is_none_or(|last| now.saturating_sub(last) >= Duration::from_secs(5))
        && !character.pending
        && !character.blessings.pending
    {
        play.link.blessings_request();
        state.last_refresh = Some(now);
    }
    let blessings = character.blessings;
    let snapshot = blessings.snapshot.as_ref().map(|s| &s.blessings);
    if new_session || state.snapshot.as_ref() != snapshot {
        commands.entity(panel.0).despawn_children();
        if let Some(keys) = snapshot {
            for (key, short, _) in BLESSINGS {
                let key = format!("oteryn:blessing.{key}");
                icon(
                    &mut commands,
                    &skins,
                    panel.0,
                    &key,
                    short,
                    keys.contains(&key),
                );
            }
            // Do not drop a blessing introduced by a newer server/model.
            for key in keys {
                if !BLESSINGS
                    .iter()
                    .any(|(known, _, _)| key.strip_prefix("oteryn:blessing.") == Some(*known))
                {
                    icon(&mut commands, &skins, panel.0, key, &label(key), true);
                }
            }
        }
        state.snapshot = snapshot.cloned();
    }
    let next = snapshot.map_or_else(
        || {
            blessings
                .feedback
                .unwrap_or("Waiting for blessing state from the server.")
                .to_owned()
        },
        |keys| {
            if keys.is_empty() {
                "You are not protected by any blessing.".into()
            } else {
                format!(
                    "Active blessings:\n{}",
                    keys.iter()
                        .map(|key| label(key))
                        .collect::<Vec<_>>()
                        .join("\n")
                )
            }
        },
    );
    if text.0 != next {
        text.0 = next;
    }
    if !hover.0 || !owner.settings.preferences.tooltips_enabled {
        state.hover_since = None;
        return;
    }
    let since = *state.hover_since.get_or_insert(now);
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
        let width = (size.x - 8.).clamp(1., 280.);
        let height = (text.0.lines().count() as f32 * 13. + 12.).min(size.y);
        let position = (pointer / scale.0 + Vec2::new(12., 20.))
            .min((size - Vec2::new(width, height)).max(Vec2::ZERO));
        tooltip.left = px(position.x);
        tooltip.top = px(position.y);
        tooltip.width = px(width);
        tooltip.max_height = px(size.y);
        tooltip.overflow = Overflow::clip();
        tooltip.display = Display::Flex;
    }
}
fn icon(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    key: &str,
    short: &str,
    active: bool,
) {
    let entity = commands
        .spawn((
            Name::new(key.to_owned()),
            source::text(
                format!("{} {short}", if active { "+" } else { "-" }),
                source::font(skins.font.clone(), false),
            ),
        ))
        .insert(TextColor(if active {
            Color::srgb_u8(120, 220, 120)
        } else {
            source::DISABLED_TEXT_COLOR
        }))
        .id();
    commands.entity(parent).add_child(entity);
}
fn label(key: &str) -> String {
    let short = key.strip_prefix("oteryn:blessing.").unwrap_or(key);
    BLESSINGS
        .iter()
        .find(|(known, _, _)| *known == short)
        .map_or_else(
            || short.replace('_', " "),
            |(_, _, name)| (*name).to_owned(),
        )
}
#[cfg(test)]
#[path = "blessings/tests.rs"]
mod tests;
