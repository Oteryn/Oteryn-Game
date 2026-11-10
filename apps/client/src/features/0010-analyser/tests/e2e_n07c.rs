//! N07c replay: the analyser facts the server pushed for a real kill, delivered as
//! `SessionEvent::AnalyserFacts` through the production client path (`Outbox::push` to `reduce` in
//! `protocol.rs`, then `oteryn_client_ui::analyser::protocol::apply`) and projected per tool by
//! `select_tool` (`input.rs`) and `panel_text`. The dev client keeps no raw analyser delta, so the
//! capture holds the server-observed facts as text, not bytes.
use crate::play::play_channel;
use oteryn_client_ui::analyser::{Tool, panel_text};
use oteryn_protocol_oteryn::damage_element::DamageElement;
use oteryn_session::{
    AppliedDelta, SessionEvent,
    analyser_transport::{AnalyserDamageSource, AnalyserFact, AnalyserImpact, AnalyserItemCount},
};
use std::num::{NonZeroU16, NonZeroU32};

type Error = Box<dyn std::error::Error>;

fn captured() -> Result<String, Error> {
    let Some(root) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        return Ok("kills=1\nraw_xp=7\ngained_xp=9\ngold=3\nloot_items=2\nfacts=kill|xp 7 9|loot 3 2|supply 1|damage 5|healing 4|received 2|dropped 1\n".into());
    };
    Ok(std::fs::read_to_string(
        std::path::Path::new(&root).join("n07c-expected.txt"),
    )?)
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    expected
        .lines()
        .find_map(|line| line.strip_prefix(&format!("{key}=")))
        .ok_or_else(|| format!("expected capture line {key} absent").into())
}

fn nz32(text: &str) -> Result<NonZeroU32, Error> {
    NonZeroU32::new(text.parse()?).ok_or_else(|| "zero analyser value".into())
}

/// One server-observed fact line, back into the typed fact the session delivered.
fn fact(line: &str) -> Result<AnalyserFact, Error> {
    let parts: Vec<&str> = line.split(' ').collect();
    let item = |count: &str| -> Result<AnalyserItemCount, Error> {
        Ok(AnalyserItemCount {
            item_type: NonZeroU32::MIN,
            count: NonZeroU16::new(count.parse()?).ok_or("zero item count")?,
        })
    };
    Ok(match parts.as_slice() {
        ["kill"] => AnalyserFact::Kill { race: None },
        ["xp", raw, gained] => AnalyserFact::Experience {
            raw: raw.parse()?,
            gained: gained.parse()?,
        },
        ["loot", gold, items] => AnalyserFact::KillLoot {
            race: None,
            corpse_items: if *items == "0" { vec![] } else { vec![item(items)?] },
            gold: gold.parse()?,
        },
        ["supply", count] => AnalyserFact::SupplyUsed(item(count)?),
        ["damage", value] => AnalyserFact::Impact(AnalyserImpact::Damage {
            value: nz32(value)?,
            element: DamageElement::Physical,
        }),
        ["healing", value] => AnalyserFact::Impact(AnalyserImpact::Healing(nz32(value)?)),
        ["received", value] => AnalyserFact::DamageInput {
            value: nz32(value)?,
            element: DamageElement::Physical,
            source: AnalyserDamageSource::None,
        },
        ["dropped", count] => AnalyserFact::Dropped(nz32(count)?),
        _ => return Err(format!("unknown fact line {line}").into()),
    })
}

fn delta(facts: Vec<AnalyserFact>, revision: u64) -> SessionEvent {
    SessionEvent::AnalyserFacts(AppliedDelta {
        server_sequence: revision + 7,
        base_revision: revision - 1,
        new_revision: revision,
        value: facts,
    })
}

#[test]
fn e2e_n07c_replays_kill_facts_into_the_analyser_selector() -> Result<(), Error> {
    let e = captured()?;
    let lines: Vec<&str> = value(&e, "facts")?.split('|').filter(|l| !l.is_empty()).collect();
    let facts = lines.iter().map(|line| fact(line)).collect::<Result<Vec<_>, _>>()?;
    let dropped: u64 = lines
        .iter()
        .filter_map(|line| line.strip_prefix("dropped "))
        .map(str::parse::<u64>)
        .sum::<Result<_, _>>()?;
    let (kills, raw, gained) = (
        value(&e, "kills")?,
        value(&e, "raw_xp")?,
        value(&e, "gained_xp")?,
    );
    let (gold, items) = (value(&e, "gold")?, value(&e, "loot_items")?);

    let (link, _, outbox) = play_channel();
    outbox.analyser_initialize(true);
    // The batch is delivered in two session events, as separate server pushes would be.
    let (head, tail) = facts.split_at(facts.len() / 2);
    outbox.push(vec![delta(head.to_vec(), 2)]);
    outbox.push(vec![delta(tail.to_vec(), 3)]);
    let panel = link.analyser_panel();
    assert_eq!(panel.kills.to_string(), kills);
    assert_eq!(panel.raw_xp.to_string(), raw);
    assert_eq!(panel.gained_xp.to_string(), gained);
    assert_eq!(panel.gold.to_string(), gold);
    assert_eq!(panel.loot_items.to_string(), items);
    assert_eq!(panel.dropped, dropped);

    let select = |index| {
        let mut mailbox = crate::play::mailbox::lock(&link.mailbox);
        crate::play::feature_0010_analyser::input::select_tool(&mut mailbox.analyser, index);
        panel_text(&mailbox.analyser)
    };
    // Each tool shows its title, its totals line and the dropped-facts warning.
    let shows = |text: &str, title: &str, line: &str, warned: bool| {
        let warning = format!("\r\nSession feed incomplete: {dropped} fact(s) dropped");
        text.starts_with(&format!("{title}\r\n"))
            && text.contains(&format!("\r\n{line}"))
            && (text.ends_with(&warning) == warned)
            && (warned || !text.contains("Session feed incomplete"))
    };
    let experience = select(2);
    let line = format!("Experience: {raw} raw / {gained} gained\r\n");
    assert!(shows(&experience, "XP Analyser", &line, true), "{experience}");
    let hunting = select(1);
    let line = format!("Kills: {kills}\r\n");
    assert!(shows(&hunting, "Hunting analyser", &line, true), "{hunting}");
    let loot = select(3);
    let line = format!("Gold: {gold} | Looted items: {items}");
    assert!(shows(&loot, "Loot Analyser", &line, true), "{loot}");
    // Reset (index 7) clears the totals and the warning but keeps the selected tool.
    let reset = select(7);
    assert!(shows(&reset, "Loot Analyser", "Gold: 0 | Looted items: 0", false), "{reset}");
    assert_eq!(link.analyser_panel().tool, Tool::Loot);

    // Facts after the session ended are ignored: the panel is unselected and stays empty.
    outbox.logged_out();
    outbox.push(vec![delta(facts, 4)]);
    let panel = link.analyser_panel();
    assert!(!panel.selected);
    assert_eq!((panel.kills, panel.raw_xp, panel.gold, panel.dropped), (0, 0, 0, 0));
    Ok(())
}
