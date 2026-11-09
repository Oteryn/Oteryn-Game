//! Read-only names and stage descriptions from the exact server-owned Charm catalogue.
use crate::content::charm_source::CanonicalCharmCatalogue;
use oteryn_gameplay_domain::domain::{
    charm::CharmCategory,
    charm_data::{CharmCatalogueRead, CharmStageValue},
};
use oteryn_protocol_oteryn::charm_progression::catalogue::{Card, PAGE_SIZE, Page};
use std::fmt::Write;
#[path = "metadata/description.rs"]
mod description;
pub(super) fn project(catalogue: &CanonicalCharmCatalogue, after: u16) -> Option<Page> {
    let definitions: Vec<_> = catalogue.catalogue().definitions().collect();
    if usize::from(after) > definitions.len() {
        return None;
    }
    let source: serde_json::Value = serde_json::from_slice(catalogue.source_bytes().1).ok()?;
    let records = source.get("records")?.as_array()?;
    let categories = vec!["Major".to_owned(), "Minor".to_owned()];
    let mut cards = Vec::new();
    for (index, definition) in definitions
        .iter()
        .enumerate()
        .skip(usize::from(after))
        .take(PAGE_SIZE)
    {
        let key = definition.key.as_str();
        let name = records.iter().find_map(|record| {
            let d = record.get("definition")?;
            (d.get("identity")?.get("key")?.as_str()? == key).then_some(d.get("name")?.as_str()?)
        })?;
        let effect = catalogue.charm(key)?;
        let (category, currency, stage) = match definition.category {
            CharmCategory::Major => ("Major", "Charm Points", 3),
            CharmCategory::Minor => ("Minor", "Minor Charm Echoes", 1),
        };
        let mut detail = description::describe(effect.effect());
        let _ = write!(
            detail,
            "\r\nAssignment requires discovery stage {stage}.\r\n"
        );
        for stage in 1..=3u8 {
            let value = effect.stage(stage)?.hundredths();
            let label = match effect.effect().stage_value() {
                CharmStageValue::TriggerChancePercent => "trigger chance",
                CharmStageValue::EffectPercent => "effect bonus",
            };
            let _ = write!(
                detail,
                "Stage {stage}: {} {currency}; {label} {}.{:02}%\r\n",
                definition.stage_costs[usize::from(stage - 1)],
                value / 100,
                value % 100
            );
        }
        cards.push(Card {
            race: (index + 1).try_into().ok()?,
            name: name.into(),
            category: category.into(),
            detail,
        });
    }
    let next = (usize::from(after) + cards.len() < definitions.len())
        .then(|| cards.last().map(|c| c.race))
        .flatten();
    Some(Page {
        next,
        categories,
        cards,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn charm_names_and_descriptions_match_the_actual_server_catalogue()
    -> Result<(), Box<dyn std::error::Error>> {
        let catalogue = CanonicalCharmCatalogue::embedded().map_err(|e| format!("{e:?}"))?;
        let mut after = 0;
        let mut rows = Vec::new();
        loop {
            let page = project(&catalogue, after).ok_or("metadata page")?;
            let bytes = oteryn_protocol_oteryn::charm_progression::catalogue::encode_page(&page)
                .map_err(|e| format!("{e:?}"))?;
            assert_eq!(
                oteryn_protocol_oteryn::charm_progression::catalogue::decode_page(&bytes)
                    .map_err(|e| format!("{e:?}"))?,
                page
            );
            rows.extend(page.cards);
            let Some(next) = page.next else { break };
            assert!(next > after);
            after = next;
        }
        assert_eq!(rows.len(), 25);
        assert_eq!(rows.len(), catalogue.catalogue().definitions().count());
        assert!(rows.windows(2).all(|w| w[0].race < w[1].race));
        let dodge = rows.iter().find(|r| r.name == "Dodge").ok_or("Dodge")?;
        assert_eq!(dodge.category, "Major");
        assert!(dodge.detail.contains("avoided entirely"));
        assert!(
            dodge
                .detail
                .contains("Stage 1: 240 Charm Points; trigger chance 5.00%")
        );
        assert!(project(&catalogue, 26).is_none());
        Ok(())
    }
}
