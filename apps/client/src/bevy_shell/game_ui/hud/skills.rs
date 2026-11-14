//! SkillWidgetEntry.qml: right-aligned values and a five-pixel progress line.
use super::*;
use crate::play::feature_0274_loyalty_skills::view::SkillsNumbers;
#[derive(Component)]
struct Header;
#[derive(Component)]
struct Label(usize);
#[derive(Component)]
struct Value(usize);
#[derive(Component)]
struct Slot(usize);
#[derive(Component)]
struct Track(usize);
#[derive(Component)]
struct Fill(usize);
pub(super) fn install(app: &mut App) {
    app.add_systems(Startup, setup.after(super::setup))
        .add_systems(Update, project.after(super::project));
}
fn setup(mut commands: Commands, regions: Res<super::super::GameRegions>, skins: Res<Skins>) {
    // skillswidget.qml: x=marginRelated(5), y=8, row spacing=1, bottom padding=5.
    let root = commands
        .spawn(Node {
            width: percent(100.),
            padding: UiRect {
                left: px(5.),
                right: px(5.),
                top: px(8.),
                bottom: px(5.),
            },
            row_gap: px(1.),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    let header = commands
        .spawn((
            Header,
            source::text("", source::font(skins.font.clone(), false)),
            Node {
                width: percent(100.),
                ..default()
            },
        ))
        .insert(TextLayout::default())
        .id();
    commands.entity(root).add_child(header);
    for index in 0..8 {
        let slot = commands
            .spawn((
                Slot(index),
                Node {
                    width: percent(100.),
                    flex_direction: FlexDirection::Column,
                    display: Display::None,
                    ..default()
                },
            ))
            .id();
        let line = commands
            .spawn(Node {
                width: percent(100.),
                justify_content: JustifyContent::SpaceBetween,
                overflow: Overflow::clip(),
                ..default()
            })
            .id();
        let label = commands
            .spawn((
                Label(index),
                source::text("", source::font(skins.font.clone(), false)),
            ))
            .id();
        let value = commands
            .spawn((
                Value(index),
                source::text("", source::font(skins.font.clone(), false)),
            ))
            .id();
        let track = commands
            .spawn((
                Track(index),
                Node {
                    width: percent(100.),
                    height: px(5.),
                    border: UiRect::all(px(1.)),
                    margin: UiRect::bottom(px(2.)),
                    ..default()
                },
                BorderColor::all(Color::BLACK),
                Pickable::IGNORE,
            ))
            .id();
        let fill = commands
            .spawn((
                Fill(index),
                Node {
                    width: percent(0.),
                    height: percent(100.),
                    ..default()
                },
                BackgroundColor(Color::WHITE),
                Pickable::IGNORE,
            ))
            .id();
        commands.entity(track).add_child(fill);
        commands.entity(line).add_children(&[label, value]);
        commands.entity(slot).add_children(&[line, track]);
        commands.entity(root).add_child(slot);
    }
    commands.entity(regions.skills).add_child(root);
}
// Queries combine the three node roles and three text roles without aliasing mutable access.
#[allow(clippy::type_complexity)]
fn project(
    owner: NonSend<HostOwner>,
    mut text: Query<
        (&mut Text, Option<&Header>, Option<&Label>, Option<&Value>),
        Or<(With<Header>, With<Label>, With<Value>)>,
    >,
    mut nodes: Query<
        (&mut Node, Option<&Slot>, Option<&Track>, Option<&Fill>),
        Or<(With<Slot>, With<Track>, With<Fill>)>,
    >,
) {
    let Some(play) = &owner.play else { return };
    let widget = play.link.skills_widget();
    let layout = &play.view.preferences().skills_layout;
    for (mut text, header, label, value) in &mut text {
        let next = if header.is_some() {
            widget.header.replace("\r\n", "\n")
        } else if let Some(label) = label {
            widget
                .rows
                .get(label.0)
                .and_then(|row| {
                    oteryn_client_ui::skill_progress::SKILL_LABELS.get(usize::from(row.family))
                })
                .copied()
                .unwrap_or("")
                .into()
        } else if let Some(value) = value {
            widget
                .rows
                .get(value.0)
                .map(|row| {
                    let loyalty = row
                        .loyalty
                        .map_or_else(|| "—".into(), |value| value.to_string());
                    let current = row
                        .current
                        .map_or_else(|| "—".into(), |value| value.to_string());
                    match layout.numbers {
                        SkillsNumbers::All => format!("{} / {loyalty} / {current}", row.trained),
                        SkillsNumbers::Trained => row.trained.to_string(),
                        SkillsNumbers::Loyalty => loyalty,
                        SkillsNumbers::Current => current,
                    }
                })
                .unwrap_or_default()
        } else {
            String::new()
        };
        if text.0 != next {
            text.0 = next;
        }
    }
    for (mut node, slot, track, fill) in &mut nodes {
        if let Some(slot) = slot {
            let display = if widget.rows.get(slot.0).is_some() {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
        } else if let Some(track) = track {
            let display = if layout.progress
                && widget
                    .rows
                    .get(track.0)
                    .is_some_and(|row| row.progress.is_some())
            {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
        } else if let Some(fill) = fill {
            let width = percent(
                widget
                    .rows
                    .get(fill.0)
                    .and_then(|row| row.progress)
                    .map_or(0., |value| f32::from(value.min(10000)) / 100.),
            );
            if node.width != width {
                node.width = width;
            }
        }
    }
}

#[cfg(test)]
#[path = "skills_tests.rs"]
mod tests;
