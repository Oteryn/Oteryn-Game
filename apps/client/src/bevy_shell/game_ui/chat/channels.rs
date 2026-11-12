use super::*;

#[derive(Component)]
struct Channels;
#[derive(Component)]
struct Entries;
#[derive(Resource, Default)]
struct Rows {
    generation: Option<u64>,
    entries: Vec<oteryn_client_ui::chat::ChannelEntry>,
    labels: Vec<String>,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<Rows>()
        .add_systems(Update, project.after(super::project));
}

pub(super) fn setup(commands: &mut Commands, root: Entity, skins: &Skins) {
    let panel = commands
        .spawn((
            Channels,
            source::frame(skins.frame.clone(), [1.; 4]),
            BackgroundColor(Color::srgb(0.18, 0.18, 0.18)),
            Node {
                position_type: PositionType::Absolute,
                bottom: px(25.),
                right: px(0.),
                width: px(300.),
                max_width: percent(100.),
                max_height: px(110.),
                padding: UiRect::all(px(3.)),
                flex_direction: FlexDirection::Column,
                display: Display::None,
                ..default()
            },
            ZIndex(3),
        ))
        .id();
    let rows = commands
        .spawn((
            Entries,
            bevy::ui_widgets::ScrollArea,
            Node {
                width: percent(100.),
                min_height: px(0.),
                max_height: px(75.),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    let footer = commands
        .spawn(Node {
            width: percent(100.),
            height: px(23.),
            min_height: px(23.),
            column_gap: px(2.),
            ..default()
        })
        .id();
    let input = widgets::text_input(commands, skins, Field::Recipient, 120);
    commands.entity(input).insert(Node {
        flex_grow: 1.,
        min_width: px(0.),
        height: px(23.),
        padding: UiRect::all(px(3.)),
        ..default()
    });
    let open = widgets::button(commands, skins, "Private", 55., Action::Conversation, 0);
    let close = widgets::button(commands, skins, "×", 22., Action::ChannelCancel, 0);
    commands.entity(footer).add_children(&[input, open, close]);
    commands.entity(panel).add_children(&[rows, footer]);
    commands.entity(root).add_child(panel);
}

fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut held: ResMut<Rows>,
    mut panels: Query<&mut Node, With<Channels>>,
    entries: Query<Entity, With<Entries>>,
) {
    let Some(play) = &owner.play else {
        held.generation = None;
        for mut panel in &mut panels {
            panel.display = Display::None;
        }
        return;
    };
    let snapshot = play.link.chat_panel();
    let dialog = &snapshot.console.dialog;
    for mut panel in &mut panels {
        panel.display = if dialog.open && !owner.modal_active() {
            Display::Flex
        } else {
            Display::None
        };
    }
    if !dialog.open {
        return;
    }
    let offered = play.link.chat_dialog_entries();
    // Derive each caption from the exact offered entry, even if the mailbox changed meanwhile.
    let labels = oteryn_client_ui::chat::dialog_rows(&offered, &snapshot.console);
    if held.generation == Some(owner.generation())
        && held.entries == offered
        && held.labels == labels
    {
        return;
    }
    for entity in &entries {
        commands.entity(entity).despawn_children();
        for (entry, label) in offered.iter().zip(&labels) {
            let button = widgets::button(
                &mut commands,
                &skins,
                label,
                285.,
                Action::Entry(entry.clone()),
                owner.generation(),
            );
            commands.entity(entity).add_child(button);
        }
    }
    held.generation = Some(owner.generation());
    held.entries = offered;
    held.labels = labels;
}
