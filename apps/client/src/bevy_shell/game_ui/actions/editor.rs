//! Assignment edits call the retained local owners; no native platform dialog is opened.
use super::*;
use crate::play::{BarAction, ItemUseMode, PassiveAbility};
use bevy::{
    input_focus::{
        FocusCause, InputFocus,
        tab_navigation::{TabGroup, TabIndex},
    },
    text::{EditableText, TextCursorStyle},
    ui_widgets::TextInput,
};

mod catalog;
#[cfg(test)]
mod tests;

#[derive(Resource, Default)]
pub(super) struct EditorState {
    selected: Option<(usize, u64)>,
    rendered: Option<(usize, u64)>,
    message: String,
    page: usize,
    catalog_key: Option<catalog::Key>,
}
impl EditorState {
    pub(super) fn active(&self) -> bool {
        self.selected.is_some()
    }
    pub(super) fn open(&mut self, slot: usize, generation: u64) {
        self.selected = Some((slot, generation));
        self.message.clear();
        self.page = 0;
        self.catalog_key = None;
    }
}
#[derive(Component)]
pub(super) struct EditorRoot;
#[derive(Component)]
pub(super) struct Field(usize);
#[derive(Component)]
pub(super) struct Description;
#[derive(Component)]
pub(super) struct Feedback;
#[derive(Component, Clone, Copy)]
enum Edit {
    PreviousSpells,
    NextSpells,
    Text,
    InsertText,
    Equip,
    Use,
    SelfActor,
    Target,
    Cursor,
    Instinct,
    Harmony,
    Serene,
    AimTarget,
    AimFacing,
    AimCursor,
    Key,
    Clear,
    Close,
}

pub(super) fn setup(mut commands: Commands, skins: Res<Skins>) {
    let root = commands
        .spawn((
            EditorRoot,
            super::super::GameModal,
            GlobalZIndex(120),
            Node {
                position_type: PositionType::Absolute,
                width: percent(100.),
                height: percent(100.),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                display: Display::None,
                ..default()
            },
            BackgroundColor(Color::srgba(0., 0., 0., 0.65)),
        ))
        .id();
    let panel = commands
        .spawn((
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            TabGroup {
                order: 5,
                modal: true,
            },
            bevy::ui_widgets::ScrollArea::default(),
            Node {
                width: px(530.),
                max_width: percent(98.),
                max_height: percent(98.),
                padding: UiRect {
                    left: px(12.),
                    right: px(12.),
                    top: px(25.),
                    bottom: px(12.),
                },
                flex_direction: FlexDirection::Column,
                row_gap: px(5.),
                overflow: Overflow::scroll_y(),
                ..default()
            },
        ))
        .id();
    commands.entity(root).add_child(panel);
    source::dialog_caption(&mut commands, panel, "Assign Action", skins.font.clone());
    let description = commands
        .spawn((
            Description,
            source::text("", source::font(skins.font.clone(), false)),
        ))
        .insert(TextLayout::default())
        .id();
    commands.entity(panel).add_child(description);
    for (index, caption) in [
        "Search spells by name or words",
        "Carried item definition",
        "Text",
        "Keyboard chord",
    ]
    .into_iter()
    .enumerate()
    {
        let label = commands
            .spawn(source::text(
                caption,
                source::font(skins.font.clone(), false),
            ))
            .id();
        let frame = commands
            .spawn((
                source::frame(skins.inset.clone(), [1.; 4]),
                Node {
                    width: percent(100.),
                    height: px(24.),
                    min_height: px(24.),
                    ..default()
                },
            ))
            .id();
        let field = commands
            .spawn((
                Field(index),
                TextInput,
                EditableText {
                    max_characters: Some(if index == 2 { 1020 } else { 64 }),
                    ..default()
                },
                source::font(skins.font.clone(), false),
                TextColor(source::TEXT_COLOR),
                TextLayout::no_wrap(),
                TextCursorStyle::default(),
                TabIndex(index as i32),
                Node {
                    width: percent(100.),
                    height: px(24.),
                    padding: px(4.).all(),
                    ..default()
                },
            ))
            .id();
        commands.entity(frame).add_child(field);
        commands.entity(panel).add_children(&[label, frame]);
        if index == 0 {
            let choices = commands
                .spawn((
                    catalog::CatalogRoot,
                    Node {
                        width: percent(100.),
                        height: px(92.),
                        min_height: px(92.),
                        flex_wrap: FlexWrap::Wrap,
                        column_gap: px(3.),
                        row_gap: px(3.),
                        overflow: Overflow::scroll_y(),
                        ..default()
                    },
                    bevy::ui_widgets::ScrollArea::default(),
                ))
                .id();
            commands.entity(panel).add_child(choices);
        }
    }
    let actions = commands
        .spawn(Node {
            flex_wrap: FlexWrap::Wrap,
            column_gap: px(4.),
            row_gap: px(4.),
            ..default()
        })
        .id();
    for (action, caption) in [
        (Edit::PreviousSpells, "Previous spells"),
        (Edit::NextSpells, "Next spells"),
        (Edit::Text, "Send text"),
        (Edit::InsertText, "Insert text"),
        (Edit::Equip, "Equip"),
        (Edit::Use, "Use position"),
        (Edit::SelfActor, "Use on self"),
        (Edit::Target, "Use on target"),
        (Edit::Cursor, "Use at cursor"),
        (Edit::Instinct, "Instinct"),
        (Edit::Harmony, "Harmony"),
        (Edit::Serene, "Serene"),
        (Edit::AimTarget, "Towards target"),
        (Edit::AimFacing, "Facing"),
        (Edit::AimCursor, "Towards cursor"),
        (Edit::Key, "Bind key"),
        (Edit::Clear, "Clear"),
        (Edit::Close, "Close"),
    ] {
        let button = commands
            .spawn((source::button(skins.buttons.clone(), 90.), action))
            .observe(activate)
            .id();
        let caption = commands
            .spawn(source::button_caption(caption, skins.font.clone()))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(actions).add_child(button);
    }
    let feedback = commands
        .spawn((
            Feedback,
            source::text("", source::font(skins.font.clone(), false)),
        ))
        .insert(TextLayout::default())
        .id();
    commands.entity(panel).add_children(&[actions, feedback]);
}

fn activate(
    event: On<Activate>,
    edits: Query<&Edit>,
    fields: Query<(&Field, &EditableText)>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<EditorState>,
    mut focus: ResMut<InputFocus>,
) {
    let Ok(action) = edits.get(event.entity) else {
        return;
    };
    let Some((slot, generation)) = state.selected else {
        return;
    };
    if matches!(action, Edit::Close) {
        state.selected = None;
        focus.clear();
        return;
    }
    if owner.modal_active() || generation != owner.generation() {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let mut values: [String; 4] = Default::default();
    for (field, text) in &fields {
        if let Some(value) = values.get_mut(field.0) {
            *value = text.value().to_string();
        }
    }
    match action {
        Edit::PreviousSpells => {
            state.page = state.page.saturating_sub(1);
            return;
        }
        Edit::NextSpells => {
            state.page = state.page.saturating_add(1);
            return;
        }
        _ => {}
    }
    state.message = if apply(&play.link, slot, *action, &values) {
        "Saved action assignment".into()
    } else {
        "Choose a valid live spell, carried item, text or unused keyboard chord.".into()
    };
}

fn apply(link: &crate::play::PlayLink, slot: usize, action: Edit, values: &[String; 4]) -> bool {
    if slot >= ROW_SLOTS * 3 {
        return false;
    }
    match action {
        Edit::AimTarget | Edit::AimFacing | Edit::AimCursor => link.bar_gesture(BarGesture::Aim {
            slot,
            aim: match action {
                Edit::AimTarget => crate::play::SpellAim::Target,
                Edit::AimFacing => crate::play::SpellAim::Facing,
                _ => crate::play::SpellAim::Cursor,
            },
        }),
        Edit::Key => link.bind_action_key(slot, &values[3]),
        Edit::Clear => {
            link.clear_bar_action(slot);
            link.assign_spell_slot(slot, None)
        }
        Edit::Text | Edit::InsertText => link.assign_bar_action(
            slot,
            BarAction::Text {
                text: values[2].clone(),
                send_automatically: matches!(action, Edit::Text),
            },
        ),
        Edit::Instinct | Edit::Harmony | Edit::Serene => link.assign_bar_action(
            slot,
            BarAction::Passive {
                ability: match action {
                    Edit::Instinct => PassiveAbility::BattleInstinct,
                    Edit::Harmony => PassiveAbility::Harmony,
                    _ => PassiveAbility::Serene,
                },
            },
        ),
        Edit::Equip | Edit::Use | Edit::SelfActor | Edit::Target | Edit::Cursor => {
            let Ok(definition) = values[1].trim().parse::<std::num::NonZeroU32>() else {
                return false;
            };
            link.assign_bar_action(
                slot,
                if matches!(action, Edit::Equip) {
                    BarAction::Equip {
                        definition,
                        upgrade_tier: 0,
                    }
                } else {
                    BarAction::Use {
                        definition,
                        mode: match action {
                            Edit::SelfActor => ItemUseMode::SelfActor,
                            Edit::Target => ItemUseMode::Target,
                            Edit::Cursor => ItemUseMode::Cursor,
                            _ => ItemUseMode::CursorPosition,
                        },
                    }
                },
            )
        }
        Edit::Close | Edit::PreviousSpells | Edit::NextSpells => false,
    }
}

// Bevy injects the editor owner, disjoint widget queries and focus/input resources separately.
#[allow(clippy::too_many_arguments)]
pub(super) fn project(
    owner: NonSend<HostOwner>,
    mut state: ResMut<EditorState>,
    mut root: Query<&mut Node, With<EditorRoot>>,
    mut fields: Query<(Entity, &Field, &mut EditableText)>,
    mut descriptions: Query<&mut Text, (With<Description>, Without<Feedback>)>,
    mut feedback: Query<&mut Text, (With<Feedback>, Without<Description>)>,
    mut focus: ResMut<InputFocus>,
    keys: Option<Res<ButtonInput<KeyCode>>>,
) {
    if state.active()
        && (owner.play.is_none()
            || state
                .selected
                .is_some_and(|(_, generation)| generation != owner.generation())
            || keys.is_some_and(|keys| keys.just_pressed(KeyCode::Escape)))
    {
        state.selected = None;
        focus.clear();
    }
    for mut node in &mut root {
        let next = if state.active() {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != next {
            node.display = next;
        }
    }
    if state.rendered != state.selected {
        if let Some((slot, _)) = state.selected {
            let mut values: [String; 4] = Default::default();
            values[3] = owner
                .settings
                .preferences
                .action_keys
                .0
                .get(&slot)
                .cloned()
                .unwrap_or_default();
            match owner.settings.preferences.bar_actions.0.get(&slot) {
                Some(BarAction::Text { text, .. }) => values[2].clone_from(text),
                Some(BarAction::Use { definition, .. } | BarAction::Equip { definition, .. }) => {
                    values[1] = definition.to_string()
                }
                _ => {}
            }
            for (entity, field, mut text) in &mut fields {
                text.pending_edits.clear();
                text.pending_paste = None;
                text.editor_mut().set_text(&values[field.0]);
                if field.0 == 0 {
                    focus.set(entity, FocusCause::Navigated);
                }
            }
            let description = owner
                .play
                .as_ref()
                .and_then(|play| play.link.spell_slot_tooltips().get(slot).cloned())
                .unwrap_or_default();
            for mut text in &mut descriptions {
                text.0 = format!("Slot {}\n{}", slot + 1, description);
            }
        }
        state.rendered = state.selected;
    }
    for mut text in &mut feedback {
        if text.0 != state.message {
            text.0.clone_from(&state.message);
        }
    }
}

pub(super) fn install(app: &mut App) {
    app.add_systems(Update, catalog::project.after(project));
}
