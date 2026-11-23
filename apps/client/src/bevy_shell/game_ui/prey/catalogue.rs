//! The wildcard picker searches the shipped species list. The server validates selection.
use super::*;
use bevy::ui_widgets::Button;

pub(super) struct Species {
    pub key: &'static str,
    name: &'static str,
    pub object: bool,
    pub look: u32,
}
const SPECIES: &[Species] = include!(concat!(env!("OUT_DIR"), "/prey_species.rs"));
pub(super) fn species(key: &str) -> Option<&'static Species> {
    SPECIES
        .binary_search_by_key(&key, |s| s.key)
        .ok()
        .map(|i| &SPECIES[i])
}
#[derive(Component)]
pub(super) struct Results {
    editor: Entity,
    query: Option<String>,
    page: usize,
}
impl Results {
    pub fn new(editor: Entity) -> Self {
        Self {
            editor,
            query: None,
            page: 0,
        }
    }
}
#[derive(Component)]
enum Choice {
    Page(usize),
    Creature(&'static str),
}
const PAGE: usize = 8;
pub(super) fn project(
    mut commands: Commands,
    skin: Res<Skins>,
    fields: Query<&EditableText>,
    mut results: Query<(Entity, &mut Results)>,
) {
    for (root, mut list) in &mut results {
        let Ok(field) = fields.get(list.editor) else {
            continue;
        };
        let query = field.value().to_string().trim().to_lowercase();
        if list.query.as_ref() == Some(&query) && !list.is_changed() {
            continue;
        }
        if list.query.as_ref() != Some(&query) {
            list.page = 0;
            list.query = Some(query.clone());
        }
        commands.entity(root).despawn_children();
        let mut matches = SPECIES
            .iter()
            .filter(|s| s.name.to_lowercase().contains(&query) || s.key.contains(&query))
            .skip(list.page * PAGE)
            .peekable();
        for species in matches.by_ref().take(PAGE) {
            let opt = commands
                .spawn((
                    Button,
                    Choice::Creature(species.key),
                    Name::new(format!("prey.catalogue.{}", species.key)),
                    source::text(species.name, source::font(skin.font.clone(), false)),
                ))
                .insert(Pickable::default())
                .observe(choose)
                .id();
            commands.entity(root).add_child(opt);
        }
        for (caption, page) in [
            ("Previous creatures", list.page.checked_sub(1)),
            ("More creatures", matches.peek().map(|_| list.page + 1)),
        ] {
            if let Some(page) = page {
                let opt = commands
                    .spawn((
                        Button,
                        Choice::Page(page),
                        source::text(caption, source::font(skin.font.clone(), false)),
                    ))
                    .insert(Pickable::default())
                    .observe(choose)
                    .id();
                commands.entity(root).add_child(opt);
            }
        }
    }
}
fn choose(
    event: On<Activate>,
    guard: ModalGuard,
    controls: Query<(&Choice, &ChildOf)>,
    mut results: Query<&mut Results>,
    mut fields: Query<&mut EditableText>,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok((choice, parent)) = controls.get(event.entity) else {
        return;
    };
    let Ok(mut list) = results.get_mut(parent.parent()) else {
        return;
    };
    match choice {
        Choice::Page(page) => list.page = *page,
        Choice::Creature(key) => {
            if let Ok(mut field) = fields.get_mut(list.editor) {
                *field = EditableText::new(*key);
                field.max_characters = Some(256);
            }
        }
    }
}
