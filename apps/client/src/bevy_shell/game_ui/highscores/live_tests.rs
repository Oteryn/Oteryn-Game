//! Drive native Activate observers through the real retained session owner.
use super::*;
use crate::{
    combat_input::{CombatCommand, CombatReport},
    play::{Stepper, play_channel, run_session},
};
use oteryn_client_runtime::{
    highscores::wire,
    server_mechanic::{ServerMechanicCommand, ServerMechanicResponse},
};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
type Result<T = ()> = std::result::Result<T, String>;
struct Peer(Arc<Mutex<Vec<wire::HighscoresQuery>>>);
impl Stepper for Peer {
    fn drain(&mut self) -> Vec<SessionEvent> {
        Vec::new()
    }
    async fn step(&mut self, _: StepDirection) -> std::result::Result<StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn combat(
        &mut self,
        _: CombatCommand,
    ) -> std::result::Result<CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn logout(&mut self) -> std::result::Result<Option<LogoutResult>, SessionError> {
        Ok(None)
    }
    async fn serve(&mut self, duration: Duration) -> std::result::Result<(), SessionError> {
        tokio::time::sleep(duration).await;
        Ok(())
    }
    fn server_mechanic_selected(&self, capability: u32) -> bool {
        capability == 9
    }
    async fn server_mechanic(
        &mut self,
        command: &ServerMechanicCommand,
    ) -> std::result::Result<ServerMechanicResponse, SessionError> {
        let query = wire::decode_highscores_query(&command.payload)
            .map_err(|_| SessionError::SessionUnusable)?;
        self.0
            .lock()
            .map_err(|_| SessionError::SessionUnusable)?
            .push(query);
        let own = wire::HighscoreRow {
            rank: 51,
            name: "Own Hero".into(),
            vocation: "knight".into(),
            level: 100,
            value: 900,
        };
        let rows = (0..50)
            .map(|index| wire::HighscoreRow {
                rank: u64::from((query.page - 1) * 50 + index + 1),
                ..own.clone()
            })
            .collect();
        let reply = wire::HighscoresResult {
            world: query.world.or(wire::WorldId::decode(&[
                1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 1,
            ])
            .ok()),
            own_page: Some(2),
            category: query.category,
            snapshot_id: 7,
            computed_at: 60000,
            page: query.page,
            has_more: query.page < 3,
            rows,
            own_row: Some(own),
        };
        Ok(ServerMechanicResponse {
            command_id: 1,
            server_sequence: 2,
            status: CommandStatus::Accepted,
            payload: wire::encode_highscores_result(&reply)
                .map_err(|_| SessionError::SessionUnusable)?,
        })
    }
}
fn settle(app: &mut App) -> Result {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app.update();
        let state = app.world().resource::<State>();
        let owner = app.world().non_send::<HostOwner>();
        let panel = owner.play.as_ref().ok_or("play")?.link.highscores_panel();
        if !panel.pending
            && panel.reply.is_some()
            && state.seek.is_none()
            && state.seen.as_ref() == Some(&panel)
        {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err("highscores UI deadline".into());
        }
        std::thread::yield_now();
    }
}
fn click(app: &mut App, kind: Kind) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(entity, action)| (action.kind == kind).then_some(entity))
        .ok_or("button")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    settle(app)
}
#[test]
fn highscores_bevy_buttons_request_filters_and_bounded_pages() -> Result {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(scenario)
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "highscores UI scenario panicked".to_string())?
}
fn scenario() -> Result {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let queries = Arc::new(Mutex::new(Vec::new()));
    let (link, commands, outbox) = play_channel();
    let runtime = oteryn_client_runtime::ClientRuntime::new().map_err(|e| e.to_string())?;
    runtime
        .spawn(run_session(Peer(queries.clone()), commands, outbox))
        .map_err(|e| e.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(5);
    while !link.highscores_panel().selected {
        if Instant::now() >= deadline {
            return Err("highscores initialization deadline".into());
        }
        std::thread::yield_now();
    }
    let mut owner = HostOwner::new(None).map_err(|e| e.to_string())?;
    let (bridge, selector) = crate::bevy_shell::selection::SelectionBridge::new();
    owner.selection = bridge;
    let picker = std::thread::spawn(move || {
        selector(
            &[crate::character_selection::CharacterRow {
                character: oteryn_platform_client::native_login::OwnerCharacter {
                    character_id: "hero".into(),
                    world_id: "01020304-0506-7008-800a-0b0c0d0e0f02".into(),
                    name: "Own Hero".into(),
                    availability:
                        oteryn_platform_client::native_login::CharacterAvailability::Available,
                },
                world_name: "Named World".into(),
                channels: vec![],
                available: true,
            }],
            "",
            oteryn_foundation::CancellationToken::new(),
        )
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    while owner.selection.pending().is_none() {
        owner.selection.poll();
        if Instant::now() >= deadline {
            return Err("selection deadline".into());
        }
        std::thread::yield_now();
    }
    let id = owner.selection.pending().ok_or("selection")?.id;
    assert!(owner.selection.respond(
        id,
        crate::character_selection::SelectionAction::Play {
            character_id: "hero".into(),
            channel_id: None,
        }
    ));
    picker.join().map_err(|_| "picker panicked")?;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])
            .map_err(|e| e.to_string())?,
        link,
    });
    owner
        .settings
        .preferences
        .rebind("0009-highscores-open", "ALT+H")
        .map_err(|e| e.to_string())?;
    owner.route_input(
        &[NormalizedInputEvent::Key {
            code: KeyCode::new(11).map_err(|e| e.to_string())?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::one(Modifier::Alt),
            repeat: false,
        }],
        false,
    );
    let mut app = App::new();
    app.insert_non_send(owner)
        .init_resource::<State>()
        .init_resource::<InputFocus>()
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
        .add_systems(Startup, setup)
        .add_systems(Update, project)
        .add_systems(PostUpdate, finish);
    settle(&mut app)?;
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "World: Current world")
    );
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.contains("World type unavailable"))
    );
    for kind in [Kind::Category, Kind::Vocation] {
        click(&mut app, kind)?;
    }
    let world =
        wire::WorldId::decode(&[1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 2])
            .map_err(|e| format!("{e:?}"))?;
    let named = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(entity, action)| (action.kind == Kind::World(Some(world))).then_some(entity))
        .ok_or("named world button")?;
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "World: Named World")
    );
    app.world_mut().trigger(Activate { entity: named });
    app.update();
    settle(&mut app)?;
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "Named World")
    );
    let field = app
        .world_mut()
        .query_filtered::<Entity, With<WorldField>>()
        .single(app.world())
        .map_err(|e| e.to_string())?;
    app.world_mut().entity_mut(field).insert(EditableText::new(
        oteryn_client_ui::highscores::world_text(world),
    ));
    click(&mut app, Kind::WorldInput)?;
    for kind in [
        Kind::Next,
        Kind::Previous,
        Kind::Last,
        Kind::First,
        Kind::Own,
    ] {
        click(&mut app, kind)?;
    }
    let queries = queries.lock().map_err(|e| e.to_string())?;
    assert_eq!(
        queries.iter().map(|q| q.page).collect::<Vec<_>>(),
        [1, 1, 1, 1, 1, 2, 1, 2, 3, 2, 1, 2]
    );
    assert_eq!(queries[1].category, 2);
    assert_eq!(queries[2].vocation, 1);
    assert_eq!(queries[3].world, Some(world));
    assert_eq!(queries[4].world, Some(world));
    assert!(
        queries[3..5]
            .iter()
            .all(|q| q.page == 1 && q.snapshot_id == 0)
    );
    assert!(
        queries
            .iter()
            .skip(5)
            .all(|q| q.snapshot_id == 7 && q.world == Some(world))
    );
    drop(queries);
    // Closing removes the visible window and its controls.
    let close = app
        .world_mut()
        .query::<(Entity, &Action)>()
        .iter(app.world())
        .find_map(|(e, a)| (a.kind == Kind::Close).then_some(e))
        .ok_or("close")?;
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    app.update();
    assert!(app.world().resource::<State>().frame.is_none());
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    assert_eq!(owner.selection.known_worlds().len(), 1);
    owner.close();
    assert!(owner.selection.known_worlds().is_empty());
    Ok(())
}
