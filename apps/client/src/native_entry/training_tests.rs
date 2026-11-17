use super::test_support::Stack;
use super::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn training_selection_enters_the_owned_character_and_opens_training() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let run = || -> TestResult {
                let stack = Stack::start_training()?;
                let mut config = stack.config();
                config.select_character = std::sync::Arc::new(|rows, _, _| {
                    crate::character_selection::SelectionAction::Train {
                        character_id: rows[0].character.character_id.clone(),
                        channel_id: None,
                    }
                });
                let client = crate::ClientBootstrap::new()?.with_native_login(config);
                let admitted = client.request_gameplay_entry()?;
                assert!(admitted.training_requested());
                assert_eq!(admitted.character_name(), "Ari");
                let (_, link) = client.start_play(admitted)?;
                assert!(link.training_panel().open);
                drop(link);
                client.shutdown();
                stack.finish();
                Ok(())
            };
            run().map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "training entry test thread failed")?
        .map_err(Into::into)
}

#[test]
fn training_selection_refuses_an_unowned_character_before_ticket_issue() -> TestResult {
    let stack = Stack::start(vec![])?;
    let mut config = stack.config();
    config.select_character =
        std::sync::Arc::new(
            |_, _, _| crate::character_selection::SelectionAction::Train {
                character_id: "01890a5d-ac96-774b-bcce-b302099a8000".into(),
                channel_id: None,
            },
        );
    assert_eq!(
        stack.entry(config)?,
        GameplayEntryError::Rejected(PublicClass::RetryLogin)
    );
    assert_eq!(
        test_support::count(&stack.requests(), "POST /api/v1/game-auth/tickets"),
        0
    );
    stack.finish();
    Ok(())
}
