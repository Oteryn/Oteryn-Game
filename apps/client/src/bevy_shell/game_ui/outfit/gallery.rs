//! Local preview controls for the signed-in character's gallery.
use super::*;
use oteryn_client_runtime::cyclopedia_character::gallery::{Kind as EntryKind, Page};
use oteryn_client_ui::cyclopedia_character::Intent as GalleryIntent;

#[derive(Clone)]
pub(super) enum Action {
    Ownership,
    Title,
    Familiar(u32),
    FamiliarVisibility,
    Page(GalleryIntent),
}

#[derive(Default)]
pub(super) struct Gallery {
    all: bool,
    title_visible: bool,
    title: Option<&'static str>,
    page: Option<Page>,
    familiar: Option<u32>,
}
impl Gallery {
    pub(super) fn attach_title(&self, commands: &mut Commands, skin: &Skins, picture: Entity) {
        if self.title_visible
            && let Some(title) = self.title
        {
            let label = commands
                .spawn(source::button_caption(title, skin.font.clone()))
                .id();
            commands.entity(picture).add_child(label);
        }
    }
    pub(super) fn refresh(&mut self, link: &crate::play::PlayLink) -> bool {
        let title = link
            .character_info_panel()
            .own_titles
            .and_then(|titles| titles.selected)
            .map(|title| title.name());
        let page = link.cyclopedia_character_panel().gallery.page;
        let changed = self.title != title || self.page != page;
        self.title = title;
        self.page = page;
        changed
    }
}

pub(super) fn activate(action: &Action, link: &crate::play::PlayLink, state: &mut State) {
    let gallery = &mut state.gallery;
    match action {
        Action::Ownership => gallery.all = !gallery.all,
        Action::Title => {
            gallery.title_visible = !gallery.title_visible;
            if gallery.title_visible && link.character_info_panel().own_titles.is_none() {
                link.outfit_request_own_title();
            }
        }
        Action::Page(intent) => {
            link.cyclopedia_character_request(*intent);
        }
        Action::Familiar(look) => {
            if !gallery.page.as_ref().is_some_and(|page| {
                page.entries
                    .iter()
                    .any(|e| e.kind == EntryKind::Familiar && e.look_type == *look)
            }) {
                return;
            }
            gallery.familiar = Some(*look);
            state.playback.layers.familiar = Some(*look);
        }
        Action::FamiliarVisibility => {
            state.playback.layers.familiar = if state.playback.layers.familiar.is_some() {
                None
            } else {
                gallery.familiar
            };
        }
    }
    state.dirty = true;
}

pub(super) fn build(
    commands: &mut Commands,
    skin: &Skins,
    root: Entity,
    generation: u64,
    state: &State,
    panel: &Panel,
) {
    let gallery = &state.gallery;
    let mut control = |label: &str, action| {
        button(
            commands,
            skin,
            root,
            generation,
            Kind::Gallery(action),
            label,
        );
    };
    control(
        if gallery.all {
            "Show owned appearances"
        } else {
            "Show all appearances"
        },
        Action::Ownership,
    );
    control(
        if gallery.title_visible {
            "Hide title"
        } else {
            "Show title"
        },
        Action::Title,
    );
    if let Some(page) = &gallery.page {
        if page.offset > 0 {
            control(
                "Previous gallery page",
                Action::Page(GalleryIntent::GalleryPrevious),
            );
        }
        if usize::from(page.offset) + page.entries.len() < usize::from(page.total) {
            control(
                "Next gallery page",
                Action::Page(GalleryIntent::GalleryNext),
            );
        }
        for entry in page
            .entries
            .iter()
            .filter(|entry| entry.kind == EntryKind::Familiar)
        {
            control(
                &format!("Preview {}", entry.key),
                Action::Familiar(entry.look_type),
            );
        }
    } else {
        control("Browse familiars", Action::Page(GalleryIntent::Appearances));
    }
    if gallery.familiar.is_some() {
        control(
            if state.playback.layers.familiar.is_some() {
                "Hide familiar"
            } else {
                "Show familiar"
            },
            Action::FamiliarVisibility,
        );
    }
    let mut label = |value: String| {
        let entity = commands
            .spawn(source::button_caption(value, skin.font.clone()))
            .id();
        commands.entity(root).add_child(entity);
    };
    if gallery.all
        && let Some(catalogue) = &panel.catalogue
    {
        if let Some(metadata) = state
            .metadata
            .as_ref()
            .filter(|m| m.digest == catalogue.context.cosmetics_digest)
        {
            let locked_outfits = metadata
                .outfits
                .iter()
                .filter(|key| !catalogue.outfits.iter().any(|owned| owned.key == ***key));
            let locked_mounts = metadata
                .mounts
                .keys()
                .filter(|key| !catalogue.mounts.contains(key));
            for key in locked_outfits
                .chain(locked_mounts)
                .filter(|key| search::matches(key, &state.search))
            {
                label(format!("{key} (not owned)"));
            }
        } else {
            label("Full appearance catalogue unavailable".into());
        }
    }
}

impl State {
    pub(super) fn reset_gallery(&mut self) {
        self.gallery = Default::default();
        self.preset = None;
        self.preset_riding = None;
        self.playback = Default::default();
        self.rendered = None;
    }
}
