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
    pub(super) journal: Arc<revocation_journal::Journal>,
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
        self.remember_revocation(session.device_credential.clone());
        // Delete BEFORE the uncertain request. A crash must never reuse a consumed predecessor.
        self.clear_vault()?;
        Ok(session)
    }
    async fn save(
        &mut self,
        client: &NativeLoginClient,
        session: DeviceSession,
    ) -> Result<(), NativeLoginError> {
        if self.remembered && self.vault.write(&session.vault_bytes()?).is_err() {
            self.remember_revocation(session.device_credential);
            self.sign_out(client).await;
            return Err(storage_error());
        }
        self.journal
            .observe_process_credential((!self.remembered).then_some(&session.device_credential))?;
        self.state.remembered = self.remembered;
        self.state.session = Some(session);
        self.state.vault_clear_failed = false;
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
                self.state
                    .pending
                    .retain(|pending| pending != &previous.device_credential);
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
        if now().is_ok_and(|when| when >= previous.idle_expires_at) {
            self.sign_out(client).await;
            return Err(NativeLoginError::TicketSpent);
        }
        match client.device_ticket(&previous, cancel).await {
            Ok((session, ticket)) => {
                self.state
                    .pending
                    .retain(|pending| pending != &previous.device_credential);
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
        let _ = self.revoke(client).await;
    }

    pub(super) async fn revoke(
        &mut self,
        client: &NativeLoginClient,
    ) -> Result<(), NativeLoginError> {
        if let Some(session) = self.state.session.take() {
            self.remember_revocation(session.device_credential);
        }
        self.state.key = None;
        self.state.remembered = false;
        self.journal.observe_process_credential(None)?;
        let local = self.clear_vault();
        let remote = self.revoke_pending(client).await;
        local.and(remote)
    }

    pub(super) async fn revoke_before_vault_clear(
        &mut self,
        client: &NativeLoginClient,
    ) -> Result<(), NativeLoginError> {
        if let Some(session) = self.state.session.take() {
            self.remember_revocation(session.device_credential);
        }
        self.state.key = None;
        self.state.remembered = false;
        if let Some(bytes) = self.vault.read().map_err(|_| storage_error())?
            && bytes.len() <= 1024
            && let Ok(session) = serde_json::from_slice::<DeviceSession>(&bytes)
        {
            self.remember_revocation(session.device_credential);
        }
        if self.journal.has_folder() {
            for credential in &self.state.pending {
                self.journal.stage(credential)?;
            }
            self.clear_vault()?;
            self.state.pending.clear();
            self.journal.replay(client).await
        } else {
            self.revoke_pending(client).await?;
            self.clear_vault()
        }
    }

    fn clear_vault(&mut self) -> Result<(), NativeLoginError> {
        let failed = self.vault.available() && self.vault.clear().is_err();
        self.state.vault_clear_failed = failed;
        if failed { Err(storage_error()) } else { Ok(()) }
    }

    fn remember_revocation(&mut self, credential: DeviceCredential) {
        if !self.state.pending.contains(&credential) {
            self.state.pending.push(credential);
        }
    }

    pub(super) async fn revoke_pending(
        &mut self,
        client: &NativeLoginClient,
    ) -> Result<(), NativeLoginError> {
        let mut index = 0;
        let mut result = Ok(());
        while let Some(credential) = self.state.pending.get(index) {
            match client
                .revoke_device(credential, CancellationToken::new())
                .await
            {
                Ok(()) => {
                    self.state.pending.remove(index);
                }
                Err(error) => {
                    result = Err(error);
                    index += 1;
                }
            }
        }
        result
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
            .is_some_and(|id| !row.channels.iter().any(|c| &c.id == id))
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
