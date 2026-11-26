use super::*;
use crate::imbue::imbuement;
use oteryn_session::refusal_detail::{RefusalDetail, append_refusal_detail};

fn decode(status: CommandStatus, payload: Vec<u8>, capabilities: &[u32]) -> Response {
    decode_response(
        &Command {
            action: Action::Use,
            target: Target::Object(WorldObjectTarget {
                placement: vec![1; 40],
                expected_revision: 1,
            }),
            selection: None,
        },
        "Use",
        &GatedCommandResult {
            command_id: 7,
            status,
            server_sequence: 41,
            payload,
        },
        capabilities,
    )
}

#[test]
fn shrine_use_decodes_the_server_window() -> Result<(), String> {
    // Server shape: crates/gameplay/dispatch/src/gameplay_transport/imbuement.rs:352
    // and clear cost: crates/gameplay/items/src/imbuement.rs:24.
    // The dispatch wrapper at imbuement.rs:926 calls these same wire encoders.
    let reply = imbuement::ImbuementReply {
        result: imbuement::ImbuementResult::Ok,
        window: Some(imbuement::ImbuementWindow {
            slot_count: 0,
            applied: Vec::new(),
            offers: Vec::new(),
            clear_gold: 15_000,
        }),
    };
    let capabilities = [
        imbuement::CAPABILITY_IMBUE_V1,
        imbuement::CAPABILITY_IMBUEMENT_SUCCESS_CHANCE_V1,
    ];
    for selected in [&capabilities[..1], &capabilities[..]] {
        let payload = if !selected.contains(&imbuement::CAPABILITY_IMBUEMENT_SUCCESS_CHANCE_V1) {
            imbuement::encode_imbuement_reply(&reply)
        } else {
            imbuement::encode_imbuement_reply_with_success(&reply, &[])
        }
        .map_err(|error| format!("{error:?}"))?;
        assert_eq!(
            decode(CommandStatus::Accepted, payload.clone(), selected),
            Response::Imbuement(ImbueResponse {
                status: CommandStatus::Accepted,
                reply: Some(reply.clone()),
                success: Vec::new(),
            }),
        );
        assert_eq!(
            decode(CommandStatus::Accepted, payload.clone(), &[]),
            Response::Feedback("Use: unknown result".into()),
        );
        assert_eq!(
            decode(CommandStatus::Rejected, payload, selected),
            Response::Feedback("Use refused".into()),
        );
    }
    Ok(())
}

#[test]
fn shrine_use_refusal_keeps_the_existing_feedback() -> Result<(), String> {
    use world_object::UseDisposition;
    let capabilities = [66, imbuement::CAPABILITY_IMBUE_V1];
    for (disposition, status, expected) in [
        (
            UseDisposition::StaleState,
            CommandStatus::Accepted,
            "Use: target changed",
        ),
        (
            UseDisposition::TooFar,
            CommandStatus::Accepted,
            "Too far away.",
        ),
        (
            UseDisposition::RequirementNotMet,
            CommandStatus::Accepted,
            "Use: requirements not met",
        ),
        (
            UseDisposition::Rejected,
            CommandStatus::Rejected,
            "Sorry, not possible.",
        ),
    ] {
        let mut payload =
            world_object::encode_use_result(disposition).map_err(|error| format!("{error:?}"))?;
        append_refusal_detail(
            &mut payload,
            &capabilities,
            RefusalDetail::of_use(disposition),
        );
        assert_eq!(
            decode(status, payload, &capabilities),
            Response::Feedback(expected.into()),
        );
    }
    Ok(())
}
