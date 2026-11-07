//! Stable native CommandRef association, recovered through the existing APP-1 receipt.
//! Reconciliation is historical data; it never authorizes a new write or publication.
use super::*;
use crate::foundation::CommandRef;

impl AppearanceChangeOccurrence {
    /// Namespace the server-admitted session/command identity, never a client UUID.
    pub fn for_command(command: CommandRef) -> Self {
        let digest = Sha256::new()
            .chain_update(b"oteryn:appearance-command:v1")
            .chain_update(command.game_session_id().as_bytes())
            .chain_update(command.command_id().get().to_be_bytes())
            .finalize();
        let mut bytes = [0; 16];
        bytes.copy_from_slice(&digest[..16]);
        bytes[6] = (bytes[6] & 15) | 0x70;
        bytes[8] = (bytes[8] & 63) | 0x80;
        Self(bytes)
    }
}

impl DurabilityRoot {
    /// Resolve a lost acknowledgement after owner restart without acquiring live authority.
    /// The independently admitted Character and original request must match the receipt.
    pub async fn reconcile_character_appearance_request(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        revision: CharacterRevision,
        request: AppearanceChangeRequest,
    ) -> Result<Option<CommittedCharacterAppearance>, AppearanceWriteError> {
        request.validate()?;
        let recovery = authority.record_for(self).map_err(|_| {
            AppearanceWriteError::Fence(CharacterProgressionError::AuthorityRejected)
        })?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let Some(row) = load_appearance_receipt(&mut tx, request.occurrence).await?
                    else {
                        commit_semantic_transaction(tx, deadline).await?;
                        return Ok(Ok(None));
                    };
                    let receipt = decode_appearance_receipt(&row)?;
                    if receipt.character_id != character {
                        return Ok(Err(CharacterProgressionError::ConflictingOccurrence.into()));
                    }
                    let sex = match character_sex::read(&mut tx, character).await {
                        Ok(sex) => sex,
                        Err(error) => return Ok(Err(error)),
                    };
                    if !receipt.matches_semantics(character, revision, &request, sex) {
                        return Ok(Err(CharacterProgressionError::ConflictingOccurrence.into()));
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(Some(receipt)))
                })
            })
            .await?
    }
}

impl CommittedCharacterAppearance {
    pub(super) fn matches_semantics(
        &self,
        character: CharacterId,
        revision: CharacterRevision,
        request: &AppearanceChangeRequest,
        sex: Sex,
    ) -> bool {
        request.validate().is_ok()
            && self.occurrence == request.occurrence
            && self.character_id == character
            && self.original_character_revision == revision
            && self.before == request.before
            && self.after == request.after
            && self.content_revision == request.content_revision
            && self.binding == semantic_binding(character, revision, request, sex)
    }
}

pub(super) fn semantic_binding(
    character: CharacterId,
    revision: CharacterRevision,
    request: &AppearanceChangeRequest,
    sex: Sex,
) -> Vec<u8> {
    fn text(out: &mut Vec<u8>, value: &str) {
        out.extend_from_slice(&(value.len() as u64).to_be_bytes());
        out.extend_from_slice(value.as_bytes());
    }
    fn selection(out: &mut Vec<u8>, value: &AppearanceSelection<String>) {
        text(out, &value.outfit_key);
        for colour in value.colours {
            out.extend_from_slice(&colour.to_be_bytes());
        }
        out.push(value.addons);
        out.push(u8::from(value.mount_key.is_some()));
        if let Some(mount) = &value.mount_key {
            text(out, mount);
        }
    }
    let mut semantic = vec![BINDING_VERSION];
    semantic.extend_from_slice(&request.occurrence.0);
    semantic.extend_from_slice(character.as_bytes());
    semantic.extend_from_slice(&revision.get().to_be_bytes());
    semantic.push(match sex {
        Sex::Female => 0,
        Sex::Male => 1,
    });
    semantic.push(u8::from(request.before.is_some()));
    if let Some(before) = &request.before {
        selection(&mut semantic, &before.selection);
        text(&mut semantic, &before.content_revision);
        semantic.extend_from_slice(&before.committed_character_revision.get().to_be_bytes());
    }
    selection(&mut semantic, &request.after);
    text(&mut semantic, &request.content_revision);
    if let Some(transport) = request.transport_binding {
        semantic.extend_from_slice(b"oteryn:appearance-transport:v1");
        semantic.extend_from_slice(&transport);
    }
    let mut binding = vec![BINDING_VERSION];
    binding.extend_from_slice(&Sha256::digest(&semantic));
    binding
}
