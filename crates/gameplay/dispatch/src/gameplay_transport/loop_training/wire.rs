use super::super::connection::{AdmittedSession, FreshAdmissionAuthority, UseCommand};
use super::super::item_view::SessionItemView;
use oteryn_protocol_oteryn::loop_training::*;

pub(in crate::gameplay_transport) async fn dispatch(
    authority: &impl FreshAdmissionAuthority,
    admitted: AdmittedSession,
    command: UseCommand,
    kind: u32,
    payload: &[u8],
    items: Option<&SessionItemView>,
) -> Option<TrainingResult> {
    if !admitted
        .continuity
        .selected_capabilities
        .contains(CAPABILITY_TRAINING_V1)
    {
        return Some(TrainingResult::REFUSED);
    }
    match kind {
        COMMAND_TYPE_OFFLINE_TRAINING => {
            let Ok(handle) = decode_offline_training(payload) else {
                return Some(TrainingResult::REFUSED);
            };
            let Some(target) = items.and_then(|items| items.resolve(handle)) else {
                return Some(TrainingResult::REFUSED);
            };
            authority.offline_training(admitted, command, target).await
        }
        COMMAND_TYPE_EXERCISE_TRAINING => {
            let Ok(intent) = decode_exercise_training(payload) else {
                return Some(TrainingResult::REFUSED);
            };
            let (weapon, dummy) = match intent {
                ExerciseIntent::Start { weapon, dummy } => (
                    items.and_then(|items| items.resolve(weapon)),
                    items.and_then(|items| items.resolve(dummy)),
                ),
                _ => (None, None),
            };
            authority
                .exercise_training(admitted, command, intent, weapon, dummy)
                .await
        }
        _ => Some(TrainingResult::REFUSED),
    }
}
