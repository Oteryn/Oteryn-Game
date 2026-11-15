//! Character Cyclopedia over the retained capability-199 owner and confirmed rows.
mod rows;
mod widgets;
use super::{
    super::{
        owner::{HostOwner, Phase},
        ui::Skins,
    },
    GameModal,
    modal::ModalGuard,
};
use bevy::{input_focus::InputFocus, prelude::*, ui_widgets::Activate};
use oteryn_client_runtime::cyclopedia_character::CharacterTab;
use oteryn_client_ui::cyclopedia_character::{AugmentIntent, CyclopediaCharacterPanel, Intent};
const WINDOW: &str = "0199-cyclopedia-character-open";
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Tab {
    #[default]
    General,
    Character(CharacterTab),
    Items,
    Badges,
}
impl Tab {
    pub(super) const ALL: [Self; 12] = [
        Self::General,
        Self::Character(CharacterTab::Offence),
        Self::Character(CharacterTab::Defence),
        Self::Character(CharacterTab::Misc),
        Self::Character(CharacterTab::RecentDeaths),
        Self::Character(CharacterTab::RecentPvpKills),
        Self::Character(CharacterTab::Achievements),
        Self::Items,
        Self::Character(CharacterTab::Appearances),
        Self::Character(CharacterTab::StoreSummary),
        Self::Character(CharacterTab::Inspection),
        Self::Badges,
    ];
    pub(super) fn intent(self) -> Intent {
        match self {
            Self::General => Intent::SelectTab(CharacterTab::GeneralStats),
            Self::Character(CharacterTab::Appearances) => Intent::Appearances,
            Self::Character(CharacterTab::Inspection) => Intent::ProfileInspection,
            Self::Character(CharacterTab::Offence) => Intent::SpellAugments(AugmentIntent::Refresh),
            Self::Character(tab) => Intent::SelectTab(tab),
            Self::Items => Intent::Items,
            Self::Badges => Intent::ProfileBadges,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::General | Self::Character(CharacterTab::GeneralStats) => "General stats",
            Self::Character(CharacterTab::Offence) => "Offence",
            Self::Character(CharacterTab::Defence) => "Defence",
            Self::Character(CharacterTab::Misc) => "Miscellaneous",
            Self::Character(CharacterTab::RecentDeaths) => "Recent deaths",
            Self::Character(CharacterTab::RecentPvpKills) => "Recent PvP kills",
            Self::Character(CharacterTab::Achievements) => "Achievements",
            Self::Character(CharacterTab::Appearances) => "Outfits / mounts",
            Self::Character(CharacterTab::StoreSummary) => "Store summary",
            Self::Character(CharacterTab::Inspection) => "Inspection",
            Self::Items => "Item summary",
            Self::Badges => "Badges",
        }
    }
    fn page(self, next: bool) -> Option<Intent> {
        Some(match self {
            Self::Character(CharacterTab::Offence) => Intent::SpellAugments(if next {
                AugmentIntent::Next
            } else {
                AugmentIntent::Previous
            }),
            Self::Items => {
                if next {
                    Intent::ItemsNext
                } else {
                    Intent::ItemsPrevious
                }
            }
            Self::Character(CharacterTab::Appearances) => {
                if next {
                    Intent::GalleryNext
                } else {
                    Intent::GalleryPrevious
                }
            }
            Self::Character(CharacterTab::RecentDeaths | CharacterTab::RecentPvpKills) => {
                if next {
                    Intent::Next
                } else {
                    Intent::Previous
                }
            }
            _ => return None,
        })
    }
}
#[derive(Clone, Copy, Component)]
pub(super) struct Control {
    generation: u64,
    serial: u64,
    pub(super) kind: Kind,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    Tab(Tab),
    AugmentSources(u8),
    Refresh,
    Previous,
    Next,
    Close,
}
#[derive(Resource, Default)]
struct State {
    generation: u64,
    serial: u64,
    tab: Tab,
    snapshot: Option<CyclopediaCharacterPanel>,
    /// Shown tab's request that the retained builder refused; resent once it accepts.
    unsent: Option<Intent>,
    widgets: Option<widgets::Widgets>,
}
pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain());
}
fn inside(mut entity: Option<Entity>, ancestor: Entity, parents: &Query<&ChildOf>) -> bool {
    while let Some(current) = entity {
        if current == ancestor {
            return true;
        }
        entity = parents.get(current).ok().map(ChildOf::parent);
    }
    false
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    mut state: ResMut<State>,
    mut focus: ResMut<InputFocus>,
    parents: Query<&ChildOf>,
) {
    let panel = owner
        .play
        .as_ref()
        .filter(|_| owner.phase == Phase::Playing)
        .map(|play| play.link.cyclopedia_character_panel())
        .filter(|p| p.open);
    let changed_session = state.generation != owner.generation();
    if panel.is_none() || changed_session {
        if let Some(widgets) = state.widgets.take() {
            if inside(focus.get(), widgets.overlay, &parents) {
                focus.clear();
            }
            commands.entity(widgets.overlay).despawn();
        }
        state.snapshot = None;
        state.unsent = None;
        state.generation = owner.generation();
        state.tab = Tab::default();
    }
    let Some(mut panel) = panel else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    if state.widgets.is_none() {
        state.unsent = Some(state.tab.intent());
    }
    if let Some(intent) = state.unsent
        && panel.selected
        && !panel.pending
        && !panel.blessings.pending
    {
        if play.link.cyclopedia_character_request(intent) {
            state.unsent = None;
        }
        panel = play.link.cyclopedia_character_panel();
    }
    if !panel.pending && state.tab == Tab::Character(CharacterTab::Misc) && panel.misc_resources_due
    {
        play.link
            .cyclopedia_character_request(Intent::MiscResources);
        panel = play.link.cyclopedia_character_panel();
    }
    if !panel.pending
        && state.tab == Tab::Character(CharacterTab::Achievements)
        && panel.achievements.catalog.collecting
        && panel.achievements.has_next()
        && panel.achievements.error.is_none()
    {
        play.link.cyclopedia_character_request(Intent::Next);
        panel = play.link.cyclopedia_character_panel();
    }
    if state.snapshot.as_ref() == Some(&panel) {
        return;
    }
    let Some(serial) = state.serial.checked_add(1) else {
        return;
    };
    state.serial = serial;
    let tab = state.tab;
    let control = Control {
        generation: state.generation,
        serial,
        kind: Kind::Close,
    };
    let widgets = state
        .widgets
        .get_or_insert_with(|| widgets::build(&mut commands, &skins));

    let retained = super::refresh::preserve(&mut commands, widgets.panel, focus.get());
    if let Some(content) = widgets.content.take() {
        if inside(focus.get(), content, &parents) {
            focus.clear();
        }
        commands.entity(content).despawn();
    }
    widgets.content = Some(widgets::render(
        &mut commands,
        &skins,
        widgets.panel,
        control,
        tab,
        &panel,
    ));

    retained.restore(&mut commands, widgets.panel);
    state.snapshot = Some(panel);
}
fn perform(owner: &HostOwner, state: &mut State, control: Control) {
    if control.generation != owner.generation()
        || control.serial != state.serial
        || owner.modal_active()
    {
        return;
    }
    let Some(play) = &owner.play else {
        return;
    };
    let panel = play.link.cyclopedia_character_panel();
    if !panel.open || state.snapshot.as_ref() != Some(&panel) {
        return;
    }
    if control.kind == Kind::Close {
        play.link.window_toggle(WINDOW, &[]);
        return;
    }
    if play.view.gameplay_input_blocked(&play.link) || panel.pending || !panel.selected {
        return;
    }
    let intent = match control.kind {
        Kind::Tab(tab) => {
            state.tab = tab;
            tab.intent()
        }
        Kind::AugmentSources(sources) => Intent::SpellAugments(AugmentIntent::Sources(sources)),
        Kind::Refresh => state.tab.intent(),
        Kind::Previous => {
            let Some(intent) = state.tab.page(false) else {
                return;
            };
            intent
        }
        Kind::Next => {
            let Some(intent) = state.tab.page(true) else {
                return;
            };
            intent
        }
        Kind::Close => return,
    };
    state.unsent = (!play.link.cyclopedia_character_request(intent)).then_some(intent);
    state.snapshot = None;
}
fn activate(
    event: On<Activate>,
    controls: Query<&Control>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if guard.allows(event.entity)
        && let Ok(control) = controls.get(event.entity)
    {
        perform(&owner, &mut state, *control);
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let Some(widgets) = &state.widgets else {
        events.clear();
        return;
    };
    if !guard.allows(widgets.panel) {
        events.clear();
        return;
    }
    if events.read().any(|e| {
        e.key_code == KeyCode::Escape && e.state == bevy::input::ButtonState::Pressed && !e.repeat
    }) {
        let control = Control {
            generation: state.generation,
            serial: state.serial,
            kind: Kind::Close,
        };
        perform(&owner, &mut state, control);
    }
}
