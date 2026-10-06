//! Generated minimal replies (NPC-0 section 3.3): a fixed Oteryn template filled with data (the
//! NPC name, destination keys and prices). The template is code and the output is data, never
//! wiki text. An admitted Dialogue replaces these replies.
use super::service::NpcServiceEntry;
use crate::content::ProjectV2DefinitionRef;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NpcReplyTemplate {
    Greeting,
    Farewell,
    Trade,
    Travel,
    /// A service with both admitted offers and routes.
    TradeAndTravel,
    /// A service with no admitted offer and no route (empty or fully held).
    Idle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcReplyLine {
    pub template: NpcReplyTemplate,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NpcGeneratedReplies {
    pub greeting: NpcReplyLine,
    pub farewell: NpcReplyLine,
    /// Exactly one line per referenced service, in the NPC's service order; empty and fully held
    /// services are retained with an `Idle` line.
    pub services: Vec<(ProjectV2DefinitionRef, NpcReplyLine)>,
}

/// Display name from the NPC key slug (`oteryn:npc.a_seagull` is `A Seagull`).
pub(super) fn npc_display_name(key: &str) -> String {
    let slug = key.rsplit('.').next().unwrap_or(key);
    slug.split('_')
        .filter(|word| !word.is_empty())
        .map(|word| {
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn route_phrase(key: &str, price: u64) -> String {
    if price == 0 {
        format!("{key} for free")
    } else {
        format!("{key} for {price} gold")
    }
}

pub(super) fn generate<'a>(
    name: &str,
    services: impl Iterator<Item = (&'a ProjectV2DefinitionRef, &'a NpcServiceEntry)>,
) -> NpcGeneratedReplies {
    let mut lines = Vec::new();
    for (reference, entry) in services {
        let routes = entry
            .routes
            .iter()
            .map(|route| route_phrase(&route.key, route.price))
            .collect::<Vec<_>>()
            .join(", ");
        let (template, text) = match (entry.offers.is_empty(), entry.routes.is_empty()) {
            (false, true) => (
                NpcReplyTemplate::Trade,
                format!("{name} buys and sells goods."),
            ),
            (true, false) => (
                NpcReplyTemplate::Travel,
                format!("{name} can take you to {routes}."),
            ),
            (false, false) => (
                NpcReplyTemplate::TradeAndTravel,
                format!("{name} buys and sells goods and can take you to {routes}."),
            ),
            (true, true) => (
                NpcReplyTemplate::Idle,
                format!("{name} has nothing to offer right now."),
            ),
        };
        lines.push((reference.clone(), NpcReplyLine { template, text }));
    }
    NpcGeneratedReplies {
        greeting: NpcReplyLine {
            template: NpcReplyTemplate::Greeting,
            text: format!("Greetings, I am {name}."),
        },
        farewell: NpcReplyLine {
            template: NpcReplyTemplate::Farewell,
            text: "Farewell.".to_owned(),
        },
        services: lines,
    }
}
