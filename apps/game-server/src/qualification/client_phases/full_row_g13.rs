//! G13 spellbook details query and spell augments (capabilities 449 and 199, commands 733, 388).
//! 1. Admit the node-boot character; read every spellbook page (command 733) with no filter.
//! 2. Re-query with vocation and premium/level filters; every returned entry must satisfy the
//!    filter and the filtered total must equal the matching count of the unfiltered catalogue.
//! 3. Open the Cyclopedia offence augment page (command 388) and require at least one augment
//!    row from an equipped source; the node-boot character carries none, so the row then fails
//!    naming that precondition. It never equips or grants anything.

use super::*;
use oteryn_dev_client::DevClientSession;
use oteryn_protocol_oteryn::CommandStatus;
use oteryn_protocol_oteryn::cyclopedia_character as cyclopedia;
use oteryn_protocol_oteryn::spellbook_details as wire;

pub(super) async fn run_packet(clients: &SeamClients<'_>) {
    let started = std::time::Instant::now();
    let result = verdict::indexed_timeout(Duration::from_secs(90), verdict::observe(journey(clients))).await;
    let (status, detail) = match result {
        Ok(Ok(())) => (
            "PASS",
            "spellbook-filters-and-augments-validated".to_owned(),
        ),
        Ok(Err(error)) => ("FAIL", compact(&error.to_string())),
        Err(_) => ("FAIL", "step=journey-observed=timeout".to_owned()),
    };
    evidence(&format!(
        "e2e_all scenario=G13.full status={status} seconds={:.3} evidence={detail}",
        started.elapsed().as_secs_f64()
    ));
}

fn compact(value: &str) -> String {
    let joined = value.split_whitespace().collect::<Vec<_>>().join("-");
    joined.chars().take(400).collect()
}

fn gate(client: &DevClientSession, capability: u32) -> TestResult {
    if client.selected_capabilities().contains(&capability) {
        return Ok(());
    }
    Err(format!(
        "step=admit observed=capability-{capability}-not-selected needed=capability-{capability}-selected"
    )
    .into())
}

async fn send(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<(CommandStatus, Vec<u8>)> {
    gate(client, capability)?;
    let reply = client
        .session_mut()
        .gated_command(capability, command, payload)
        .await?;
    client.service_liveness(Duration::from_millis(50)).await?;
    Ok((reply.status, reply.payload))
}

async fn accepted(
    client: &mut DevClientSession,
    capability: u32,
    command: u32,
    payload: &[u8],
) -> TestResult<Vec<u8>> {
    let (status, payload) = send(client, capability, command, payload).await?;
    if status != CommandStatus::Accepted {
        return Err(format!(
            "step=command-{command} observed=status:{status:?} needed=accepted-reply"
        )
        .into());
    }
    Ok(payload)
}

async fn finish(client: &mut DevClientSession) -> TestResult {
    if client.session_mut().logout().await?.result
        != Some(oteryn_protocol_oteryn::logout::LogoutResult::Accepted)
    {
        return Err("step=logout observed=refused".into());
    }
    Ok(())
}

async fn page(client: &mut DevClientSession, query: &wire::Query) -> TestResult<wire::Page> {
    let payload = query
        .encode()
        .ok_or("step=spellbook observed=query-encode")?;
    let reply = accepted(
        client,
        wire::CAPABILITY_SPELLBOOK_DETAILS_V1,
        wire::COMMAND_TYPE_SPELLBOOK_QUERY,
        &payload,
    )
    .await?;
    wire::Page::decode(&reply).ok_or_else(|| "step=spellbook observed=decode-failed".into())
}

fn query(page: u16, vocation: u8, premium: u8, min_level: u32) -> wire::Query {
    wire::Query {
        page,
        vocation,
        premium,
        min_level,
        max_level: 0,
        group: 0,
        search: String::new(),
    }
}

fn page_matches(number: u16, total: Option<u16>, page: &wire::Page, seen: &mut std::collections::BTreeSet<u32>) -> bool {
    page.page == number && total.is_none_or(|known| known == page.total)
        && page.entries.iter().all(|entry| seen.insert(entry.spell.cast_index))
}

fn filter_matches(filter: &wire::Query, catalogue: &[wire::DetailedSpell], total: u16, entries: &[wire::DetailedSpell]) -> bool {
    let expected = catalogue.iter().filter(|entry| filter.matches(entry));
    usize::from(total) == entries.len() && expected.eq(entries)
}

async fn all(
    client: &mut DevClientSession,
    template: &wire::Query,
) -> TestResult<(u16, Vec<wire::DetailedSpell>)> {
    let mut entries = Vec::new();
    let mut number = 0u16;
    let mut total = None;
    let mut seen = std::collections::BTreeSet::new();
    loop {
        let next = wire::Query {
            page: number,
            vocation: template.vocation,
            premium: template.premium,
            min_level: template.min_level,
            max_level: template.max_level,
            group: template.group,
            search: template.search.clone(),
        };
        let current = page(client, &next).await?;
        if !page_matches(number, total, &current, &mut seen) {
            return Err(format!("step=spellbook observed=page:{}-total:{};needed=page:{number}-total:{total:?}-unique-spells", current.page, current.total).into());
        }
        total = Some(current.total);
        entries.extend(current.entries);
        number += 1;
        if usize::from(number) * wire::PAGE_SIZE >= usize::from(current.total) {
            return Ok((current.total, entries));
        }
    }
}

async fn body(client: &mut DevClientSession, _clients: &SeamClients<'_>) -> TestResult {
    let (total, catalogue) = all(client, &query(0, 0, 0, 0)).await?;
    if total == 0 || catalogue.len() != usize::from(total) {
        return Err(format!(
            "step=spellbook observed=total:{total}-entries:{}",
            catalogue.len()
        )
        .into());
    }
    for filter in [query(0, 1, 0, 0), query(0, 0, 1, 0), query(0, 2, 0, 8)] {
        let (filtered, entries) = all(client, &filter).await?;
        if !filter_matches(&filter, &catalogue, filtered, &entries) {
            return Err(format!(
                "step=filter observed=total:{filtered}-entries:{}-expected=matching-catalogue-entries",
                entries.len()
            )
            .into());
        }
    }
    let augment = cyclopedia::offence::Query::default()
        .encode()
        .map_err(|error| format!("step=augments observed=encode-{error:?}"))?;
    let reply = accepted(
        client,
        cyclopedia::CAPABILITY_CYCLOPEDIA_CHARACTER_V1,
        cyclopedia::COMMAND_TYPE_CYCLOPEDIA_CHARACTER_TAB,
        &augment,
    )
    .await?;
    let augments = cyclopedia::offence::Page::decode(&reply)
        .map_err(|error| format!("step=augments observed=decode-{error:?}"))?;
    let rows = augments.facts.as_ref().map_or(0, |facts| facts.rows.len());
    if rows == 0 || augments.total == 0 {
        return Err(format!(
            "step=augments observed=total:{}-rows:{rows} needed=equipped-spell-augment-source",
            augments.total
        )
        .into());
    }
    Ok(())
}

async fn journey(clients: &SeamClients<'_>) -> TestResult {
    let mut client = connect_world(clients, 0x74).await?;
    let outcome = verdict::observe(body(&mut client, clients)).await;
    let closed = verdict::observe(finish(&mut client)).await;
    outcome?;
    closed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spell(cast_index: u32) -> wire::DetailedSpell {
        wire::DetailedSpell {
            spell: oteryn_protocol_oteryn::actor_spell_catalog::SpellCatalogEntry {
                cast_index, name: format!("Spell {cast_index}"), words: "exura".into(), level: 1,
                mana: 1, mana_percent: false, soul: 0, learned: true, available: true,
                cooldown_groups: Vec::new(), rune_item: None,
            },
            vocations: 1, premium: false, magic_level: 0,
        }
    }

    #[test]
    fn spellbook_pages_require_requested_offset_and_unique_spell_ids() {
        let mut seen = std::collections::BTreeSet::from([1]);
        let mut page = wire::Page { page: 0, total: 33, entries: vec![spell(1)] };
        assert!(!page_matches(1, Some(33), &page, &mut seen));
        page.page = 1;
        assert!(!page_matches(1, Some(33), &page, &mut seen));
        page.entries[0] = spell(33);
        assert!(page_matches(1, Some(33), &page, &mut seen));
    }

    #[test]
    fn filtered_spellbook_cannot_substitute_a_duplicate_match() {
        let catalogue = [spell(1), spell(2)];
        let duplicate = [spell(1), spell(1)];
        let filter = wire::Query::default();
        assert!(!filter_matches(&filter, &catalogue, 2, &duplicate));
        assert!(filter_matches(&filter, &catalogue, 2, &catalogue));
    }

    #[test]
    fn compact_is_one_bounded_token() {
        assert_eq!(
            compact("step=a  observed=b\nneeded=c"),
            "step=a-observed=b-needed=c"
        );
        assert!(!compact(&"x ".repeat(500)).contains(' '));
        assert!(compact(&"y".repeat(900)).len() <= 400);
    }
}
