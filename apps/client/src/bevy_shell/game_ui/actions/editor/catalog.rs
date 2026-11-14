//! A bounded search/picker over the current server catalogue, retaining display identity.
use super::*;
use crate::bevy_shell::game_ui::modal::ModalGuard;

const PAGE: usize = 12;
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

pub(super) fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<EditorState>,
    roots: Query<Entity, With<CatalogRoot>>,
    fields: Query<(&Field, &EditableText)>,
) {
    let Some((slot, generation)) = state.selected else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
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
    state.message = if same
        && play.link.assign_bar_action(
            choice.slot,
            BarAction::Spell {
                name: choice.name.clone(),
                words: choice.words.clone(),
                aim: crate::play::SpellAim::Target,
            },
        ) {
        format!("Assigned {}", choice.name)
    } else {
        "Spell catalogue changed; choose the spell again.".into()
    };
}

#[cfg(test)]
mod tests;
