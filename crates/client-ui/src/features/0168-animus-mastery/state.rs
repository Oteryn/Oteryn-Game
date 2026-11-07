use super::*;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AnimusPanel {
    pub selected: bool,
    pub open: bool,
    pub pending: Option<AnimusPage>,
    pub total: u16,
    pub offset: u16,
    pub keys: Vec<String>,
    pub loaded: bool,
    pub error: Option<&'static str>,
}

impl AnimusPanel {
    pub fn has_previous(&self) -> bool {
        self.loaded && self.offset > 0
    }

    pub fn has_next(&self) -> bool {
        self.loaded
            && !self.keys.is_empty()
            && usize::from(self.offset) + self.keys.len() < usize::from(self.total)
    }

    /// A refusal or malformed reply preserves the last confirmed page.
    pub fn finish(&mut self, accepted: bool, payload: &[u8]) -> bool {
        let Some(page) = self.pending.take() else {
            return false;
        };
        let decoded = accepted.then(|| decode_page(payload)).flatten();
        let Some(decoded) = decoded.filter(|decoded| {
            decoded.keys.len() <= usize::from(page.limit)
                && (decoded.keys.is_empty()
                    || usize::from(page.offset) + decoded.keys.len() <= usize::from(decoded.total))
        }) else {
            self.error = Some("Animus request refused; refresh to try again");
            return false;
        };
        self.total = decoded.total;
        self.offset = page.offset;
        self.keys = decoded.keys;
        self.loaded = true;
        self.error = None;
        true
    }
}
