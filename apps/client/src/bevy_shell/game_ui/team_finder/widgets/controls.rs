use super::*;

pub(super) fn input(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    field: Field,
    value: String,
) {
    let mut text = EditableText::new(value);
    text.max_characters = Some(64);
    let input = commands
        .spawn((
            field,
            TextInput,
            text,
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            TextCursorStyle::default(),
            TabIndex(0),
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                height: px(25.),
                padding: UiRect::all(px(4.)),
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(input);
}

pub(super) fn caption(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    value: fn() -> &'static str,
) {
    let entity = label(commands, skins, parent, value());
    commands.entity(entity).insert(Caption(value));
}

pub(super) fn label(commands: &mut Commands, skins: &Skins, parent: Entity, value: &str) -> Entity {
    let child = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .id();
    commands.entity(parent).add_child(child);
    child
}

pub(super) fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let child = commands
        .spawn(Node {
            column_gap: px(5.),
            flex_wrap: FlexWrap::Wrap,
            row_gap: px(4.),
            ..default()
        })
        .id();
    commands.entity(parent).add_child(child);
    child
}

pub(super) fn buttons<const N: usize>(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
) -> [Button; N] {
    std::array::from_fn(|_| button(commands, skins, parent, false))
}

pub(super) fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    listing_row: bool,
) -> Button {
    let mut entity = commands.spawn((source::button(skins.buttons.clone(), 150.),));
    if listing_row {
        entity.insert(Node {
            width: percent(100.),
            min_height: px(24.),
            padding: UiRect::all(px(4.)),
            flex_shrink: 0.,
            ..default()
        });
    }
    let child = entity.observe(activate).id();
    commands.entity(parent).add_child(child);
    let text = commands
        .spawn(source::button_caption("", skins.font.clone()))
        .id();
    if listing_row {
        commands.entity(text).insert((
            TextLayout::default(),
            Node {
                max_width: percent(100.),
                flex_shrink: 1.,
                ..default()
            },
        ));
    }
    commands.entity(child).add_child(text);
    Button {
        entity: child,
        text,
    }
}
