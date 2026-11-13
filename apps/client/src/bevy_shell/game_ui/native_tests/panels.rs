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
        Some("market") => {
            market(link, outbox)?;
            Ok(Some("Buy offers"))
        }
        Some("party") => {
            party(link, outbox)?;
            Ok(Some("Fixture member"))
        }
        Some("vip") => {
            use oteryn_input_actions::{
                ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent,
            };
            outbox.bevy_vip_fixture(
                r#"{"entries":[{"entry":73,"name":"Fixture contact","description":"Friend","icon":2,"notify":true,"groups":[]}],"groups":[],"next_entry":0}"#,
                1,
                oteryn_session::VipPresenceBatch::default(),
            );
            if !link.vip_input(&[NormalizedInputEvent::Key {
                code: KeyCode::new(25)?,
                state: ButtonState::Pressed,
                modifiers: Modifiers::one(Modifier::Control),
                repeat: false,
            }]) {
                return Err("native VIP fixture shortcut refused".into());
            }
            Ok(Some("Fixture contact"))
        }
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

fn market(link: &PlayLink, outbox: &Outbox) -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::market::{market, market_details as wire};
    outbox.bevy_market_initialize(true);
    link.market_set_open(true);
    let catalogue = wire::Query::Catalogue {
        filter: wire::Filter::default(),
        page: 0,
    };
    market_reply(
        link,
        outbox,
        &catalogue,
        &wire::Reply::Catalogue {
            items: vec![wire::Ware {
                definition: 42,
                name: "Fixture sword".into(),
                categories: vec!["Swords".into()],
                vocations: Some(vec![2]),
                level: Some(30),
                forge_class: Some(2),
                depot_units: 4,
            }],
            categories: vec!["Swords".into(), "Shields".into()],
            page: 0,
            has_next: false,
        },
    )?;
    let summary = |price| wire::PriceSummary {
        trades: 1,
        units: 2,
        minimum: Some(price),
        maximum: Some(price),
        average: Some(price),
    };
    let rows = [(1, 100), (2, 120)]
        .into_iter()
        .map(|(side, price)| {
            let mut offer = [7; 16];
            offer[0] = side;
            offer[6] = 0x70;
            offer[8] = 0x80;
            market::MarketRow {
                offer,
                definition: 42,
                side,
                amount: 2,
                price,
                state: 1,
                anonymous: true,
            }
        })
        .collect();
    market_reply(
        link,
        outbox,
        &wire::Query::Book {
            definition: 42,
            page: 0,
        },
        &wire::Reply::Book {
            definition: 42,
            page: 0,
            rows,
            has_next: false,
            statistics: wire::Statistics {
                buy: summary(100),
                sell: summary(120),
                definition: 42,
                since_us: 1,
                as_of_us: 2592000000001,
                trades: 2,
                units: 4,
                minimum: Some(100),
                maximum: Some(120),
                average: Some(110),
            },
        },
    )?;
    let panel = link.market_panel();
    if panel.details.pending.is_some()
        || panel.definition != 42
        || panel.rows.len() != 2
        || panel
            .details
            .statistics
            .as_ref()
            .is_none_or(|stats| stats.trades != 2)
        || panel
            .details
            .items
            .first()
            .is_none_or(|ware| ware.name != "Fixture sword")
    {
        return Err("native market fixture did not reach its retained book owner".into());
    }
    Ok(())
}
fn market_reply(
    link: &PlayLink,
    outbox: &Outbox,
    query: &oteryn_client_runtime::market::market_details::Query,
    reply: &oteryn_client_runtime::market::market_details::Reply,
) -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::{
        market::{market, market_details as wire},
        raw_feature::FeatureCommand,
    };
    if !link.market_query(query.clone()) {
        return Err("native market fixture query refused".into());
    }
    let command = FeatureCommand {
        capability: market::CAPABILITY_MARKET_V1,
        kind: wire::COMMAND_TYPE_MARKET_DETAILS,
        payload: wire::encode_query(query)
            .map_err(|error| format!("native market query: {error:?}"))?,
    };
    outbox.bevy_market_reply(
        &command,
        &oteryn_session::GatedCommandResult {
            command_id: 1,
            server_sequence: 1,
            status: oteryn_session::CommandStatus::Accepted,
            payload: wire::encode_reply(reply)
                .map_err(|error| format!("native market reply: {error:?}"))?,
        },
    );
    Ok(())
}
fn party(link: &PlayLink, outbox: &Outbox) -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_client_runtime::party::{PartyResponse, party};
    outbox.bevy_party_selected(true, true, "Fixture leader");
    link.open_party();
    let member = |handle, name: &str, leader, expires_at| party::PartyEntry {
        handle,
        name: name.into(),
        leader,
        expires_at,
        list: vec![],
        party_spell_remaining_ms: None,
    };
    let view = party::PartyResult {
        disposition: party::PartyDisposition::Ok,
        shared_xp_enabled: true,
        members: vec![
            member(1, "Fixture leader", true, 0),
            member(2, "Fixture member", false, 0),
        ],
        invitations: vec![member(3, "Fixture inviter", false, 4_000_000_000)],
    };
    outbox.bevy_party_response(&PartyResponse {
        status: oteryn_session::CommandStatus::Accepted,
        operation: party::encode_party_result(&party::PartyResult::refused(
            party::PartyDisposition::Ok,
        ))
        .map_err(|error| format!("native party operation: {error:?}"))?,
        view: Some(
            party::encode_party_result(&view)
                .map_err(|error| format!("native party view: {error:?}"))?,
        ),
    });
    let panel = link.party_panel();
    if !panel.open || panel.pending.is_some() || panel.state.as_ref() != Some(&view) {
        return Err("native party fixture did not reach its retained roster owner".into());
    }
    Ok(())
}

#[test]
fn market_and_party_capture_fixtures_reach_accepted_retained_owners()
-> Result<(), Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = crate::play::play_channel();
    link.use_shared_frontend();
    market(&link, &outbox)?;
    assert_eq!(link.market_panel().rows.len(), 2);
    assert!(link.market_panel().details.pending.is_none());
    let (link, _commands, outbox) = crate::play::play_channel();
    link.use_shared_frontend();
    party(&link, &outbox)?;
    assert_eq!(
        link.party_panel()
            .state
            .as_ref()
            .ok_or("party state")?
            .members
            .len(),
        2
    );
    assert!(link.party_panel().pending.is_none());
    Ok(())
}
