//! A bounded search/picker over the current server catalogue, retaining display identity.
use super::*;
use crate::bevy_shell::game_ui::modal::ModalGuard;

const PAGE: usize = 12;
pub(super) const PARAMETER_FIELD: usize = 6;

#[derive(Component)]
pub(super) struct ParameterControl;
#[derive(Component)]
pub(super) struct CatalogRoot;
#[derive(Component, Clone)]
struct Choice {
    slot: usize,
    generation: u64,
    cast_index: u32,
    name: String,
    words: String,
}
#[derive(PartialEq, Eq)]
pub(super) struct Key {
    selected: (usize, u64),
    query: String,
    page: usize,
    entries: Vec<(u32, String, String)>,
}

pub(super) fn spawn_parameter_field(commands: &mut Commands, panel: Entity, skins: &Skins) {
    let label = commands
        .spawn((
            ParameterControl,
            source::text(
                "Spell parameter (enter before choosing the spell)",
                source::font(skins.font.clone(), false),
            ),
            Node {
                display: Display::None,
                ..default()
            },
        ))
        .id();
    let frame = commands
        .spawn((
            ParameterControl,
            source::frame(skins.inset.clone(), [1.; 4]),
            Node {
                width: percent(100.),
                height: px(24.),
                min_height: px(24.),
                display: Display::None,
                ..default()
            },
        ))
        .id();
    let field = commands
        .spawn((
            Field(PARAMETER_FIELD),
            TextInput,
            EditableText {
                max_characters: Some(128),
                ..default()
            },
            source::font(skins.font.clone(), false),
            TextColor(source::TEXT_COLOR),
            TextLayout::no_wrap(),
            TextCursorStyle::default(),
            TabIndex(PARAMETER_FIELD as i32),
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
}

pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<EditorState>,
    roots: Query<Entity, With<CatalogRoot>>,
    mut fields: Query<(&Field, &mut EditableText)>,
    mut controls: Query<&mut Node, With<ParameterControl>>,
) {
    let Some((slot, generation)) = state.selected else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    let parameters_selected = play.link.spell_parameters_selected();
    let display = if parameters_selected {
        Display::Flex
    } else {
        Display::None
    };
    for mut node in &mut controls {
        if node.display != display {
            node.display = display;
        }
    }
    if parameters_selected
        && state.catalog_key.is_none()
        && let Some(BarAction::Text {
            text,
            send_automatically: true,
        }) = owner.settings.preferences.bar_actions.0.get(&slot)
        && let Some(parameter) = play.link.stored_spell_parameter(text)
        && let Some((_, mut field)) = fields
            .iter_mut()
            .find(|(field, _)| field.0 == PARAMETER_FIELD)
    {
        field.editor_mut().set_text(&parameter);
    }
    let query = fields
        .iter()
        .find(|(field, _)| field.0 == 0)
        .map_or(String::new(), |(_, text)| {
            text.value().to_string().to_lowercase()
        });
    if state
        .catalog_key
        .as_ref()
        .is_some_and(|key| key.query != query)
    {
        state.page = 0;
    }
    let panel = play.link.spell_catalog_panel();
    let entries: Vec<_> = panel
        .catalog
        .entries()
        .iter()
        .filter(|entry| {
            entry.name.to_lowercase().contains(&query)
                || entry.words.to_lowercase().contains(&query)
        })
        .collect();
    state.page = state.page.min(entries.len().saturating_sub(1) / PAGE);
    let key = Key {
        selected: (slot, generation),
        query,
        page: state.page,
        entries: entries
            .iter()
            .skip(state.page * PAGE)
            .take(PAGE)
            .map(|entry| (entry.cast_index, entry.name.clone(), entry.words.clone()))
            .collect(),
    };
    if state.catalog_key.as_ref() == Some(&key) {
        return;
    }
    for root in &roots {
        commands.entity(root).despawn_children();
        for (cast_index, name, words) in &key.entries {
            let button = commands
                .spawn((
                    source::button(skins.buttons.clone(), 150.),
                    Choice {
                        slot,
                        generation,
                        cast_index: *cast_index,
                        name: name.clone(),
                        words: words.clone(),
                    },
                ))
                .observe(choose)
                .id();
            let text = commands
                .spawn(source::button_caption(name, skins.font.clone()))
                .insert(Node {
                    max_width: px(146.),
                    overflow: Overflow::clip(),
                    ..default()
                })
                .id();
            commands.entity(button).add_child(text);
            commands.entity(root).add_child(button);
        }
        if key.entries.is_empty() {
            let text = commands
                .spawn(source::text(
                    "No matching spell in the current catalogue.",
                    source::font(skins.font.clone(), false),
                ))
                .id();
            commands.entity(root).add_child(text);
        }
    }
    state.catalog_key = Some(key);
}

fn choose(
    event: On<Activate>,
    guard: ModalGuard,
    choices: Query<&Choice>,
    fields: Query<(&Field, &EditableText)>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<EditorState>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok(choice) = choices.get(event.entity) else {
        return;
    };
    if owner.modal_active()
        || owner.generation() != choice.generation
        || state.selected != Some((choice.slot, choice.generation))
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let same = play
        .link
        .spell_catalog_panel()
        .catalog
        .entries()
        .iter()
        .any(|entry| {
            entry.cast_index == choice.cast_index
                && entry.name == choice.name
                && entry.words == choice.words
        });
    let parameter = play
        .link
        .spell_parameters_selected()
        .then(|| {
            fields
                .iter()
                .find_map(|(field, text)| (field.0 == PARAMETER_FIELD).then_some(text))
        })
        .flatten();
    if parameter.is_some_and(editing) {
        state.message = "Finish entering the parameter, then choose the spell.".into();
        return;
    }
    let parameter = parameter
        .map(|text| text.value().to_string())
        .unwrap_or_default();
    let spell = if parameter.is_empty() {
        BarAction::Spell {
            name: choice.name.clone(),
            words: choice.words.clone(),
            aim: crate::play::SpellAim::Target,
        }
    } else {
        if state.multi.is_some() {
            state.message = "Assign the parameter spell to its own action button.".into();
            return;
        }
        let Some(spell) =
            crate::play::PlayLink::parameter_spell_assignment(&choice.words, &parameter)
        else {
            state.message = "Choose a spell that takes a parameter and enter up to 128 UTF-8 bytes without quotes or control characters.".into();
            return;
        };
        spell
    };
    let message = if !same {
        "Spell catalogue changed; choose the spell again.".into()
    } else if let Some(draft) = state.multi.as_mut() {
        add_to_multi(draft, spell)
    } else if play.link.assign_bar_action(choice.slot, spell) {
        format!("Assigned {}", choice.name)
    } else {
        "Spell catalogue changed; choose the spell again.".into()
    };
    state.message = message;
}

#[cfg(test)]
mod tests;
