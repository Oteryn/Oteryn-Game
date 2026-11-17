//! Game screen region layout spawned at play start.
use super::*;

pub(in crate::bevy_shell) fn setup(mut commands: Commands, skins: Res<Skins>) {
    let root = commands
        .spawn((
            GameRoot,
            bevy::input_focus::tab_navigation::TabGroup::new(0),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                display: Display::None,
                ..default()
            },
            GlobalZIndex(0),
        ))
        .id();
    // The status widget of the right sidebar: hitpoints and mana.
    let vitals = commands
        .spawn(Node {
            width: percent(100.),
            flex_shrink: 0.,
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    // Left sidebar | centre | right sidebars, every column the full window height.
    let main = commands
        .spawn(Node {
            flex_grow: 1.,
            min_width: px(0.),
            min_height: px(0.),
            height: percent(100.),
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id();
    let map_row = commands
        .spawn(Node {
            width: percent(100.),
            min_height: px(0.),
            flex_grow: 1.,
            ..default()
        })
        .id();
    let status_bar = region(&mut commands);
    // The grey client background shows around the map.
    let world = commands
        .spawn((
            WorldRegion,
            Node {
                min_width: px(0.),
                min_height: px(0.),
                flex_grow: 1.,
                overflow: Overflow::clip(),
                ..default()
            },
            BackgroundColor(Color::srgb_u8(70, 70, 70)),
        ))
        .id();
    let bars = commands
        .spawn((
            Node {
                width: percent(100.),
                min_height: px(36.),
                max_height: px(156.),
                flex_shrink: 0.,
                overflow: Overflow::clip(),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            BackgroundColor(Color::srgb(0.12, 0.12, 0.12)),
        ))
        .id();
    let chat = commands
        .spawn((
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                height: px(150.),
                min_height: px(90.),
                max_height: percent(50.),
                flex_shrink: 0.,
                padding: UiRect::all(px(2.)),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .id();
    let column = |commands: &mut Commands, shown: bool| {
        commands
            .spawn((
                hud_panes::Column,
                Node {
                    width: px(SIDEBAR_WIDTH),
                    min_width: px(SIDEBAR_WIDTH),
                    max_width: px(SIDEBAR_WIDTH),
                    height: percent(100.),
                    flex_shrink: 0.,
                    flex_direction: FlexDirection::Column,
                    display: if shown { Display::Flex } else { Display::None },
                    ..default()
                },
                BackgroundColor(Color::srgb(0.18, 0.18, 0.18)),
            ))
            .id()
    };
    let left = column(&mut commands, false);
    let extra = column(&mut commands, false);
    let right = column(&mut commands, true);
    // The fixed upper part of the right sidebar: minimap, status, inventory, control buttons.
    let panel = commands
        .spawn(Node {
            width: percent(100.),
            flex_shrink: 0.,
            flex_direction: FlexDirection::Column,
            padding: UiRect::axes(px(PANEL_MARGIN[0] / 3.), px(PANEL_MARGIN[1] / 2.)),
            row_gap: px(PANEL_MARGIN[1] / 2.),
            ..default()
        })
        .id();
    let widget_pane = |commands: &mut Commands| {
        let pane = commands
            .spawn((
                hud_panes::WidgetPane,
                Node {
                    width: percent(100.),
                    min_height: px(0.),
                    flex_grow: 1.,
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                bevy::ui_widgets::ScrollArea,
            ))
            .id();
        let fill = commands
            .spawn((
                hud_panes::FillPane,
                Node {
                    width: percent(100.),
                    min_height: px(0.),
                    flex_grow: 1.,
                    ..default()
                },
            ))
            .id();
        commands.entity(pane).add_child(fill);
        pane
    };
    let left_widgets = widget_pane(&mut commands);
    let extra_widgets = widget_pane(&mut commands);
    let right_widgets = widget_pane(&mut commands);
    let toolbar = commands
        .spawn((
            ToolbarRegion,
            Node {
                width: percent(100.),
                min_height: px(24.),
                flex_shrink: 0.,
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    super::super::ui::toolbar::grid::spawn(&mut commands, &skins, toolbar);
    let minimap = region(&mut commands);
    let equipment = region(&mut commands);
    let containers = region(&mut commands);
    let trade_dock = region(&mut commands);
    commands.entity(trade_dock).insert(trade::TradeDock);
    let battle = region(&mut commands);
    let party_dock = region(&mut commands);
    commands.entity(party_dock).insert(party::PartyDock);
    let skills = region(&mut commands);
    let vip = region(&mut commands);
    let analyzers = region(&mut commands);
    let analytics = region(&mut commands);
    let console_splitter = layout::spawn(&mut commands, chat, "console");
    // The minimap and the chat float as windows of their own and dock back in place.
    commands
        .entity(minimap)
        .insert(hud_panes::float::Floatable {
            id: hud_panes::float::MINIMAP,
            home: panel,
            index: 0,
            companion: None,
        });
    commands.entity(chat).insert(hud_panes::float::Floatable {
        id: hud_panes::float::CHAT,
        home: main,
        index: 4,
        companion: Some(console_splitter),
    });
    let chat_float = art::fixed(&mut commands, &skins.font, "chat-float", Vec2::splat(14.));
    commands.entity(chat_float).insert((
        Name::new("Chat float"),
        bevy::ui_widgets::Button,
        hud_panes::float::FloatToggle(hud_panes::float::CHAT),
        ZIndex(1),
    ));
    commands
        .entity(chat_float)
        .entry::<Node>()
        .and_modify(|mut node| {
            node.position_type = PositionType::Absolute;
            node.right = px(2.);
            node.top = px(2.);
        });
    commands.entity(chat).add_child(chat_float);
    let arrows =
        |commands: &mut Commands, side: hud_panes::Side| hud_panes::arrows(commands, &skins, side);
    let left_arrows = arrows(&mut commands, hud_panes::Side::Left);
    let right_arrows = arrows(&mut commands, hud_panes::Side::Right);
    commands
        .entity(root)
        .add_children(&[left, main, extra, right]);
    commands
        .entity(main)
        .add_children(&[status_bar, map_row, bars, console_splitter, chat]);
    commands
        .entity(map_row)
        .add_children(&[left_arrows, world, right_arrows]);
    commands.entity(left).add_child(left_widgets);
    commands.entity(extra).add_child(extra_widgets);
    commands.entity(right).add_children(&[panel, right_widgets]);
    commands
        .entity(panel)
        .add_children(&[minimap, vitals, equipment, toolbar]);
    // Tibia sidebars.json: battleList, container, skills and vip open in this order; party,
    // trackers, analysers and trade dock below them and stay empty until opened.
    let mut docked = Vec::new();
    for (widget, content) in [
        (hud_panes::Widget::BattleList, battle),
        (hud_panes::Widget::Containers, containers),
        (hud_panes::Widget::Skills, skills),
        (hud_panes::Widget::Vip, vip),
        (hud_panes::Widget::AnalyticsSelector, analytics),
    ] {
        docked.push(hud_panes::frame(&mut commands, &skins, widget, content));
    }
    docked.extend([party_dock, trade_dock, analyzers]);
    commands.entity(right_widgets).insert_children(0, &docked);
    commands.insert_resource(GameRegions {
        vitals,
        minimap,
        equipment,
        containers,
        battle,
        skills,
        vip,
        analyzers,
        chat,
        bars,
        world,
    });
    commands.insert_resource(StatusBar(status_bar));
    commands.insert_resource(Sidebar {
        left,
        extra,
        left_widgets,
        extra_widgets,
        right_widgets,
        analytics,
    });
}

pub(super) fn region(commands: &mut Commands) -> Entity {
    commands
        .spawn(Node {
            width: percent(100.),
            flex_shrink: 0.,
            flex_direction: FlexDirection::Column,
            ..default()
        })
        .id()
}
