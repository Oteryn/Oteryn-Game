use super::*;
use crate::character_selection::{CharacterRow, TrainingSelection};
use oteryn_platform_client::native_login::{CharacterAvailability, OwnerCharacter};

#[test]
fn training_batch_reentry_advances_only_after_confirmation_and_cancel_clears_the_queue()
-> Result<(), Box<dyn std::error::Error>> {
    for confirmed in [false, true] {
        let mut config = crate::NativeLoginConfig::from_env(|name| match name {
            "OTERYN_PLATFORM_URL" | "OTERYN_GATEWAY_URL" => Some("http://127.0.0.1:1/".into()),
            "OTERYN_OAUTH_CLIENT_ID" => Some("training-entry-test".into()),
            "OTERYN_DEV_ROOT" => Some("/tmp/oteryn-training-entry-missing-root".into()),
            _ => None,
        })?
        .ok_or("configuration")?;
        config.open_browser = std::sync::Arc::new(|_| false);
        let rows: Vec<_> = [
            "01890a5d-ac96-774b-bcce-b302099a8058",
            "01890a5d-ac96-774b-bcce-b302099a8060",
        ]
        .into_iter()
        .map(|id| CharacterRow {
            character: OwnerCharacter {
                character_id: id.into(),
                world_id: "world".into(),
                name: id.into(),
                availability: CharacterAvailability::Available,
            },
            world_name: "Alpha".into(),
            channels: vec![],
            available: true,
        })
        .collect();
        let queue = config.training_queue.clone();
        queue
            .start(
                &rows,
                rows.iter()
                    .map(|row| TrainingSelection {
                        character_id: row.character.character_id.clone(),
                        channel_id: None,
                    })
                    .collect(),
            )
            .ok_or("batch")?;
        let mut entry = Reentry::new(ClientBootstrap::new()?.with_native_login(config));
        entry.state = EntryState::Playing;
        entry.training_completion = Some(Box::new(move || confirmed));
        entry.leave_play();
        if confirmed {
            assert_eq!(entry.state, EntryState::Entering);
            assert!(entry.pending.is_some());
            assert_eq!(queue.remaining(), 1);
            assert!(entry.cancel_entry());
            assert!(!entry.client.runtime.cancellation().is_cancelled());
        } else {
            assert_eq!(entry.state, EntryState::Offline);
            assert!(entry.pending.is_none());
        }
        assert_eq!(queue.remaining(), 0);
        entry.shutdown();
    }
    Ok(())
}
