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
            Ok(mut intent) => {
                if let oteryn_protocol_oteryn::actor_spell::SpellTarget::Position(position) =
                    intent.target
                {
                    let Some(position) = authority.physical_native_cast_position(position).await
                    else {
                        return Some(Dispatch::Spell(SpellCastOutcome::rejected()));
                    };
                    intent.target =
                        oteryn_protocol_oteryn::actor_spell::SpellTarget::Position(position);
                }
                // Only the copied decoded coordinates change. Continuity keeps
                // command.payload, including its original wire replay binding.
                match authority
                    .cast_spell_completion(
                        actor,
                        admitted.game_session_id,
                        command.command_id,
                        intent,
                    )
                    .await
                {
                    Some(outcome) => Dispatch::Spell(outcome),
                    // Preserve continuity's original next command and no terminal
                    // outcome; reconnect/reconciliation cannot pay or reroll twice.
                    None => return None,
                }
            }
            Err(_) => Dispatch::Spell(SpellCastOutcome::rejected()),
        }
    })
}
