//! E06 replay ownership: `request` and `apply_result` in `protocol.rs` consume the Market wares,
//! book, refusal and fresh-admission My Offers/History replies; `panel_text` and `row_text` in
//! `view.rs` project them.
use super::*;

type Error = Box<dyn std::error::Error>;

struct Capture {
    wares: Vec<u8>,
    book: Vec<u8>,
    refused: Vec<u8>,
    own: Vec<u8>,
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

fn capture() -> Result<Capture, Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok(Capture {
            wares: std::fs::read(root.join("e06-wares.bin"))?,
            book: std::fs::read(root.join("e06-book.bin"))?,
            refused: std::fs::read(root.join("e06-refused.bin"))?,
            own: std::fs::read(root.join("e06-own.bin"))?,
            history: std::fs::read(root.join("e06-history.bin"))?,
            expected: std::fs::read_to_string(root.join("e06-expected.txt"))?,
        });
    }
    let mut offer = [0x66; 16];
    offer[6] = 0x76;
    offer[8] = 0x86;
    let ended = market::MarketRow {
        offer,
        definition: 3031,
        side: 1,
        amount: 1,
        price: 1,
        state: 2,
        anonymous: false,
    };
    let ware = market::MarketRow {
        offer: [0; 16],
        definition: 3031,
        side: 0,
        amount: 0,
        price: 0,
        state: 0,
        anonymous: false,
    };
    let reply = |action, code, rows| market::MarketReply {
        action,
        code,
        offer: [0; 16],
        rows,
    };
    Ok(Capture {
        wares: encode(&reply(1, 1, vec![ware]))?,
        book: encode(&reply(2, 1, vec![]))?,
        refused: encode(&reply(2, 14, vec![]))?,
        own: encode(&reply(3, 1, vec![]))?,
        history: encode(&reply(4, 1, vec![ended]))?,
        expected: format!(
            "definition=3031\noffer={}\nrefusal=14\nfresh_own=0\nhistory_state=2\n",
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
fn e06_real_server_payloads_replay_into_market_panel() -> Result<(), Error> {
    let data = capture()?;
    let definition: u32 = value(&data.expected, "definition")?.parse()?;
    let offer = unhex(value(&data.expected, "offer")?)?;
    let refusal: u8 = value(&data.expected, "refusal")?.parse()?;
    assert_eq!(refusal, 14);
    let mut panel = MarketPanel {
        selected: true,
        ..Default::default()
    };

    request(&mut panel, Intent::Wares).ok_or("wares")?;
    assert!(apply_result(&mut panel, &response(&data.wares)?));
    assert_eq!(panel.rows[0].definition, definition);

    panel.definition = definition;
    let command = request(&mut panel, Intent::Browse).ok_or("browse")?;
    assert_eq!((command.0.action, command.0.definition), (2, definition));
    assert!(apply_result(&mut panel, &response(&data.book)?));
    assert_eq!(panel.view, MarketView::Browse);
    assert!(panel.rows.iter().all(|row| row.definition == definition));

    // The real non-marketable refusal keeps the last rows and shows the reason.
    let shown = panel.rows.clone();
    request(&mut panel, Intent::Browse).ok_or("browse refused")?;
    assert!(!apply_result(&mut panel, &response(&data.refused)?));
    assert_eq!(panel.rows, shown);
    assert_eq!(
        panel.feedback,
        Some("Market refused: item is not marketable")
    );
    assert!(panel_text(&panel).contains("Market refused: item is not marketable"));
    assert!(panel.pending.is_none());

    request(&mut panel, Intent::OwnOffers).ok_or("own")?;
    assert!(apply_result(&mut panel, &response(&data.own)?));
    assert_eq!(panel.view, MarketView::Own);
    assert_eq!(
        panel.rows.len(),
        value(&data.expected, "fresh_own")?.parse::<usize>()?
    );

    request(&mut panel, Intent::History).ok_or("history")?;
    assert!(apply_result(&mut panel, &response(&data.history)?));
    assert_eq!(panel.view, MarketView::History);
    assert_eq!(panel.rows.len(), 1);
    assert_eq!(panel.rows[0].offer, offer);
    assert_eq!(
        panel.rows[0].state,
        value(&data.expected, "history_state")?.parse::<u8>()?
    );
    assert_eq!(
        row_text(&panel.rows[0]),
        format!("Item {definition} · buy · amount 1 · price 1 · total 1")
    );

    // A reply for a different action leaves the history rows unchanged.
    let history = panel.rows.clone();
    request(&mut panel, Intent::History).ok_or("history again")?;
    assert!(!apply_result(&mut panel, &response(&data.own)?));
    assert_eq!(panel.rows, history);
    assert_eq!(
        panel.feedback,
        Some("The market response did not match the request")
    );
    Ok(())
}
