use super::*;

#[test]
fn empty_account_create_and_refresh_never_issue_a_ticket() -> TestResult {
    use crate::character_selection::SelectionAction;
    use std::sync::{Arc, atomic::AtomicUsize};
    let stack = Stack::start_with_account(vec![], vec![], true)?;
    let mut config = stack.config();
    let calls = Arc::new(AtomicUsize::new(0));
    let seen = calls.clone();
    config.select_character = Arc::new(move |rows, _, _| {
        assert!(rows.is_empty());
        match seen.fetch_add(1, Ordering::SeqCst) {
            0 => SelectionAction::Create,
            1 => SelectionAction::Refresh,
            _ => SelectionAction::Cancel,
        }
    });
    let opener = config.open_browser.clone();
    let create_url = format!("{}account/characters/create", config.platform_url);
    let opened = Arc::new(AtomicUsize::new(0));
    let created = opened.clone();
    config.open_browser = Arc::new(move |url| {
        if url == create_url {
            created.fetch_add(1, Ordering::SeqCst);
            true
        } else {
            opener(url)
        }
    });
    assert_eq!(
        stack.entry(config)?,
        GameplayEntryError::Rejected(PublicClass::RetryLogin)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 3);
    assert_eq!(opened.load(Ordering::SeqCst), 1);
    let requests = stack.requests();
    assert_eq!(
        test_support::count(&requests, "GET /api/v1/game-auth/native-characters"),
        3
    );
    assert_eq!(test_support::count(&requests, "POST /oauth/token"), 1);
    assert_eq!(
        test_support::count(&requests, "POST /api/v1/game-auth/tickets"),
        0
    );
    stack.finish();
    Ok(())
}

#[test]
fn selection_cannot_enter_an_unowned_character_or_channel() -> TestResult {
    use crate::character_selection::SelectionAction;
    use std::sync::Arc;
    for channel_id in [
        None,
        Some("01890a5d-ac96-774b-bcce-b302099a8000".to_owned()),
    ] {
        let stack = Stack::start(vec![])?;
        let mut config = stack.config();
        config.select_character = Arc::new(move |rows, _, _| SelectionAction::Play {
            character_id: if channel_id.is_none() {
                "01890a5d-ac96-774b-bcce-b302099a8000".into()
            } else {
                rows[0].character.character_id.clone()
            },
            channel_id: channel_id.clone(),
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

#[test]
fn training_selection_enters_the_owned_character_and_opens_training() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| -> Result<(), String> {
            let run = || -> TestResult {
                let stack = Stack::start_with_frames(
                    vec![],
                    super::super::test_support::admitted_frames(),
                )?;
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
