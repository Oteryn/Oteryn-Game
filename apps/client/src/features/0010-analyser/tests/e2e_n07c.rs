//! N07c replay: the analyser facts the server pushed for a real kill, delivered as
//! legacy or attributed session events through the production client path (`Outbox::push` to `reduce` in
//! `protocol.rs`, then `oteryn_client_ui::analyser::protocol::apply`) and projected per tool by
//! `select_tool` (`input.rs`) and `panel_text`. The dev client keeps no raw analyser delta, so the
//! live capture holds complete re-encoded typed events and negotiated selection as text.
//! Attacker identities are preserved; combat-context/flash coverage is not claimed.
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
        return Ok("capture_kind=synthetic-plain-v1\nattribution_selected=false\nkills=1\nraw_xp=7\ngained_xp=9\ngold=3\nloot_items=2\nfacts=kill|xp 7 9|loot 3 2|supply 1|damage 5|healing 4|received 2|dropped 1\n".into());
    };
    Ok(std::fs::read_to_string(
        std::path::Path::new(&root).join("n07c-expected.txt"),
    )?)
}

fn value<'a>(expected: &'a str, key: &str) -> Result<&'a str, Error> {
    let prefix = format!("{key}=");
    let mut values = expected
        .lines()
        .filter_map(|line| line.strip_prefix(&prefix));
    let found = values
        .next()
        .ok_or_else(|| format!("expected capture line {key} absent"))?;
    if values.next().is_some() {
        return Err(format!("duplicate capture line {key}").into());
    }
    Ok(found)
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
            corpse_items: if *items == "0" {
                vec![]
            } else {
                vec![item(items)?]
            },
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

fn captured_events(e: &str) -> Result<Vec<SessionEvent>, Error> {
    let selected: bool = value(e, "attribution_selected")?.parse()?;
    if value(e, "capture_kind")? == "synthetic-plain-v1" {
        if std::env::var_os("E2E_ALL_CAPTURE_DIR").is_some() || selected {
            return Err("synthetic analyser events cannot stand in for live capture".into());
        }
        let facts = value(e, "facts")?
            .split('|')
            .map(fact)
            .collect::<Result<Vec<_>, _>>()?;
        let (head, tail) = facts.split_at(facts.len() / 2);
        return Ok(vec![delta(head.to_vec(), 2), delta(tail.to_vec(), 3)]);
    }
    if value(e, "capture_kind")? != "typed-events-v1" {
        return Err("unknown analyser capture encoding".into());
    }
    let count: usize = value(e, "event_count")?.parse()?;
    if count == 0 {
        return Err("empty analyser event capture".into());
    }
    (0..count)
        .map(|index| {
            let fields: Vec<_> = value(e, &format!("event_{index}"))?.split(' ').collect();
            let [kind, sequence, base, revision, facts] = fields.as_slice() else {
                return Err("invalid analyser event metadata".into());
            };
            let bytes = facts
                .split(',')
                .map(|hex| {
                    if hex.is_empty() || hex.len() % 2 != 0 || !hex.is_ascii() {
                        return Err("invalid analyser hex".into());
                    }
                    (0..hex.len())
                        .step_by(2)
                        .map(|at| u8::from_str_radix(&hex[at..at + 2], 16).map_err(Into::into))
                        .collect::<Result<Vec<_>, Error>>()
                })
                .collect::<Result<Vec<_>, Error>>()?;
            let server_sequence = sequence.parse()?;
            let base_revision = base.parse()?;
            let new_revision = revision.parse()?;
            if server_sequence == 0 || new_revision <= base_revision {
                return Err("invalid analyser event revisions".into());
            }
            use oteryn_session::analyser_transport::{
                decode_analyser_fact, decode_attributed_analyser_fact,
            };
            match (*kind, selected) {
                ("attributed", true) => Ok(SessionEvent::AnalyserAttributedFacts(AppliedDelta {
                    server_sequence,
                    base_revision,
                    new_revision,
                    value: bytes
                        .iter()
                        .map(|bytes| {
                            decode_attributed_analyser_fact(bytes)
                                .map_err(|error| format!("{error:?}").into())
                        })
                        .collect::<Result<Vec<_>, Error>>()?,
                })),
                ("legacy", false) => Ok(SessionEvent::AnalyserFacts(AppliedDelta {
                    server_sequence,
                    base_revision,
                    new_revision,
                    value: bytes
                        .iter()
                        .map(|bytes| {
                            decode_analyser_fact(bytes).map_err(|error| format!("{error:?}").into())
                        })
                        .collect::<Result<Vec<_>, Error>>()?,
                })),
                _ => Err("analyser event kind disagrees with negotiated selection".into()),
            }
        })
        .collect()
}

#[test]
fn e2e_n07c_replays_kill_facts_into_the_analyser_selector() -> Result<(), Error> {
    let e = captured()?;
    let lines: Vec<&str> = value(&e, "facts")?
        .split('|')
        .filter(|l| !l.is_empty())
        .collect();
    let events = captured_events(&e)?;
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
    // Live captures retain every original typed event and its revision/attribution metadata.
    outbox.push(events.clone());
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
    assert!(
        shows(&experience, "XP Analyser", &line, dropped > 0),
        "{experience}"
    );
    let hunting = select(1);
    let line = format!("Kills: {kills}\r\n");
    assert!(
        shows(&hunting, "Hunting analyser", &line, dropped > 0),
        "{hunting}"
    );
    let loot = select(3);
    let line = format!("Gold: {gold} | Looted items: {items}");
    assert!(shows(&loot, "Loot Analyser", &line, dropped > 0), "{loot}");
    // Reset (index 7) clears the totals and the warning but keeps the selected tool.
    let reset = select(7);
    assert!(
        shows(&reset, "Loot Analyser", "Gold: 0 | Looted items: 0", false),
        "{reset}"
    );
    assert_eq!(link.analyser_panel().tool, Tool::Loot);

    // Facts after the session ended are ignored: the panel is unselected and stays empty.
    outbox.logged_out();
    outbox.push(events);
    let panel = link.analyser_panel();
    assert!(!panel.selected);
    assert_eq!(
        (panel.kills, panel.raw_xp, panel.gold, panel.dropped),
        (0, 0, 0, 0)
    );
    Ok(())
}

#[test]
fn attributed_capture_retains_attacker_and_requires_selection() -> Result<(), Error> {
    use oteryn_session::analyser_transport::encode_attributed_analyser_fact;
    let attacker = oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
        identity: [7; oteryn_protocol_oteryn::world_spatial_entities::ENTITY_IDENTITY_BYTES],
        generation: 9,
    };
    let fact = AnalyserFact::DamageInput {
        value: NonZeroU32::MIN,
        element: DamageElement::Physical,
        source: AnalyserDamageSource::Player,
    };
    // Local codec regression; no bytes or metadata are written into a live capture.
    let bytes = encode_attributed_analyser_fact(&fact, Some(&attacker))
        .map_err(|error| format!("{error:?}"))?;
    let hex: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
    let e = format!(
        "capture_kind=typed-events-v1\nattribution_selected=true\nevent_count=1\nevent_0=attributed 12 3 4 {hex}\n"
    );
    let events = captured_events(&e)?;
    let [SessionEvent::AnalyserAttributedFacts(delta)] = events.as_slice() else {
        return Err("attributed event kind lost".into());
    };
    assert_eq!(
        (
            delta.server_sequence,
            delta.base_revision,
            delta.new_revision
        ),
        (12, 3, 4)
    );
    assert_eq!(delta.value.len(), 1);
    assert_eq!(delta.value[0].fact, fact);
    assert_eq!(delta.value[0].attacker, Some(attacker));
    assert!(captured_events(&e.replace("attribution_selected=true\n", "")).is_err());
    assert!(
        captured_events(&e.replace("attribution_selected=true", "attribution_selected=false"))
            .is_err()
    );
    assert!(
        captured_events(&e.replace("attribution_selected=true", "attribution_selected=unknown"))
            .is_err()
    );
    Ok(())
}
