//! Offline guide browsing; reading progress is local to the current session.
use super::*;
use oteryn_client_ui_bevy as source;

const CATEGORIES: [&str; 5] = [
    "Player Guide",
    "Major Updates",
    "Client Features",
    "Useful Info",
    "Support",
];
const ARTICLES: &[(usize, &str, &str)] = &[
    (
        1,
        "Daily rewards",
        "After the Winter Update, collecting at a reward shrine grants the day's rewards in one step. Open Daily Rewards to inspect each day's free and Premium rewards and your recent collections.",
    ),
    (
        2,
        "Outfit editor",
        "Choose Outfit in the control buttons to change your appearance. Select a look, choose its available addons and colours, then press OK to apply it.",
    ),
    (
        3,
        "Event calendar",
        "Open News on the login screen and choose Calendar. Previous month and Next month browse the events published by the operator.",
    ),
    (
        4,
        "Support tickets",
        "Open the support tickets page to contact the operator about an account or game issue. For an in-game bug report, use Ctrl+Z and describe how to reproduce the issue.",
    ),
    (
        1,
        "Weapon Proficiency",
        "The Summer Update 2025 introduced weapon proficiency. Choose Weapon Proficiency from the control buttons to inspect progress with your weapons.",
    ),
    (
        2,
        "Battle List",
        "The Battle List shows nearby creatures. Use its filters to narrow the list and select a creature to attack.",
    ),
    (
        3,
        "The Store",
        "Open Store in the sidebar to see the current catalogue and prices. Select a product to inspect its offer before confirming a purchase.",
    ),
    (
        4,
        "Bug Reporting",
        "Press Ctrl+Z to report a bug. Include the action you took, what happened, and what you expected to happen.",
    ),
];

const TOPICS: [(&str, &str); 5] = [
    ("Notices", ""),
    (
        "Movement",
        "Use arrow keys or click a reachable map tile to move. Right-click a creature or object to see available actions.",
    ),
    (
        "Combat",
        "Select a creature in Battle to attack. Choose attack, balanced or defensive stance and stand or chase. Monitor health, mana and conditions, and keep healing supplies ready.",
    ),
    (
        "Inventory",
        "Select an equipment or container item to inspect its actions. Drag items to move them; choose an amount for stacks. Inventory changes require server confirmation.",
    ),
    (
        "Quests and help",
        "Open Journal with Ctrl+J to browse quests and missions. Use Spellbook to browse spells. Open Options to configure the client. Ctrl+Z opens a bug report; include steps to reproduce the problem.",
    ),
];

#[derive(Resource, Default)]
pub(super) struct State {
    generation: u64,
    selected: usize,
    read: u8,
    category: usize,
    article: Option<usize>,
}
impl State {
    pub(super) fn session(&mut self, generation: u64) {
        if self.generation != generation {
            *self = Self {
                generation,
                ..default()
            };
        }
    }
    fn select(&mut self, topic: usize) {
        if topic < TOPICS.len() {
            self.selected = topic;
            if topic > 0 {
                self.read |= 1 << topic;
            }
        }
    }
    pub(super) fn text(&self, panel: &oteryn_client_ui::tutorial_notices::NoticePanel) -> String {
        if self.category > 0 {
            return if let Some(index) = self.article {
                let (_, title, body) = ARTICLES[index];
                format!("{}\n\n{title}\n\n{body}", CATEGORIES[self.category])
            } else {
                format!("{}\n\nChoose an article.", CATEGORIES[self.category])
            };
        }
        let (title, body) = TOPICS[self.selected];
        let content = if self.selected == 0 {
            oteryn_client_ui::tutorial_notices::panel_text(panel)
        } else {
            format!("{title}\n\n{body}")
        };
        format!(
            "Guide — {} / 4 topics read this session\n\n{content}",
            self.read.count_ones()
        )
    }
}
#[derive(Component)]
pub(super) struct Topic(usize);
#[derive(Component, Clone, Copy)]
pub(super) enum Browse {
    Category(usize),
    Article(usize),
    SupportLink,
}

fn browse(
    event: On<Activate>,
    controls: Query<&Browse>,
    guard: ModalGuard,
    mut owner: NonSendMut<HostOwner>,
    mut state: ResMut<State>,
) {
    if !guard.allows(event.entity)
        || !owner
            .play
            .as_ref()
            .is_some_and(|play| !play.link.server_modal_active() && play.link.notices_panel().open)
    {
        return;
    }
    state.session(owner.generation());
    match controls.get(event.entity) {
        Ok(Browse::Category(category)) => {
            state.category = *category;
            state.article = None;
        }
        Ok(Browse::Article(index)) if ARTICLES[*index].0 == state.category => {
            state.article = Some(*index)
        }
        Ok(Browse::SupportLink)
            if state
                .article
                .is_some_and(|index| ARTICLES[index].1 == "Support tickets") =>
        {
            owner.open_web_link(crate::web_links::WebLink::SupportTickets)
        }
        _ => return,
    }
    state.set_changed();
}

type IndexFilter = Or<(With<Browse>, With<Topic>)>;
pub(super) fn project_index(
    state: Res<State>,
    mut controls: Query<(&mut Node, Option<&Browse>, Has<Topic>), IndexFilter>,
) {
    for (mut node, control, topic) in &mut controls {
        let visible = match control {
            Some(Browse::Article(id)) => ARTICLES[*id].0 == state.category,
            Some(Browse::SupportLink) => state
                .article
                .is_some_and(|index| ARTICLES[index].1 == "Support tickets"),
            _ => !topic || state.category == 0,
        };
        let display = if visible {
            Display::Flex
        } else {
            Display::None
        };
        if node.display != display {
            node.display = display;
        }
    }
}

pub(super) fn setup(commands: &mut Commands, skins: &Skins, parent: Entity) {
    let controls = CATEGORIES
        .iter()
        .enumerate()
        .map(|(index, title)| (*title, Browse::Category(index)))
        .chain(
            ARTICLES
                .iter()
                .enumerate()
                .map(|(index, (_, title, _))| (*title, Browse::Article(index))),
        )
        .chain([("Open support tickets", Browse::SupportLink)]);
    for (title, control) in controls {
        let button = commands
            .spawn((source::button(skins.buttons.clone(), 180.), control))
            .observe(browse)
            .id();
        let caption = commands
            .spawn(source::button_caption(title, skins.font.clone()))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(parent).add_child(button);
    }
    for (index, (title, _)) in TOPICS.iter().enumerate() {
        let button = commands
            .spawn((source::button(skins.buttons.clone(), 180.), Topic(index)))
            .observe(select)
            .id();
        let caption = commands
            .spawn(source::button_caption(*title, skins.font.clone()))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(parent).add_child(button);
    }
}
fn select(
    event: On<Activate>,
    topics: Query<&Topic>,
    guard: ModalGuard,
    owner: NonSend<HostOwner>,
    mut state: ResMut<State>,
) {
    if guard.allows(event.entity)
        && let Some(play) = &owner.play
        && !play.link.server_modal_active()
        && play.link.notices_panel().open
        && let Ok(topic) = topics.get(event.entity)
    {
        state.session(owner.generation());
        if state.category == 0 {
            state.select(topic.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn guide_app() -> Result<App, Box<dyn std::error::Error>> {
        use crate::play::{PlayView, play_channel};
        let (link, _, _) = play_channel();
        link.notices_set_open(true);
        let mut host = HostOwner::new(None)?;
        host.play = Some(crate::bevy_shell::owner::ActivePlay {
            view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
            link,
        });
        let mut app = App::new();
        app.insert_non_send(host)
            .insert_resource(Skins {
                buttons: source::ButtonImages {
                    up: Handle::default(),
                    down: Handle::default(),
                },
                checked: Handle::default(),
                unchecked: Handle::default(),
                frame: Handle::default(),
                inset: Handle::default(),
                font: Handle::default(),
            })
            .init_resource::<State>()
            .add_systems(Startup, super::super::notices::setup)
            .add_systems(Update, project_index);
        app.update();
        for mut node in app
            .world_mut()
            .query_filtered::<&mut Node, With<super::super::notices::NoticeOverlay>>()
            .iter_mut(app.world_mut())
        {
            node.display = Display::Flex;
        }
        Ok(app)
    }

    #[test]
    fn guide_categories_open_articles_and_the_support_destination()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut app = guide_app()?;
        for (category, article) in [
            ("Major Updates", "Daily rewards"),
            ("Client Features", "Outfit editor"),
            ("Useful Info", "Event calendar"),
            ("Support", "Support tickets"),
        ] {
            for caption in [category, article] {
                let entity = app
                    .world_mut()
                    .query::<(&Text, &ChildOf)>()
                    .iter(app.world())
                    .find_map(|(text, parent)| (text.0 == caption).then_some(parent.parent()))
                    .ok_or_else(|| format!("missing guide button {caption}"))?;
                app.world_mut().trigger(Activate { entity });
                app.update();
            }
            assert!(
                app.world()
                    .resource::<State>()
                    .text(&Default::default())
                    .contains(article)
            );
        }
        let link = app
            .world_mut()
            .query::<(&Text, &ChildOf)>()
            .iter(app.world())
            .find_map(|(text, parent)| {
                (text.0 == "Open support tickets").then_some(parent.parent())
            })
            .ok_or("support link")?;
        app.world_mut().trigger(Activate { entity: link });
        assert!(
            app.world()
                .non_send::<HostOwner>()
                .login
                .form
                .message()
                .contains("connection settings")
        );
        let folder = tempfile::tempdir()?;
        let origin = "https://support.example.test";
        let mut config = crate::reentry::Reentry::device_test_config(folder.path().into(), origin)?;
        let (sent, received) = std::sync::mpsc::channel();
        config.open_browser = std::sync::Arc::new(move |uri| sent.send(uri.to_owned()).is_ok());
        assert!(
            app.world_mut()
                .non_send_mut::<HostOwner>()
                .configure_test_web_link(config)
        );
        app.world_mut().trigger(Activate { entity: link });
        let destination = received.recv_timeout(std::time::Duration::from_secs(3))?;
        assert!(
            destination.starts_with(origin) && destination.ends_with("/support/tickets"),
            "{destination}"
        );
        Ok(())
    }
    #[test]
    fn compendium_buttons_browse_real_guide_and_preserve_notices()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut app = guide_app()?;
        let button = app
            .world_mut()
            .query::<(Entity, &Topic)>()
            .iter(app.world())
            .find_map(|(entity, topic)| (topic.0 == 2).then_some(entity))
            .ok_or("missing guide topic button")?;
        app.world_mut().trigger(Activate { entity: button });
        let state = app.world().resource::<State>();
        assert_eq!(state.selected, 2);
        assert_eq!(state.read.count_ones(), 1);
        assert!(
            state
                .text(&Default::default())
                .contains("balanced or defensive")
        );
        Ok(())
    }
    #[test]
    fn guide_browses_topics_tracks_unique_reads_and_resets_on_session_change() {
        let mut state = State::default();
        state.session(1);
        state.select(2);
        state.select(2);
        assert_eq!(state.read.count_ones(), 1);
        assert!(
            state
                .text(&Default::default())
                .contains("balanced or defensive")
        );
        state.select(3);
        assert_eq!(state.read.count_ones(), 2);
        state.session(1);
        assert_eq!(state.selected, 3);
        state.session(2);
        assert_eq!(state.read, 0);
        assert_eq!(state.selected, 0);
    }
}
