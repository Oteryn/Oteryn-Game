use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::player_trade::{
    EntityRef, PlayerTradeCommand, PlayerTradeResponse, player_trade,
};
use oteryn_client_ui::player_trade::{Intent, panel_text};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::{
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};
type Error = Box<dyn std::error::Error + Send + Sync>;
struct Peer {
    state: player_trade::State,
    commands: Arc<Mutex<Vec<player_trade::Intent>>>,
}
impl Stepper for Peer {
    fn player_trade_selected(&self) -> bool {
        true
    }
    fn player_trade_state(&self) -> Option<player_trade::State> {
        Some(self.state.clone())
    }
    async fn player_trade(
        &mut self,
        request: &PlayerTradeCommand,
    ) -> Result<PlayerTradeResponse, SessionError> {
        let intent = player_trade::decode_intent(&request.payload)
            .map_err(|_| SessionError::SessionUnusable)?;
        self.commands
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(intent);
        self.state.look = if matches!(
            intent,
            player_trade::Intent::Look {
                counter_offer: true,
                ..
            }
        ) {
            "a steel sword".into()
        } else {
            String::new()
        };
        let encoded =
            player_trade::encode_state(&self.state).map_err(|_| SessionError::SessionUnusable)?;
        Ok(PlayerTradeResponse {
            status: CommandStatus::Accepted,
            result: player_trade::decode_result(&player_trade::ResultCode::Accepted.payload())
                .map_err(|_| SessionError::SessionUnusable)?,
            state: player_trade::decode_state(&encoded)
                .map_err(|_| SessionError::SessionUnusable)?,
        })
    }
    async fn serve(&mut self, d: Duration) -> Result<(), SessionError> {
        tokio::time::sleep(d).await;
        Ok(())
    }
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
}
async fn ready(
    link: &PlayLink,
    commands: &Arc<Mutex<Vec<player_trade::Intent>>>,
    count: usize,
) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let panel = link.player_trade_panel();
            let received = commands
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .len();
            if panel.selected && !panel.pending && received == count {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn player_trade_controls_encode_requests_and_apply_real_wire_state() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let state = player_trade::State {
                    trade_id: Some([3; 16]),
                    partner: Some(EntityRef {
                        identity: [4; 16],
                        generation: 2,
                    }),
                    own: Some(player_trade::Offer {
                        definition: 10,
                        count: 1,
                        sub_type: 0,
                    }),
                    counter: Some(player_trade::Offer {
                        definition: 11,
                        count: 2,
                        sub_type: 3,
                    }),
                    accepted: [false, false],
                    phase: 2,
                    reason: 0,
                    message: String::new(),
                    look: String::new(),
                };
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(
                    Peer {
                        state,
                        commands: Arc::clone(&commands),
                    },
                    incoming,
                    outbox,
                ));
                ready(&link, &commands, 0).await?;
                for (index, intent) in [
                    Intent::Accept,
                    Intent::Cancel,
                    Intent::LookOwn,
                    Intent::LookCounter,
                ]
                .into_iter()
                .enumerate()
                {
                    link.player_trade_request(intent);
                    ready(&link, &commands, index + 1).await?;
                }
                assert!(panel_text(&link.player_trade_panel()).contains("steel sword"));
                {
                    let got = commands.lock().unwrap_or_else(PoisonError::into_inner);
                    assert_eq!(got.len(), 4);
                }
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "player trade UI test thread panicked")?
}

#[test]
fn player_trade_inventory_push_reaches_visible_picker() -> Result<(), Error> {
    use oteryn_session::{AppliedDelta, CharacterInventory, ItemEntry};
    let (link, _commands, outbox) = play_channel();
    outbox.player_trade_initialize(true, None);
    outbox.push(vec![SessionEvent::Inventory(AppliedDelta {
        server_sequence: 41,
        base_revision: 1,
        new_revision: 2,
        value: CharacterInventory {
            entries: vec![ItemEntry {
                handle: 7.try_into()?,
                item_definition_ref: 123.try_into()?,
                count: 2.try_into()?,
                sub_type: 0,
            }],
            ..Default::default()
        },
    })]);
    assert!(panel_text(&link.player_trade_panel()).contains("Backpack: item 123 x2"));
    Ok(())
}

#[test]
fn player_trade_world_context_only_targets_visible_players_on_this_floor() -> Result<(), Error> {
    use oteryn_renderer::TileCoord;
    use oteryn_session::{ActorPosition, EntityDetail, EntityKind, WorldSpatialEntity};
    let player = WorldSpatialEntity {
        display_name: None,
        kind: EntityKind::Player,
        entity: EntityRef {
            identity: [4; 16],
            generation: 2,
        },
        position: ActorPosition {
            x: 1,
            y: 0,
            floor: 0,
        },
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    };
    let view = crate::play::PlayView::new(TileCoord::new(0, 0), 0, &[])?.with_entities(vec![
        player,
        WorldSpatialEntity {
            kind: EntityKind::Npc,
            position: ActorPosition {
                x: 2,
                ..player.position
            },
            ..player
        },
        WorldSpatialEntity {
            position: ActorPosition {
                x: 3,
                floor: 1,
                ..player.position
            },
            ..player
        },
    ])?;
    assert_eq!(
        view.player_trade_context_target(TileCoord::new(1, 0)),
        Some(player.entity)
    );
    for x in [0, 2, 3] {
        assert_eq!(view.player_trade_context_target(TileCoord::new(x, 0)), None);
    }
    Ok(())
}

#[test]
fn player_trade_unsolicited_incoming_and_terminal_updates_reach_panel_and_reset_selection() {
    use oteryn_session::AppliedDelta;
    let (link, _commands, outbox) = play_channel();
    outbox.player_trade_initialize(true, None);
    let partner = EntityRef {
        identity: [4; 16],
        generation: 2,
    };
    let state = player_trade::State {
        partner: Some(partner),
        phase: 1,
        reason: 0,
        message: "Player wants to trade with you.".into(),
        ..Default::default()
    };
    outbox.push(vec![SessionEvent::PlayerTrade(AppliedDelta {
        server_sequence: 41,
        base_revision: 1,
        new_revision: 2,
        value: state,
    })]);
    assert_eq!(link.player_trade_panel().partner, Some(partner));
    assert!(link.player_trade_panel().show_requested);
    assert!(panel_text(&link.player_trade_panel()).contains("wants to trade"));
    outbox.push(vec![SessionEvent::PlayerTrade(AppliedDelta {
        server_sequence: 42,
        base_revision: 2,
        new_revision: 3,
        value: player_trade::State {
            message: "Trade cancelled.".into(),
            ..Default::default()
        },
    })]);
    assert!(link.player_trade_panel().partner.is_none());
    assert!(panel_text(&link.player_trade_panel()).contains("Trade cancelled."));
    outbox.end(oteryn_platform_client::native_login::PublicClass::SessionUnavailable);
    assert!(!link.player_trade_panel().selected);
}

#[cfg(not(windows))]
#[test]
fn player_trade_secondary_click_uses_pointer_and_focus_loss_clears_it() -> Result<(), Error> {
    use oteryn_input_actions::{
        ButtonState, Modifiers, MouseButton, NormalizedInputEvent as E, PointerCoordinate,
        PointerDelta, PointerMotion, PointerPosition,
    };
    use oteryn_renderer::TileCoord;
    use oteryn_session::{ActorPosition, EntityDetail, EntityKind, WorldSpatialEntity};
    let partner = EntityRef {
        identity: [4; 16],
        generation: 2,
    };
    let view = crate::play::PlayView::new(TileCoord::new(0, 0), 0, &[])?.with_entities(vec![
        WorldSpatialEntity {
            display_name: None,
            kind: EntityKind::Player,
            entity: partner,
            position: ActorPosition {
                x: 1,
                y: 0,
                floor: 0,
            },
            detail: EntityDetail::Actor {
                direction: StepDirection::South,
                appearance_ref: 1,
                health_percent: 100,
            },
        },
    ])?;
    let [x, y] = view.scene().view().tile_to_screen(TileCoord::new(1, 0));
    // Test coordinates are small exact integer tile origins.
    #[allow(clippy::cast_possible_truncation)]
    let moved = E::PointerMoved {
        position: PointerPosition::new(
            PointerCoordinate::new(x as i32 + 1)?,
            PointerCoordinate::new(y as i32 + 1)?,
        ),
        motion: PointerMotion::new(PointerDelta::new(0)?, PointerDelta::new(0)?),
    };
    let click = E::MouseButton {
        button: MouseButton::SECONDARY,
        state: ButtonState::Pressed,
        modifiers: Modifiers::NONE,
    };
    let (link, _commands, outbox) = play_channel();
    let clicks = link.player_trade_track_pointer(&[moved.clone(), click.clone()]);
    assert!(!view.player_trade_context_input(&link, &clicks));
    outbox.player_trade_initialize(true, None);
    // Another feature consumes this batch, so the context handler never sees its move.
    link.player_trade_track_pointer(&[moved]);
    let clicks = link.player_trade_track_pointer(std::slice::from_ref(&click));
    assert!(view.player_trade_context_input(&link, &clicks));
    assert_eq!(link.player_trade_panel().partner, Some(partner));
    let clicks = link.player_trade_track_pointer(&[E::FocusChanged { focused: false }, click]);
    assert!(!view.player_trade_context_input(&link, &clicks));
    Ok(())
}
