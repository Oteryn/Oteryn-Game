use crate::GameplayEntryError;
use oteryn_platform_client::native_login::PublicClass;
use std::time::Instant;

pub fn error_text(error: GameplayEntryError) -> String {
    match error {
        GameplayEntryError::NativeProtocolUnavailable => {
            tr!("Enter connection settings and choose Save and sign in.")
        }
        GameplayEntryError::InvalidConfiguration => {
            tr!(
                "Check Platform and Gateway URLs, OAuth client ID, optional character preselect and the trust certificate path, then sign in again."
            )
        }
        GameplayEntryError::ClientUpdateRequired { manual_download_url, updater_started } => {
            return if updater_started {
                lfmt!("Update required. Oteryn will restart after the update. Manual download at {manual_download_url}", manual_download_url = manual_download_url)
            } else {
                lfmt!("Update required. Download the current client at {manual_download_url}, then sign in again.", manual_download_url = manual_download_url)
            };
        }
        GameplayEntryError::Maintenance { .. } => tr!("Sign-in was paused for maintenance."),
        GameplayEntryError::ChannelFull => tr!("This channel is full."),
        GameplayEntryError::WorldUnavailable => {
            tr!(
                "World is absent from the Platform directory. Check the World reference and Platform URL, then sign in again."
            )
        }
        GameplayEntryError::Rejected(
            PublicClass::RetryLogin | PublicClass::AuthenticationRequired,
        ) => {
            tr!(
                "Browser sign-in failed or expired. Check the OAuth client ID and complete a fresh browser sign-in within five minutes."
            )
        }
        GameplayEntryError::Rejected(PublicClass::TemporarilyUnavailable) => {
            tr!(
                "Connection unavailable. Check your network and the Platform/Gateway URLs; if correct, try again later."
            )
        }
        GameplayEntryError::Rejected(PublicClass::SessionUnavailable) => {
            tr!(
                "Game connection ended or could not be established. Check your network and trust certificate; sign in again. Contact the operator if this repeats."
            )
        }
        GameplayEntryError::Rejected(PublicClass::ClientUpdateRequired) => {
            tr!("Install the current Oteryn client, then sign in again.")
        }
        GameplayEntryError::Rejected(PublicClass::CharacterAlreadyActive) => {
            tr!(
                "Character is already active. Log out its other session or wait for it to end, then sign in again."
            )
        }
    }.into()
}

pub(super) fn maintenance_text(retry_at: Instant, now: Instant) -> String {
    let remaining = retry_at.saturating_duration_since(now);
    if remaining.is_zero() {
        return tr!("The maintenance wait has ended. You can sign in again.").into();
    }
    let seconds = remaining.as_secs() + u64::from(remaining.subsec_nanos() > 0);
    lfmt!(
        "Oteryn is undergoing maintenance. Try signing in again in {seconds} s.",
        seconds = seconds
    )
}
