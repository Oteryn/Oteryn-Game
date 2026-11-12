//! Server-owned NPC conversation, independent of the trade window.
pub use oteryn_client_runtime::npc_shop::{EntityRef, NpcIntentDisposition, npc_service};
mod input;
pub use input::open_key;
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DialoguePanel {
    pub selected: bool,
    pub open: bool,
    pub target: Option<EntityRef>,
    pub pending: bool,
    pub lines: Vec<String>,
    pub feedback: Option<String>,
}
impl DialoguePanel {
    pub fn request(&mut self, npc: EntityRef, text: &str) -> Option<npc_service::NpcTalkIntent> {
        if !self.selected || self.pending {
            return None;
        }
        let intent = npc_service::NpcTalkIntent {
            npc_actor: npc,
            text: text.trim().into(),
        };
        npc_service::encode_npc_talk_intent(&intent).ok()?;
        if self.target != Some(npc) {
            self.lines.clear();
        }
        self.target = Some(npc);
        self.open = true;
        self.pending = true;
        self.feedback = None;
        Some(intent)
    }
    pub fn apply(&mut self, conversation: npc_service::NpcConversation) {
        self.pending = false;
        if conversation.open.is_some() {
            self.feedback = None;
        }
        self.open = conversation.open.is_some();
        self.target = conversation.open.map(|(npc, _)| npc);
        self.lines = conversation.lines;
    }
    /// A closed conversation can still display its final reply or refusal until dismissed.
    pub fn visible(&self) -> bool {
        self.open || !self.lines.is_empty() || self.feedback.is_some()
    }
    pub fn result(&mut self, result: NpcIntentDisposition) {
        self.pending = false;
        if result != NpcIntentDisposition::Ok {
            self.feedback = Some(lfmt!("Talk refused: {result:?}", result = result));
            if matches!(
                result,
                NpcIntentDisposition::NotInRange
                    | NpcIntentDisposition::NoConversation
                    | NpcIntentDisposition::Stale
            ) {
                self.open = false;
                self.target = None;
            }
        }
    }
    pub fn keywords(&self) -> Vec<&str> {
        let mut words = Vec::new();
        for line in &self.lines {
            for part in line.split('{').skip(1) {
                if let Some((word, _)) = part.split_once('}') {
                    let word = word.trim();
                    if !word.is_empty()
                        && !words.contains(&word)
                        && word.len() <= npc_service::MAX_TALK_TEXT_BYTES
                        && !word.chars().any(char::is_control)
                    {
                        words.push(word);
                    }
                }
            }
        }
        words
    }
    pub fn text(&self) -> String {
        let mut text = self.lines.join("\n");
        if let Some(feedback) = &self.feedback {
            text.push_str(&format!("\n{feedback}"));
        }
        text
    }
}
#[cfg(test)]
#[path = "tests/state.rs"]
mod tests;

/// Registered input adapter for talk command 7.
pub fn request(
    panel: &mut DialoguePanel,
    npc: EntityRef,
    text: &str,
) -> Option<npc_service::NpcTalkIntent> {
    panel.request(npc, text)
}
/// Registered visible reply projection.
pub fn panel_text(panel: &DialoguePanel) -> String {
    panel.text()
}

/// NPC service command 8 shares the live shop selection reducer.
pub fn trade_request(
    panel: &mut crate::npc_shop::LiveShop,
    quote: crate::npc_shop::Quote,
    amount: u32,
) -> Option<npc_service::NpcTradeIntent> {
    panel.trade(quote, amount)
}
