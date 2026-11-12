//! W22b replay, Prey loot-bonus slice: the server PREY Query reply with an active rat loot-bonus
//! slot and a refused SelectAny of a non-offered creature, applied by the production reducer
//! `apply_result` (`protocol.rs`) after the production `request`. Charm, daily boost, Wealth
//! Duplex and stamina modifiers are not covered by this test.
use super::*;

type Error = Box<dyn std::error::Error>;

struct Captured {
    active: Vec<u8>,
    refused: Vec<u8>,
    refused_current: Vec<u8>,
    expected: String,
}

fn rat_slot() -> prey::Slot {
    prey::Slot {
        slot: 1,
        state: 2,
        creatures: Vec::new(),
        selected: "oteryn:creature.rat".into(),
        bonus: 3,
        grade: 4,
        percent: 20,
        hunting_seconds: 7_200,
        option: 0,
        free_reroll_at: 0,
        chosen_tier: 1,
        allowed_tier: 3,
        selection_epoch: 5,
    }
}

fn reply(status: u8) -> prey::Reply {
    prey::Reply {
        status,
        wildcard_balance: 4,
        slots: vec![rat_slot()],
        list_size: 9,
        expansion_owned: false,
        offered_today: Vec::new(),
        offered_next: String::new(),
    }
}

fn captured() -> Result<Captured, Error> {
    if let Some(path) = std::env::var_os("E2E_ALL_CAPTURE_DIR") {
        let path = std::path::Path::new(&path);
        let read = |name: &str| std::fs::read(path.join(name));
        return Ok(Captured {
            active: read("w22b-active.bin")?,
            refused: read("w22b-refused.bin")?,
            refused_current: read("w22b-refused-current.bin")?,
            expected: std::fs::read_to_string(path.join("w22b-expected.txt"))?,
        });
    }
    let encode = |reply: &prey::Reply| prey::encode_reply(reply).map_err(|e| format!("{e:?}"));
    Ok(Captured {
        active: encode(&reply(0))?,
        refused: encode(&reply(prey::STATUS_NOT_OFFERED))?,
        refused_current: encode(&reply(0))?,
        expected: format!(
            "slot=1\nstate=2\nselected=oteryn:creature.rat\nbonus=3\ngrade=4\npercent=20\nselection_epoch=5\nrefusal_status={}\n",
            prey::STATUS_NOT_OFFERED
        ),
    })
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected capture line {key} absent").into())
}

fn answer(request: prey::Request, payload: &[u8], current: Option<&[u8]>) -> PreyResponse {
    PreyResponse {
        request,
        status: CommandStatus::Accepted,
        payload: payload.to_vec(),
        current: current.map(<[u8]>::to_vec),
    }
}

#[test]
fn e2e_w22b_replays_active_rat_loot_bonus_and_refusal() -> Result<(), Error> {
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
        &answer(query, &captured.active, None)
    ));
    assert_eq!(panel.feedback, None);
    let slot = panel.slot(number).ok_or("slot absent")?.clone();
    assert_eq!(slot.state.to_string(), value(expected, "state")?);
    assert_eq!(slot.selected, value(expected, "selected")?);
    assert_eq!(slot.bonus.to_string(), value(expected, "bonus")?);
    assert_eq!(slot.grade.to_string(), value(expected, "grade")?);
    assert_eq!(slot.percent.to_string(), value(expected, "percent")?);
    assert_eq!(
        slot.selection_epoch.to_string(),
        value(expected, "selection_epoch")?
    );

    // A refused SelectAny reports the refusal and leaves the active slot exactly as it was.
    let refusal: u8 = value(expected, "refusal_status")?.parse()?;
    assert_eq!(refusal, prey::STATUS_NOT_OFFERED);
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
        &answer(select, &captured.refused, Some(&captured.refused_current)),
    );
    assert_eq!(
        panel.feedback,
        Some("This creature has not been offered to you today.")
    );
    assert_eq!(panel.slot(number), Some(&slot));

    // An undecodable Query reply neither updates nor clears the slot.
    let query = request(&mut panel, Intent::Refresh).ok_or("query request")?;
    assert!(!apply_result(
        &mut panel,
        &answer(query, &[0xff, 0x01], None)
    ));
    assert_eq!(
        panel.feedback,
        Some("The server returned an invalid Prey reply.")
    );
    assert_eq!(panel.slot(number), Some(&slot));
    Ok(())
}
