//! W13 replay: the DAILY_BOOST_QUERY reply applied by the production client reducer
//! `apply_result` (`protocol.rs`) and projected by `panel_text` (`view.rs`).
use super::*;
use oteryn_client_runtime::cui3::daily_boost::{DailyBoostView, encode_view};
use oteryn_client_runtime::raw_feature::{CommandStatus, GatedCommandResult};

type Error = Box<dyn std::error::Error>;

fn captured() -> Result<(Vec<u8>, String), Error> {
    if let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let root = std::path::Path::new(&root);
        return Ok((
            std::fs::read(root.join("w13-boost.bin"))?,
            std::fs::read_to_string(root.join("w13-expected.txt"))?,
        ));
    }
    let view = DailyBoostView {
        reset_epoch: 20_388,
        creature_key: "oteryn:creature.rat".into(),
        boss_key: "oteryn:creature.boss_a".into(),
    };
    Ok((
        encode_view(&view).map_err(|error| format!("{error:?}"))?,
        "epoch=20388\ncreature=oteryn:creature.rat\nboss=oteryn:creature.boss_a\n".into(),
    ))
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected line {key} absent").into())
}

fn result(status: CommandStatus, payload: &[u8]) -> GatedCommandResult {
    GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status,
        payload: payload.to_vec(),
    }
}

#[test]
fn e2e_w13_replays_daily_boost_selection_and_refusal() -> Result<(), Error> {
    let (payload, expected) = captured()?;
    let (epoch, creature, boss) = (
        value(&expected, "epoch")?,
        value(&expected, "creature")?,
        value(&expected, "boss")?,
    );
    let mut panel = DailyBoostPanel {
        selected: true,
        ..DailyBoostPanel::default()
    };
    request(
        &mut panel,
        Intent {
            command: 177,
            payload: Vec::new(),
        },
    )
    .ok_or("query request")?;
    assert_eq!(panel_text(&panel), "Loading daily boost...");
    apply_result(&mut panel, &result(CommandStatus::Accepted, &payload));
    let view = panel.boost.clone().ok_or("boost view absent")?;
    assert_eq!(view.reset_epoch.to_string(), epoch);
    assert_eq!(view.creature_key, creature);
    assert_eq!(view.boss_key, boss);
    assert_eq!(panel.feedback, None);
    let cards = spotlight_cards(&panel);
    assert_eq!(cards.len(), 2);
    assert_eq!(
        (cards[0].key.as_str(), cards[1].key.as_str()),
        (creature, boss)
    );
    assert_eq!(
        panel_text(&panel),
        format!(
            "Daily spotlight\nReset epoch: {epoch}\n{}: {}\n{creature}\n{}: {}\n{boss}\n",
            cards[0].kind, cards[0].name, cards[1].kind, cards[1].name
        )
    );
    assert_eq!(
        (cards[0].kind, cards[1].kind),
        ("Boosted creature", "Boosted boss")
    );

    // A refused query (the server rejects a malformed request) keeps the confirmed selection.
    request(
        &mut panel,
        Intent {
            command: 177,
            payload: Vec::new(),
        },
    )
    .ok_or("second query request")?;
    apply_result(&mut panel, &result(CommandStatus::Rejected, &[]));
    assert_eq!(panel.boost, Some(view));
    assert_eq!(panel.feedback, Some("The server refused the action"));
    assert!(!panel.pending);
    Ok(())
}
