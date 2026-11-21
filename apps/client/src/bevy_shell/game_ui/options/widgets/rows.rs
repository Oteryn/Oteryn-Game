use super::*;
use bevy::ui_widgets::{Slider, SliderRange, SliderStep, SliderThumb, SliderValue};
fn row(commands: &mut Commands, skins: &Skins, label: &str) -> Entity {
    let entity = commands
        .spawn(Node {
            width: percent(100.),
            min_height: px(22.),
            flex_shrink: 0.,
            column_gap: px(5.),
            align_items: AlignItems::Center,
            flex_wrap: FlexWrap::Wrap,
            ..default()
        })
        .id();
    let text = commands
        .spawn(source::text(label, source::font(skins.font.clone(), false)))
        .insert((
            TextLayout::default(),
            Node {
                flex_grow: 1.,
                min_width: px(80.),
                flex_basis: px(160.),
                ..default()
            },
        ))
        .id();
    commands.entity(entity).add_child(text);
    entity
}
pub(super) fn number(
    commands: &mut Commands,
    skins: &Skins,
    field: Row,
    min: u16,
    max: u16,
    value: u16,
) -> Entity {
    let row = row(commands, skins, field.label);
    let slider = commands
        .spawn((
            Slider::default(),
            SliderRange::new(f32::from(min), f32::from(max)),
            SliderStep(1.),
            SliderValue(f32::from(value)),
            Control::Number(field.id),
            Name::new(field.id),
            TabIndex(0),
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: px(175.),
                min_width: px(70.),
                height: px(16.),
                flex_shrink: 1.,
                ..default()
            },
        ))
        .observe(slide)
        .id();
    let travel = commands
        .spawn((
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                left: px(0.),
                right: px(8.),
                height: percent(100.),
                ..default()
            },
        ))
        .id();
    let thumb = commands
        .spawn((
            SliderThumb,
            Thumb(slider),
            source::frame(skins.buttons.up.clone(), [1.; 4]),
            Node {
                position_type: PositionType::Absolute,
                left: percent(
                    SliderRange::new(f32::from(min), f32::from(max))
                        .thumb_position(f32::from(value))
                        .clamp(0., 1.)
                        * 100.,
                ),
                width: px(8.),
                height: percent(100.),
                ..default()
            },
        ))
        .id();
    commands.entity(travel).add_child(thumb);
    commands.entity(slider).add_child(travel);
    let label = commands
        .spawn((
            NumberCaption(field.id),
            source::text(value.to_string(), source::font(skins.font.clone(), false)),
            Node {
                width: px(34.),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(row).add_children(&[slider, label]);
    row
}
pub(super) fn choice(
    commands: &mut Commands,
    skins: &Skins,
    field: Row,
    values: &[&str],
    value: usize,
) -> Entity {
    let row = row(commands, skins, field.label);
    let editor = button(
        commands,
        skins,
        "",
        175.,
        Control::Choice(field.id),
        field.id,
    );
    let caption = commands
        .spawn((
            ChoiceCaption(field.id),
            source::button_caption(values.get(value).copied().unwrap_or(""), skins.font.clone()),
        ))
        .id();
    commands.entity(editor).add_child(caption);
    let popup = commands
        .spawn((
            ChoicePopup(field.id),
            source::frame(skins.inset.clone(), [1.; 4]),
            BackgroundColor(Color::srgb(0.18, 0.18, 0.18)),
            Node {
                position_type: PositionType::Absolute,
                top: px(20.),
                right: px(0.),
                width: px(175.),
                display: Display::None,
                flex_direction: FlexDirection::Column,
                ..default()
            },
            ZIndex(20),
        ))
        .id();
    for (index, label) in values.iter().enumerate() {
        let entity = button(
            commands,
            skins,
            label,
            175.,
            Control::Select(field.id, index),
            &format!("{}.{}", field.id, index),
        );
        if index == value {
            commands.entity(entity).insert(bevy::ui::Checked);
        }
        commands.entity(popup).add_child(entity);
    }
    commands.entity(row).add_children(&[editor, popup]);
    row
}
