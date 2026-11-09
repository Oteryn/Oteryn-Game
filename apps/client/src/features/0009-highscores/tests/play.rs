use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{Stepper, play_channel, run_session};
use oteryn_client_runtime::{
    ClientRuntime,
    highscores::wire,
    server_mechanic::{ServerMechanicCommand, ServerMechanicResponse},
};
use oteryn_client_ui::highscores::{self, Action};
use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::time::{Duration, Instant};

struct HighscoresPeer;
impl Stepper for HighscoresPeer {
    fn drain(&mut self) -> Vec<SessionEvent> {
        vec![]
    }
    async fn step(&mut self, _: StepDirection) -> Result<StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn combat(&mut self, _: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(None)
    }
    async fn serve(&mut self, duration: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(duration).await;
        Ok(())
    }
    fn server_mechanic_selected(&self, capability: u32) -> bool {
        capability == 9
    }
    async fn server_mechanic(
        &mut self,
        command: &ServerMechanicCommand,
    ) -> Result<ServerMechanicResponse, SessionError> {
        assert_eq!((command.capability, command.command_type), (9, 14));
        let query = wire::decode_highscores_query(&command.payload)
            .map_err(|_| SessionError::SessionUnusable)?;
        let result = wire::HighscoresResult {
            world: query.details.then_some(
                query.world.unwrap_or(
                    wire::WorldId::decode(&[
                        1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 1,
                    ])
                    .map_err(|_| SessionError::SessionUnusable)?,
                ),
            ),
            own_page: (query.world.is_none() && query.details).then_some(2),
            category: query.category,
            snapshot_id: 4,
            computed_at: 123,
            page: query.page,
            has_more: false,
            rows: vec![wire::HighscoreRow {
                rank: if query.world.is_none() && query.page == 2 {
                    1500
                } else {
                    7
                },
                name: if query.world.is_none() && query.page == 2 {
                    "Me"
                } else {
                    "Ada Knight"
                }
                .into(),
                vocation: "knight".into(),
                level: 90,
                value: if query.world.is_none() && query.page == 2 {
                    100
                } else {
                    123456
                },
            }],
            own_row: query.world.is_none().then_some(wire::HighscoreRow {
                rank: 1500,
                name: "Me".into(),
                vocation: "knight".into(),
                level: 90,
                value: 100,
            }),
        };
        Ok(ServerMechanicResponse {
            command_id: 1,
            server_sequence: 2,
            status: CommandStatus::Accepted,
            payload: wire::encode_highscores_result(&result)
                .map_err(|_| SessionError::SessionUnusable)?,
        })
    }
}

#[test]
fn shell_input_queries_live_highscores_and_projects_reply() -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(scenario)
        .map_err(|e| e.to_string())?
        .join()
        .map_err(|_| "highscores scenario panicked".to_string())?
}
fn scenario() -> Result<(), String> {
    scenario_category(2)
}
fn scenario_category(category: u32) -> Result<(), String> {
    let (link, commands, outbox) = play_channel();
    let runtime = ClientRuntime::new().map_err(|e| e.to_string())?;
    runtime
        .spawn(run_session(HighscoresPeer, commands, outbox))
        .map_err(|e| e.to_string())?;
    let wait = |done: &dyn Fn() -> bool| -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(5);
        while !done() {
            if Instant::now() >= deadline {
                return Err("highscores scenario deadline".into());
            }
            std::thread::yield_now();
        }
        Ok(())
    };
    wait(&|| link.highscores_panel().selected)?;
    let event = NormalizedInputEvent::Key {
        code: KeyCode::new(11).map_err(|e| e.to_string())?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control),
        repeat: false,
    };
    assert!(link.highscores_input(&[event]));
    wait(&|| link.highscores_panel().reply.is_some())?;
    assert!(link.highscores_panel().open);
    let text = highscores::panel_text(&link.highscores_panel());
    assert!(text.contains("Ada Knight") && text.contains("123456"));
    link.highscores_request(Action::Filter {
        category,
        vocation: 1,
    });
    wait(&|| {
        link.highscores_panel()
            .reply
            .as_ref()
            .is_some_and(|r| r.category == category)
    })?;
    link.highscores_request(Action::ShowMe);
    wait(&|| {
        link.highscores_panel()
            .reply
            .as_ref()
            .is_some_and(|r| r.page == 2)
    })?;
    assert_eq!(
        link.highscores_panel()
            .reply
            .as_ref()
            .map(|r| r.snapshot_id),
        Some(4)
    );
    assert!(link.highscores_panel().reply.as_ref().is_some_and(|reply| {
        reply
            .rows
            .iter()
            .any(|row| row.name == "Me" && row.rank == 1500)
    }));
    let other = highscores::parse_world("5a020304-0506-7008-800a-0b0c0d0e0f5a")
        .flatten()
        .ok_or("foreign world")?;
    link.highscores_request(Action::World(Some(other)));
    wait(&|| {
        link.highscores_panel()
            .reply
            .as_ref()
            .is_some_and(|r| r.world == Some(other))
    })?;
    assert_eq!(
        link.highscores_panel().reply.as_ref().map(|r| r.page),
        Some(1)
    );
    assert!(
        link.highscores_panel()
            .reply
            .as_ref()
            .is_some_and(|r| r.own_row.is_none() && r.own_page.is_none())
    );
    link.highscores_request(Action::World(None));
    wait(&|| link.highscores_panel().world.is_none() && !link.highscores_panel().pending)?;
    // docs/reference/tibia-manual/combat.md:262: native filters use the existing highscore wire for both Drome lists.
    for (category, label) in [
        (14, "Tibiadrome lifetime"),
        (17, "Tibiadrome current rotation"),
    ] {
        link.highscores_request(Action::Filter {
            category,
            vocation: 0,
        });
        wait(&|| {
            link.highscores_panel()
                .reply
                .as_ref()
                .is_some_and(|r| r.category == category)
        })?;
        let text = highscores::panel_text(&link.highscores_panel());
        assert!(text.contains(label) && text.contains("Ada Knight") && text.contains("123456"));
    }
    if category == 13 {
        let text = highscores::panel_text(&link.highscores_panel());
        assert!(text.contains("Tibiadrome levels") && text.contains("123456"));
    }
    drop(link);
    runtime.shutdown(Duration::from_millis(250));
    Ok(())
}

#[test]
fn srx3_drome_lifetime_highscores_native_filter_crosses_session() -> Result<(), String> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| scenario_category(13))
        .map_err(|error| error.to_string())?
        .join()
        .map_err(|_| "Drome lifetime native Highscores thread panicked".to_owned())?
}
