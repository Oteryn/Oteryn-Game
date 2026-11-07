//! Translate the admitted command into one stable native intent for APP-1 reconciliation.
use super::*;
use sha2::{Digest, Sha256};

pub(super) fn request(
    command: UseCommand,
    change: &wire::Change,
) -> Option<AppearanceChangeRequest> {
    let command_ref = crate::foundation::CommandRef::new(
        command.game_session_id,
        crate::foundation::CommandId::new(command.command_id).ok()?,
    );
    Some(AppearanceChangeRequest {
        occurrence: AppearanceChangeOccurrence::for_command(command_ref),
        transport_binding: Some(Sha256::digest(change.encode().ok()?).into()),
        before: if let Some(prior) = &change.prior {
            Some(DurableAppearanceSelection {
                selection: selection(&prior.selection),
                content_revision: prior.provenance.clone(),
                committed_character_revision: crate::domain::CharacterRevision::new(prior.revision)
                    .ok()?,
            })
        } else {
            None
        },
        after: selection(&change.selection),
        content_revision: change.content_revision.clone(),
    })
}
