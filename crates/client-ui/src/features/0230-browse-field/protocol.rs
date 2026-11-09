use super::BrowseFieldPanel;
use oteryn_client_runtime::cui3::{
    browse_field::{self, BrowseFieldIntent},
    item_view::WorldTilePosition,
};
use oteryn_client_runtime::server_mechanic::{
    CommandStatus, ServerMechanicCommand, ServerMechanicResponse,
};
pub fn command(payload: Vec<u8>) -> ServerMechanicCommand {
    ServerMechanicCommand {
        capability: 230,
        command_type: 440,
        payload,
    }
}
pub fn open(p: &mut BrowseFieldPanel) -> Option<ServerMechanicCommand> {
    let i = BrowseFieldIntent::Open {
        position: WorldTilePosition {
            x: p.x,
            y: p.y,
            floor: p.floor,
        },
    };
    let bytes = browse_field::encode_browse_field_intent(&i)
        .ok()
        .or_else(|| {
            p.feedback = Some(tr!("Select a valid tile").into());
            None
        })?;
    request(p, command(bytes))
}
pub fn seek(p: &mut BrowseFieldPanel) -> Option<ServerMechanicCommand> {
    let bytes = browse_field::encode_browse_field_intent(&BrowseFieldIntent::Seek {
        view_id: p.view_id,
        index: p.index,
    })
    .ok()?;
    request(p, command(bytes))
}
pub fn request(
    p: &mut BrowseFieldPanel,
    c: ServerMechanicCommand,
) -> Option<ServerMechanicCommand> {
    if !p.selected || p.pending {
        return None;
    }
    p.pending = true;
    p.feedback = None;
    Some(c)
}
pub fn apply_result(p: &mut BrowseFieldPanel, r: &ServerMechanicResponse) {
    p.pending = false;
    if matches!(
        r.status,
        CommandStatus::Accepted | CommandStatus::DuplicateReplay
    ) {
        p.response.clone_from(&r.payload);
        p.feedback = Some(tr!("Field contents updated by the server").into())
    } else {
        p.feedback = Some(lfmt!("Server refused field action: {:?}", r.status))
    }
}
#[cfg(test)]
// Direct fixture construction keeps the wire assertions readable.
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    #[test]
    fn controls_encode_reply_and_refusal() {
        let mut p = BrowseFieldPanel {
            selected: true,
            x: 100,
            y: 200,
            floor: -7,
            ..Default::default()
        };
        let c = open(&mut p).unwrap();
        assert_eq!(
            browse_field::decode_browse_field_intent(&c.payload).unwrap(),
            BrowseFieldIntent::Open {
                position: WorldTilePosition {
                    x: 100,
                    y: 200,
                    floor: -7
                }
            }
        );
        apply_result(
            &mut p,
            &ServerMechanicResponse {
                command_id: 1,
                status: CommandStatus::Accepted,
                server_sequence: 1,
                payload: vec![1],
            },
        );
        assert_eq!(p.response, [1]);
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
