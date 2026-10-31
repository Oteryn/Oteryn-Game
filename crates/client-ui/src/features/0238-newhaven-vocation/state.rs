/// A choice remains pending until the server answers; opening the chooser grants no vocation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VocationPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: bool,
    pub completed: bool,
    pub refused: bool,
}

impl VocationPanel {
    pub fn hint(&mut self, id: u8) {
        if self.selected && !self.completed && id == 2 {
            self.open = true;
        }
    }

    pub fn result(&mut self, chosen: bool) {
        self.pending = false;
        self.refused = !chosen;
        if chosen {
            self.completed = true;
            self.open = false;
        }
    }
}
