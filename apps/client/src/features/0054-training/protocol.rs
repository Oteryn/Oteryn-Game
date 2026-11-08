use crate::play::feature_0115_mail::protocol::{accepted, handle};
use oteryn_client_runtime::raw_feature::{FeatureCommand, GatedCommandResult};
use oteryn_client_ui::training::{Intent, Panel};
use oteryn_session::property_services::{self as adapter, loop_training as wire};

pub(in crate::play) fn command(panel: &Panel, intent: Intent) -> Result<FeatureCommand, String> {
    let (kind, payload) = match intent {
        Intent::Offline => {
            let payload = adapter::offline_payload(handle(&panel.fields[0])?);
            wire::decode_offline_training(&payload)
                .map_err(|_| "Invalid offline training target")?;
            (wire::COMMAND_TYPE_OFFLINE_TRAINING, payload)
        }
        _ => {
            let request = match intent {
                Intent::Start => wire::ExerciseIntent::Start {
                    weapon: handle(&panel.fields[1])?,
                    dummy: handle(&panel.fields[2])?,
                },
                Intent::Status => wire::ExerciseIntent::Status,
                Intent::Stop => wire::ExerciseIntent::Stop,
                Intent::Offline => return Err("Invalid training action".into()),
            };
            let payload = adapter::exercise_payload(request);
            wire::decode_exercise_training(&payload).map_err(|_| "Invalid exercise request")?;
            (wire::COMMAND_TYPE_EXERCISE_TRAINING, payload)
        }
    };
    Ok(FeatureCommand {
        capability: 54,
        kind,
        payload,
    })
}
pub(in crate::play) fn apply(
    panel: &mut Panel,
    command: &FeatureCommand,
    result: &GatedCommandResult,
) {
    if !panel.pending || panel.pending_kind != command.kind {
        return;
    }
    panel.pending = false;
    if !accepted(result) {
        panel.feedback = "The server refused the training request".into();
        return;
    }
    let Some(reply) = adapter::training_result(&result.payload) else {
        panel.feedback = "Invalid training reply".into();
        return;
    };
    if command.kind == wire::COMMAND_TYPE_OFFLINE_TRAINING && reply.disposition == 3 {
        panel.offline_confirmed = true;
    }
    panel.feedback = match reply.disposition {
        1 => "Exercise training is running",
        2 => "Exercise training stopped or settled",
        3 => "Offline training selected; the server is logging you out",
        _ => "The server refused the training request",
    }
    .into();
    if reply.disposition != 4 {
        panel.charges = Some(reply.charges);
        panel.pool_seconds = Some(reply.pool_seconds);
        panel.summary = format!(
            "Remaining charges: {}\r\nOffline pool: {} seconds",
            reply.charges, reply.pool_seconds
        );
    }
}
