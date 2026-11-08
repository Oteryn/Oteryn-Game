//! E05 replay ownership: `request` and `apply_result` in `protocol.rs` consume the Market wares,
//! create, My Offers, cancel and History replies; `row_text` and `panel_text` in `view.rs` project
//! them. The command path (`oteryn_client_runtime::market::send`) decodes the same payloads.
use super::*;

type Error = Box<dyn std::error::Error>;

struct Capture {
    wares: Vec<u8>,
    create: Vec<u8>,
    own: Vec<u8>,
    cancel: Vec<u8>,
    own_after: Vec<u8>,
    history: Vec<u8>,
    expected: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex(text: &str) -> Result<[u8; 16], Error> {
    let mut out = [0; 16];
    for (index, byte) in out.iter_mut().enumerate() {
        *byte = u8::from_str_radix(text.get(index * 2..index * 2 + 2).ok_or("short hex")?, 16)?;
    }
    Ok(out)
}

fn encode(reply: &market::MarketReply) -> Result<Vec<u8>, Error> {
    market::encode_reply(reply).map_err(|error| format!("{error:?}").into())
}

fn row(offer: [u8; 16], state: u8) -> market::MarketRow {
    market::MarketRow {
        offer,
        definition: 3031,
        side: 1,
        amount: 1,
        price: 1,
        state,
        anonymous: false,
    }
}

fn capture() -> Result<Capture, Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok(Capture {
            wares: std::fs::read(root.join("e05-wares.bin"))?,
            create: std::fs::read(root.join("e05-create.bin"))?,
            own: std::fs::read(root.join("e05-own.bin"))?,
            cancel: std::fs::read(root.join("e05-cancel.bin"))?,
            own_after: std::fs::read(root.join("e05-own-after.bin"))?,
            history: std::fs::read(root.join("e05-history.bin"))?,
            expected: std::fs::read_to_string(root.join("e05-expected.txt"))?,
        });
    }
    let mut offer = [0x55; 16];
    offer[6] = 0x75;
    offer[8] = 0x85;
    let reply = |action, offer, rows| market::MarketReply {
        action,
        code: 1,
        offer,
        rows,
    };
    Ok(Capture {
        wares: encode(&reply(
            1,
            [0; 16],
            vec![market::MarketRow {
                offer: [0; 16],
                definition: 3031,
                side: 0,
                amount: 0,
                price: 0,
                state: 0,
                anonymous: false,
            }],
        ))?,
        create: encode(&reply(5, offer, vec![]))?,
        own: encode(&reply(3, [0; 16], vec![row(offer, 1)]))?,
        cancel: encode(&reply(7, [0; 16], vec![]))?,
        own_after: encode(&reply(3, [0; 16], vec![]))?,
        history: encode(&reply(4, [0; 16], vec![row(offer, 2)]))?,
        expected: format!(
            "definition=3031\noffer={}\nside=1\namount=1\nprice=1\nown_after_cancel=0\nhistory_state=2\n",
            hex(&offer)
        ),
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(key)?.strip_prefix('='))
        .ok_or_else(|| format!("expected.txt lacks {key}").into())
}

fn response(payload: &[u8]) -> Result<MarketResponse, Error> {
    Ok(MarketResponse {
        status: CommandStatus::Accepted,
        reply: Some(market::decode_reply(payload).map_err(|error| format!("{error:?}"))?),
    })
}

#[test]
fn e05_real_server_payloads_replay_into_market_panel() -> Result<(), Error> {
    let data = capture()?;
    let definition: u32 = value(&data.expected, "definition")?.parse()?;
    let offer = unhex(value(&data.expected, "offer")?)?;
    let state: u8 = value(&data.expected, "history_state")?.parse()?;
    let mut panel = MarketPanel {
        selected: true,
        ..Default::default()
    };

    let command = request(&mut panel, Intent::Wares).ok_or("wares")?;
    assert_eq!(command.0.action, 1);
    assert!(apply_result(&mut panel, &response(&data.wares)?));
    assert_eq!(panel.view, MarketView::Browse);
    assert_eq!(panel.rows.first().map(|r| r.definition), Some(definition));
    assert_eq!(
        row_text(&panel.rows[0]),
        format!("Item {definition} · select and Browse to see offers")
    );

    panel.definition = definition;
    panel.sell = false;
    let command = request(&mut panel, Intent::CreateOffer).ok_or("create")?;
    assert_eq!(
        (
            command.0.action,
            command.0.definition,
            command.0.amount,
            command.0.price
        ),
        (5, definition, 1, 1)
    );
    assert!(apply_result(&mut panel, &response(&data.create)?));
    assert_eq!(
        market::decode_reply(&data.create)
            .map_err(|e| format!("{e:?}"))?
            .offer,
        offer
    );

    request(&mut panel, Intent::OwnOffers).ok_or("own")?;
    assert!(apply_result(&mut panel, &response(&data.own)?));
    assert_eq!(panel.view, MarketView::Own);
    assert_eq!(panel.rows.len(), 1);
    assert_eq!(panel.rows[0].offer, offer);
    assert_eq!(
        row_text(&panel.rows[0]),
        format!("Item {definition} · buy · amount 1 · price 1 · total 1")
    );

    panel.selected_offer = Some(offer);
    let command = request(&mut panel, Intent::CancelOffer).ok_or("cancel")?;
    assert_eq!((command.0.action, command.0.offer), (7, offer));
    assert!(apply_result(&mut panel, &response(&data.cancel)?));
    assert_eq!(panel.selected_offer, None);

    request(&mut panel, Intent::OwnOffers).ok_or("own after")?;
    assert!(apply_result(&mut panel, &response(&data.own_after)?));
    assert_eq!(
        panel.rows.len(),
        value(&data.expected, "own_after_cancel")?.parse::<usize>()?
    );

    request(&mut panel, Intent::History).ok_or("history")?;
    assert!(apply_result(&mut panel, &response(&data.history)?));
    assert_eq!(panel.view, MarketView::History);
    assert_eq!((panel.rows[0].offer, panel.rows[0].state), (offer, state));

    // A reply for the wrong action, and a refused reply, leave the shown rows unchanged.
    let shown = panel.rows.clone();
    request(&mut panel, Intent::Browse).ok_or("browse")?;
    assert!(!apply_result(&mut panel, &response(&data.own_after)?));
    assert_eq!(panel.rows, shown);
    assert_eq!(
        panel.feedback,
        Some("The market response did not match the request")
    );
    request(&mut panel, Intent::History).ok_or("history again")?;
    let refused = encode(&market::MarketReply {
        action: 4,
        code: 14,
        offer: [0; 16],
        rows: vec![],
    })?;
    assert!(!apply_result(&mut panel, &response(&refused)?));
    assert_eq!(panel.rows, shown);
    assert_eq!(
        panel.feedback,
        Some("Market refused: item is not marketable")
    );
    Ok(())
}
