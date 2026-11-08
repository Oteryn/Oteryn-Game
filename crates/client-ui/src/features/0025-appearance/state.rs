#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Intent {
    Apply,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Panel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub pending_kind: u32,
    pub fields: [String; 5],
    pub feedback: String,
    pub summary: String,
    pub catalogue: Option<oteryn_client_runtime::appearance::Catalogue>,
    pub confirmed: Option<oteryn_client_runtime::appearance::Selected>,
}
impl Panel {
    pub fn begin(&mut self, kind: u32) -> bool {
        if !self.selected {
            self.feedback = "Unavailable in this session".into();
            return false;
        }
        if self.pending {
            self.feedback = "Waiting for the server".into();
            return false;
        }
        self.pending = true;
        self.pending_kind = kind;
        self.feedback.clear();
        true
    }
    pub fn failed(&mut self, message: &str) {
        self.pending = false;
        self.feedback = message.into();
    }
}

impl Panel {
    /// Browse only outfits granted by the current server catalogue.
    pub fn cycle_outfit(&mut self, forward: bool) -> bool {
        if self.pending || !self.selected {
            return false;
        }
        let Some(catalogue) = &self.catalogue else {
            return false;
        };
        if catalogue.outfits.is_empty() {
            return false;
        }
        let current = catalogue
            .outfits
            .iter()
            .position(|outfit| outfit.key == self.fields[0]);
        let index = current.map_or(0, |index| {
            if forward {
                (index + 1) % catalogue.outfits.len()
            } else {
                (index + catalogue.outfits.len() - 1) % catalogue.outfits.len()
            }
        });
        let outfit = &catalogue.outfits[index];
        self.fields[0].clone_from(&outfit.key);
        let addons = self.fields[1].parse::<u8>().unwrap_or(0) & outfit.permitted_addons;
        self.fields[1] = addons.to_string();
        true
    }
}
