//! Spell command body; predicate and connection control flow stay in `connection.rs`.

use super::super::{
    AdmittedSession, ExactActorRef, FreshAdmissionAuthority, SpellCastOutcome,
    decode_spell_cast_intent,
};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    Some({
        match decode_spell_cast_intent(command.payload) {
            Ok(intent) => match authority
                .cast_spell_completion(actor, admitted.game_session_id, command.command_id, intent)
                .await
            {
                Some(outcome) => Dispatch::Spell(outcome),
                // Preserve continuity's original next command and no terminal
                // outcome; reconnect/reconciliation cannot pay or reroll twice.
                None => return None,
            },
            Err(_) => Dispatch::Spell(SpellCastOutcome::rejected()),
        }
    })
}
