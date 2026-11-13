use super::*;
use bevy::input_focus::tab_navigation::TabIndex;
use bevy::ui_widgets::ScrollArea;
#[cfg(test)]
mod layout_tests;
mod rows;

pub(super) fn setup(
    mut commands: Commands,
    skins: Res<Skins>,
    assets: Res<AssetServer>,
    state: Res<OptionsState>,
) {
    let check_skins = CheckSkins {
        checked: assets.load("images/skin/classic/checkbox-checked.png"),
        unchecked: assets.load("images/skin/classic/checkbox-unchecked.png"),
    };
    let modal = commands
        .spawn((
            OptionsDialog,
            GameModal,
            Name::new("options.dialog"),
            GlobalZIndex(100),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                display: Display::None,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.65)),
        ))
        .id();
    // OptionsMenuDialog.qml: width685, pagepreferredheight470, leftcontent115+2*(2+5).
    let panel = commands
        .spawn((
            crate::bevy_shell::window_drag::DraggableDialog,
            TabGroup {
                order: 1,
                modal: true,
            },
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                width: px(685.),
                height: px(550.),
                max_width: percent(98.),
                max_height: percent(98.),
                padding: UiRect {
                    left: px(12.),
                    right: px(12.),
                    top: px(29.),
                    bottom: px(12.),
                },
                row_gap: px(10.),
                flex_direction: FlexDirection::Column,
                overflow: Overflow::clip(),
                ..default()
            },
        ))
        .id();
    commands.entity(modal).add_child(panel);
    source::dialog_caption(&mut commands, panel, "Options", skins.font.clone());
    let body = commands
        .spawn(Node {
            width: percent(100.),
            min_height: px(0.),
            flex_grow: 1.,
            column_gap: px(5.),
            ..default()
        })
        .id();
    let nav = commands
        .spawn((
            Name::new("options.navigation.frame"),
            ScrollArea,
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: px(129.),
                min_width: px(90.),
                flex_shrink: 0.,
                min_height: px(0.),
                height: percent(100.),
                overflow: Overflow::scroll_y(),
                padding: UiRect::all(px(7.)),
                row_gap: px(5.),
                flex_direction: FlexDirection::Column,
                ..default()
            },
        ))
        .id();
    let navigation = commands
        .spawn((
            Navigation,
            Node {
                width: percent(100.),
                min_height: px(0.),
                flex_shrink: 0.,
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    commands.entity(nav).add_child(navigation);
    let right = commands
        .spawn((
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                min_width: px(0.),
                min_height: px(0.),
                height: percent(100.),
                flex_grow: 1.,
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(px(7.)),
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    let page = commands
        .spawn((
            PageBody,
            ScrollArea,
            Node {
                width: percent(100.),
                min_height: px(0.),
                flex_grow: 1.,
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                ..default()
            },
        ))
        .id();
    let reset_row = commands
        .spawn(Node {
            width: percent(100.),
            justify_content: JustifyContent::End,
            flex_shrink: 0.,
            ..default()
        })
        .id();
    let reset = button(
        &mut commands,
        &skins,
        "Reset",
        50.,
        Control::Reset,
        "options.reset",
    );
    commands.entity(reset_row).add_child(reset);
    commands.entity(right).add_children(&[page, reset_row]);
    commands.entity(body).add_children(&[nav, right]);
    let feedback = commands
        .spawn((
            Feedback,
            source::text("", source::font(skins.font.clone(), true)),
            Node {
                width: percent(100.),
                min_height: px(0.),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .insert(TextLayout::default())
        .id();
    let footer = commands
        .spawn(Node {
            width: percent(100.),
            min_height: px(20.),
            flex_shrink: 0.,
            column_gap: px(5.),
            align_items: AlignItems::Center,
            ..default()
        })
        .id();
    let advanced = checkbox(
        &mut commands,
        &skins,
        &check_skins,
        ("Show Advanced Options", "options.advanced"),
        Control::Advanced,
        false,
        state.advanced,
    );
    commands.entity(footer).add_child(advanced);
    for (caption, control, id) in [
        ("OK", Control::Ok, "options.ok"),
        ("Apply", Control::Apply, "options.apply"),
        ("Cancel", Control::Cancel, "options.cancel"),
    ] {
        let entity = button(&mut commands, &skins, caption, 50., control, id);
        commands.entity(footer).add_child(entity);
    }
    commands
        .entity(panel)
        .add_children(&[body, feedback, footer]);
    commands.insert_resource(check_skins);
}
pub(super) fn button(
    commands: &mut Commands,
    skins: &Skins,
    label: &str,
    width: f32,
    control: Control,
    id: &str,
) -> Entity {
    let entity = commands
        .spawn((
            source::button(skins.buttons.clone(), width),
            control,
            Name::new(id.to_owned()),
        ))
        .observe(activate)
        .id();
    let text = commands
        .spawn(source::button_caption(label, skins.font.clone()))
        .id();
    commands.entity(entity).add_child(text);
    entity
}
fn checkbox(
    commands: &mut Commands,
    skins: &Skins,
    checks: &CheckSkins,
    label: (&str, &str),
    control: Control,
    framed: bool,
    checked: bool,
) -> Entity {
    let (label, id) = label;
    let mut node = source::checkbox_row(framed);
    if framed {
        node.width = percent(100.);
    } else {
        // The footer label owns the space before its fixed-width action buttons.
        node.min_width = px(0.);
        node.flex_grow = 1.;
        node.flex_shrink = 1.;
        node.flex_basis = px(0.);
    }
    let entity = commands
        .spawn((source::checkbox(node), control, Name::new(id.to_owned())))
        .observe(toggle)
        .id();
    if framed {
        commands
            .entity(entity)
            .insert(source::frame(skins.inset.clone(), [1.; 4]));
    }
    if checked {
        commands.entity(entity).insert(bevy::ui::Checked);
    }
    let indicator = commands
        .spawn((
            source::checkbox_indicator(if checked {
                checks.checked.clone()
            } else {
                checks.unchecked.clone()
            }),
            Indicator(entity),
        ))
        .id();
    let text = commands
        .spawn(source::text(label, source::font(skins.font.clone(), false)))
        .insert((
            TextLayout::default(),
            Node {
                // Fill the row after the indicator instead of using min-content width.
                min_width: px(0.),
                flex_grow: 1.,
                flex_shrink: 1.,
                flex_basis: px(0.),
                ..default()
            },
        ))
        .id();
    commands.entity(entity).add_children(&[indicator, text]);
    entity
}
pub(super) fn page(
    commands: &mut Commands,
    parent: Entity,
    page: Page,
    skins: &Skins,
    checks: &CheckSkins,
    draft: &crate::play::ReferencePreferences,
) {
    if matches!(page, Page::Sound | Page::BattleSounds | Page::UiSounds) {
        let status = commands
            .spawn((
                audio_feedback::Status,
                source::text("", source::font(skins.font.clone(), true)),
                Node {
                    width: percent(100.),
                    min_height: px(0.),
                    flex_shrink: 0.,
                    display: Display::None,
                    ..default()
                },
            ))
            .insert(TextLayout::default())
            .id();
        commands.entity(parent).add_child(status);
    }
    for row in metadata::rows(page) {
        let entity = match row.editor {
            Editor::Check { read, .. } => checkbox(
                commands,
                skins,
                checks,
                (row.label, row.id),
                Control::Check(row.id),
                true,
                read(draft),
            ),
            Editor::Number { read, min, max, .. } => {
                rows::number(commands, skins, *row, min, max, read(draft))
            }
            Editor::Choice { read, values, .. } => {
                rows::choice(commands, skins, *row, values, read(draft))
            }
        };
        commands.entity(parent).add_child(entity);
    }
}
