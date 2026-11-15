use super::super::*;
use crate::{
    combat_input::{CombatCommand, CombatReport},
    play::Stepper,
};
use bevy::ui::Pressed;
use oteryn_client_runtime::{
    cyclopedia_character::{StoreSummary, TabView},
    raw_feature::FeatureCommand,
};
use oteryn_session::{
    GatedCommandResult, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::time::Duration;
type TestResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
struct Feed(tokio::sync::mpsc::UnboundedReceiver<Vec<String>>);
impl Stepper for Feed {
    fn feature_selected(&self, capability: u32) -> bool {
        capability == 199
    }
    async fn feature(
        &mut self,
        command: &FeatureCommand,
    ) -> Result<GatedCommandResult, SessionError> {
        if command.capability != 199 || command.kind != 388 || command.payload != [6, 0, 0, 20] {
            return Err(SessionError::CapabilityNotSelected {
                capability: command.capability,
            });
        }
        let blessings = self.0.recv().await.ok_or(SessionError::SessionUnusable)?;
        let payload = TabView::StoreSummary(StoreSummary {
            blessings,
            permanent_prey_slots: 0,
            prey_wildcards: 0,
            hirelings: 0,
        })
        .encode()
        .map_err(|_| SessionError::SessionUnusable)?;
        Ok(GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload,
        })
    }
    fn drain(&mut self) -> Vec<SessionEvent> {
        Vec::new()
    }
    async fn serve(&mut self, _duration: Duration) -> Result<(), SessionError> {
        tokio::task::yield_now().await;
        Ok(())
    }
    async fn step(&mut self, _direction: StepDirection) -> Result<StepOutcome, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn combat(&mut self, _command: CombatCommand) -> Result<CombatReport, SessionError> {
        Err(SessionError::SessionUnusable)
    }
    async fn logout(&mut self) -> Result<Option<LogoutResult>, SessionError> {
        Ok(None)
    }
}
fn run(check: impl FnOnce(&mut App) -> TestResult + Send + 'static) -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> TestResult {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?;
            let (mut app, commands, outbox) =
                super::super::tests::app().map_err(|e| e.to_string())?;
            runtime.block_on(async {
                let (send, receive) = tokio::sync::mpsc::unbounded_channel();
                let task = tokio::spawn(crate::play::run_session(Feed(receive), commands, outbox));
                tokio::time::timeout(Duration::from_secs(5), async {
                    while !app
                        .world()
                        .non_send::<HostOwner>()
                        .play
                        .as_ref()
                        .is_some_and(|p| p.link.cyclopedia_character_panel().selected)
                    {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                for keys in [
                    vec!["oteryn:blessing.the_spiritual_shielding".to_string()],
                    Vec::new(),
                ] {
                    send.send(keys.clone())?;
                    app.world()
                        .non_send::<HostOwner>()
                        .play
                        .as_ref()
                        .ok_or("play")?
                        .link
                        .blessings_request();
                    tokio::time::timeout(Duration::from_secs(5), async {
                        while !app
                            .world()
                            .non_send::<HostOwner>()
                            .play
                            .as_ref()
                            .is_some_and(|p| {
                                p.link
                                    .blessings_panel()
                                    .snapshot
                                    .as_ref()
                                    .is_some_and(|s| s.blessings == keys)
                            })
                        {
                            tokio::task::yield_now().await;
                        }
                    })
                    .await?;
                    app.update();
                    check_state(&mut app, !keys.is_empty())?;
                }
                check(&mut app)?;
                task.abort();
                Ok(())
            })
        })?
        .join()
        .map_err(|_| "blessings test thread failed")?
}
fn check_state(app: &mut App, active: bool) -> TestResult {
    let world = app.world_mut();
    let mut query = world.query::<(&Name, &Text, &TextColor)>();
    let (_, text, color) = query
        .iter(world)
        .find(|(name, _, _)| name.as_str() == "oteryn:blessing.the_spiritual_shielding")
        .ok_or("blessing icon missing")?;
    assert_eq!(text.0, if active { "+ SS" } else { "- SS" });
    assert_eq!(
        color.0,
        if active {
            Color::srgb_u8(120, 220, 120)
        } else {
            source::DISABLED_TEXT_COLOR
        }
    );
    let (_, inactive, _) = query
        .iter(world)
        .find(|(name, _, _)| name.as_str() == "oteryn:blessing.twist_of_fate")
        .ok_or("inactive blessing icon missing")?;
    assert_eq!(inactive.0, "- TF");
    let (_, tooltip, _) = query
        .iter(world)
        .find(|(name, _, _)| name.as_str() == "Blessings tooltip")
        .ok_or("tooltip missing")?;
    assert_eq!(
        tooltip.0,
        if active {
            "Active blessings:\nThe Spiritual Shielding"
        } else {
            "You are not protected by any blessing."
        }
    );
    Ok(())
}
#[test]
fn ankh_opens_ownership_and_penalty_details() -> TestResult {
    run(|app| {
        let button = app
            .world_mut()
            .query::<(Entity, &super::ToggleDetails)>()
            .iter(app.world())
            .next()
            .map(|(entity, _)| entity)
            .ok_or("ankh button")?;
        app.world_mut().entity_mut(button).insert(Pressed);
        app.update();
        let (text, node) = app
            .world_mut()
            .query_filtered::<(&Text, &Node), With<super::Details>>()
            .single(app.world())?;
        assert_eq!(node.display, Display::Flex);
        assert!(text.0.contains("The Spiritual Shielding: missing"));
        assert!(text.0.contains("red or black skulls"));
        assert!(text.0.contains("Exact next-death losses"));
        app.world_mut().entity_mut(button).remove::<Pressed>();
        app.update();
        app.world_mut().entity_mut(button).insert(Pressed);
        app.update();
        assert_eq!(
            app.world_mut()
                .query_filtered::<&Node, With<super::Details>>()
                .single(app.world())?
                .display,
            Display::None
        );
        Ok(())
    })
}
#[test]
fn blessing_icons_state() -> TestResult {
    run(|_| Ok(()))
}
#[test]
fn blessing_tooltip_none() -> TestResult {
    run(|app| {
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .settings
            .preferences
            .tooltip_delay_ms = 0;
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .settings
            .preferences
            .tooltips_enabled = true;
        let mut window = Window::default();
        window.set_cursor_position(Some(Vec2::new(200., 100.)));
        app.world_mut().spawn((window, bevy::window::PrimaryWindow));
        let panel = app
            .world_mut()
            .query::<(Entity, &Name)>()
            .iter(app.world())
            .find_map(|(entity, name)| (name.as_str() == "Blessings status").then_some(entity))
            .ok_or("blessings panel")?;
        app.world_mut()
            .entity_mut(panel)
            .insert(bevy::picking::hover::Hovered(true));
        app.update();
        let visible = app
            .world_mut()
            .query::<(&Name, &Node)>()
            .iter(app.world())
            .any(|(name, node)| {
                name.as_str() == "Blessings tooltip" && node.display == Display::Flex
            });
        assert!(visible, "hover must show the actual tooltip");
        assert!(
            app.world_mut()
                .query::<(&Name, &Text)>()
                .iter(app.world())
                .any(|(name, text)| name.as_str() == "Blessings tooltip"
                    && text.0 == "You are not protected by any blessing.")
        );
        Ok(())
    })
}
