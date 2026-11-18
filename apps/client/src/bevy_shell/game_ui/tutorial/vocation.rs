//! The tutorial's five cards submit only a current server offer.
use super::super::{GameModal, modal::ModalGuard};
use crate::bevy_shell::{owner::HostOwner, ui::Skins};
use bevy::{prelude::*, ui_widgets::Activate};
use oteryn_client_ui::tutorial_vocation::{VocationPanel, choices};
use oteryn_client_ui_bevy as source;
use std::time::Duration;

#[derive(Default, Resource)]
struct State {
    generation: u64,
    last_poll: Option<Duration>,
    prompted: bool,
    panel: Option<Entity>,
    rendered: Option<VocationPanel>,
}
#[derive(Component)]
struct Choice {
    generation: u64,
    id: u8,
}

pub(super) fn install(app: &mut App) {
    app.init_resource::<State>()
        .add_systems(Update, project.after(super::super::project));
}
fn project(
    mut commands: Commands,
    owner: NonSend<HostOwner>,
    skins: Res<Skins>,
    time: Res<Time<Real>>,
    mut state: ResMut<State>,
) {
    if state.generation != owner.generation() {
        if let Some(panel) = state.panel.take() {
            commands.entity(panel).despawn();
        }
        *state = State {
            generation: owner.generation(),
            ..Default::default()
        };
    }
    let Some(play) = owner.play.as_ref() else {
        if let Some(panel) = state.panel.take() {
            commands.entity(panel).despawn();
        }
        state.rendered = None;
        return;
    };
    let panel = play.link.tutorial_vocation_panel();
    let prompted = play
        .link
        .notices_panel()
        .tutorial
        .back()
        .is_some_and(|line| line == "Tutorial: choose your vocation to begin your adventure.");
    if prompted && !state.prompted {
        state.last_poll = None;
    }
    state.prompted = prompted;
    let refresh =
        panel.observation.is_none() || (prompted && !panel.observation.is_some_and(|o| o.offered));
    if panel.selected
        && refresh
        && !panel.pending
        && !panel.observation.is_some_and(|o| o.chosen != 0)
        && state
            .last_poll
            .is_none_or(|last| time.elapsed().saturating_sub(last) >= Duration::from_secs(2))
    {
        play.link.tutorial_vocation_request(None);
        state.last_poll = Some(time.elapsed());
    }
    if state.rendered.as_ref() == Some(&panel) {
        return;
    }
    if let Some(panel) = state.panel.take() {
        commands.entity(panel).despawn();
    }
    let offered = choices(&panel);
    state.rendered = Some(panel.clone());
    if offered.is_empty() && panel.feedback.is_none() {
        return;
    }
    let root = commands
        .spawn((
            Name::new("Choose your vocation"),
            source::frame(skins.frame.clone(), [4., 17., 4., 4.]),
            Node {
                position_type: PositionType::Absolute,
                top: percent(20.),
                left: percent(20.),
                width: px(540.),
                max_width: percent(70.),
                padding: UiRect::all(px(12.)),
                row_gap: px(8.),
                flex_direction: FlexDirection::Column,
                ..default()
            },
            GlobalZIndex(150),
        ))
        .id();
    if !offered.is_empty() {
        commands.entity(root).insert(GameModal);
    }
    let title = commands
        .spawn(source::text(
            "Choose your vocation",
            source::font(skins.font.clone(), false),
        ))
        .id();
    commands.entity(root).add_child(title);
    for (id, title) in offered {
        let button = commands
            .spawn((
                source::button(skins.buttons.clone(), 220.),
                Choice {
                    generation: owner.generation(),
                    id,
                },
            ))
            .observe(activate)
            .id();
        let caption = commands
            .spawn(source::button_caption(
                &format!("Select {title}"),
                skins.font.clone(),
            ))
            .id();
        commands.entity(button).add_child(caption);
        commands.entity(root).add_child(button);
    }
    if let Some(feedback) = panel.feedback {
        let feedback = commands
            .spawn(source::text(
                feedback,
                source::font(skins.font.clone(), false),
            ))
            .id();
        commands.entity(root).add_child(feedback);
    }
    if panel.pending {
        let waiting = commands
            .spawn(source::text(
                "Waiting for the server",
                source::font(skins.font.clone(), false),
            ))
            .id();
        commands.entity(root).add_child(waiting);
    }
    state.panel = Some(root);
}
fn activate(
    event: On<Activate>,
    buttons: Query<&Choice>,
    owner: NonSend<HostOwner>,
    guard: ModalGuard,
) {
    if !guard.allows(event.entity) {
        return;
    }
    if let Ok(choice) = buttons.get(event.entity)
        && choice.generation == owner.generation()
        && let Some(play) = &owner.play
    {
        play.link.tutorial_vocation_request(Some(choice.id));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn offered_choice_survives_poll_interval() -> Result<(), Box<dyn std::error::Error>> {
        let mut owner = HostOwner::new(None)?;
        let (link, mut rx, outbox) = crate::play::play_channel();
        outbox.bevy_tutorial_vocation_initialize();
        owner.play = Some(crate::bevy_shell::owner::ActivePlay {
            link,
            view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        });
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
            .init_asset::<Image>()
            .init_asset::<Font>()
            .insert_non_send(owner)
            .add_systems(Startup, crate::bevy_shell::ui::setup);
        install(&mut app);
        app.update();
        rx.try_recv()?;
        outbox.bevy_tutorial_vocation_result(&oteryn_session::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload: include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../crates/client-ui/src/features/0504-tutorial-vocation/fixtures/offer.bin"
            ))
            .to_vec(),
        });
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_secs(3),
        ));
        app.update();
        let entity = app
            .world_mut()
            .query::<(Entity, &Choice)>()
            .iter(app.world())
            .find(|(_, c)| c.id == 4)
            .ok_or("choice")?
            .0;
        app.world_mut().trigger(Activate { entity });
        let crate::combat_input::PlayCommand::Feature(command) = rx.try_recv()? else {
            return Err("vocation command".into());
        };
        assert_eq!(command.payload, [4]);
        Ok(())
    }
}
