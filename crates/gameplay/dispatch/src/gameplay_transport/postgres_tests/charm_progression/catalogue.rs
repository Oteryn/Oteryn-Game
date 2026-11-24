//! T080 catalogue queries through the real framed admitted connection.
use super::*;
use oteryn_protocol_oteryn::charm_progression::catalogue::{self as catalogue, Request};
pub(super) async fn verify(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    races: &[BestiaryRace],
) -> TestResult {
    let mut after = 0;
    let mut seen = std::collections::BTreeSet::new();
    let mut first = None;
    loop {
        let payload = catalogue::encode_echo_request(Request {
            after,
            race: 0,
            category: 0,
        })
        .map_err(debug)?;
        let (status, bytes) = exchange_payload(owner, admitted, payload).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let page = catalogue::decode_page(&bytes).map_err(debug)?;
        for card in &page.cards {
            assert!(seen.insert(card.race), "duplicate catalogue entry");
            assert!(!card.name.is_empty());
            assert_eq!(card.echo_warden_completed, Some(false));
            first.get_or_insert(card.race);
        }
        let Some(next) = page.next else { break };
        assert!(next > after);
        after = next;
    }
    assert_eq!(
        seen.len(),
        races.len(),
        "every admitted canonical creature is browsable"
    );
    let payload = catalogue::encode_echo_request(Request {
        after: 0,
        race: first.ok_or("catalogue empty")?,
        category: 0,
    })
    .map_err(debug)?;
    let (status, bytes) = exchange_payload(owner, admitted, payload).await?;
    assert_eq!(status, CommandStatus::Accepted);
    let detail = catalogue::decode_page(&bytes).map_err(debug)?;
    assert_eq!(detail.cards.len(), 1);
    assert!(detail.cards[0].detail.contains("Health:"));
    assert!(detail.cards[0].detail.contains("Loot"));
    let payload = catalogue::encode_echo_request(Request {
        after: 0,
        race: 0,
        category: 1,
    })
    .map_err(debug)?;
    let (status, bytes) = exchange_payload(owner, admitted, payload).await?;
    assert_eq!(status, CommandStatus::Accepted);
    let filtered = catalogue::decode_page(&bytes).map_err(debug)?;
    assert!(!filtered.cards.is_empty());
    assert!(
        filtered
            .cards
            .iter()
            .all(|card| card.category == filtered.categories[0])
    );
    verify_animus(owner, admitted, races).await?;
    eprintln!(
        "POSTGRES_EXECUTED T080 complete catalogue, native combat/loot detail and category filter through framed command84"
    );
    Ok(())
}

/// T185: query on new framed connections after the durable reward and its replay.
pub(super) async fn verify_echo(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    races: &[BestiaryRace],
) -> TestResult {
    // A real durable Warden earning must change only this character/species card.
    let current = crate::gameplay_transport::character_gameplay_fence::current(
        owner, admitted.game_session_id, false,
    ).await.ok_or("current fence")?;
    let occurrence = crate::durability::bestiary_progress::BestiaryKillOccurrence::from_bytes(id(185)).map_err(debug)?;
    let race = 1;
    let species = races[usize::from(race - 1)].key();
    for replayed in [false, true] {
        let reward = owner.root.credit_echo_warden_kill(
            owner.character, owner.holder, current, occurrence, species, 1,
        ).await.map_err(debug)?;
        assert_eq!(reward.replayed, replayed);
        let payload = catalogue::encode_echo_request(Request { after: 0, race, category: 0 }).map_err(debug)?;
        let (status, bytes) = exchange_payload(owner, admitted, payload).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let page = catalogue::decode_page(&bytes).map_err(debug)?;
        assert_eq!(page.cards[0].echo_warden_completed, Some(true));
    }
    let mut stale = current.item_fence();
    stale.character_lease_generation += 1;
    assert!(owner.root.read_current_echo_warden_completions(
        owner.character, owner.holder, stale,
    ).await.is_err());
    let payload = catalogue::encode_echo_request(Request { after: 0, race: 0, category: 0 }).map_err(debug)?;
    let (status, bytes) = exchange_payload(owner, admitted, payload).await?;
    assert_eq!(status, CommandStatus::Accepted);
    let page = catalogue::decode_page(&bytes).map_err(debug)?;
    assert!(page.cards.len() >= 2);
    for card in page.cards { assert_eq!(card.echo_warden_completed, Some(card.race == 1)); }
    // Legacy clients have a strict decoder: neither legacy browse nor named cards gain fields.
    for payload in [catalogue::encode_request(Request { after: 0, race: 0, category: 0 }).map_err(debug)?, vec![255, 3, 1, 0, 2, 0]] {
        let (status, bytes) = exchange_payload(owner, admitted, payload).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let page = catalogue::decode_page(&bytes).map_err(debug)?;
        assert!(page.cards.iter().all(|card| card.echo_warden_completed.is_none()));
    }
    eprintln!("POSTGRES_EXECUTED T185 Echo Warden incomplete/complete, reward replay, opt-in browse/detail and legacy-compatible cards through framed command84");
    Ok(())
}


async fn verify_animus(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    races: &[BestiaryRace],
) -> TestResult {
    use super::super::cyclopedia_fixtures::support::exchange;
    let query = catalogue::encode_request(Request { after: 0, race: 0, category: 0 })
        .map_err(debug)?;
    let id = admitted.continuity.next_command_id;
    assert_eq!(
        exchange(owner, admitted, id, 6389, &query).await?,
        (CommandStatus::Rejected, Vec::new())
    );
    let original = admitted.continuity.selected_capabilities;
    let offered = crate::gameplay_transport::animus_mastery::offered(
        &crate::gameplay_transport::charm_progression::offered(
            owner.offered_capabilities(), owner.charm_progression_available(),
        ), true,
    );
    admitted.continuity.selected_capabilities =
        SelectedCapabilities::select(&offered, &[1, 67, 168, 613]).ok_or("Animus XP selection")?;
    let current = crate::gameplay_transport::character_gameplay_fence::current(
        owner, admitted.game_session_id, false,
    ).await.ok_or("current gameplay fence")?;
    for race in races.iter().skip(1).take(10) {
        assert!(owner.root.complete_animus_mastery(owner.character, owner.holder, current, race.key())
            .await.map_err(debug)?);
    }
    let id = admitted.continuity.next_command_id;
    let (status, payload) = exchange(owner, admitted, id, 6389, &query).await?;
    assert_eq!(status, CommandStatus::Accepted);
    let (page, bonuses) = catalogue::decode_animus_xp_page(&payload).map_err(debug)?;
    assert!(page.cards.len() >= 11);
    for (card, bonus) in page.cards.iter().zip(bonuses) {
        assert_eq!(bonus, if (2..=11).contains(&card.race) { 210 } else { 0 });
    }
    let (status, legacy) = exchange_payload(owner, admitted, query).await?;
    assert_eq!(status, CommandStatus::Accepted);
    assert_eq!(catalogue::decode_page(&legacy).map_err(debug)?, page);
    admitted.continuity.selected_capabilities = original;
    Ok(())
}
