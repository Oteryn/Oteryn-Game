/// Lens mode: the cursor reads help instead of acting, until the next click or toggle.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LensHelp {
    pub active: bool,
}
impl LensHelp {
    /// Switch the mode and return whether it is now on.
    pub fn toggle(&mut self) -> bool {
        self.active = !self.active;
        self.active
    }
    /// A click shows the help once and ends the mode.
    pub fn finish(&mut self) {
        self.active = false;
    }
}
