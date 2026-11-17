//! HUD widget setup and the combat control column.
use super::*;

pub(super) fn setup(
    mut commands: Commands,
    regions: Res<super::super::GameRegions>,
    sidebar: Option<Res<super::super::Sidebar>>,
    status_bar: Option<Res<super::super::StatusBar>>,
    skins: Res<Skins>,
    overlay: Res<DevOverlay>,
) {
    commands.entity(regions.vitals).insert(Node {
        width: percent(100.),
        flex_shrink: 0.,
        flex_direction: FlexDirection::Column,
        row_gap: px(2.),
        ..default()
    });
    let font = source::font(skins.font.clone(), false);
    let row = |commands: &mut Commands| {
        commands
            .spawn(Node {
                width: percent(100.),
                height: px(STATUS_ICON.y),
                flex_shrink: 0.,
                column_gap: px(2.),
                align_items: AlignItems::Center,
                ..default()
            })
            .id()
    };
    for (mana, color, key) in [
        (
            false,
            crate::bevy_shell::health_circle::health_colour(0, 0),
            "status-health",
        ),
        (true, Color::srgb_u8(0, 70, 180), "status-mana"),
    ] {
        let line = row(&mut commands);
        let icon = super::super::art::fixed(&mut commands, &skins.font, key, STATUS_ICON);
        let track = commands
            .spawn((
                Node {
                    width: px(STATUS_BAR.x),
                    height: px(STATUS_BAR.y),
                    flex_shrink: 0.,
                    overflow: Overflow::clip(),
                    ..default()
                },
                BackgroundColor(Color::srgb_u8(30, 30, 30)),
            ))
            .id();
        let fill = commands
            .spawn((
                VitalFill(mana),
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
        commands.entity(track).add_child(fill);
        let label = commands
            .spawn((
                VitalLabel(mana),
                source::text("—", font.clone()),
                Node {
                    width: px(STATUS_VALUE),
                    flex_shrink: 0.,
                    margin: UiRect::left(Val::Auto),
                    ..default()
                },
            ))
            .insert((
                TextLayout::justify(Justify::Center),
                TextColor(Color::WHITE),
            ))
            .id();
        commands.entity(line).add_children(&[icon, track, label]);
        commands.entity(regions.vitals).add_child(line);
    }
    let monk = row(&mut commands);
    commands.entity(monk).insert(MonkRow);
    commands
        .entity(monk)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.display = Display::None;
        });
    let harmony =
        super::super::art::fixed(&mut commands, &skins.font, "status-harmony", STATUS_ICON);
    let label = commands
        .spawn((MonkLabel, source::text("", font.clone())))
        .insert(TextColor(Color::WHITE))
        .id();
    let serene = super::super::art::fixed(&mut commands, &skins.font, "status-serene", STATUS_ICON);
    commands
        .entity(monk)
        .add_children(&[harmony, label, serene]);
    commands.entity(regions.vitals).add_child(monk);
    // The condition row sits in the status bar over the game window when there is one.
    let conditions = status_bar.map_or(regions.vitals, |bar| {
        top_bar::setup(&mut commands, &skins, bar.0)
    });
    status_icons::setup(&mut commands, conditions);
    combat_column(&mut commands, &skins, regions.equipment);
    if overlay.0 {
        let status = commands
            .spawn((
                InfoText::Status,
                source::text("", font.clone()),
                Node {
                    position_type: PositionType::Absolute,
                    left: px(4.),
                    top: px(4.),
                    right: px(4.),
                    ..default()
                },
                GlobalZIndex(5),
            ))
            .insert((TextColor(Color::srgb(1., 1., 0.6)), TextLayout::default()))
            .id();
        commands.entity(regions.world).add_child(status);
    }
    // The analytics selector widget: one button per analyser and its report.
    let Some(sidebar) = sidebar else {
        return;
    };
    let tools = commands
        .spawn(Node {
            width: percent(100.),
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(2.),
            row_gap: px(2.),
            ..default()
        })
        .id();
    for tool in oteryn_client_ui::analyser::Tool::ALL {
        let selector = button(
            &mut commands,
            &skins,
            0,
            Command::Analyser(tool),
            tool.title(),
            78.,
        );
        commands.entity(tools).add_child(selector);
    }
    let text = commands
        .spawn((
            InfoText::Analyser,
            source::text("", font),
            Node {
                width: percent(100.),
                padding: UiRect::all(px(3.)),
                ..default()
            },
        ))
        .insert(TextLayout::default())
        .id();
    commands
        .entity(sidebar.analytics)
        .add_children(&[tools, text]);
}

/// The combat column of the inventory widget, docked at the right of the equipment.
pub(super) fn combat_column(commands: &mut Commands, skins: &Skins, equipment: Entity) {
    commands.entity(equipment).insert(Node {
        width: percent(100.),
        min_height: px(142.),
        flex_shrink: 0.,
        flex_direction: FlexDirection::Column,
        padding: UiRect::right(px(COMBAT_COLUMN + 2.)),
        ..default()
    });
    let column = commands
        .spawn((
            Name::new("Combat controls"),
            Node {
                position_type: PositionType::Absolute,
                right: px(0.),
                top: px(0.),
                width: px(COMBAT_COLUMN),
                flex_direction: FlexDirection::Column,
                row_gap: px(2.),
                ..default()
            },
        ))
        .id();
    commands.entity(equipment).add_child(column);
    let stop = button(commands, skins, 0, Command::Stop, "Stop", COMBAT_COLUMN);
    commands
        .entity(stop)
        .insert((CombatControl, BorderColor::all(Color::NONE)));
    commands.entity(column).add_child(stop);
    let grid = |commands: &mut Commands, rows: u16| {
        commands
            .spawn(Node {
                display: Display::Grid,
                grid_auto_flow: GridAutoFlow::Column,
                grid_template_rows: RepeatedGridTrack::px(rows, COMBAT_BUTTON),
                grid_auto_columns: vec![GridTrack::px(COMBAT_BUTTON)],
                column_gap: px(2.),
                row_gap: px(2.),
                ..default()
            })
            .id()
    };
    let modes = grid(commands, 3);
    let pvp = grid(commands, 2);
    commands.entity(column).add_children(&[modes, pvp]);
    for (parent, command, key) in [
        (
            modes,
            Command::Fight(FightMode::Offensive),
            "combat-offensive",
        ),
        (
            modes,
            Command::Fight(FightMode::Balanced),
            "combat-balanced",
        ),
        (
            modes,
            Command::Fight(FightMode::Defensive),
            "combat-defensive",
        ),
        (modes, Command::Chase(ChaseMode::Stand), "combat-stand"),
        (modes, Command::Chase(ChaseMode::Chase), "combat-chase"),
        (modes, Command::Secure(true), "combat-secure"),
        (pvp, Command::Expert(0), "combat-pvp-dove"),
        (pvp, Command::Expert(1), "combat-pvp-whitehand"),
        (pvp, Command::Expert(2), "combat-pvp-yellowhand"),
        (pvp, Command::Expert(3), "combat-pvp-redfist"),
    ] {
        let slot = super::super::art::fixed(commands, &skins.font, key, Vec2::splat(COMBAT_BUTTON));
        commands.entity(slot).insert((
            CombatControl,
            HudAction {
                generation: 0,
                command,
            },
            bevy::ui_widgets::Button,
            BorderColor::all(Color::NONE),
        ));
        commands
            .entity(slot)
            .entry::<Node>()
            .and_modify(|mut node| {
                node.border = UiRect::all(px(1.));
            });
        commands.entity(parent).add_child(slot);
    }
}
