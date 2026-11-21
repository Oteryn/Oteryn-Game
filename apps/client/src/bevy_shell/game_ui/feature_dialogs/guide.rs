//! Offline guide browsing; reading progress is local to the current session.
use super::*;
use oteryn_client_ui_bevy as source;

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
struct Topic(usize);
pub(super) fn setup(commands: &mut Commands, skins: &Skins, parent: Entity) {
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
        state.select(topic.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compendium_buttons_browse_real_guide_and_preserve_notices()
    -> Result<(), Box<dyn std::error::Error>> {
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
            .add_systems(Startup, super::super::notices::setup);
        app.update();
        for mut node in app
            .world_mut()
            .query_filtered::<&mut Node, With<super::super::notices::NoticeOverlay>>()
            .iter_mut(app.world_mut())
        {
            node.display = Display::Flex;
        }
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
