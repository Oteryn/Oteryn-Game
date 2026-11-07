use super::super::fixture::{Harness, debug};
use super::TestResult;
use super::support::{self, Case, count};
use crate::gameplay_transport::{ComposedFreshAdmission, connection::AdmittedSession};
use oteryn_protocol_oteryn::{self as protocol, hotkey_equip as wire};

#[test]
fn client_e2e_hotkey_equip() -> TestResult {
    support::scenario(Case::HotkeyEquip)
}

pub(super) async fn exercise(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult {
    let key = "oteryn:item.tibia.i3264";
    super::super::item_forge::support::seed_items(h, &[(90, key, 1, 0)]).await?;
    let definition = owner
        .active_generation
        .and_then(|g| g.native_gameplay())
        .and_then(|n| n.item_index())
        .and_then(|index| index.definition_ref("Item", key, "definition-r1"))
        .ok_or("sword definition")?;
    // PlayLink::hotkey_equip_request -> hotkey_equip::command forwards these four
    // bytes. The session result codec is hotkey_equip::decode_reply.
    let payload = wire::Request { definition }.encode().to_vec();
    let commands = [
        (1, wire::COMMAND_TYPE_HOTKEY_EQUIP, payload.clone()),
        (1, wire::COMMAND_TYPE_HOTKEY_EQUIP, payload),
    ];
    let frames = support::events(owner, *admitted, &commands).await?;
    let mut replies = 0;
    let mut equipped = false;
    for frame in frames {
        let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
        match envelope.message_type() {
            protocol::MessageType::CommandResult => {
                let result = protocol::decode_command_result(envelope.payload()).map_err(debug)?;
                assert!(matches!(
                    result.status,
                    protocol::CommandStatus::Accepted | protocol::CommandStatus::DuplicateReplay
                ));
                assert_eq!(
                    wire::decode_reply(result.payload).map_err(debug)?,
                    protocol::item_view::ItemMoveOutcome::Moved
                );
                replies += 1;
            }
            protocol::MessageType::StateDelta => {
                let delta = protocol::decode_state_delta(envelope.payload()).map_err(debug)?;
                if delta.domain_id == protocol::item_view::STATE_DOMAIN_CHARACTER_INVENTORY {
                    let inventory =
                        protocol::item_view::decode_character_inventory_with_equip_drop(
                            delta.payload,
                            true,
                        )
                        .map_err(debug)?;
                    equipped |= inventory
                        .equipment
                        .iter()
                        .any(|entry| entry.item.item_definition_ref == definition);
                }
            }
            _ => {}
        }
    }
    assert_eq!(replies, 2);
    assert!(equipped, "client inventory event equips the sword");
    assert_eq!(
        count(h, "SELECT count(*) FROM game_character_equipment_slots").await?,
        1
    );
    assert_eq!(count(h,"SELECT count(*) FROM game_item_container_entries e JOIN game_item_instances i USING(item_instance_id) WHERE i.definition_production_key='oteryn:item.tibia.i3264'").await?,0);
    assert_eq!(
        count(h, "SELECT count(*) FROM game_character_equipment_receipts").await?,
        1
    );
    Ok(())
}
