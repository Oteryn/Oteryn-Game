//! Closed W18 furniture-shop greetings; conditional quest writes stay with their owner.
use std::sync::LazyLock;
pub(super) const SOURCE: &str = include_str!("furniture.json");
pub(super) fn get(npc: &str) -> Option<&'static super::Greeting> {
    static DATA: LazyLock<Option<Vec<super::Greeting>>> =
        LazyLock::new(|| super::parse_with_count(SOURCE, 30));
    DATA.as_ref()?.iter().find(|g| g.npc == npc)
}
#[cfg(test)]
mod tests {
    #[test]
    fn srv7_hairycles_greeting_uses_current_ape_city_stage_without_writing_it()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = super::get("oteryn:npc.hairycles").ok_or("Hairycles source")?;
        assert!(super::super::select(&source.npc, None).is_none());
        for stage in [0, 11, 12, 17, 18] {
            let tracks = std::collections::BTreeMap::from([(
                super::super::FURNITURE_TRADE_TRACK.to_owned(),
                stage,
            )]);
            let expected = if stage >= 12 {
                &source
                    .variants
                    .get("ape_friend")
                    .ok_or("Hairycles source variant")?
                    .text
            } else {
                &source.greet.text
            };
            assert_eq!(
                super::super::select(&source.npc, Some(&tracks)),
                Some(expected.as_str())
            );
        }
        Ok(())
    }
}
