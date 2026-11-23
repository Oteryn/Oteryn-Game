use super::*;
pub(super) fn spawn(
    commands: &mut Commands,
    skins: &Skins,
    list: Entity,
    (generation, revision): (u64, u64),
    entry: &Contact,
    group: Option<u8>,
    add_description: bool,
) {
    let row = commands
        .spawn((
            Row {
                group,
                generation,
                entry: entry.entry,
            },
            Control {
                generation,
                revision,
                command: Action::Edit(entry.entry),
            },
            Pickable::default(),
            Hovered::default(),
            Name::new("vip.entry"),
            TabIndex(0),
            Node {
                width: percent(100.),
                min_height: px(16.),
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .observe(input::activate)
        .observe(presentation::click)
        .observe(presentation::key)
        .observe(row_click)
        .id();
    let glyph = commands
        .spawn((
            VipMetadata {
                generation,
                entry: entry.entry,
                description: false,
            },
            Name::new(format!("vip.icon.{}", entry.entry)),
            source::text(
                vip_icon_text(entry.icon),
                source::font(skins.font.clone(), false),
            ),
            // Text alternative of the artwork icon; the ImageNode below is what is drawn.
            Node {
                display: Display::None,
                ..default()
            },
        ))
        .id();
    if add_description {
        commands.spawn((
            VipMetadata {
                generation,
                entry: entry.entry,
                description: true,
            },
            Name::new(format!("vip.description.{}", entry.entry)),
            source::text(
                entry.description.as_deref().unwrap_or(""),
                source::font(skins.font.clone(), false),
            ),
            Node {
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgb_u8(32, 32, 32)),
            GlobalZIndex(280),
        ));
    }
    let name = commands
        .spawn((
            VipNameLabel {
                group,
                generation,
                entry: entry.entry,
            },
            source::text(&entry.name, source::font(skins.font.clone(), false)),
            Node {
                flex_grow: 1.,
                min_width: px(0.),
                overflow: Overflow::clip_x(),
                ..default()
            },
        ))
        .id();
    let icon = commands
        .spawn((
            presentation::Icon(entry.entry),
            ImageNode::default(),
            Name::new("vip.icon"),
            Node {
                width: px(12.),
                height: px(12.),
                flex_shrink: 0.,
                ..default()
            },
            Pickable::IGNORE,
        ))
        .id();
    let context = button(
        commands,
        skins,
        Control {
            generation,
            revision,
            command: Action::Context(entry.entry),
        },
        "…",
        18.,
        "vip.context",
    );
    commands
        .entity(row)
        .add_children(&[icon, glyph, name, context]);
    commands.entity(list).add_child(row);
}
