//! Spell command body; predicate and connection control flow stay in `connection.rs`.

use super::super::{
    AdmittedSession, ExactActorRef, FreshAdmissionAuthority, SpellCastOutcome,
    decode_spell_cast_intent,
};
use super::Dispatch;
use oteryn_protocol_oteryn::{
    ClientCommandView, CommandStatus, actor_spell, actor_spell_v2 as parameters,
};

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    if command.command_type == parameters::COMMAND_TYPE_WORLD_ACTOR_SPELL_CAST_PARAMETERS_V2 {
        return route_parameter(authority, actor, admitted, command).await;
    }
    // WU26-14: `SPELL_AIM_MODE_V1` adds field 5 `aim_mode`, resolved into the old fields.
    let decoded = if admitted
        .continuity
        .selected_capabilities
        .contains(actor_spell::CAPABILITY_SPELL_AIM_MODE_V1)
    {
        actor_spell::decode_aimed_spell_cast_intent(command.payload).map(|aimed| aimed.resolved())
    } else {
        decode_spell_cast_intent(command.payload)
    };
    Some({
        match decoded {
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

async fn route_parameter<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &AdmittedSession,
    command: ClientCommandView<'_>,
) -> Option<Dispatch> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(parameters::CAPABILITY_ACTOR_SPELL_PARAMETERS_V2)
    {
        return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    }
    let Ok(mut intent) = parameters::decode_parameter_spell_cast_intent(command.payload) else {
        return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
    };
    if let actor_spell::SpellTarget::Position(position) = intent.intent.target {
        let Some(position) = authority.physical_native_cast_position(position).await else {
            return Some(Dispatch::Reply(CommandStatus::Rejected, Vec::new()));
        };
        intent.intent.target = actor_spell::SpellTarget::Position(position);
    }
    let (outcome, result) = authority
        .cast_parameter_spell_completion(
            actor,
            admitted.game_session_id,
            command.command_id,
            intent,
        )
        .await?;
    let result = result.unwrap_or(parameters::ParameterSpellCastResult {
        disposition: outcome.disposition,
        feedback: None,
        editor: None,
    });
    let payload = parameters::encode_parameter_spell_cast_result(&result).ok()?;
    Some(Dispatch::ParameterSpell(outcome, payload))
}
