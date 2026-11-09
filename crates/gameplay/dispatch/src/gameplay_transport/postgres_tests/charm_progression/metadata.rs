//! T081 names and previews through the negotiated authoritative command, not local indices.
use super::*;
use oteryn_protocol_oteryn::charm_progression::catalogue;
pub(super) async fn verify(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
) -> TestResult<NonZeroU32> {
    let mut after = 0;
    let mut cards = Vec::new();
    loop {
        let request = catalogue::encode_charm_request(after).map_err(debug)?;
        let (status, bytes) = exchange_payload(owner, admitted, request).await?;
        assert_eq!(status, CommandStatus::Accepted);
        let page = catalogue::decode_page(&bytes).map_err(debug)?;
        cards.extend(page.cards);
        let Some(next) = page.next else { break };
        assert!(next > after);
        after = next;
    }
    assert_eq!(cards.len(), 25);
    let dodge = cards
        .iter()
        .find(|c| c.name == "Dodge")
        .ok_or("named Dodge")?;
    assert!(
        dodge
            .detail
            .contains("Stage 1: 240 Charm Points; trigger chance 5.00%")
    );
    assert!(dodge.detail.contains("avoided entirely"));
    eprintln!(
        "POSTGRES_EXECUTED T081 all 25 named charms, actual stage costs/chances and selected Dodge wire mapping"
    );
    NonZeroU32::new(u32::from(dodge.race)).ok_or_else(|| "charm index".into())
}
