use crate::combat_input::{CombatCommand, CombatReport};
use crate::play::{PlayLink, Stepper, play_channel, run_session};
use oteryn_client_runtime::task_board::{TaskBoardCommand, TaskBoardResponse, task_board};
use oteryn_client_ui::task_board::{Intent, panel_text};
use oteryn_session::{
    CommandStatus, LogoutResult, SessionError, SessionEvent, StepDirection, StepOutcome,
};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;
type Error = Box<dyn std::error::Error + Send + Sync>;

struct Peer {
    commands: Arc<Mutex<Vec<TaskBoardCommand>>>,
    full_commands: Arc<Mutex<Vec<task_board::full::Request>>>,
}
include!("play_methods_1.rs");
async fn ready(link: &PlayLink) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let panel = link.task_board_panel();
            if panel.selected && !panel.pending && panel.state.is_some() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
async fn selected(link: &PlayLink) -> Result<(), Error> {
    tokio::time::timeout(Duration::from_secs(3), async {
        while !link.task_board_panel().selected {
            tokio::task::yield_now().await;
        }
    })
    .await?;
    Ok(())
}
#[test]
fn task_board_controls_use_the_real_codec_and_show_refusals() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let commands = Arc::new(Mutex::new(Vec::new()));
                let (link, incoming, outbox) = play_channel();
                let task = tokio::spawn(run_session(
                    Peer {
                        commands: Arc::clone(&commands),
                        full_commands: Arc::default(),
                    },
                    incoming,
                    outbox,
                ));
                selected(&link).await?;
                let input = oteryn_input_actions::NormalizedInputEvent::Key {
                    code: oteryn_input_actions::KeyCode::new(18)?,
                    state: oteryn_input_actions::ButtonState::Pressed,
                    modifiers: oteryn_input_actions::Modifiers::one(
                        oteryn_input_actions::Modifier::Control,
                    ),
                    repeat: false,
                };
                assert!(link.task_board_input(&[input]));
                assert!(link.task_board_panel().open);
                tokio::time::timeout(Duration::from_secs(3), async {
                    while link.task_board_panel().pending {
                        tokio::task::yield_now().await;
                    }
                })
                .await?;
                assert!(link.task_board_panel().weekly.is_some());
                assert!(panel_text(&link.task_board_panel()).contains("3031: 75/100"));
                // Keep the legacy codec/reader assertions after the full-board opening.
                link.task_board_request(Intent::Refresh);
                ready(&link).await?;
                assert!(panel_text(&link.task_board_panel()).contains("oteryn:dragon"));
                link.task_board_request(Intent::Claim(1));
                ready(&link).await?;
                assert!(panel_text(&link.task_board_panel()).contains("server refused"));
                assert_eq!(
                    commands
                        .lock()
                        .unwrap_or_else(PoisonError::into_inner)
                        .len(),
                    2
                );
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "Task Board UI test thread panicked")?
}

fn full_reply() -> task_board::full::Reply {
    use task_board::full::*;
    Reply {
        weekly_results: None,
        refusal: 0,
        period: 2965,
        points: 150,
        bounty_points: 20,
        soulseals: 2,
        weekly: Weekly {
            started: true,
            difficulty: 0,
            expanded: false,
            multiplier: 1,
            kill_experience: 1850,
            delivery_experience: 1850,
            pending_points: 25,
            pending_soulseals: 0,
            kills: vec![KillTask {
                race: "oteryn:creature.rat".into(),
                required: 50,
                current: 25,
            }],
            deliveries: vec![DeliveryTask {
                item: 3031,
                required: 100,
                current: 75,
                delivered: false,
            }],
        },
        bounty: Bounty {
            elite: false,
            state: 0,
            difficulty: 0,
            selected: 0,
            current: 0,
            rerolls: 3,
            daily_reroll_at: 1_800_000_000,
            offers: vec![],
            active: None,
            lists: vec![],
        },
        talisman: [TalismanPath {
            level: 1,
            bonus: 250,
            next_cost: 17,
        }; 4],
        shop: vec![ShopOffer {
            index: 0,
            kind: 2,
            price: 100_000,
            primary: 1907,
            secondary: 0,
            addon: 0,
            bought: false,
        }],
    }
}

#[test]
fn full_task_board_reply_reaches_the_play_panel() -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            let runtime = oteryn_client_runtime::ClientRuntime::new()?;
            runtime.block_on(async {
                let (link, incoming, outbox) = play_channel();
                let full_commands = Arc::new(Mutex::new(Vec::new()));
                let task = tokio::spawn(run_session(
                    Peer {
                        commands: Arc::new(Mutex::new(vec![])),
                        full_commands: Arc::clone(&full_commands),
                    },
                    incoming,
                    outbox,
                ));
                selected(&link).await?;
                use oteryn_client_ui::task_board::FullIntent;
                for intent in [
                    FullIntent::Refresh,
                    FullIntent::StartWeekly(0),
                    FullIntent::Deliver(0),
                    FullIntent::OpenBounty,
                    FullIntent::SelectBounty(0),
                    FullIntent::ClaimBounty,
                    FullIntent::BountyDifficulty(1),
                    FullIntent::Reroll,
                    FullIntent::Preferred(0, "oteryn:creature.rat".into()),
                    FullIntent::Unwanted(0, String::new()),
                    FullIntent::UnlockList,
                    FullIntent::BuyShop(0),
                ] {
                    let refused = matches!(intent, FullIntent::Deliver(_));
                    link.task_board_full_request(intent);
                    tokio::time::timeout(Duration::from_secs(3), async {
                        while link.task_board_panel().pending {
                            tokio::task::yield_now().await;
                        }
                    })
                    .await?;
                    let panel = link.task_board_panel();
                    assert!(panel.weekly.is_some());
                    assert_eq!(panel.feedback.is_some(), refused);
                    let text = panel_text(&panel);
                    assert!(text.contains("3031: 75/100"));
                    assert!(text.contains("1907"));
                }
                {
                    let recorded = full_commands.lock().unwrap_or_else(PoisonError::into_inner);
                    assert_eq!(
                        recorded
                            .iter()
                            .map(|request| request.action)
                            .collect::<Vec<_>>(),
                        (4..=16).filter(|action| *action != 12).collect::<Vec<_>>()
                    );
                    assert_eq!(recorded[6].selection, 1);
                    assert_eq!(recorded[8].race, "oteryn:creature.rat");
                }
                assert_eq!(
                    oteryn_client_ui::task_board::creature_choices(&link.task_board_panel()),
                    vec![String::new(), "oteryn:creature.rat".into()]
                );
                drop(link);
                task.await?;
                Ok::<(), Error>(())
            })?
        })?
        .join()
        .map_err(|_| "full Task Board test panicked")?
}

#[test]
fn talisman_purchase_uses_full_owner_and_displays_refusal() -> Result<(), Error> {
    use oteryn_client_ui::task_board::{TaskBoardPanel, talisman};
    let mut panel = TaskBoardPanel {
        selected: true,
        ..Default::default()
    };
    let command = talisman::request(&mut panel, talisman::Intent::Buy).ok_or("purchase command")?;
    assert_eq!((command.capability, command.kind), (72, 89));
    assert_eq!(
        task_board::full::decode_request(&command.payload)
            .map_err(|_| "purchase decode")?
            .action,
        17
    );
    assert!(talisman::request(&mut panel, talisman::Intent::Buy).is_none());
    let (link, mut incoming, outbox) = play_channel();
    outbox.task_board_initialize(true);
    let command = talisman::request(
        &mut crate::play::mailbox::lock(&link.mailbox).task_board,
        talisman::Intent::Buy,
    )
    .ok_or("native purchase")?;
    link.commands
        .send(crate::combat_input::PlayCommand::Feature(command))?;
    let command = match incoming.try_recv()? {
        crate::combat_input::PlayCommand::Feature(value) => value,
        _ => return Err("purchase did not reach feature command queue".into()),
    };
    assert_eq!(
        task_board::full::decode_request(&command.payload)
            .map_err(|_| "purchase decode")?
            .action,
        17
    );
    let mut reply = full_reply();
    reply.refusal = 14;
    outbox.task_board_full_result(&oteryn_session::GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload: task_board::full::encode_reply(&reply).map_err(|_| "purchase reply")?,
    });
    assert!(!link.task_board_panel().pending);
    assert!(panel_text(&link.task_board_panel()).contains("server refused"));
    Ok(())
}

#[path = "controls.rs"]
mod controls;

#[path = "tree.rs"]
mod tree;

#[path = "soulseals.rs"]
mod soulseals;
