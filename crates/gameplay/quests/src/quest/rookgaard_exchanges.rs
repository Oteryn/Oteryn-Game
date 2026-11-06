//! Repeatable Rookgaard exchanges and Sinatuki's cumulative fish hand-ins. Crystal201189f NPC scripts and the pinned
//! TibiaWiki Short Sword, Small Health Potion, Studded Shield and Studded Legs walkthroughs.

pub struct Exchange {
    pub npc: &'static str,
    pub keywords: &'static [&'static [&'static str]],
    pub input: &'static str,
    pub output: u32,
    pub prompt: Option<&'static str>,
    pub success: &'static str,
    pub missing: &'static str,
    pub declined: &'static str,
}

const EXCHANGES: &[Exchange] = &[
    Exchange {
        npc: "oteryn:npc.amber",
        keywords: &[&["book"], &["notebook"]],
        input: "oteryn:item.tibia.i2821",
        output: 3294,
        prompt: Some("Do you bring me my notebook?"),
        success: "Excellent. Here, take this short sword as a reward.",
        missing: "Mhm, whatever you have there, it is not my notebook.",
        declined: "Too bad.",
    },
    Exchange {
        npc: "oteryn:npc.billy",
        keywords: &[&["pan"]],
        input: "oteryn:item.tibia.i3466",
        output: 7876,
        prompt: Some("Have you found a pan for me?"),
        success: "A pan! At last! Take this in case you eat something my cousin has cooked.",
        missing: "Hey! You don't have one!",
        declined: "Then go and look for one!",
    },
    Exchange {
        npc: "oteryn:npc.lee_delle",
        keywords: &[&["honey", "flower"]],
        input: "oteryn:item.tibia.i2984",
        output: 3362,
        prompt: None,
        success: "Oh, thank you so much! Please take this piece of armor as reward.",
        missing: "Honey flowers are my favourites <sighs>.",
        declined: "Honey flowers are my favourites <sighs>.",
    },
    // Wiki Sinatuki Quest spoiler (pinned in .questpc3/wiki): first 99 fish
    // may be mixed species, and only a later stack earns the final reward.
    Exchange {
        npc: "oteryn:npc.sinatuki",
        keywords: &[&["nupi"]],
        input: "oteryn:item.tibia.i7159",
        output: 7290,
        prompt: None,
        success: "Jinuma, suvituka siq chuqua!! Nguraka, nguraka! <happily takes the food from you>",
        missing: "Kisavuta! <giggles>",
        declined: "Kisavuta! <giggles>",
    },
    Exchange {
        npc: "oteryn:npc.willie",
        keywords: &[&["banana"]],
        input: "oteryn:item.tibia.i3587",
        output: 3426,
        prompt: Some("Have you found a banana for me?"),
        success: "A banana! Great. Here, take this shield, I don't need it anyway.",
        missing: "Are you trying to mess with me?!",
        declined: "Too bad.",
    },
];

pub fn exchange(npc: &str) -> Option<&'static Exchange> {
    EXCHANGES.iter().find(|entry| entry.npc == npc)
}

impl Exchange {
    /// The source accepts either fish species; other NPCs retain their exact leaf.
    pub fn accepts_input(&self, key: &str) -> bool {
        key == self.input || (self.npc == "oteryn:npc.sinatuki" && key == "oteryn:item.tibia.i7158")
    }

    pub fn offered_by(&self, text: &str) -> bool {
        let text = text.to_ascii_lowercase();
        !preceding_keyword(self.npc, &text)
            && self
                .keywords
                .iter()
                .any(|keywords| ordered(&text, keywords))
    }

    pub fn confirmed_by(&self, text: &str) -> bool {
        if self.prompt.is_some() {
            text.to_ascii_lowercase().contains("yes")
        } else {
            self.offered_by(text)
        }
    }
}

// Crystal201189f data/npclib/npc_system/keyword_handler.lua:42-75:
// each keyword's first substring occurrence must be in registration order.
// This is KeywordNode matching, distinct from the default callback's MsgContains.
fn ordered(text: &str, keywords: &[&str]) -> bool {
    let mut last = 0;
    for keyword in keywords {
        let Some(at) = text.find(keyword) else {
            return false;
        };
        if at < last {
            return false;
        }
        last = at;
    }
    true
}

// Root KeywordHandler registrations and aliases preceding the barter in each
// pinned Crystal201189f NPC source (amber.lua:before399, billy.lua:before134,
// lee_delle.lua:before139, willie.lua:before258). They win before an offer;
// a pending offer's yes/catch-all child wins before these root registrations.
fn preceding_keyword(npc: &str, text: &str) -> bool {
    let singles: &[&str] = match npc {
        "oteryn:npc.amber" => &[
            "hint",
            "job",
            "explore",
            "adventure",
            "sea",
            "time",
            "help",
            "information",
            "dungeon",
            "sewer",
            "monster",
            "cyclops",
            "dragon",
            "raft",
            "quest",
            "mission",
            "seymour",
            "academy",
            "king",
            "thais",
            "weapon",
            "magic",
            "tibia",
            "castle",
            "mainland",
            "tools",
            "rope",
            "shovel",
            "torch",
            "bank",
            "destiny",
            "trade",
            "premium",
            "loui",
            "zirella",
            "santiago",
            "amber",
            "tom",
            "lee'delle",
            "oracle",
            "norma",
            "lily",
            "billy",
            "willie",
            "paulie",
            "cipfried",
            "hyacinth",
            "obi",
            "dixi",
            "zerbrus",
            "dallheim",
            "orc",
            "prisoner",
            "language",
            "food",
            "salmon",
        ],
        "oteryn:npc.billy" => &[
            "hint",
            "name",
            "job",
            "time",
            "bank",
            "information",
            "help",
            "cook",
            "citizen",
            "rat",
            "shop",
            "tibia",
            "equipment",
            "academy",
            "king",
            "weapon",
            "rookgaard",
            "monster",
            "dungeon",
            "sewer",
            "sell",
            "merchant",
            "god",
            "recipe",
            "armor",
            "shield",
            "backpack",
            "shovel",
            "rope",
            "fishing",
            "money",
            "gold",
            "magic",
            "spell",
            "food",
            "buy",
            "stuff",
            "offer",
            "obi",
            "norma",
            "loui",
            "santiago",
            "zirella",
            "amber",
            "billy",
            "willie",
            "cipfried",
            "dixi",
            "hyacinth",
            "lee'delle",
            "lily",
            "oracle",
            "paulie",
            "seymour",
            "tom",
            "dallheim",
            "zerbrus",
        ],
        "oteryn:npc.lee_delle" => &[
            "name",
            "job",
            "time",
            "bank",
            "cookie",
            "merchant",
            "information",
            "help",
            "tibia",
            "temple",
            "health",
            "equipment",
            "fishing",
            "weapon",
            "food",
            "king",
            "academy",
            "rookgaard",
            "mainland",
            "monster",
            "sell",
            "dungeon",
            "sewer",
            "offer",
            "stuff",
            "wares",
            "buy",
            "quest",
            "mission",
            "armor",
            "shield",
            "backpack",
            "rope",
            "shovel",
            "torch",
            "money",
            "gold",
            "hint",
            "obi",
            "norma",
            "loui",
            "santiago",
            "zirella",
            "amber",
            "billy",
            "willie",
            "cipfried",
            "dixi",
            "hyacinth",
            "lily",
            "oracle",
            "paulie",
            "seymour",
            "tom",
            "dallheim",
            "zerbrus",
            "football",
        ],
        "oteryn:npc.willie" => &[
            "offer",
            "sell",
            "buy",
            "food",
            "information",
            "help",
            "hint",
            "name",
            "job",
            "cook",
            "recipe",
            "citizen",
            "rookgaard",
            "tibia",
            "spell",
            "magic",
            "weapon",
            "king",
            "god",
            "sewer",
            "dungeon",
            "rat",
            "monster",
            "time",
            "amber",
            "billy",
            "cipfried",
            "dallheim",
            "dixi",
            "hyacinth",
            "lee'delle",
            "lily",
            "loui",
            "norma",
            "obi",
            "oracle",
            "paulie",
            "santiago",
            "seymour",
            "tom",
            "willie",
            "zerbrus",
            "zirella",
        ],
        _ => return false,
    };
    singles.iter().any(|keyword| text.contains(keyword))
        || ordered(text, &["how", "are", "you"])
        || ordered(text, &["al", "dee"])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rookgaard_keywords_follow_ordered_source_substrings() -> Result<(), String> {
        let lee = exchange("oteryn:npc.lee_delle").ok_or("Lee'Delle")?;
        assert!(lee.offered_by("honey scented flowers"));
        assert!(!lee.offered_by("flowers with honey"));
        assert!(!lee.offered_by("mission honey flower"));
        let amber = exchange("oteryn:npc.amber").ok_or("Amber")?;
        assert!(amber.offered_by("books"));
        assert!(amber.offered_by("NOTEBOOK"));
        assert!(amber.confirmed_by("yesterday"));
        assert!(!amber.confirmed_by("no"));
        Ok(())
    }
}
