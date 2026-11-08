//! P03 replay: server PREY replies applied by the production client reducer `apply_result`
//! (`protocol.rs`), requested through the production `request`. The replies are the Query, a free
//! list reroll with its current-state query, and a refused SelectAny of a non-offered creature.
use super::*;

type Error = Box<dyn std::error::Error>;

struct Captured {
    query: Vec<u8>,
    reroll: Vec<u8>,
    reroll_current: Vec<u8>,
    refused: Vec<u8>,
    refused_current: Vec<u8>,
    expected: String,
}

fn slot(creatures: &[&str], free_reroll_at: u64) -> prey::Slot {
    prey::Slot {
        slot: 1,
        state: 1,
        creatures: creatures.iter().map(|c| (*c).to_owned()).collect(),
        selected: String::new(),
        bonus: 0,
        grade: 0,
        percent: 0,
        hunting_seconds: 0,
        option: 0,
        free_reroll_at,
        chosen_tier: 1,
        allowed_tier: 3,
        selection_epoch: 0,
    }
}

fn reply(status: u8, slot: prey::Slot) -> prey::Reply {
    prey::Reply {
        status,
        wildcard_balance: 4,
        slots: vec![slot],
        list_size: 9,
        expansion_owned: false,
        offered_today: Vec::new(),
        offered_next: String::new(),
    }
}

fn fixture() -> Result<Captured, Error> {
    let encode = |reply: &prey::Reply| prey::encode_reply(reply).map_err(|e| format!("{e:?}"));
    let before = slot(&["oteryn:creature.rat", "oteryn:creature.wolf"], 100);
    let after = slot(
        &[
            "oteryn:creature.bear",
            "oteryn:creature.deer",
            "oteryn:creature.pig",
        ],
        200,
    );
    Ok(Captured {
        query: encode(&reply(0, before.clone()))?,
        reroll: encode(&reply(0, after.clone()))?,
        reroll_current: encode(&reply(0, after.clone()))?,
        refused: encode(&reply(prey::STATUS_NOT_OFFERED, after.clone()))?,
        refused_current: encode(&reply(0, after))?,
        expected: format!(
            "slot=1\nstate_before=1\ncreatures_before=oteryn:creature.rat|oteryn:creature.wolf\nfree_reroll_before=100\nreroll_status=0\nstate_after=1\ncreatures_after=oteryn:creature.bear|oteryn:creature.deer|oteryn:creature.pig\nfree_reroll_after=200\nwildcards=4\nrefusal_status={}\n",
            prey::STATUS_NOT_OFFERED
        ),
    })
}

fn captured() -> Result<Captured, Error> {
    let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return fixture();
    };
    let path = std::path::Path::new(&path);
    let read = |name: &str| std::fs::read(path.join(name));
    Ok(Captured {
        query: read("p03-query.bin")?,
        reroll: read("p03-reroll.bin")?,
        reroll_current: read("p03-reroll-current.bin")?,
        refused: read("p03-refused.bin")?,
        refused_current: read("p03-refused-current.bin")?,
        expected: std::fs::read_to_string(path.join("p03-expected.txt"))?,
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected capture line {key} absent").into())
}

fn creatures(text: &str) -> Vec<String> {
    text.split('|')
        .filter(|c| !c.is_empty())
        .map(str::to_owned)
        .collect()
}

fn answer(
    request: prey::Request,
    payload: &[u8],
    current: Option<&[u8]>,
    status: CommandStatus,
) -> PreyResponse {
    PreyResponse {
        request,
        status,
        payload: payload.to_vec(),
        current: current.map(<[u8]>::to_vec),
    }
}

/// The text the player must see for a Prey refusal status, stated independently of the reducer.
fn refusal_text(status: u8) -> &'static str {
    match status {
        8 => "This creature’s loot is unavailable for Prey.",
        9 => "No complete list of unoffered creatures is available today. No gold was charged.",
        10 => "This difficulty is unavailable at your current level.",
        11 => "This creature has not been offered to you today.",
        12 => "This creature is already in another Prey slot.",
        13 => "Prey List Expansion is already owned.",
        _ => "The server refused this Prey action.",
    }
}

#[test]
fn e2e_p03_replays_query_reroll_and_refusal() -> Result<(), Error> {
    let captured = captured()?;
    let expected = captured.expected.as_str();
    let number: u8 = value(expected, "slot")?.parse()?;
    let mut panel = PreyPanel {
        selected: true,
        ..PreyPanel::default()
    };
    let query = request(&mut panel, Intent::Refresh).ok_or("query request")?;
    assert!(apply_result(
        &mut panel,
        &answer(query, &captured.query, None, CommandStatus::Accepted)
    ));
    let loaded = panel.slot(number).ok_or("slot absent")?;
    assert_eq!(loaded.state.to_string(), value(expected, "state_before")?);
    assert_eq!(
        loaded.creatures,
        creatures(value(expected, "creatures_before")?)
    );
    assert_eq!(
        loaded.free_reroll_at.to_string(),
        value(expected, "free_reroll_before")?
    );
    let before = loaded.clone();

    let reroll = request(&mut panel, Intent::ListReroll { slot: number }).ok_or("reroll")?;
    assert!(apply_result(
        &mut panel,
        &answer(
            reroll,
            &captured.reroll,
            Some(&captured.reroll_current),
            CommandStatus::Accepted
        )
    ));
    let rerolled = panel.slot(number).ok_or("slot absent")?.clone();
    assert_eq!(
        panel
            .confirmed
            .as_ref()
            .map(|r| r.wildcard_balance.to_string())
            .as_deref(),
        Some(value(expected, "wildcards")?)
    );
    if value(expected, "reroll_status")? == "0" {
        assert_eq!(panel.feedback, None);
        assert_eq!(
            rerolled.creatures,
            creatures(value(expected, "creatures_after")?)
        );
        assert_eq!(
            rerolled.free_reroll_at.to_string(),
            value(expected, "free_reroll_after")?
        );
        assert!(rerolled.free_reroll_at > before.free_reroll_at);
        assert!(!rerolled.creatures.is_empty());
    } else {
        assert_eq!(rerolled, before);
    }

    // A refused SelectAny reports the refusal and leaves the slot exactly as it was.
    let refusal: u8 = value(expected, "refusal_status")?.parse()?;
    assert_ne!(refusal, 0);
    let select = request(
        &mut panel,
        Intent::SelectAny {
            slot: number,
            creature: "oteryn:creature.not_a_creature".into(),
        },
    )
    .ok_or("select request")?;
    apply_result(
        &mut panel,
        &answer(
            select,
            &captured.refused,
            Some(&captured.refused_current),
            CommandStatus::Accepted,
        ),
    );
    assert_eq!(panel.feedback, Some(refusal_text(refusal)));
    assert_eq!(panel.slot(number), Some(&rerolled));

    // An undecodable Query reply neither updates the slots nor clears them.
    let query = request(&mut panel, Intent::Refresh).ok_or("query request")?;
    assert!(!apply_result(
        &mut panel,
        &answer(query, &[0xff, 0x01], None, CommandStatus::Accepted)
    ));
    assert_eq!(
        panel.feedback,
        Some("The server returned an invalid Prey reply.")
    );
    assert_eq!(panel.slot(number), Some(&rerolled));
    Ok(())
}
