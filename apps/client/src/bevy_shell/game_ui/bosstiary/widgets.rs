use super::*;
use bevy::{
    input_focus::tab_navigation::{TabGroup, TabIndex},
    text::TextCursorStyle,
    ui_widgets::{ScrollArea, TextInput},
};
use oteryn_client_ui_bevy as source;
pub(super) fn build(
    commands: &mut Commands,
    skins: &Skins,
    key: Key,
    snapshot: &Snapshot,
    confirmation: Option<&Control>,
) -> Vec<Entity> {
    let mut roots = Vec::new();
    // Independent cooldown window survives closing the main Bosstiary.
    for cooldowns in [true, false] {
        if !(if cooldowns {
            snapshot.panel.cooldowns_open
        } else {
            snapshot.panel.open
        }) {
            continue;
        }
        let root = commands
            .spawn((
                Pickable::IGNORE,
                GlobalZIndex(if cooldowns { 163 } else { 164 }),
                TabGroup {
                    order: 2,
                    modal: false,
                },
                Node {
                    position_type: PositionType::Absolute,
                    width: percent(100.),
                    height: percent(100.),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    ..default()
                },
            ))
            .id();
        if confirmation.is_some_and(|c| c.cooldowns == cooldowns) {
            commands.entity(root).insert((
                GameModal,
                Pickable::default(),
                TabGroup {
                    order: 2,
                    modal: true,
                },
            ));
        }
        roots.push(root);
        let panel = commands
            .spawn((
                crate::bevy_shell::window_drag::DraggableDialog,
                source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
                Node {
                    width: px(if cooldowns { 520. } else { 850. }),
                    max_width: percent(95.),
                    max_height: percent(95.),
                    padding: UiRect {
                        left: px(12.),
                        right: px(12.),
                        top: px(24.),
                        bottom: px(10.),
                    },
                    flex_direction: FlexDirection::Column,
                    row_gap: px(5.),
                    ..default()
                },
            ))
            .id();
        commands.entity(root).add_child(panel);
        source::dialog_caption(
            commands,
            panel,
            if cooldowns {
                "Boss cooldown tracker"
            } else {
                "Boss encounter / Bosstiary"
            },
            skins.font.clone(),
        );
        if let Some(control) = confirmation.filter(|c| c.cooldowns == cooldowns) {
            label(
                commands,
                skins,
                panel,
                format!(
                    "Confirm {}\nBoss: {} | Slot: {} | Hazard: {}",
                    match control.kind {
                        Kind::Request(Intent::Enter) => "enter encounter",
                        Kind::Request(Intent::Defeated) => "return after defeat",
                        Kind::Request(Intent::Timeout) => "return after timeout",
                        Kind::Request(Intent::SlotEquipSelected) => "equip boss slot",
                        Kind::Request(Intent::SlotRemoveSelected) => "remove boss slot",
                        Kind::Difficulty(oteryn_client_ui::boss_difficulty::Intent::Start, _) =>
                            "start encounter",
                        _ => "cancel encounter",
                    },
                    snapshot.panel.fields[0],
                    snapshot.panel.fields[1],
                    snapshot.panel.fields[2]
                ),
            );
            if matches!(control.kind, Kind::Request(Intent::SlotRemoveSelected))
                && let Some(slots) = &snapshot.panel.slots
            {
                label(
                    commands,
                    skins,
                    panel,
                    format!("Removal fee: {} gold", slots.remove_fee),
                );
            }
            if let Kind::Difficulty(_, level) = control.kind {
                label(commands, skins, panel, format!("Difficulty: {level}"));
            }
            for (yes, caption) in [(true, "Confirm"), (false, "Cancel")] {
                let button = commands
                    .spawn((source::button(skins.buttons.clone(), 100.), Confirm(yes)))
                    .observe(confirm)
                    .id();
                caption_child(commands, skins, button, caption);
                commands.entity(panel).add_child(button);
            }
            continue;
        }
        let body = commands
            .spawn((
                ScrollArea,
                Node {
                    width: percent(100.),
                    height: px(if cooldowns { 320. } else { 380. }),
                    min_height: px(0.),
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::scroll_y(),
                    ..default()
                },
                source::frame(skins.inset.clone(), [1.; 4]),
            ))
            .id();
        commands.entity(panel).add_child(body);
        let body_text = label(
            commands,
            skins,
            body,
            if cooldowns {
                boss::cooldown_text(&snapshot.panel, elapsed(snapshot))
            } else {
                main_text(snapshot)
            },
        );
        if cooldowns {
            commands.entity(body_text).insert(CooldownText);
        }
        if !cooldowns {
            for entry in boss::catalogue_cards(&snapshot.panel) {
                button(
                    commands,
                    skins,
                    body,
                    key,
                    Kind::Select(entry.key.clone()),
                    false,
                    &format!("Select {}", entry.name),
                );
            }
            let fields = row(commands, panel);
            for (index, caption, value) in [
                (0, "Boss key", snapshot.panel.fields[0].as_str()),
                (1, "Slot (1-2)", &snapshot.panel.fields[1]),
                (2, "Hazard (1-12)", &snapshot.panel.fields[2]),
                (3, "Boss search", &snapshot.panel.browser_search),
                (4, "Difficulty", &snapshot.difficulty.difficulty.to_string()),
            ] {
                let column = commands
                    .spawn(Node {
                        flex_direction: FlexDirection::Column,
                        width: px(145.),
                        ..default()
                    })
                    .id();
                commands.entity(fields).add_child(column);
                label(commands, skins, column, caption);
                let mut text = EditableText::new(value);
                text.max_characters = Some(if index == 3 { 48 } else { 512 });
                let field = commands
                    .spawn((
                        Field(index),
                        TextInput,
                        text,
                        source::font(skins.font.clone(), false),
                        TextColor(source::TEXT_COLOR),
                        TextLayout::no_wrap(),
                        TextCursorStyle::default(),
                        TabIndex(index as i32),
                        source::frame(skins.inset.clone(), [1.; 4]),
                        Node {
                            height: px(25.),
                            width: percent(100.),
                            padding: px(3.).all(),
                            ..default()
                        },
                    ))
                    .id();
                commands.entity(column).add_child(field);
                if index < 3 {
                    button(
                        commands,
                        skins,
                        column,
                        key,
                        Kind::Edit(index, String::new()),
                        false,
                        "Apply field",
                    );
                }
            }
        }
        let actions = row(commands, panel);
        for (intent, caption) in [(Intent::Refresh, "Refresh"), (Intent::Next, "Next page")] {
            button(
                commands,
                skins,
                actions,
                key,
                Kind::Request(intent),
                cooldowns,
                caption,
            );
        }
        if !cooldowns {
            for (intent, caption) in [
                (Intent::Enter, "Enter"),
                (Intent::Defeated, "Return after defeat"),
                (Intent::Timeout, "Return after timeout"),
                (Intent::HazardQuery, "Hazard query"),
                (Intent::HazardSelect, "Set hazard"),
                (Intent::SlotsDetailed, "Slots query"),
                (Intent::SlotEquipSelected, "Equip slot"),
                (Intent::SlotRemoveSelected, "Remove slot"),
                (Intent::Rewards, "Rewards"),
                (Intent::RewardsNext, "Next rewards"),
                (Intent::HazardLower, "Lower hazard"),
                (Intent::HazardRaise, "Raise hazard"),
                (Intent::Catalogue, "Browse boss gallery"),
                (Intent::CatalogueNext, "Next gallery page"),
            ] {
                button(
                    commands,
                    skins,
                    actions,
                    key,
                    Kind::Request(intent),
                    false,
                    caption,
                );
            }
            for (kind, caption) in [
                (Kind::Browse(true), "Previous boss"),
                (Kind::Browse(false), "Next boss"),
                (Kind::Category, "Boss category"),
                (Kind::Chain(boss::EndgameChain::SoulWar), "Soul War boss"),
                (
                    Kind::Chain(boss::EndgameChain::RottenBlood),
                    "Rotten Blood boss",
                ),
                (Kind::Search(String::new()), "Apply search"),
                (Kind::Slot, "Next unlocked slot"),
                (Kind::SlotBoss(true), "Previous eligible slot boss"),
                (Kind::SlotBoss(false), "Next eligible slot boss"),
            ] {
                button(commands, skins, actions, key, kind, false, caption);
            }
            for (intent, caption) in [
                (
                    oteryn_client_ui::boss_difficulty::Intent::Preview,
                    "Preview",
                ),
                (oteryn_client_ui::boss_difficulty::Intent::Start, "Start"),
                (
                    oteryn_client_ui::boss_difficulty::Intent::Cancel,
                    "Cancel encounter",
                ),
            ] {
                button(
                    commands,
                    skins,
                    actions,
                    key,
                    Kind::Difficulty(intent, snapshot.difficulty.difficulty),
                    false,
                    caption,
                );
            }
            button(
                commands,
                skins,
                actions,
                key,
                Kind::Toggle(true),
                false,
                "Cooldown tracker",
            );
        }
        button(
            commands,
            skins,
            actions,
            key,
            Kind::Close(cooldowns),
            cooldowns,
            "Close",
        );
    }
    roots
}
fn row(commands: &mut Commands, parent: Entity) -> Entity {
    let entity = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4.),
            row_gap: px(4.),
            flex_shrink: 0.,
            ..default()
        })
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn label(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    value: impl Into<String>,
) -> Entity {
    let entity = commands
        .spawn(source::text(value, source::font(skins.font.clone(), false)))
        .insert((
            TextLayout::default(),
            Node {
                max_width: percent(100.),
                flex_shrink: 0.,
                ..default()
            },
        ))
        .id();
    commands.entity(parent).add_child(entity);
    entity
}
fn caption_child(commands: &mut Commands, skins: &Skins, button: Entity, caption: &str) {
    let text = commands
        .spawn(source::button_caption(caption, skins.font.clone()))
        .id();
    commands.entity(button).add_child(text);
}
fn button(
    commands: &mut Commands,
    skins: &Skins,
    parent: Entity,
    key: Key,
    kind: Kind,
    cooldowns: bool,
    caption: &str,
) {
    let button = commands
        .spawn((
            source::button(skins.buttons.clone(), 160.),
            Control {
                key,
                kind,
                cooldowns,
            },
        ))
        .observe(activate)
        .id();
    caption_child(commands, skins, button, caption);
    commands.entity(parent).add_child(button);
}
