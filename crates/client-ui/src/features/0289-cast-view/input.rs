/// Canary livestream.cpp:441: viewer text goes only to the isolated cast room.
#[derive(Debug, Clone, Default)]
pub struct ChatDraft(pub String);
impl ChatDraft {
    pub fn append(&mut self, text: &str) {
        for ch in text.chars().filter(|ch| !ch.is_control()) {
            if self.0.len() + ch.len_utf8() > 256 {
                break;
            }
            self.0.push(ch);
        }
    }
    pub fn backspace(&mut self) {
        self.0.pop();
    }
    pub fn take(&mut self) -> Option<String> {
        (!self.0.trim().is_empty()).then(|| std::mem::take(&mut self.0))
    }
}
