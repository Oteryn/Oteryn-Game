//! Memorial artwork and source dialogue shortcuts never grant an outfit locally.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemorialAction {
    Golden,
    Tokens,
    Previous,
    Next,
    Gender,
    Addons,
    Ask,
    Yes,
    No,
    Armour,
    Boots,
    Helmet,
}
#[derive(Debug, Clone, Copy)]
pub struct MemorialEntry {
    pub key: &'static str,
    pub label: &'static str,
    pub npc: &'static str,
    pub male: u32,
    pub female: u32,
    pub phrase: &'static str,
    pub instructions: &'static str,
}
const GOLDEN: &[MemorialEntry] = &[
    MemorialEntry {
        key: "oteryn:outfit.golden",
        label: "Golden armour",
        npc: "King Tibianus",
        male: 1210,
        female: 1211,
        phrase: "outfit",
        instructions: "Outfit -> Yes -> Armour -> read server quote -> Yes. Accepted base price: 100,000 gold.",
    },
    MemorialEntry {
        key: "oteryn:outfit.golden",
        label: "Golden boots (addon 1)",
        npc: "King Tibianus",
        male: 1210,
        female: 1211,
        phrase: "outfit",
        instructions: "Outfit -> Yes -> Boots -> read server quote -> Yes. Accepted price: 50,000 gold; base required.",
    },
    MemorialEntry {
        key: "oteryn:outfit.golden",
        label: "Golden helmet (addon 2)",
        npc: "King Tibianus",
        male: 1210,
        female: 1211,
        phrase: "outfit",
        instructions: "Outfit -> Yes -> Helmet -> read server quote -> Yes. Accepted price: 50,000 gold; boots required.",
    },
];
const TOKENS: &[MemorialEntry] = &[
    MemorialEntry {
        key: "oteryn:outfit.soil_guardian",
        label: "Soil Guardian outfit",
        npc: "Gnomally",
        male: 516,
        female: 514,
        phrase: "basic soil guardian outfit",
        instructions: "20 major crystalline tokens. Read the NPC quote, then Yes. Use the received outfit item from your inventory.",
    },
    MemorialEntry {
        key: "oteryn:outfit.soil_guardian",
        label: "Soil Guardian first addon",
        npc: "Gnomally",
        male: 516,
        female: 514,
        phrase: "addition to the soil guardian outfit",
        instructions: "70 minor crystalline tokens. Read the NPC quote, then Yes. Use the received addon item from your inventory.",
    },
    MemorialEntry {
        key: "oteryn:outfit.crystal_warlord",
        label: "Crystal Warlord outfit",
        npc: "Gnomally",
        male: 512,
        female: 513,
        phrase: "basic crystal warlord outfit",
        instructions: "20 major crystalline tokens. Read the NPC quote, then Yes. Use the received outfit item from your inventory.",
    },
    MemorialEntry {
        key: "oteryn:outfit.crystal_warlord",
        label: "Crystal Warlord first addon",
        npc: "Gnomally",
        male: 512,
        female: 513,
        phrase: "addition to the crystal warlord armor outfit",
        instructions: "70 minor crystalline tokens. Read the NPC quote, then Yes. Use the received addon item from your inventory.",
    },
];
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemorialPanel {
    pub open: bool,
    pub requested: bool,
    pub watching: bool,
    pub tokens: bool,
    pub selected: usize,
    pub female: bool,
    pub addons: u8,
}
impl MemorialPanel {
    fn entries(&self) -> &'static [MemorialEntry] {
        if self.tokens { TOKENS } else { GOLDEN }
    }
    pub fn offer(&self) -> &'static MemorialEntry {
        &self.entries()[self.selected % self.entries().len()]
    }
    pub fn look(&self) -> u32 {
        let offer = self.offer();
        if self.female {
            offer.female
        } else {
            offer.male
        }
    }
    pub fn navigate(&mut self, action: MemorialAction) {
        match action {
            MemorialAction::Golden => {
                self.tokens = false;
                self.selected = 0
            }
            MemorialAction::Tokens => {
                self.tokens = true;
                self.selected = 0
            }
            MemorialAction::Previous => {
                self.selected = (self.selected + self.entries().len() - 1) % self.entries().len()
            }
            MemorialAction::Next => self.selected = (self.selected + 1) % self.entries().len(),
            MemorialAction::Gender => self.female = !self.female,
            MemorialAction::Addons => self.addons = (self.addons + 1) % 4,
            _ => {}
        }
    }
    pub fn phrase(&self, action: MemorialAction) -> Option<&'static str> {
        match action {
            MemorialAction::Ask => Some(self.offer().phrase),
            MemorialAction::Yes => Some("yes"),
            MemorialAction::No => Some("no"),
            MemorialAction::Armour if !self.tokens => Some("armor"),
            MemorialAction::Boots if !self.tokens => Some("boots"),
            MemorialAction::Helmet if !self.tokens => Some("helmet"),
            _ => None,
        }
    }
    pub fn text(&self) -> String {
        let offer = self.offer();
        format!(
            "{}\r\n{} | {} | addons {} (preview only)\r\nTalk to {} in the world first.\r\n{}\r\nYes/No reply to the current NPC question. The server checks costs, prerequisites and inventory capacity.",
            if self.tokens {
                "Token exchange"
            } else {
                "Golden outfits"
            },
            offer.label,
            if self.female { "Female" } else { "Male" },
            self.addons,
            offer.npc,
            offer.instructions
        )
    }
}
