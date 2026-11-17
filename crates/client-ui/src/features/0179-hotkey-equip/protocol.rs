use super::HotkeyEquipPanel;
use oteryn_client_runtime::cui3::item_view::ItemMoveOutcome;
use oteryn_client_runtime::cui3::{hotkey_equip, refusal_detail};
use oteryn_client_runtime::server_mechanic::{
    CommandStatus, ServerMechanicCommand, ServerMechanicResponse,
};
use std::num::NonZeroU32;
pub fn command(payload: Vec<u8>) -> ServerMechanicCommand {
    ServerMechanicCommand {
        capability: 179,
        command_type: 348,
        payload,
    }
}
pub fn control_request(p: &mut HotkeyEquipPanel) -> Option<ServerMechanicCommand> {
    let definition = NonZeroU32::new(p.item_definition).or_else(|| {
        p.feedback = Some(tr!("Select an item").into());
        None
    })?;
    let tier = std::mem::take(&mut p.upgrade_tier);
    let Some(payload) = crate::hotkey_equip_upgrade_tier::equip_payload(definition, p.tiered, tier)
    else {
        p.feedback = Some(tr!("Select an item").into());
        return None;
    };
    request(p, command(payload))
}
pub fn request(
    p: &mut HotkeyEquipPanel,
    c: ServerMechanicCommand,
) -> Option<ServerMechanicCommand> {
    if !p.selected || p.pending {
        return None;
    }
    p.pending = true;
    p.feedback = None;
    Some(c)
}
pub fn apply_result(p: &mut HotkeyEquipPanel, r: &ServerMechanicResponse) {
    p.pending = false;
    if !matches!(
        r.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        p.feedback = Some(lfmt!("Server refused hotkey equip: {:?}", r.status));
        return;
    }
    // Capability 66 appends field 15 to a refusal; the outcome codec still refuses it.
    let decoded = refusal_detail::split_refusal_detail(&r.payload)
        .ok()
        .and_then(|(payload, detail)| {
            let outcome = hotkey_equip::decode_reply(payload).ok()?;
            (outcome != ItemMoveOutcome::Moved || detail.is_none()).then_some(outcome)
        })
        .ok_or(());
    match decoded {
        Ok(v) => {
            p.outcome = Some(v);
            p.feedback = None
        }
        Err(_) => p.feedback = Some(tr!("The server returned an invalid equip result").into()),
    }
}
#[cfg(test)]
// Direct fixture construction keeps the wire assertions readable.
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    #[test]
    fn controls_encode_reply_and_refusal() {
        let mut p = HotkeyEquipPanel {
            selected: true,
            item_definition: 256,
            ..Default::default()
        };
        let c = control_request(&mut p).unwrap();
        assert_eq!(c.payload, [0, 0, 1, 0]);
        apply_result(
            &mut p,
            &ServerMechanicResponse {
                command_id: 1,
                status: CommandStatus::Accepted,
                server_sequence: 1,
                payload: vec![8, 1],
            },
        );
        assert!(p.outcome.is_some());
        apply_result(
            &mut p,
            &ServerMechanicResponse {
                command_id: 2,
                status: CommandStatus::Rejected,
                server_sequence: 2,
                payload: vec![],
            },
        );
        assert!(p.feedback.unwrap().contains("refused"));
    }

    #[test]
    fn refusal_detail_suffix_is_split_before_the_outcome_decodes() {
        // A bundle World selects capability 66: the server appends field 15 to a refusal.
        let mut payload = vec![8, 10];
        oteryn_client_runtime::cui3::refusal_detail::append_refusal_detail(
            &mut payload,
            &[oteryn_client_runtime::cui3::refusal_detail::CAPABILITY_REFUSAL_DETAIL_V1],
            Some(oteryn_client_runtime::cui3::refusal_detail::RefusalDetail::NotPossible),
        );
        let mut p = HotkeyEquipPanel {
            selected: true,
            item_definition: 256,
            ..Default::default()
        };
        control_request(&mut p).unwrap();
        apply_result(
            &mut p,
            &ServerMechanicResponse {
                command_id: 1,
                status: CommandStatus::Accepted,
                server_sequence: 1,
                payload,
            },
        );
        assert_eq!(
            p.outcome,
            Some(oteryn_client_runtime::cui3::item_view::ItemMoveOutcome::SlotMismatch)
        );
    }
}
