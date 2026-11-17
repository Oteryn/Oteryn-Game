//! C12 scenario: live snapshot -> visible target -> menu -> session task -> HUD receipt.
use oteryn_client::combat_input::{CombatCommand, CombatReport};
use oteryn_client::input::click_tile;
use oteryn_client::play::{PlayView, Stepper, play_channel, run_session};
use oteryn_client_ui::world_interaction::MouseActions;
use oteryn_client_ui::world_interaction::menu::Menu;
use oteryn_input_actions::*;
use oteryn_protocol_oteryn::world_map::*;
use oteryn_renderer::TileCoord;
use oteryn_session::{
    ActorPosition, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::time::Duration;
use tokio::sync::mpsc;

// The public menu's action type is inferred, keeping the harness on its existing UI/session edges.
struct ContextSession {
    map: WorldMapViewSnapshot,
    receipts: mpsc::UnboundedSender<String>,
}
impl Stepper for ContextSession {
    fn feature_selected(&self, capability: u32) -> bool {
        matches!(capability, 11 | 28)
    }
    fn feature_snapshot(&self, domain: u32) -> Option<Vec<u8>> {
        (domain == 17)
            .then(|| encode_world_map_snapshot(&self.map).ok())
            .flatten()
    }
    async fn world_context(
        &mut self,
        command: &oteryn_client_ui::world_interaction::menu::Command,
    ) -> Result<String, SessionError> {
        self.receipts
            .send(format!("{:?}", command.action))
            .map_err(|_| SessionError::SessionUnusable)?;
        Ok(match command.action {
            oteryn_client_ui::world_interaction::menu::Action::Look => "You see a bed.",
            oteryn_client_ui::world_interaction::menu::Action::Use => "Use: completed",
            oteryn_client_ui::world_interaction::menu::Action::Open => "Open: completed",
            _ => "Bed: this bed is occupied",
        }
        .into())
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
    fn drain(&mut self) -> Vec<SessionEvent> {
        Vec::new()
    }
}
#[test]
fn native_world_context_right_click_actions_and_feedback()
-> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let position = ActorPosition {
                        x: 101,
                        y: 200,
                        floor: 0,
                    };
                    let map = WorldMapViewSnapshot {
                        header: MapViewHeader {
                            content_generation: [1; 32],
                            bundle_digest: [2; 32],
                            reset_epoch: 1,
                            origin: position,
                            first_visible_floor: 0,
                        },
                        tiles: vec![MapTile {
                            position,
                            items: vec![MapItem {
                                definition: MapDefinition::Item(std::num::NonZeroU32::MIN),
                                count: 1,
                                sub_type: 0,
                                origin: MapOrigin::BaseOrdinal {
                                    ordinal: 0,
                                    object_revision: 0,
                                },
                                appearance_id: 0,
                            }],
                            more: false,
                            ground_speed: 0,
                        }],
                    };
                    let mut view = PlayView::new(TileCoord::new(100, 200), 0, &[])?;
                    let payload = encode_world_map_snapshot(&map).map_err(|e| format!("{e:?}"))?;
                    // Admission uses this same validated snapshot before handing the session to the task.
                    view.set_live_map(
                        &decode_world_map_snapshot(&payload).map_err(|e| format!("{e:?}"))?,
                    )?;
                    let (link, commands, outbox) = play_channel();
                    let (receipts, mut received) = mpsc::unbounded_channel();
                    let task = tokio::spawn(run_session(
                        ContextSession { map, receipts },
                        commands,
                        outbox,
                    ));
                    tokio::time::timeout(Duration::from_secs(5), async {
                        loop {
                            view.tick(&link).map_err(|e| format!("{e:?}"))?;
                            if view
                                .world_context(TileCoord::new(101, 200), &link)
                                .is_some()
                            {
                                break;
                            }
                            tokio::task::yield_now().await;
                        }
                        Ok::<_, String>(())
                    })
                    .await??;
                    // Normalize a real secondary press at the center of the tile, through production routing.
                    let mut pointer = MouseActions::new()?;
                    let tile = TileCoord::new(101, 200);
                    let viewport = view.scene().view();
                    let origin = viewport.origin();
                    let pixels = i32::try_from(viewport.tile_px())?;
                    let x = (tile.x - origin.x) * pixels + pixels / 2;
                    let y = (tile.y - origin.y) * pixels + pixels / 2;
                    let clicks = pointer.route(&[
                        NormalizedInputEvent::PointerMoved {
                            position: PointerPosition::new(
                                PointerCoordinate::new(x)?,
                                PointerCoordinate::new(y)?,
                            ),
                            motion: PointerMotion::new(
                                PointerDelta::new(0)?,
                                PointerDelta::new(0)?,
                            ),
                        },
                        NormalizedInputEvent::MouseButton {
                            button: MouseButton::SECONDARY,
                            state: ButtonState::Pressed,
                            modifiers: Modifiers::NONE,
                        },
                    ]);
                    assert_eq!(clicks.len(), 1);
                    assert!(clicks[0].secondary);
                    assert_eq!(click_tile(viewport, clicks[0].x, clicks[0].y), Some(tile));
                    let menu: Menu = view.world_context(tile, &link).ok_or("menu missing")?;
                    for (label, action) in &menu.entries {
                        assert!(view.world_context_choose(&link, &menu, *action), "{label}");
                        assert_eq!(
                            tokio::time::timeout(Duration::from_secs(5), received.recv())
                                .await?
                                .ok_or("receipt missing")?,
                            format!("{action:?}")
                        );
                        view.tick(&link).map_err(|e| format!("{e:?}"))?;
                        let expected = match action {
                            oteryn_client_ui::world_interaction::menu::Action::Look => {
                                "You see a bed."
                            }
                            oteryn_client_ui::world_interaction::menu::Action::Use => {
                                "Use: completed"
                            }
                            oteryn_client_ui::world_interaction::menu::Action::Open => {
                                "Open: completed"
                            }
                            _ => "Bed: this bed is occupied",
                        };
                        assert!(view.hud_status().contains(expected));
                    }
                    drop(link);
                    task.abort();
                    Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
                })
        })?
        .join()
        .map_err(|_| "context scenario thread failed")?
}
