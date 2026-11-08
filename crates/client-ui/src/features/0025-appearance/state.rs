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

impl Panel {
    fn colour_indices(&self) -> Option<[u16; 4]> {
        self.fields[3]
            .split(',')
            .map(|part| part.trim().parse::<u16>().ok())
            .collect::<Option<Vec<_>>>()?
            .try_into()
            .ok()
    }
    pub fn preview_colours(&self) -> Option<[[u8; 3]; 4]> {
        let catalogue = self.catalogue.as_ref()?;
        let indices = self.colour_indices()?;
        let mut colours = [[0; 3]; 4];
        for (colour, index) in colours.iter_mut().zip(indices) {
            *colour = *catalogue.palette.get(usize::from(index))?;
        }
        Some(colours)
    }
    pub fn cycle_colour(&mut self, part: usize) -> bool {
        if self.pending || !self.selected || part >= 4 {
            return false;
        }
        let Some(catalogue) = &self.catalogue else {
            return false;
        };
        let Some(mut indices) = self.colour_indices() else {
            return false;
        };
        if catalogue.palette.is_empty() || self.preview_colours().is_none() {
            return false;
        }
        let Ok(next) = u16::try_from((usize::from(indices[part]) + 1) % catalogue.palette.len())
        else {
            return false;
        };
        indices[part] = next;
        let [head, body, legs, feet] = indices;
        self.fields[3] = format!("{head},{body},{legs},{feet}");
        true
    }
    pub fn toggle_addon(&mut self, bit: u8) -> bool {
        if self.pending || !self.selected || !bit.is_power_of_two() {
            return false;
        }
        let Some(outfit) = self.catalogue.as_ref().and_then(|catalogue| {
            catalogue
                .outfits
                .iter()
                .find(|outfit| outfit.key == self.fields[0])
        }) else {
            return false;
        };
        let Ok(addons) = self.fields[1].trim().parse::<u8>() else {
            return false;
        };
        if bit & outfit.permitted_addons == 0 || addons & !outfit.permitted_addons != 0 {
            return false;
        }
        self.fields[1] = (addons ^ bit).to_string();
        true
    }
}

/// A draft projected from the currently granted outfit and palette.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    pub look_type: u32,
    pub addons: u8,
    pub colours: [[u8; 3]; 4],
}
impl Panel {
    pub fn preview(&self) -> Option<Preview> {
        if !self.selected {
            return None;
        }
        let outfit = self
            .catalogue
            .as_ref()?
            .outfits
            .iter()
            .find(|o| o.key == self.fields[0])?;
        let addons = self.fields[1].parse::<u8>().ok()?;
        if addons & !outfit.permitted_addons != 0 {
            return None;
        }
        Some(Preview {
            look_type: outfit.look_type,
            addons,
            colours: self.preview_colours()?,
        })
    }
}
