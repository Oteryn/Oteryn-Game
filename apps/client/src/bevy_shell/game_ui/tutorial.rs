//! Server tutorial steps projected onto the gameplay HUD.
use super::{GameRegions, modal::ModalGuard};
use crate::bevy_shell::{owner::HostOwner, ui::Skins};
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui::tutorial_notices::NoticePanel;
use oteryn_client_ui_bevy as source;
use std::collections::VecDeque;

#[derive(Default)]
struct TutorialSteps {
    generation: u64,
    revision: u64,
    seen: VecDeque<String>,
    cursor: usize,
}
impl TutorialSteps {
    fn sync(&mut self, generation: u64, panel: Option<&NoticePanel>) {
        if self.generation != generation {
            self.generation = generation;
            self.seen.clear();
            self.cursor = 0;
            self.revision = self.revision.saturating_add(1);
        }
        let empty = VecDeque::new();
        let lines = panel
            .filter(|panel| panel.tutorial_selected)
            .map_or(&empty, |panel| &panel.tutorial);
        if &self.seen == lines {
            return;
        }
        // The retained notice log evicts its oldest entry at 32 lines. Preserve
        // the current/dismissed position across that rolling window.
        let overlap = (0..=self.seen.len().min(lines.len()))
            .rev()
            .find(|&count| {
                self.seen
                    .iter()
                    .skip(self.seen.len() - count)
                    .eq(lines.iter().take(count))
            })
            .unwrap_or(0);
        self.cursor = self.cursor.saturating_sub(self.seen.len() - overlap);
        self.seen.clone_from(lines);
        self.revision = self.revision.saturating_add(1);
    }
    fn current(&self) -> Option<&str> {
        self.seen.get(self.cursor).map(String::as_str)
    }
    fn next(&mut self) {
        self.cursor = self.cursor.saturating_add(1).min(self.seen.len());
        self.revision = self.revision.saturating_add(1);
    }
    fn close(&mut self) {
        self.cursor = self.seen.len();
        self.revision = self.revision.saturating_add(1);
    }
    fn key(&self) -> (u64, u64) {
        (self.generation, self.revision)
    }
}
#[derive(Resource, Default)]
struct State {
    steps: TutorialSteps,
    panel: Option<Entity>,
    rendered: Option<(u64, u64)>,
}
#[derive(Component)]
struct Action {
    key: (u64, u64),
    next: bool,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<State>().add_systems(
        Update,
        (sync, render, highlight).chain().after(super::project),
    );
}
fn sync(owner: NonSend<HostOwner>, mut state: ResMut<State>) {
    let panel = owner.play.as_ref().map(|play| play.link.notices_panel());
    state.steps.sync(owner.generation(), panel.as_ref());
}
fn render(mut commands: Commands, skins: Res<Skins>, mut state: ResMut<State>) {
    let key = state.steps.current().map(|_| state.steps.key());
    if state.rendered == key {
        return;
    }
    if let Some(panel) = state.panel.take() {
        commands.entity(panel).despawn();
    }
    state.rendered = key;
    let (Some(key), Some(text)) = (key, state.steps.current()) else {
        return;
    };
    let panel = commands
        .spawn((
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                position_type: PositionType::Absolute,
                top: px(108.),
                left: percent(20.),
                width: px(360.),
                max_width: percent(60.),
                padding: UiRect::all(px(8.)),
                row_gap: px(8.),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            GlobalZIndex(100),
        ))
        .id();
    let text = commands
        .spawn(source::text(
            &format!(
                "Tutorial — Step {} of {}\n\n{text}",
                state.steps.cursor + 1,
                state.steps.seen.len()
            ),
            source::font(skins.font.clone(), false),
        ))
        .id();
    commands.entity(panel).add_child(text);
    let controls = commands
        .spawn(Node {
            column_gap: px(8.),
            ..default()
        })
        .id();
    commands.entity(panel).add_child(controls);
    for (caption, next) in [("Next", true), ("Close", false)] {
        let button = commands
            .spawn((
                source::button(skins.buttons.clone(), 80.),
                Action { key, next },
            ))
            .observe(activate)
            .id();
        let text = commands
            .spawn(source::button_caption(caption, skins.font.clone()))
            .id();
        commands.entity(button).add_child(text);
        commands.entity(controls).add_child(button);
    }
    state.panel = Some(panel);
}
fn activate(
    event: On<Activate>,
    actions: Query<&Action>,
    mut state: ResMut<State>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity) {
        return;
    }
    let Ok(action) = actions.get(event.entity) else {
        return;
    };
    if action.key != state.steps.key() {
        return;
    }
    if action.next {
        state.steps.next();
    } else {
        state.steps.close();
    }
}
fn target(text: &str, regions: &GameRegions) -> Option<Entity> {
    text.to_lowercase()
        .split(|ch: char| !ch.is_alphanumeric())
        .find_map(|word| match word {
            "health" | "mana" | "vitals" => Some(regions.vitals),
            "minimap" => Some(regions.minimap),
            "inventory" | "equipment" => Some(regions.equipment),
            "backpack" | "containers" => Some(regions.containers),
            "battle" => Some(regions.battle),
            "skills" => Some(regions.skills),
            "vip" => Some(regions.vip),
            "analyzers" => Some(regions.analyzers),
            "chat" | "console" => Some(regions.chat),
            "hotbar" | "hotbars" => Some(regions.bars),
            "map" | "viewport" => Some(regions.world),
            _ => None,
        })
}
fn highlight(
    mut commands: Commands,
    state: Res<State>,
    regions: Res<GameRegions>,
    outlines: Query<&Outline>,
    mut previous: Local<Option<(Entity, Option<Outline>)>>,
) {
    let target = state
        .steps
        .current()
        .and_then(|text| target(text, &regions));
    if previous.as_ref().map(|(entity, _)| *entity) == target {
        return;
    }
    if let Some((entity, outline)) = previous.take()
        && let Ok(mut entity) = commands.get_entity(entity)
    {
        if let Some(outline) = outline {
            entity.insert(outline);
        } else {
            entity.remove::<Outline>();
        }
    }
    if let Some(entity) = target {
        *previous = Some((entity, outlines.get(entity).ok().cloned()));
        commands
            .entity(entity)
            .insert(Outline::new(px(3.), px(-3.), Color::srgb(1., 0.85, 0.15)));
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tutorial_highlight_target() {
        let mut app = App::new();
        let ids: Vec<_> = (0..11)
            .map(|_| app.world_mut().spawn(Node::default()).id())
            .collect();
        app.insert_resource(GameRegions {
            vitals: ids[0],
            minimap: ids[1],
            equipment: ids[2],
            containers: ids[3],
            battle: ids[4],
            skills: ids[5],
            vip: ids[6],
            analyzers: ids[7],
            chat: ids[8],
            bars: ids[9],
            world: ids[10],
        });
        app.world_mut()
            .entity_mut(ids[2])
            .insert(Outline::new(px(1.), px(1.), Color::WHITE));
        app.init_resource::<State>();
        app.add_systems(Update, highlight);
        app.world_mut().resource_mut::<State>().steps.sync(
            1,
            Some(&oteryn_client_ui::tutorial_notices::NoticePanel {
                tutorial_selected: true,
                tutorial: [
                    "Open your inventory.".into(),
                    "Read the skills panel.".into(),
                ]
                .into(),
                ..default()
            }),
        );
        app.update();
        assert_eq!(
            app.world()
                .get::<Outline>(ids[2])
                .map(|outline| (outline.width, outline.offset)),
            Some((px(3.), px(-3.)))
        );
        assert!(app.world().get::<Outline>(ids[5]).is_none());
        app.world_mut().resource_mut::<State>().steps.next();
        app.update();
        assert_eq!(
            app.world()
                .get::<Outline>(ids[2])
                .map(|outline| (outline.width, outline.offset)),
            Some((px(1.), px(1.)))
        );
        assert!(app.world().get::<Outline>(ids[5]).is_some());
        app.world_mut().resource_mut::<State>().steps.close();
        app.update();
        assert!(app.world().get::<Outline>(ids[5]).is_none());
    }
}

#[cfg(test)]
mod step_tests {
    use super::*;
    use oteryn_client_runtime::tutorial_notices::{Notice, TutorialNotice, tutorial};
    use oteryn_client_ui::tutorial_notices::NoticePanel;

    #[test]
    fn tutorial_retained_step_flow() {
        let mut panel = NoticePanel {
            tutorial_selected: true,
            ..Default::default()
        };
        panel.apply(tutorial(TutorialNotice::Hint(1)));
        panel.apply(Notice::Tutorial("Open your inventory.".into()));
        let mut steps = TutorialSteps::default();
        steps.sync(1, Some(&panel));
        assert_eq!(steps.current(), panel.tutorial.front().map(String::as_str));
        steps.next();
        assert_eq!(steps.current(), Some("Open your inventory."));
        steps.close();
        steps.sync(1, Some(&panel));
        assert_eq!(steps.current(), None);
        panel.apply(Notice::Tutorial("Read the skills panel.".into()));
        steps.sync(1, Some(&panel));
        assert_eq!(steps.current(), Some("Read the skills panel."));
        steps.next();
        assert_eq!(steps.current(), None);
        steps.sync(2, Some(&panel));
        assert_eq!(steps.current(), panel.tutorial.front().map(String::as_str));
        steps.sync(2, None);
        assert_eq!(steps.current(), None);
        let mut panel = NoticePanel {
            tutorial_selected: true,
            ..Default::default()
        };
        for id in 0..32 {
            panel.apply(Notice::Tutorial(format!("Hint {id}")));
        }
        steps.sync(3, Some(&panel));
        steps.close();
        panel.apply(Notice::Tutorial("New hint after rollover".into()));
        steps.sync(3, Some(&panel));
        assert_eq!(steps.current(), Some("New hint after rollover"));
    }
}

#[cfg(test)]
mod widget_tests {
    use super::*;

    #[test]
    fn tutorial_step_flow() -> Result<(), Box<dyn std::error::Error>> {
        std::thread::Builder::new()
            .stack_size(64 << 20)
            .spawn(|| step_flow().map_err(|error| error.to_string()))?
            .join()
            .map_err(|_| "tutorial test thread failed")??;
        Ok(())
    }

    fn step_flow() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new();
        app.init_resource::<State>();
        app.insert_resource(Skins {
            buttons: source::ButtonImages {
                up: default(),
                down: default(),
            },
            checked: default(),
            unchecked: default(),
            frame: default(),
            inset: default(),
            font: default(),
        });
        let (link, _commands, outbox) = crate::play::play_channel();
        outbox.tutorial_test_selected();
        let mut owner = HostOwner::new(None)?;
        owner.play = Some(crate::bevy_shell::owner::ActivePlay {
            view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
            link,
        });
        app.insert_non_send(owner);
        app.add_systems(Update, (sync, render).chain());
        for (sequence, text) in [(1, "First server hint"), (2, "Second server hint")] {
            outbox.push(vec![oteryn_session::SessionEvent::TutorialNotice(
                oteryn_session::AppliedDelta {
                    server_sequence: sequence,
                    base_revision: sequence - 1,
                    new_revision: sequence,
                    value: oteryn_client_runtime::tutorial_notices::TutorialNotice::StatusText(
                        text.into(),
                    ),
                },
            )]);
        }
        app.update();
        let button = |app: &mut App, next: bool| -> Result<Entity, Box<dyn std::error::Error>> {
            app.world_mut()
                .query::<(Entity, &Action)>()
                .iter(app.world())
                .find_map(|(entity, action)| (action.next == next).then_some(entity))
                .ok_or("tutorial control missing".into())
        };
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0.contains("First server hint"))
        );
        let next = button(&mut app, true)?;
        app.world_mut().trigger(Activate { entity: next });
        app.update();
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0.contains("Second server hint"))
        );
        let close = button(&mut app, false)?;
        app.world_mut().trigger(Activate { entity: close });
        app.update();
        assert_eq!(
            app.world_mut().query::<&Action>().iter(app.world()).count(),
            0
        );
        Ok(())
    }
}
