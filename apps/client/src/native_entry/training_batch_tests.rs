use super::test_support::Stack;
use super::*;
use crate::character_selection::{SelectionAction, TrainingSelection};
use std::sync::Arc;

#[test]
fn training_batch_rejects_duplicate_or_unowned_selections_before_ticket_issue()
-> Result<(), Box<dyn std::error::Error>> {
    for unowned in [false, true] {
        let stack = Stack::start_training()?;
        let mut config = stack.config();
        config.select_character = Arc::new(move |rows, _, _| {
            let first = TrainingSelection {
                character_id: rows[0].character.character_id.clone(),
                channel_id: None,
            };
            let second = if unowned {
                TrainingSelection {
                    character_id: "01890a5d-ac96-774b-bcce-b302099a8000".into(),
                    channel_id: None,
                }
            } else {
                first.clone()
            };
            SelectionAction::TrainMany {
                characters: vec![first, second],
            }
        });
        assert_eq!(
            stack.entry(config)?,
            GameplayEntryError::Rejected(PublicClass::RetryLogin)
        );
        assert_eq!(
            test_support::count(&stack.requests(), "POST /api/v1/game-auth/tickets"),
            0
        );
        stack.finish();
    }
    Ok(())
}

pub(super) fn roster() -> String {
    serde_json::json!({ "protocol_version": 2, "characters": [
        { "character_id": "01890a5d-ac96-774b-bcce-b302099a8058", "world_id": "01890a5d-ac96-774b-bcce-b302099a8059", "name": "Ari", "availability": "AVAILABLE" },
        { "character_id": "01890a5d-ac96-774b-bcce-b302099a8060", "world_id": "01890a5d-ac96-774b-bcce-b302099a8059", "name": "Bo", "availability": "AVAILABLE" }
    ] }).to_string()
}

#[test]
fn training_batch_admits_each_owned_character_with_a_fresh_ticket()
-> Result<(), Box<dyn std::error::Error>> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let run = || -> Result<(), Box<dyn std::error::Error>> {
                let stack = Stack::start_training()?;
                let mut config = stack.config();
                let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
                let observed = calls.clone();
                config.select_character = Arc::new(move |rows, _, _| {
                    observed.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    SelectionAction::TrainMany {
                        characters: rows
                            .iter()
                            .map(|row| TrainingSelection {
                                character_id: row.character.character_id.clone(),
                                channel_id: None,
                            })
                            .collect(),
                    }
                });
                let queue = config.training_queue.clone();
                let client = crate::ClientBootstrap::new()?.with_native_login(config);
                let first = client.request_gameplay_entry()?;
                assert_eq!(first.character_name(), "Ari");
                assert!(first.training_requested());
                assert_eq!(queue.remaining(), 1);
                assert!(queue.next().is_none());
                drop(first);
                // The session receipt is checked separately through the live training caller.
                assert!(queue.finish_current(true));
                let second = client.request_gameplay_entry()?;
                assert_eq!(second.character_name(), "Bo");
                assert_eq!(
                    second.character_id(),
                    "01890a5d-ac96-774b-bcce-b302099a8060"
                );
                assert!(second.training_requested());
                assert_eq!(queue.remaining(), 0);
                assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 1);
                let requests = stack.requests();
                assert_eq!(
                    test_support::count(&requests, "POST /api/v1/game-auth/tickets"),
                    2
                );
                assert_eq!(
                    test_support::count(&requests, "GET /api/v1/game-auth/native-characters"),
                    2
                );
                assert_eq!(test_support::count(&requests, "POST /oauth/token"), 2);
                drop(second);
                client.shutdown();
                stack.finish();
                Ok(())
            };
            run().map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "batch entry thread failed")?
        .map_err(Into::into)
}
