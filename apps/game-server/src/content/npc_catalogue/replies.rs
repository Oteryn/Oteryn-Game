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
    /// One line per admitted service and kind, in the NPC's service order.
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
        if !entry.offers.is_empty() {
            lines.push((
                reference.clone(),
                NpcReplyLine {
                    template: NpcReplyTemplate::Trade,
                    text: format!("{name} buys and sells goods."),
                },
            ));
        }
        if !entry.routes.is_empty() {
            let routes = entry
                .routes
                .iter()
                .map(|route| route_phrase(&route.key, route.price))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push((
                reference.clone(),
                NpcReplyLine {
                    template: NpcReplyTemplate::Travel,
                    text: format!("{name} can take you to {routes}."),
                },
            ));
        }
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
