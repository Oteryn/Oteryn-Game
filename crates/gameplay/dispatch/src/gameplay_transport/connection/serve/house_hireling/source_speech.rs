//! Existing chat protocol bound to the real named-House admission, never its origin Channel.
use super::*;
use crate::gameplay_transport::item_view::ItemViewSnapshotDomain as OwnedDomain;
use oteryn_protocol_oteryn::chat::{ChatLine, STATE_DOMAIN_CHAT};

pub(super) async fn snapshot<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
) -> Result<Option<OwnedDomain>, ServeEnd> {
    if !admitted
        .continuity
        .selected_capabilities
        .domain_selected(STATE_DOMAIN_CHAT)
    {
        return Ok(None);
    }
    // serve_house_scope already proved the native House acknowledgement. Keep
    // this exact admitted session/fence in the existing bound chat owners.
    let rooms = authority.chat_admit(actor, admitted.game_session_id).await;
    let (snapshot_type, payload) =
        crate::gameplay_transport::guild_chat::wire::snapshot(authority, *admitted, rooms)
            .await
            .ok_or(ServeEnd::Disconnected)?;
    let revision = admitted
        .continuity
        .chat_revision
        .checked_add(1)
        .ok_or(ServeEnd::Disconnected)?;
    admitted.continuity.chat_revision = revision;
    Ok(Some(OwnedDomain {
        domain_id: STATE_DOMAIN_CHAT,
        revision,
        snapshot_type,
        payload,
    }))
}
impl State {
    pub(super) async fn chat<S: AsyncWrite + Unpin, A: FreshAdmissionAuthority>(
        &self,
        stream: &mut S,
        authority: &A,
        admitted: &mut AdmittedSession,
    ) -> Result<(), ServeEnd> {
        if !admitted
            .continuity
            .selected_capabilities
            .domain_selected(STATE_DOMAIN_CHAT)
            || admitted.continuity.chat_revision == 0
        {
            return Ok(());
        }
        // A local line was queued under its actual House audience. Reprove the
        // current recipient before any delivery; an origin Channel is no alias.
        authority
            .house_scope_ack(*admitted)
            .await
            .map_err(|_| ServeEnd::Disconnected)?
            .ok_or(ServeEnd::Disconnected)?;
        let mut remaining = crate::chat::EGRESS_MAX_LINES;
        for _ in 0..crate::chat::EGRESS_MAX_LINES {
            let Some(line) = authority.chat_next_bound(*admitted, &mut remaining).await else {
                break;
            };
            let (sequence, frame, revision) = chat_line_delta(
                admitted.continuity.connection_generation,
                admitted.continuity.server_sequence,
                admitted.continuity.chat_revision,
                &line,
            )
            .ok_or(ServeEnd::Disconnected)?;
            write_frame(stream, &frame)
                .await
                .map_err(|_| ServeEnd::Disconnected)?;
            admitted.continuity.server_sequence = sequence;
            admitted.continuity.chat_revision = revision;
            if !matches!(line, ChatLine::Dropped) {
                authority.chat_delivered(admitted.game_session_id).await;
            }
        }
        Ok(())
    }
}
