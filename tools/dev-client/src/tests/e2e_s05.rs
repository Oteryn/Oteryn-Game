//! S05 client replay: the real Thais House catalogue pages and a guildhall refusal enter the
//! native House panel through `request` and `Panel::apply` (`crates/client-ui/src/features/
//! 0049-house/{protocol,state}.rs`), which decode them with `house::page` / `house::result`.
use oteryn_client_runtime::house::{self, Action};
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};
use oteryn_client_ui::house::{Panel, panel_text, request};
use oteryn_protocol_oteryn::house::{
    HouseDisposition, HouseKind, HouseSummary, encode_house_page, encode_house_result,
};

type Error = Box<dyn std::error::Error + Send + Sync>;

const TOWN: &str = "oteryn:content.area.city.thais";

struct Fixture {
    page: Vec<u8>,
    guildhall_page: Vec<u8>,
    refusal: Vec<u8>,
    /// `row` lines (tab separated, `-` for none) then `guildhall=<key>`.
    expected: String,
}

/// A UUIDv7-shaped world id (version 7, RFC variant) the panel accepts.
const WORLD: [u8; 16] = [1, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, 16];

fn row_line(row: &house::Row) -> String {
    let optional = |value: Option<u64>| value.map_or_else(|| "-".to_owned(), |n| n.to_string());
    format!(
        "row\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        row.house,
        row.kind,
        row.state,
        row.rent,
        row.price,
        row.revision,
        row.acl_revision,
        u8::from(row.owned),
        optional(row.maximum),
        optional(row.ends),
        optional(row.notice)
    )
}

fn fixture() -> Result<Fixture, Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok(Fixture {
            page: std::fs::read(root.join("s05-page.bin"))?,
            guildhall_page: std::fs::read(root.join("s05-guildhall-page.bin"))?,
            refusal: std::fs::read(root.join("s05-refusal.bin"))?,
            expected: std::fs::read_to_string(root.join("s05-expected.txt"))?,
        });
    }
    let private = HouseSummary {
        details: None,
        house: "oteryn:house.thais.a".into(),
        kind: HouseKind::PrivateHouse,
        state: 2,
        rent: 1_000,
        revision: 4,
        acl_revision: 2,
        owned: false,
        move_out_ms: None,
        price: 50_000,
        ends_ms: Some(1_700_000_000_000),
        own_maximum: Some(60_000),
    };
    let guildhall = HouseSummary {
        details: None,
        house: "oteryn:house.thais.guild".into(),
        kind: HouseKind::Guildhall,
        state: 1,
        rent: 9_000,
        revision: 6,
        acl_revision: 0,
        owned: false,
        move_out_ms: None,
        price: 400_000,
        ends_ms: None,
        own_maximum: None,
    };
    let page = encode_house_page(&[private.clone(), guildhall.clone()], None, true)
        .map_err(|error| format!("{error:?}"))?;
    let refusal = encode_house_result(HouseDisposition::NotGuildLeader, 0);
    let optional = |value: Option<u64>| value.map_or_else(|| "-".to_owned(), |n| n.to_string());
    let mut expected = String::new();
    for summary in [&private, &guildhall] {
        expected.push_str(&format!(
            "row\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
            summary.house,
            summary.kind as u64,
            summary.state,
            summary.rent,
            summary.price,
            summary.revision,
            summary.acl_revision,
            u8::from(summary.owned),
            optional(summary.own_maximum),
            optional(summary.ends_ms),
            optional(summary.move_out_ms)
        ));
    }
    expected.push_str(&format!("guildhall={}\n", guildhall.house));
    Ok(Fixture {
        guildhall_page: page.clone(),
        page,
        refusal,
        expected,
    })
}

fn reply(payload: Vec<u8>) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 7,
        status: CommandStatus::Accepted,
        server_sequence: 41,
        payload,
    }
}

#[test]
fn e2e_s05_house_catalogue_and_guildhall_refusal_replay() -> Result<(), Error> {
    let Fixture {
        page,
        guildhall_page,
        refusal,
        expected,
    } = fixture()?;
    let expected_rows: Vec<&str> = expected
        .lines()
        .filter(|l| l.starts_with("row\t"))
        .collect();
    let guildhall = expected
        .lines()
        .find_map(|line| line.strip_prefix("guildhall="))
        .ok_or("s05 expected lacks guildhall")?;
    let mut panel = Panel {
        selected: true,
        town: TOWN.into(),
        world: WORLD,
        ..Panel::default()
    };
    let command = request(&mut panel, None, false)?;
    panel.apply(&command, &reply(page));
    assert_eq!(panel.feedback, "House updated");
    let shown: Vec<String> = panel.page.rows.iter().map(row_line).collect();
    assert_eq!(shown, expected_rows);
    let text = panel_text(&panel);
    for row in &panel.page.rows {
        assert!(
            text.contains(&format!(
                "\r\n{}: rent {}, price {}, ends {:?}, maximum {:?}, notice {:?}",
                row.house, row.rent, row.price, row.ends, row.maximum, row.notice
            )),
            "panel text lacks the {} row",
            row.house
        );
    }

    // The guildhall page, then a bid refused by the server: nothing in the panel changes.
    let command = request(&mut panel, None, false)?;
    panel.apply(&command, &reply(guildhall_page));
    panel.index = panel
        .page
        .rows
        .iter()
        .position(|row| row.house == guildhall)
        .ok_or("guildhall row absent from its page")?;
    let before = panel.page.clone();
    let price = panel.page.rows[panel.index].price;
    let command = request(&mut panel, Some(&Action::Bid(price.max(1))), false)?;
    panel.apply(&command, &reply(refusal));
    assert_eq!(panel.feedback, "The server refused the action");
    assert_eq!(panel.page, before);

    // A malformed catalogue payload is refused and leaves the page unchanged.
    let command = request(&mut panel, None, false)?;
    panel.apply(&command, &reply(vec![0xff]));
    assert_eq!(panel.feedback, "Truncated varint");
    assert_eq!(panel.page, before);
    Ok(())
}
