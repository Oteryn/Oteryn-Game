use super::super::fixture::{Harness, debug};
use super::TestResult;
use super::support::{self, Case, count};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::{self as protocol, browse_field as wire};

#[test]
fn client_e2e_browse_field() -> TestResult {
    support::scenario(Case::BrowseField)
}

pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let position = protocol::item_view::WorldTilePosition {
        x: 7,
        y: 0,
        floor: -7,
    };
    // PlayLink::browse_field_request -> browse_field::command forwards command 440;
    // container events use the session's decode_container_views_for(..., true).
    let payload = wire::encode_browse_field_intent(&wire::BrowseFieldIntent::Open { position })
        .map_err(debug)?;
    let before = count(h, "SELECT count(*) FROM game_item_instances").await?;
    let frames = support::events(
        owner,
        *admitted,
        &[(1, wire::COMMAND_TYPE_BROWSE_FIELD_INTENT, payload)],
    )
    .await?;
    let mut replied = false;
    let mut opened = false;
    for frame in frames {
        let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
        if envelope.message_type() == protocol::MessageType::CommandResult {
            let result = protocol::decode_command_result(envelope.payload()).map_err(debug)?;
            assert_eq!(result.status, protocol::CommandStatus::Accepted);
            assert_eq!(
                protocol::container_tree::decode_container_view_result(result.payload)
                    .map_err(debug)?,
                protocol::container_tree::ContainerViewOutcome::Opened
            );
            replied = true;
        } else if envelope.message_type() == protocol::MessageType::StateDelta {
            let delta = protocol::decode_state_delta(envelope.payload()).map_err(debug)?;
            if delta.domain_id == protocol::container_tree::STATE_DOMAIN_CONTAINER_VIEWS {
                let views =
                    protocol::container_tree::decode_container_views_for(delta.payload, true)
                        .map_err(debug)?;
                let field = views.views.first().ok_or("browse view event")?;
                let page = field.browse.ok_or("browse metadata")?;
                assert_eq!(page.position, position);
                assert_eq!(page.index, 0);
                assert!(page.total > 0);
                assert!(!field.entries.is_empty());
                opened = true;
            }
        }
    }
    assert!(replied && opened);
    assert_eq!(
        count(h, "SELECT count(*) FROM game_item_instances").await?,
        before
    );
    assert_eq!(
        count(h, "SELECT count(*) FROM game_item_loop_move_receipts").await?,
        0
    );
    Ok(())
}
