//! Shared subject selector for the live Cyclopedia windows.
use super::{feature_dialogs::houses::HouseWindow, modal::ModalGuard};
use crate::bevy_shell::{
    owner::{HostOwner, Phase},
    ui::{Control, Skins, toolbar::open_toolbar},
};
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui_bevy as source;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Subject {
    Bestiary,
    Bosstiary,
    Character,
    Houses,
    Items,
    Map,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Destination {
    Open(Subject),
    Back,
}
#[derive(Component)]
pub(super) struct SubjectButton {
    generation: u64,
    pub(super) from: Subject,
    pub(super) to: Destination,
}
#[derive(Resource, Default)]
pub(super) struct History {
    generation: u64,
    current: Option<Subject>,
    previous: Vec<Subject>,
}
impl History {
    pub(super) fn visit(&mut self, generation: u64, from: Subject, to: Subject) {
        if self.current != Some(from) || self.generation != generation {
            self.previous.clear();
        }
        self.generation = generation;
        self.current = Some(to);
        if self.previous.len() == 64 {
            self.previous.remove(0);
        }
        self.previous.push(from);
    }
}
pub(super) fn bar(
    commands: &mut Commands,
    skin: &Skins,
    parent: Entity,
    generation: u64,
    from: Subject,
) {
    let row = commands
        .spawn((
            ChildOf(parent),
            Node {
                flex_wrap: FlexWrap::Wrap,
                ..default()
            },
        ))
        .id();
    for (to, caption) in [
        (Destination::Back, "Back"),
        (Destination::Open(Subject::Bestiary), "Bestiary"),
        (Destination::Open(Subject::Bosstiary), "Bosstiary"),
        (Destination::Open(Subject::Character), "Character"),
        (Destination::Open(Subject::Houses), "Houses"),
        (Destination::Open(Subject::Items), "Items"),
        (Destination::Open(Subject::Map), "Map"),
    ] {
        if to == Destination::Open(from) {
            continue;
        }
        let button = commands
            .spawn((
                ChildOf(row),
                source::button(skin.buttons.clone(), 84.),
                SubjectButton {
                    generation,
                    from,
                    to,
                },
            ))
            .observe(activate)
            .id();
        commands.spawn((
            ChildOf(button),
            source::button_caption(caption, skin.font.clone()),
        ));
    }
}
fn activate(
    event: On<Activate>,
    buttons: Query<&SubjectButton>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
    history: Option<ResMut<History>>,
    mut houses: Option<ResMut<HouseWindow>>,
) {
    let (Ok(button), Some(mut history)) = (buttons.get(event.entity), history) else {
        return;
    };
    let Some(play) = &owner.play else { return };
    if owner.phase != Phase::Playing
        || owner.generation() != button.generation
        || owner.modal_active()
        || play.view.gameplay_input_blocked(&play.link)
        || !guard.allows(event.entity)
    {
        return;
    }
    if !button.from.is_open(&owner, houses.as_deref()) {
        return;
    }
    let to = match button.to {
        Destination::Open(to) => to,
        Destination::Back
            if history.generation == button.generation && history.current == Some(button.from) =>
        {
            let Some(to) = history.previous.last() else {
                return;
            };
            *to
        }
        Destination::Back => return,
    };
    if to == Subject::Houses && houses.is_none() {
        return;
    }
    match button.from {
        Subject::Character => {
            play.link
                .window_toggle("0199-cyclopedia-character-open", &[]);
        }
        Subject::Items => play.link.item_catalogue_set_open(false),
        Subject::Bestiary => play.link.bestiary_close(),
        Subject::Map => play.link.cyclopedia_atlas_close(),
        Subject::Bosstiary => {
            open_toolbar(&mut owner, Control::Panel("bosstiary"));
        }
        Subject::Houses => {
            if let Some(h) = houses.as_mut() {
                h.navigate(false, button.generation)
            }
        }
    }
    let Some(play) = &owner.play else { return };
    match to {
        Subject::Items => play.link.item_catalogue_open(),
        Subject::Character => {
            if !play.link.cyclopedia_character_panel().open {
                open_toolbar(&mut owner, Control::Shortcut("cyclopedia"));
            }
        }
        Subject::Bestiary => play.link.charm_progression_open(),
        Subject::Map => play.link.cyclopedia_atlas_open(),
        Subject::Houses => {
            if let Some(h) = houses.as_mut() {
                h.navigate(true, button.generation)
            }
        }
        Subject::Bosstiary => {
            if !owner.settings.preferences.panel_visible("bosstiary") {
                open_toolbar(&mut owner, Control::Panel("bosstiary"));
            }
        }
    }
    if button.to == Destination::Back {
        history.previous.pop();
        history.current = Some(to);
    } else {
        history.visit(button.generation, button.from, to);
    }
}

impl Subject {
    fn is_open(self, owner: &HostOwner, houses: Option<&HouseWindow>) -> bool {
        let Some(play) = &owner.play else {
            return false;
        };
        match self {
            Subject::Character => play.link.cyclopedia_character_panel().open,
            Subject::Items => play.link.inspection_panel().catalogue_open,
            Subject::Bestiary => play.link.charm_progression_panel().open,
            Subject::Map => play.link.cyclopedia_atlas_panel().is_some(),
            Subject::Bosstiary => owner.settings.preferences.panel_visible("bosstiary"),
            Subject::Houses => houses
                .as_ref()
                .is_some_and(|h| h.is_open(owner.generation())),
        }
    }
}
pub(super) fn update_history(
    owner: NonSend<HostOwner>,
    houses: Option<Res<HouseWindow>>,
    mut history: ResMut<History>,
    mut buttons: Query<(&SubjectButton, &mut Node)>,
) {
    if history.generation != owner.generation()
        || history
            .current
            .is_some_and(|s| !s.is_open(&owner, houses.as_deref()))
    {
        *history = History::default();
    }
    for (button, mut node) in &mut buttons {
        if button.to == Destination::Back {
            let display = if history.current == Some(button.from) && !history.previous.is_empty() {
                Display::Flex
            } else {
                Display::None
            };
            if node.display != display {
                node.display = display;
            }
        }
    }
}
