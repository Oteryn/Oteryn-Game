use super::{
    Error, Settings,
    preferences::{Preferences, PreferencesStore, authorize},
};
struct Store;
impl PreferencesStore for Store {
    async fn load(&self, _: [u8; 16]) -> Result<Preferences, Error> {
        Ok(Preferences {
            settings: Settings::default(),
            description: "A live adventure".into(),
        })
    }
}
#[test]
fn platform_preferences_are_authoritative_and_description_is_bounded()
-> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let p = authorize(&Store, [1; 16], Settings::default()).await?;
            assert_eq!(p.description, "A live adventure");
            assert!(
                authorize(
                    &Store,
                    [1; 16],
                    Settings {
                        password_required: true,
                        ..Settings::default()
                    }
                )
                .await
                .is_err()
            );
            assert!(
                Preferences {
                    settings: Settings::default(),
                    description: "x".repeat(257)
                }
                .validate()
                .is_err()
            );
            Ok(())
        })
}

struct Disabled;
impl PreferencesStore for Disabled {
    async fn load(&self, _: [u8; 16]) -> Result<Preferences, Error> {
        Ok(Preferences {
            settings: Settings {
                chat_enabled: false,
                ..Settings::default()
            },
            description: String::new(),
        })
    }
}
#[test]
fn stored_chat_preference_is_applied_without_an_extra_control_wire_field()
-> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            assert!(
                !authorize(&Disabled, [1; 16], Settings::default())
                    .await?
                    .settings
                    .chat_enabled
            );
            Ok(())
        })
}
