//! P02 replay feeds live replies through the production reducer after matching the requests.
use super::*;
use oteryn_client_runtime::raw_feature::{CommandStatus, FeatureCommand, GatedCommandResult};

const BOSS: &str = "oteryn:creature.rotworm_queen";
const RAT: &str = "oteryn:creature.rat";
const BELOW_PROWESS: &str = "oteryn:creature.ferumbras";
type Error = Box<dyn std::error::Error>;

struct Exchange {
    session: [u8; 16],
    request: FeatureCommand,
    reply: GatedCommandResult,
}

fn read_exchange(directory: &std::path::Path, name: &str) -> Result<Exchange, Error> {
    let bytes = std::fs::read(directory.join(name))?;
    let (session, bytes) = bytes
        .split_first_chunk::<16>()
        .ok_or("P02 session missing")?;
    let (command, bytes) = bytes
        .split_first_chunk::<8>()
        .ok_or("P02 command missing")?;
    let (sequence, bytes) = bytes
        .split_first_chunk::<8>()
        .ok_or("P02 sequence missing")?;
    let (status, bytes) = bytes.split_first_chunk::<4>().ok_or("P02 status missing")?;
    let (capability, bytes) = bytes
        .split_first_chunk::<4>()
        .ok_or("P02 capability missing")?;
    let (kind, bytes) = bytes.split_first_chunk::<4>().ok_or("P02 kind missing")?;
    let (length, bytes) = bytes
        .split_first_chunk::<4>()
        .ok_or("P02 request length missing")?;
    let (request, payload) = bytes
        .split_at_checked(usize::try_from(u32::from_be_bytes(*length))?)
        .ok_or("P02 truncated request")?;
    if u32::from_be_bytes(*status) != CommandStatus::Accepted as u32 {
        return Err(format!("P02 capture {name} has non-accepted envelope").into());
    }
    Ok(Exchange {
        session: *session,
        request: FeatureCommand {
            capability: u32::from_be_bytes(*capability),
            kind: u32::from_be_bytes(*kind),
            payload: request.to_vec(),
        },
        reply: GatedCommandResult {
            command_id: u64::from_be_bytes(*command),
            server_sequence: u64::from_be_bytes(*sequence),
            status: CommandStatus::Accepted,
            payload: payload.to_vec(),
        },
    })
}

fn apply_exchange(
    panel: &mut BossEncounterPanel,
    intent: Intent,
    exchange: &Exchange,
) -> Result<(), Error> {
    let actual = request(panel, intent).ok_or("P02 UI command absent")?;
    if actual != exchange.request {
        return Err(format!(
            "P02 UI request mismatch: sent={:?} generated={actual:?}",
            exchange.request
        )
        .into());
    }
    if !apply_result(panel, &exchange.reply) {
        return Err(format!("P02 UI did not apply live reply for {intent:?}").into());
    }
    Ok(())
}

fn catalogue_exchanges(
    directory: &std::path::Path,
    prefix: &str,
    key: &str,
) -> Result<Vec<Exchange>, Error> {
    use oteryn_client_runtime::boss_encounter::catalogue::Page;
    let mut exchanges = Vec::new();
    for index in 0..16 {
        let exchange = read_exchange(directory, &format!("{prefix}-{index}.bin"))?;
        let page = Page::decode(&exchange.reply.payload).ok_or("P02 malformed catalogue page")?;
        exchanges.push(exchange);
        if page.entries.iter().any(|entry| entry.key == key) {
            return Ok(exchanges);
        }
        if page.next.is_none() {
            break;
        }
    }
    Err(format!("P02 catalogue captures missing exact boss {key}").into())
}

fn apply_catalogue(
    panel: &mut BossEncounterPanel,
    search: &str,
    exchanges: &[Exchange],
) -> Result<(), Error> {
    panel.browser_search = search.into();
    for (index, exchange) in exchanges.iter().enumerate() {
        let intent = if index == 0 {
            Intent::Catalogue
        } else {
            Intent::CatalogueNext
        };
        apply_exchange(panel, intent, exchange)?;
    }
    Ok(())
}

fn assert_credit(panel: &BossEncounterPanel) {
    let snapshot = panel.snapshot.as_ref();
    assert_eq!(snapshot.map(|value| value.points), Some(10));
    assert_eq!(
        snapshot.map(|value| value.progress.as_slice()),
        Some([(BOSS.to_owned(), 1)].as_slice())
    );
    let text = panel_text(panel);
    assert!(text.lines().any(|line| line == "Boss points: 10"));
    assert!(
        text.lines()
            .any(|line| line == format!("  {BOSS}: 1 kills"))
    );
}

fn assert_slots(
    panel: &BossEncounterPanel,
    status: u8,
    boss: &str,
    ready: u64,
    unlocked: u8,
    bonus: u16,
    points: u32,
) {
    let slots = panel.slots.as_ref();
    assert_eq!(
        slots.map(|value| (value.status, value.unlocked, value.bonus, value.points)),
        Some((status, unlocked, bonus, points))
    );
    assert_eq!(
        slots.map(|value| value.bosses.as_slice()),
        Some([boss.to_owned(), String::new()].as_slice())
    );
    assert_eq!(slots.map(|value| value.ready_at_ms), Some(ready));
    let label = if status == 1 {
        "Locked or ineligible"
    } else {
        "Updated"
    };
    assert_eq!(slots.map(|value| value.remove_fee), Some(0));
    assert_eq!(
        panel.summary,
        format!(
            "Boss slots: {label}\r\n1: {boss} | 2: \r\nUnlocked mask: {unlocked} | Bonus: {bonus} | Points: {points}\r\nRemoval fee: 0 | Ready at: {ready}"
        )
    );
}

fn assert_malformed_preserves_state(
    panel: &mut BossEncounterPanel,
    slots: &Exchange,
    cooldowns: &Exchange,
) -> Result<(), Error> {
    let (summary, confirmed_slots, snapshot) = (
        panel.summary.clone(),
        panel.slots.clone(),
        panel.snapshot.clone(),
    );
    request(panel, Intent::SlotsQuery).ok_or("malformed slot request")?;
    let invalid_slots = GatedCommandResult {
        payload: vec![9, 9],
        ..slots.reply.clone()
    };
    assert!(!apply_result(panel, &invalid_slots));
    assert_eq!(
        panel.feedback,
        Some("The server returned invalid boss data")
    );
    assert_eq!((&panel.summary, &panel.slots), (&summary, &confirmed_slots));
    request(panel, Intent::Refresh).ok_or("malformed cooldown request")?;
    let invalid_cooldowns = GatedCommandResult {
        payload: vec![8],
        ..cooldowns.reply.clone()
    };
    assert!(!apply_result(panel, &invalid_cooldowns));
    assert_eq!(
        panel.feedback,
        Some("The server returned invalid boss status")
    );
    assert_eq!(panel.snapshot, snapshot);
    Ok(())
}

#[test]
fn e2e_p02_replays_bosstiary_progress_slots_and_refusal() -> Result<(), Error> {
    let Some(directory) = std::env::var_os("E2E_ALL_CAPTURE_DIR") else {
        // The required node-boot replay supplies the new capture directory and checks its marker.
        return Ok(());
    };
    let directory = std::path::Path::new(&directory);
    let locked = read_exchange(directory, "p02-locked.bin")?;
    let cooldowns = read_exchange(directory, "p02-cooldowns.bin")?;
    let slots = read_exchange(directory, "p02-slots.bin")?;
    let refused = read_exchange(directory, "p02-refused.bin")?;
    let ineligible_catalogue =
        catalogue_exchanges(directory, "p02-ineligible-catalogue", BELOW_PROWESS)?;
    let ineligible = read_exchange(directory, "p02-ineligible.bin")?;
    let catalogue = catalogue_exchanges(directory, "p02-catalogue", BOSS)?;
    let ready = read_exchange(directory, "p02-ready.bin")?;
    let invested = read_exchange(directory, "p02-equipped.bin")?;
    let assigned_clock = read_exchange(directory, "p02-assigned-clock.bin")?;
    let mut exchanges = vec![&locked, &cooldowns, &slots, &refused];
    exchanges.extend(&ineligible_catalogue);
    exchanges.push(&ineligible);
    exchanges.extend(&catalogue);
    exchanges.extend([&ready, &invested, &assigned_clock]);
    if cooldowns.session == [0; 16]
        || exchanges.iter().any(|item| {
            item.session != cooldowns.session
                || item.reply.command_id == 0
                || item.reply.server_sequence == 0
        })
        || exchanges.windows(2).any(|pair| {
            pair[0].reply.command_id >= pair[1].reply.command_id
                || pair[0].reply.server_sequence >= pair[1].reply.server_sequence
        })
    {
        return Err("P02 captures do not share an ordered live session".into());
    }

    let mut panel = BossEncounterPanel {
        selected: true,
        ..BossEncounterPanel::default()
    };
    apply_exchange(&mut panel, Intent::SlotsQuery, &locked)?;
    assert_slots(&panel, 0, "", 0, 0, 25, 0);
    assert!(panel.snapshot.is_none());
    apply_exchange(&mut panel, Intent::Refresh, &cooldowns)?;
    assert_credit(&panel);
    assert!(
        panel
            .snapshot
            .as_ref()
            .and_then(|snapshot| snapshot.taints)
            .is_some_and(|taints| (
                taints.soul_war,
                taints.rotten_blood,
                taints.rotten_blood_flags,
                taints.rotten_blood_final
            ) == (0, 0, 0, false))
    );
    apply_exchange(&mut panel, Intent::SlotsDetailed, &slots)?;
    assert_slots(&panel, 0, "", 0, 1, 26, 10);
    let mut empty = panel.slots.clone().ok_or("P02 UI slot state missing")?;
    let detailed = empty.details.take().ok_or("P02 UI slot details missing")?;
    assert_eq!((detailed.base_bonus, detailed.bonuses), (26, [0, 0]));
    assert_ne!(detailed.boosted, BELOW_PROWESS);
    empty.status = 1;
    panel.fields = [RAT.into(), "1".into(), String::new()];
    apply_exchange(&mut panel, Intent::SlotEquip, &refused)?;
    assert_slots(&panel, 1, "", 0, 1, 26, 10);
    assert_eq!(panel.slots.as_ref(), Some(&empty));

    apply_catalogue(&mut panel, "Ferumbras", &ineligible_catalogue)?;
    assert!(
        panel
            .catalogue
            .as_ref()
            .is_some_and(
                |page| page.entries.iter().any(|entry| entry.key == BELOW_PROWESS
                    && entry.category == 3
                    && entry.kills == 0
                    && entry.thresholds[0] == 1
                    && entry.stage() == 0)
            )
    );
    panel.fields[0] = BELOW_PROWESS.into();
    apply_exchange(&mut panel, Intent::SlotEquip, &ineligible)?;
    assert_slots(&panel, 1, "", 0, 1, 26, 10);
    assert_eq!(panel.slots.as_ref(), Some(&empty));

    apply_catalogue(&mut panel, "Rotworm Queen", &catalogue)?;
    apply_exchange(&mut panel, Intent::SlotsDetailed, &ready)?;
    assert_slots(&panel, 0, "", 0, 1, 26, 10);
    assert!(
        super::slot_candidates(&panel)
            .iter()
            .any(|entry| entry.key == BOSS)
    );
    let before = panel
        .slots
        .as_ref()
        .and_then(|state| state.details.as_ref())
        .ok_or("P02 assignment starting timestamp missing")?
        .now;
    panel.fields[0] = BOSS.into();
    apply_exchange(&mut panel, Intent::SlotEquip, &invested)?;
    let equipped = panel.slots.clone().ok_or("P02 assigned slot reply")?;
    assert_slots(&panel, 0, BOSS, equipped.ready_at_ms, 1, 26, 10);
    apply_exchange(&mut panel, Intent::SlotsDetailed, &assigned_clock)?;
    let mut confirmed = panel
        .slots
        .clone()
        .ok_or("P02 post-assignment slot reply")?;
    let details = confirmed
        .details
        .take()
        .ok_or("P02 assignment ending timestamp missing")?;
    assert_ne!(before, 0);
    assert!(
        details
            .now
            .checked_sub(before)
            .is_some_and(|span| span <= 2_000)
    );
    assert!(
        equipped
            .ready_at_ms
            .checked_sub(490)
            .is_some_and(|operation_time| (before..=details.now).contains(&operation_time))
    );
    assert_eq!((details.base_bonus, details.bonuses), (26, [26, 0]));
    assert_eq!(details.boosted, detailed.boosted);
    assert_eq!(confirmed, equipped);
    assert_slots(&panel, 0, BOSS, equipped.ready_at_ms, 1, 26, 10);
    assert_malformed_preserves_state(&mut panel, &slots, &cooldowns)?;
    println!(
        "SEAM_EVIDENCE P02_REPLAY progress=exact requests=matched catalogue=verified refusals=rat,below-prowess assignment=exact cooldown_ms=490 clock=bracketed malformed=2"
    );
    Ok(())
}

#[test]
fn p02_replay_catches_wrong_slot_and_boss_requests() {
    for payload in [
        [&[1, 2][..], RAT.as_bytes()].concat(),
        [&[1, 1][..], BOSS.as_bytes()].concat(),
    ] {
        let mut panel = BossEncounterPanel {
            selected: true,
            fields: [RAT.into(), "1".into(), String::new()],
            ..BossEncounterPanel::default()
        };
        let exchange = Exchange {
            session: [0; 16],
            request: FeatureCommand {
                capability: 117,
                kind: 491,
                payload,
            },
            reply: GatedCommandResult {
                command_id: 1,
                server_sequence: 1,
                status: CommandStatus::Accepted,
                payload: Vec::new(),
            },
        };
        assert!(
            apply_exchange(&mut panel, Intent::SlotEquip, &exchange)
                .is_err_and(|error| error.to_string().contains("request mismatch"))
        );
    }
}
