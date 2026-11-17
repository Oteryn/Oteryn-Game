use super::*;
use oteryn_platform_client::native_login::{AccessToken, NativeTicket, OwnerCharacter};
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct LastCharacter {
    character: String,
    channel: Option<String>,
}
pub struct DeviceAttempt {
    pub(super) state: tokio::sync::OwnedMutexGuard<State>,
    pub(super) vault: Arc<dyn vault::Vault>,
    pub(super) _lock: Option<std::fs::File>,
    pub(super) remembered: bool,
    pub(super) options: MiscellaneousPreferences,
    pub(super) folder: Option<PathBuf>,
}
impl DeviceAttempt {
    pub fn has_session(&self) -> bool {
        self.state.session.is_some()
    }
    pub async fn enroll(
        &mut self,
        client: &NativeLoginClient,
        token: &AccessToken,
        cancel: CancellationToken,
    ) -> Result<(), NativeLoginError> {
        if !self.options.stay_logged_in {
            return Ok(());
        }
        self.sign_out(client).await;
        if !self.remembered {
            self.state.key = Some(SessionKey::generate()?);
        }
        let session = client
            .enroll_device(token, self.remembered, self.state.key.as_ref(), cancel)
            .await?;
        self.save(client, session).await
    }
    fn take(&mut self) -> Result<DeviceSession, NativeLoginError> {
        let session = self.state.session.take().ok_or_else(storage_error)?;
        self.state.pending = Some(session.device_credential.clone());
        // Delete BEFORE the uncertain request. A crash must never reuse a consumed predecessor.
        self.vault.clear().map_err(|_| storage_error())?;
        Ok(session)
    }
    async fn save(
        &mut self,
        client: &NativeLoginClient,
        session: DeviceSession,
    ) -> Result<(), NativeLoginError> {
        if self.remembered && self.vault.write(&session.vault_bytes()?).is_err() {
            let _ = client
                .revoke_device(&session.device_credential, CancellationToken::new())
                .await;
            let _ = self.vault.clear();
            self.state.session = None;
            self.state.pending = None;
            return Err(storage_error());
        }
        self.state.remembered = self.remembered;
        self.state.session = Some(session);
        self.state.pending = None;
        Ok(())
    }
    pub async fn characters(
        &mut self,
        client: &NativeLoginClient,
        cancel: CancellationToken,
    ) -> Result<Vec<OwnerCharacter>, NativeLoginError> {
        let previous = self.take()?;
        match client.device_characters(&previous, cancel).await {
            Ok((session, characters)) => {
                self.save(client, session).await?;
                Ok(characters)
            }
            Err(error) => {
                self.sign_out(client).await;
                Err(error)
            }
        }
    }
    pub async fn ticket(
        &mut self,
        client: &NativeLoginClient,
        cancel: CancellationToken,
    ) -> Result<NativeTicket, NativeLoginError> {
        let previous = self.take()?;
        match client.device_ticket(&previous, cancel).await {
            Ok((session, ticket)) => {
                self.save(client, session).await?;
                Ok(ticket)
            }
            Err(error) => {
                self.sign_out(client).await;
                Err(error)
            }
        }
    }
    pub async fn sign_out(&mut self, client: &NativeLoginClient) {
        let credential = self
            .state
            .session
            .take()
            .map(|s| s.device_credential)
            .or_else(|| self.state.pending.take());
        let _ = self.vault.clear();
        self.state.key = None;
        self.state.pending = None;
        if let Some(credential) = credential {
            let _ = client
                .revoke_device(&credential, CancellationToken::new())
                .await;
        }
    }
    pub fn quick_selection(
        &self,
        rows: &[crate::character_selection::CharacterRow],
    ) -> Option<crate::character_selection::SelectionAction> {
        if !self.options.quick_login {
            return None;
        }
        let bytes = std::fs::read(self.folder.as_ref()?.join("last-character.json")).ok()?;
        if bytes.len() > 1024 {
            return None;
        }
        let saved: LastCharacter = serde_json::from_slice(&bytes).ok()?;
        let row = rows
            .iter()
            .find(|row| row.available && row.character.character_id == saved.character)?;
        if saved
            .channel
            .as_ref()
            .is_some_and(|id| !row.channels.iter().any(|c| &c.0 == id))
        {
            return None;
        }
        Some(crate::character_selection::SelectionAction::Play {
            character_id: saved.character,
            channel_id: saved.channel,
        })
    }
    pub fn remember_selection(&self, character: &str, channel: Option<&str>) {
        if let Some(folder) = &self.folder {
            let data = LastCharacter {
                character: character.into(),
                channel: channel.map(String::from),
            };
            if let Ok(bytes) = serde_json::to_vec(&data)
                && std::fs::write(folder.join("last-character.next"), bytes).is_ok()
            {
                let _ = std::fs::rename(
                    folder.join("last-character.next"),
                    folder.join("last-character.json"),
                );
            }
        }
    }
}
