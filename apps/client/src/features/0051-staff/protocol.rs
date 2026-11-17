//! Existing GM-1 transport. This client never signs an action or grants staff rights.
use super::state::Panel;
use oteryn_client_runtime::raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult};
use oteryn_session::property_services::staff as wire;

pub(super) fn command(panel: &Panel, receipt: bool) -> Result<FeatureCommand, &'static str> {
    let signature = panel.fields[2].as_bytes();
    if signature.is_empty()
        || signature.len() > wire::MAX_SIGNATURE_BYTES * 2
        || !signature.len().is_multiple_of(2)
    {
        return Err(tr!("Invalid authorization signature"));
    }
    let signature = signature
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| {
            let digit = |b: u8| (b as char).to_digit(16).map(|n| n as u8);
            Some(digit(pair[0])? * 16 + digit(pair[1])?)
        })
        .collect::<Option<Vec<_>>>()
        .ok_or(tr!("Invalid authorization signature"))?;
    let envelope = wire::SignedEnvelope {
        principal: panel.fields[0].clone(),
        // Preserve the signed bytes exactly, including whitespace. Never reserialize.
        canonical: panel.fields[1].as_bytes().to_vec(),
        signature,
    };
    if panel.tools {
        return oteryn_client_ui::staff_tools::request(&envelope, receipt)
            .map_err(|_| tr!("Invalid authorized action"));
    }
    oteryn_client_ui::staff::request(&envelope, receipt)
        .map_err(|_| tr!("Invalid authorized action"))
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
    if !matches!(
        result.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        panel.feedback =
            tr!("Action not confirmed; check the administrator ledger before retrying").into();
        return;
    }
    let decoded = if panel.tools {
        oteryn_client_ui::staff_tools::decode_reply(&result.payload)
            .map(|reply| (reply.result, reply.info))
    } else {
        oteryn_client_ui::staff::decode_result(&result.payload).map(|reply| (reply, String::new()))
    };
    let Ok((reply, info)) = decoded else {
        panel.feedback =
            tr!("Invalid staff reply; check the administrator ledger before retrying").into();
        return;
    };
    let code = match reply.code {
        wire::StaffCode::Ok => {
            if reply.disposition == 3 {
                tr!("Signed receipt stored")
            } else {
                tr!("Audited action confirmed")
            }
        }
        wire::StaffCode::RoleRevoked => tr!("Staff authorization revoked"),
        wire::StaffCode::TierInsufficient => tr!("Insufficient staff permission"),
        wire::StaffCode::TargetNotFound => tr!("Character not found"),
        wire::StaffCode::TargetNotInChannel => tr!("Character is not in this channel"),
        wire::StaffCode::Stale => tr!("Authorization is stale"),
        wire::StaffCode::AlreadyReleased => tr!("Character already released"),
        wire::StaffCode::RateLimited => tr!("Staff action rate limit reached"),
        wire::StaffCode::AlreadyBanned => tr!("This account is already banned"),
    };
    panel.feedback = if info.is_empty() { code.into() } else { info };
    panel.summary = lfmt!(
        "Control sequence {}; mute until {} ms",
        reply.control_seq,
        reply.mute_until_ms
    );
    // Authorization material is not retained after a confirmed result.
    panel.fields = Default::default();
}

// A guarded arm precedes the shared raw-feature fallback without editing its owner.
macro_rules! dispatch {
    ($command:expr, $session:ident, $outbox:ident, $held:ident, $publish:ident; $($arms:tt)*) => {
        super::feature_0037_depot::protocol::dispatch! {
            $command, $session, $outbox, $held, $publish;
            Ok(crate::combat_input::PlayCommand::Feature(command)) if matches!(command.capability, 51 | 330) => {
                match $session.feature(&command).await {
                    Ok(result) => {
                        $outbox.properties_result(&command, &result);
                        $publish(&mut $session, &$outbox, &mut $held);
                        continue;
                    }
                    Err(error) => error,
                }
            }
            $($arms)*
        }
    };
}
pub(in crate::play) use dispatch;
