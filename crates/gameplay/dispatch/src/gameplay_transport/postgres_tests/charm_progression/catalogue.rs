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
        let payload = catalogue::encode_request(Request {
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
    let payload = catalogue::encode_request(Request {
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
    let payload = catalogue::encode_request(Request {
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
    eprintln!(
        "POSTGRES_EXECUTED T080 complete catalogue, native combat/loot detail and category filter through framed command84"
    );
    Ok(())
}
