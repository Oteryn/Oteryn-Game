//! Optional retained-owner fixtures for private native panel captures.
use crate::play::{Outbox, PlayLink};
use oteryn_session::{AppliedDelta, EntityRef, SessionEvent, npc_service};

pub(super) fn configure(
    link: &PlayLink,
    outbox: &Outbox,
) -> Result<Option<&'static str>, Box<dyn std::error::Error>> {
    let Some(panel) = std::env::var_os("OTERYN_BEVY_NATIVE_FIXTURE_PANEL") else {
        return Ok(None);
    };
    let npc = EntityRef {
        identity: [9; 16],
        generation: 2,
    };
    match panel.to_str() {
        Some("quick-loot") => {
            outbox.bevy_quick_loot_selected(true);
            link.discovery_open(98);
            Ok(Some("Loot containers"))
        }
        Some("npc-shop") => {
            outbox.bevy_npc_shop_supported();
            outbox.push(vec![SessionEvent::NpcTradeWindow(AppliedDelta {
                server_sequence: 1,
                base_revision: 0,
                new_revision: 1,
                value: npc_service::NpcTradeWindow {
                    open: Some((npc, 1)),
                    offers: vec![npc_service::NpcOffer {
                        item_definition_ref: 3,
                        side: oteryn_client_ui::npc_shop::NpcTradeSide::Buy,
                        count: 1,
                        unit_price: 12,
                    }],
                },
            })]);
            Ok(Some("Buy"))
        }
        Some("inspection") => {
            use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
            use oteryn_client_ui::inspection::{InspectedItem, Reply, Request, encode_reply};
            link.bevy_item_details_selected(true);
            link.inspection_set_open(true);
            if !link.inspect(Request::Item(7.try_into()?)) {
                return Err("native inspection fixture request refused".into());
            }
            let reply = Reply {
                status: 1,
                requests: vec![],
                catalogue: None,
                items: vec![InspectedItem {
                    slot: 0,
                    definition_ref: 3,
                    count: 3,
                    tier: 1,
                    name: "Fixture sword".into(),
                    attributes: vec![("Attack".into(), "30".into())],
                }],
            };
            link.bevy_item_details_response(&ServerMechanicResponse {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload: encode_reply(&reply)
                    .map_err(|error| format!("native inspection fixture: {error:?}"))?,
            });
            Ok(Some("Inspecting Fixture sword"))
        }
        Some("trade") => {
            use oteryn_client_runtime::player_trade::player_trade::{Offer, State, encode_state};
            let state = State {
                trade_id: Some([3; 16]),
                partner: Some(npc),
                own: Some(Offer {
                    definition: 3,
                    count: 3,
                    sub_type: 0,
                }),
                counter: Some(Offer {
                    definition: 4,
                    count: 2,
                    sub_type: 0,
                }),
                phase: 2,
                reason: 0,
                ..Default::default()
            };
            encode_state(&state).map_err(|error| format!("native trade fixture: {error:?}"))?;
            outbox.bevy_trade_initialize(Some(state));
            Ok(Some("Your offer"))
        }
        _ => Err("unknown native panel fixture".into()),
    }
}
