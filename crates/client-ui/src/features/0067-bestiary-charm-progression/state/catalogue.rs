use oteryn_client_runtime::charm_progression::catalogue::{Page, Request};
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Catalogue {
    pub open: bool,
    pub detail_card: Option<oteryn_client_runtime::charm_progression::catalogue::Card>,
    pub page: Page,
    pub detail: String,
    pub category: u8,
    pub next_loot: Option<u16>,
    pub pending: Option<Request>,
}
pub fn text(panel: &super::Panel) -> String {
    use std::fmt::Write;
    let catalogue = &panel.catalogue;
    let category = catalogue
        .category
        .checked_sub(1)
        .and_then(|i| catalogue.page.categories.get(usize::from(i)))
        .map_or("All categories", String::as_str);
    let mut out = format!("Creature catalogue — {category}\r\n{}\r\n", panel.feedback);
    if panel.pending {
        out.push_str("Waiting for the server\r\n");
    }
    for (index, category) in catalogue.page.categories.iter().enumerate() {
        let _ = writeln!(out, "Category {}: {category}\r", index + 1);
    }
    if catalogue.page.cards.is_empty() {
        out.push_str("No creatures on this page. Browse or change category.\r\n");
    }
    for card in &catalogue.page.cards {
        let _ = writeln!(
            out,
            "{}Race {}: {} [{}]\r",
            if panel.fields[1] == card.race.to_string() {
                "> "
            } else {
                ""
            },
            card.race,
            card.name,
            card.category
        );
    }
    if catalogue.page.next.is_some() {
        out.push_str("Next catalogue page available\r\n");
    }
    out.push_str(&catalogue.detail);
    if catalogue.next_loot.is_some() {
        out.push_str("\r\nMore loot entries available");
    }
    out
}

impl Catalogue {
    pub fn selected_card(
        &self,
        race: &str,
    ) -> Option<&oteryn_client_runtime::charm_progression::catalogue::Card> {
        self.detail_card
            .as_ref()
            .filter(|c| c.race.to_string() == race)
            .or_else(|| self.page.cards.iter().find(|c| c.race.to_string() == race))
    }
}
