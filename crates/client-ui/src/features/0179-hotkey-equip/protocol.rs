use super::HotkeyEquipPanel;
use oteryn_client_runtime::cui3::hotkey_equip::{self, Request};
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
    request(p, command(Request { definition }.encode().to_vec()))
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
    match hotkey_equip::decode_reply(&r.payload) {
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
}
