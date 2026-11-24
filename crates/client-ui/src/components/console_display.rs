use oteryn_client_runtime::chat::ChatLine;
use std::collections::BTreeMap;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConsoleDisplay {
    pub font_px: u16,
    pub time: bool,
    pub seconds: bool,
    pub levels: bool,
}
impl Default for ConsoleDisplay {
    fn default() -> Self {
        Self {
            font_px: 14,
            time: false,
            seconds: false,
            levels: false,
        }
    }
}
impl ConsoleDisplay {
    pub fn valid(self) -> bool {
        (10..=32).contains(&self.font_px)
    }
    pub fn timestamp(self, time: u64) -> String {
        if !self.time {
            return String::new();
        }
        let (hours, minutes, seconds) = ((time / 3600) % 24, (time / 60) % 60, time % 60);
        if self.seconds {
            format!("[{hours:02}:{minutes:02}:{seconds:02} UTC] ")
        } else {
            format!("[{hours:02}:{minutes:02} UTC] ")
        }
    }
}
pub fn speaker(line: &ChatLine) -> Option<&str> {
    match line {
        ChatLine::Local {
            mode: oteryn_client_runtime::chat::ChatSpeechMode::MonsterSay,
            ..
        } => None,
        ChatLine::Local { speaker_name, .. }
        | ChatLine::Private { speaker_name, .. }
        | ChatLine::Room { speaker_name, .. }
        | ChatLine::Party { speaker_name, .. } => Some(speaker_name),
        ChatLine::Guild(line) => Some(&line.speaker_name),
        ChatLine::PrivateChannel(line) => Some(&line.speaker_name),
        ChatLine::System { .. } | ChatLine::Dropped => None,
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Metadata {
    pub levels: BTreeMap<String, u32>,
    pub pending: Option<Vec<String>>,
    pub next_names: usize,
    queried: std::collections::BTreeSet<String>,
    speakers_changed: bool,
    retry_at: Option<std::time::Instant>,
    previous: Vec<ChatLine>,
    receipts: Vec<u64>,
}
impl Metadata {
    pub(super) fn begin_query(&mut self, names: Vec<String>) -> Option<Vec<u8>> {
        let payload = oteryn_client_runtime::chat::console::encode_names(&names)?;

        self.pending = Some(names);
        Some(payload)
    }

    pub fn finish_query(&mut self, names: &[String], accepted: bool) {
        if accepted {
            self.queried.extend(names.iter().cloned());
        } else {
            for name in names {
                self.queried.remove(name);
                self.levels.remove(name);
            }
            self.speakers_changed = true;
            self.retry_at = Some(std::time::Instant::now() + std::time::Duration::from_secs(2));
        }
    }
    /// Query new speakers once, including an unavailable level. Opening the console
    /// explicitly can still refresh a previously answered name.
    pub fn query_new_speakers(&mut self, lines: &[ChatLine]) -> Option<Vec<u8>> {
        if self.pending.is_some()
            || !self.speakers_changed
            || self
                .retry_at
                .is_some_and(|time| time > std::time::Instant::now())
        {
            return None;
        }
        let visible: std::collections::BTreeSet<_> = lines.iter().filter_map(speaker).collect();
        self.queried.retain(|name| visible.contains(name.as_str()));
        let names: Vec<_> = visible
            .into_iter()
            .filter(|name| !self.queried.contains(*name))
            .take(32)
            .map(str::to_owned)
            .collect();
        self.speakers_changed = names.len() == 32;
        if names.is_empty() {
            return None;
        }
        self.begin_query(names)
    }
    /// Capture receipt before retained-log coalescing can hide an identical new line.
    pub fn received(&mut self, line: &ChatLine, received_seconds: u64) {
        self.speakers_changed = true;
        let max = oteryn_client_runtime::chat::MAX_CHAT_LOG_LINES;
        if self.previous.len() >= max {
            let remove = self.previous.len() + 1 - max;
            self.previous.drain(..remove);
            self.receipts.drain(..remove);
        }
        self.previous.push(line.clone());
        self.receipts.push(received_seconds);
    }
    pub fn reset_history(&mut self) {
        self.retry_at = None;
        self.queried.clear();
        self.speakers_changed = true;
        self.previous.clear();
        self.receipts.clear();
        self.levels.clear();
    }
    pub fn observe(&mut self, lines: &[ChatLine], received_seconds: u64) {
        self.speakers_changed |= self.previous != lines;
        let overlap = (0..=self.previous.len().min(lines.len()))
            .rev()
            .find(|&n| self.previous[self.previous.len() - n..] == lines[..n])
            .unwrap_or(0);
        let old = self.receipts.len().saturating_sub(overlap);
        self.receipts = self.receipts[old..].to_vec();
        self.receipts.resize(lines.len(), received_seconds);
        self.previous = lines.to_vec();
        self.queried.retain(|name| {
            lines
                .iter()
                .any(|line| speaker(line) == Some(name.as_str()))
        });
        self.levels.retain(|name, _| {
            lines
                .iter()
                .any(|line| speaker(line) == Some(name.as_str()))
        });
    }
    pub(super) fn skip_receipts(&mut self, count: usize) {
        self.receipts.drain(..count.min(self.receipts.len()));
    }
    pub fn prefix(&self, display: ConsoleDisplay, index: usize) -> String {
        if !display.time {
            return String::new();
        }
        let Some(&time) = self.receipts.get(index) else {
            return String::new();
        };
        display.timestamp(time)
    }

    pub fn name(&self, display: ConsoleDisplay, name: &str) -> String {
        if display.levels
            && let Some(level) = self.levels.get(name)
        {
            return format!("{name} [current level {level}]");
        }
        name.to_owned()
    }
}
