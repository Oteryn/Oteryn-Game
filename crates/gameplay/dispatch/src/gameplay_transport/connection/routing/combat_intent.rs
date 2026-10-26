//! Combat intent command body; predicate and connection control flow stay in `connection.rs`.

use super::super::{
    AdmittedSession, ExactActorRef, FreshAdmissionAuthority, decode_attack_target_intent,
    decode_fight_modes_intent,
};
use super::Dispatch;
use oteryn_protocol_oteryn::ClientCommandView;

pub(in super::super) async fn route_expert_modes<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    command: ClientCommandView<'_>,
) -> Dispatch {
    // Both fight-mode commands share ATTACK0-RL-02; alternating them cannot bypass the rate.
    let admitted_rate = admitted
        .continuity
        .combat
        .mode_intents
        .admit(tokio::time::Instant::now());
    match oteryn_protocol_oteryn::pvp_expert::decode_expert_fight_modes(command.payload) {
        Ok(modes) if admitted_rate => Dispatch::Attack(
            authority
                .expert_fight_modes(actor, admitted.game_session_id, command.command_id, modes)
                .await,
        ),
        _ => Dispatch::Unregistered,
    }
}

pub(in super::super) async fn route<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    command: ClientCommandView<'_>,
) -> Dispatch {
    // ATTACK-1b: over ATTACK0-RL-01 (25 per second per GameSession, sliding window) it is
    // REJECTED with an empty payload before decoding, as is a malformed intent.
    let admitted_rate = admitted
        .continuity
        .combat
        .target_intents
        .admit(tokio::time::Instant::now());
    match decode_attack_target_intent(command.payload) {
        Ok(target) if admitted_rate => Dispatch::Attack(
            authority
                .attack_target(actor, admitted.game_session_id, command.command_id, target)
                .await,
        ),
        _ => Dispatch::Unregistered,
    }
}

pub(in super::super) async fn route_modes<A: FreshAdmissionAuthority>(
    authority: &A,
    actor: ExactActorRef,
    admitted: &mut AdmittedSession,
    command: ClientCommandView<'_>,
) -> Dispatch {
    // ATTACK-1b: ATTACK0-RL-02, as command 11.
    let admitted_rate = admitted
        .continuity
        .combat
        .mode_intents
        .admit(tokio::time::Instant::now());
    match decode_fight_modes_intent(command.payload) {
        Ok(modes) if admitted_rate => Dispatch::Attack(
            authority
                .fight_modes(actor, admitted.game_session_id, modes)
                .await,
        ),
        _ => Dispatch::Unregistered,
    }
}
