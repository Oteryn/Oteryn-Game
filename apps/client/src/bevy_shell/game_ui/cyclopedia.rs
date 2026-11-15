//! Character Cyclopedia over the retained capability-199 owner and confirmed rows.
pub(super) mod artwork;
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
use oteryn_client_ui::cyclopedia_character::{CyclopediaCharacterPanel, Intent};
const WINDOW: &str = "0199-cyclopedia-character-open";
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Tab {
    #[default]
    General,
    Character(CharacterTab),
    Items,
    Familiars,
    Badges,
}
impl Tab {
    pub(super) const ALL: [Self; 13] = [
        Self::General,
        Self::Character(CharacterTab::Offence),
        Self::Character(CharacterTab::Defence),
        Self::Character(CharacterTab::Misc),
        Self::Character(CharacterTab::RecentDeaths),
        Self::Character(CharacterTab::RecentPvpKills),
        Self::Character(CharacterTab::Achievements),
        Self::Items,
        Self::Character(CharacterTab::Appearances),
        Self::Familiars,
        Self::Character(CharacterTab::StoreSummary),
        Self::Character(CharacterTab::Inspection),
        Self::Badges,
    ];
    pub(super) fn intent(self) -> Intent {
        match self {
            Self::General => Intent::SelectTab(CharacterTab::GeneralStats),
            Self::Character(CharacterTab::Appearances) | Self::Familiars => Intent::Appearances,
            Self::Character(CharacterTab::Inspection) => Intent::ProfileInspection,
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
            Self::Character(CharacterTab::Appearances) => "All appearances",
            Self::Character(CharacterTab::StoreSummary) => "Store summary",
            Self::Character(CharacterTab::Inspection) => "Character profile",
            Self::Items => "Item summary",
            Self::Familiars => "Familiars",
            Self::Badges => "Badges",
        }
    }
    fn can_page(self, next: bool, panel: &CyclopediaCharacterPanel) -> bool {
        if matches!(
            self,
            Self::Familiars | Self::Character(CharacterTab::Appearances)
        ) {
            return panel.gallery.page.as_ref().is_some_and(|page| {
                if next {
                    usize::from(page.offset) + page.entries.len() < usize::from(page.total)
                } else {
                    page.offset > 0
                }
            });
        }
        if self == Self::Items {
            return panel.items.page.as_ref().is_some_and(|page| {
                if next {
                    usize::from(page.offset) + page.rows.len() < usize::from(page.total)
                } else {
                    page.offset > 0
                }
            });
        }
        self.page(next).is_some()
    }
    fn page(self, next: bool) -> Option<Intent> {
        Some(match self {
            Self::Items => {
                if next {
                    Intent::ItemsNext
                } else {
                    Intent::ItemsPrevious
                }
            }
            Self::Character(CharacterTab::Appearances) | Self::Familiars => {
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
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Encyclopedia,
    HoldingsFilter,
    HoldingsLocation(Option<oteryn_client_runtime::cyclopedia_character::items::Location>),
    Tab(Tab),
    Refresh,
    Previous,
    Next,
    Close,
}
#[derive(Component)]
pub(super) struct HoldingsSearch;
#[derive(Resource, Default)]
struct State {
    generation: u64,
    serial: u64,
    tab: Tab,
    snapshot: Option<CyclopediaCharacterPanel>,
    /// Shown tab's request that the retained builder refused; resent once it accepts.
    unsent: Option<Intent>,
    pending_filter: Option<(Control, Entity)>,
    widgets: Option<widgets::Widgets>,
}
pub(super) fn install(app: &mut App) {
    artwork::install(app);
    app.init_resource::<State>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .add_systems(Update, (keyboard, project).chain())
        .add_systems(
            PostUpdate,
            finish_filter.after(bevy::text::EditableTextSystems),
        );
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
    if control.kind == Kind::Encyclopedia {
        play.link.window_toggle(WINDOW, &[]);
        play.link.item_catalogue_open();
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
        Kind::Close | Kind::Encyclopedia | Kind::HoldingsFilter | Kind::HoldingsLocation(_) => {
            return;
        }
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
        if matches!(
            control.kind,
            Kind::HoldingsFilter | Kind::HoldingsLocation(_)
        ) {
            state.pending_filter = Some((*control, event.entity));
        } else {
            perform(&owner, &mut state, *control);
        }
    }
}
fn finish_filter(
    guard: ModalGuard,
    search: Query<(Entity, &bevy::text::EditableText), With<HoldingsSearch>>,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    let Some((control, source)) = state.pending_filter.take() else {
        return;
    };
    let Some(play) = &owner.play else {
        return;
    };
    let panel = play.link.cyclopedia_character_panel();
    if control.generation != owner.generation()
        || control.serial != state.serial
        || owner.modal_active()
        || !panel.open
        || panel.pending
        || !panel.holdings_selected
        || state.snapshot.as_ref() != Some(&panel)
        || !guard.allows(source)
    {
        return;
    }
    let Ok((field, search)) = search.single() else {
        return;
    };
    if !guard.allows(field) {
        return;
    }
    if search.pending_paste.is_some()
        || search
            .pending_edits
            .iter()
            .any(bevy::text::TextEdit::is_destructive)
    {
        state.pending_filter = Some((control, source));
        return;
    }
    let location = if let Kind::HoldingsLocation(location) = control.kind {
        location
    } else {
        panel.items.location
    };
    if play
        .link
        .cyclopedia_holdings_filter(&search.value().to_string(), location)
    {
        state.snapshot = None;
    }
}
fn keyboard(
    mut events: MessageReader<bevy::input::keyboard::KeyboardInput>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
    focus: Res<InputFocus>,
    search: Query<Entity, With<HoldingsSearch>>,
) {
    let Some(panel) = state.widgets.as_ref().map(|widgets| widgets.panel) else {
        events.clear();
        return;
    };
    if !guard.allows(panel) {
        events.clear();
        return;
    }
    for event in events.read() {
        if event.state != bevy::input::ButtonState::Pressed || event.repeat {
            continue;
        }
        if event.key_code == KeyCode::Escape {
            let control = Control {
                generation: state.generation,
                serial: state.serial,
                kind: Kind::Close,
            };
            perform(&owner, &mut state, control);
        } else if matches!(event.key_code, KeyCode::Enter | KeyCode::NumpadEnter)
            && state.tab == Tab::Items
            && focus.get().is_some_and(|field| search.contains(field))
        {
            let control = Control {
                generation: state.generation,
                serial: state.serial,
                kind: Kind::HoldingsFilter,
            };
            state.pending_filter = Some((control, focus.get().unwrap_or(panel)));
        }
    }
}
