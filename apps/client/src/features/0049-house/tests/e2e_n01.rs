//! N01 replay: a captured house-owner guest-list update and its stale refusal, applied through the
//! production apps/client house path (`house_request` -> `prepare` -> `properties_result` ->
//! client-ui `Panel::apply`). The server-side decoder checks the request the client builds.
use crate::combat_input::PlayCommand;
use crate::play::{mailbox::lock, play_channel};
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
use oteryn_protocol_oteryn::house as wire;

const TOWN: &str = "oteryn:content.area.city.thais";

struct Capture {
    world: [u8; 16],
    house: String,
    member: [u8; 16],
    page: Vec<u8>,
    request: Option<Vec<u8>>,
    acl: Vec<u8>,
    stale: Vec<u8>,
    page_acl_revision: u64,
    acl_revision: u64,
}

fn hex(text: &str) -> Vec<u8> {
    (0..text.len() / 2)
        .filter_map(|at| u8::from_str_radix(text.get(at * 2..at * 2 + 2)?, 16).ok())
        .collect()
}

fn id(text: &str) -> Result<[u8; 16], Box<dyn std::error::Error>> {
    hex(text).try_into().map_err(|_| "capture id is not 16 bytes".into())
}

fn text(hex: &[u8]) -> String {
    hex.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn summary(house: &str, acl_revision: u64) -> wire::HouseSummary {
    wire::HouseSummary { details: None,
        house: house.into(),
        kind: wire::HouseKind::PrivateHouse,
        state: 1,
        rent: 1_000,
        revision: 4,
        acl_revision,
        owned: true,
        move_out_ms: None,
        price: 50_000,
        ends_ms: None,
        own_maximum: Some(50_000),
    }
}

fn load() -> Result<Capture, Box<dyn std::error::Error>> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        let expected = std::fs::read_to_string(root.join("n01-expected.txt"))?;
        let value = |key: &str| -> Result<&str, Box<dyn std::error::Error>> {
            expected
                .lines()
                .find_map(|line| line.strip_prefix(&format!("{key}=")))
                .ok_or_else(|| format!("n01-expected.txt lacks {key}").into())
        };
        return Ok(Capture {
            world: id(value("world")?)?,
            house: value("house")?.into(),
            member: id(value("member")?)?,
            page: std::fs::read(root.join("n01-page.bin"))?,
            request: Some(std::fs::read(root.join("n01-acl-request.bin"))?),
            acl: std::fs::read(root.join("n01-acl.bin"))?,
            stale: std::fs::read(root.join("n01-stale.bin"))?,
            page_acl_revision: value("page_acl_revision")?.parse()?,
            acl_revision: value("acl_revision")?.parse()?,
        });
    }
    let house = "oteryn:house.thais.test_row";
    Ok(Capture {
        world: [0x01, 1, 1, 1, 1, 1, 0x71, 1, 0x81, 1, 1, 1, 1, 1, 1, 1],
        house: house.into(),
        member: [0x02, 2, 2, 2, 2, 2, 0x72, 2, 0x82, 2, 2, 2, 2, 2, 2, 2],
        page: wire::encode_house_page(&[summary(house, 3)], None, true)
            .map_err(|error| format!("{error:?}"))?,
        request: None,
        acl: wire::encode_house_result(wire::HouseDisposition::Ok, 4),
        stale: wire::encode_house_result(wire::HouseDisposition::Conflict, 4),
        page_acl_revision: 3,
        acl_revision: 4,
    })
}

fn reply(payload: &[u8]) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: payload.to_vec(),
    }
}

#[test]
fn e2e_n01_house_owner_guest_update_and_stale_refusal() -> Result<(), Box<dyn std::error::Error>> {
    let capture = load()?;
    let (link, mut receiver, outbox) = play_channel();
    lock(&link.mailbox).properties.house.initialize(true);
    link.house_world(&text(&capture.world));
    link.house_edit(1, TOWN.into());
    link.house_request(0);
    let PlayCommand::Feature(query) = receiver.try_recv()? else {
        return Err("catalogue query was not enqueued".into());
    };
    assert_eq!((query.capability, query.kind), (49, 57));
    outbox.properties_result(&query, &reply(&capture.page));
    let panel = link.house_panel();
    let position = panel
        .model
        .page
        .rows
        .iter()
        .position(|row| row.house == capture.house)
        .ok_or("captured house is not in the client page")?;
    let row = &panel.model.page.rows[position];
    assert!(row.owned);
    assert_eq!(row.acl_revision, capture.page_acl_revision);
    assert_eq!(panel.model.feedback, "House updated");

    let member = text(&capture.member);
    link.house_edit(2, (position + 1).to_string());
    link.house_edit(4, member);
    link.house_edit(6, "CONFIRM".into());
    link.house_request(3);
    let PlayCommand::Feature(command) = receiver.try_recv()? else {
        return Err("guest update was not enqueued".into());
    };
    assert_eq!((command.capability, command.kind), (49, 56));
    if let Some(request) = &capture.request {
        assert_eq!(&command.payload, request, "client request differs from the accepted one");
    }
    assert_eq!(
        wire::decode_house_intent(&command.payload).map_err(|error| format!("{error:?}"))?,
        wire::HouseIntent {
            world: capture.world,
            house: capture.house.clone(),
            action: wire::HouseAction::Acl {
                subowners: false,
                revision: capture.page_acl_revision,
                members: vec![capture.member],
            },
        }
    );
    outbox.properties_result(&command, &reply(&capture.acl));
    let panel = link.house_panel();
    assert!(panel.model.pending.is_none());
    assert_eq!(panel.model.feedback, "House updated");
    assert_eq!(panel.model.page.rows[position].acl_revision, capture.acl_revision);
    assert!(capture.acl_revision > capture.page_acl_revision);

    // The stale refusal (disposition 3) answers a resend of the update and changes nothing.
    link.house_edit(6, "CONFIRM".into());
    link.house_request(3);
    let PlayCommand::Feature(resend) = receiver.try_recv()? else {
        return Err("second guest update was not enqueued".into());
    };
    outbox.properties_result(&resend, &reply(&capture.stale));
    let panel = link.house_panel();
    assert!(panel.model.pending.is_none());
    assert_eq!(
        panel.model.feedback,
        "The server refused the action: catalogue changed; refresh before retrying"
    );
    assert_eq!(panel.model.page.rows[position].acl_revision, capture.acl_revision);
    Ok(())
}
