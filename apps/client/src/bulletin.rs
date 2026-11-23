//! Local release/operator news and month browsing; no external service is contacted.
use oteryn_client_ui::bulletin::{Article, Bulletin, days_in_month};

pub struct Browser {
    feed: Bulletin,
    year: u16,
    month: u8,
}
impl Browser {
    pub fn new(year: u16, month: u8, json: &str) -> Result<Self, String> {
        if !(1..=9999).contains(&year) || !(1..=12).contains(&month) {
            return Err(tr!("Invalid calendar month").into());
        }
        Ok(Self {
            feed: Bulletin::parse(json)?,
            year,
            month,
        })
    }
    /// The read is bounded even if a local bulletin file is unexpectedly large.
    pub fn load(directory: &std::path::Path) -> Result<Self, String> {
        use std::io::Read;
        let mut browser = Self::default();
        match std::fs::File::open(directory.join("bulletin.json")) {
            Ok(file) => {
                let mut json = String::new();
                file.take(256 * 1024 + 1)
                    .read_to_string(&mut json)
                    .map_err(|_| "Cannot read bulletin.json")?;
                browser.feed = Bulletin::parse(&json)?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err("Cannot open bulletin.json".into()),
        }
        Ok(browser)
    }
    /// Replace only a complete validated publication; malformed updates retain the last feed.
    pub fn replace(&mut self, document: &str) -> Result<(), String> {
        let feed = Bulletin::parse(document)?;
        self.feed = feed;
        Ok(())
    }
    pub fn news(&self) -> String {
        self.feed.news_text()
    }
    pub fn admission_news(&self, now: std::time::SystemTime) -> String {
        let mut body = self.news();
        for (offset, heading) in [(0, "Today"), (1, "Tomorrow")] {
            let date = utc_date(now, offset);
            body.push_str(&format!(
                "\n\n{heading} ({:04}-{:02}-{:02} UTC)",
                date.0, date.1, date.2
            ));
            let mut events = self
                .feed
                .events
                .iter()
                .filter(|event| (event.year, event.month, event.day) == date)
                .peekable();
            if events.peek().is_none() {
                body.push_str("\nNo events published.");
            }
            for event in events {
                body.push_str(&format!(
                    "\n{} — {}",
                    event.article.title, event.article.body
                ));
            }
        }
        body
    }
    pub fn month(&self) -> String {
        self.feed.month_text(self.year, self.month)
    }
    pub fn next_month(&mut self) {
        if self.month < 12 {
            self.month += 1;
        } else if self.year < 9999 {
            self.year += 1;
            self.month = 1;
        }
    }
    pub fn previous_month(&mut self) {
        if self.month > 1 {
            self.month -= 1;
        } else if self.year > 1 {
            self.year -= 1;
            self.month = 12;
        }
    }
}
fn utc_date(now: std::time::SystemTime, offset: u64) -> (u16, u8, u8) {
    let mut days = now
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        / 86400
        + offset;
    let mut year = 1970u16;
    let mut month = 1u8;
    // Bounded for system clocks outside the supported calendar range.
    while year < 9999 || month < 12 {
        let length = u64::from(days_in_month(year, month));
        if days < length {
            break;
        }
        days -= length;
        if month == 12 {
            year += 1;
            month = 1;
        } else {
            month += 1;
        }
    }
    (
        year,
        month,
        (days.min(u64::from(days_in_month(year, month) - 1)) + 1) as u8,
    )
}

impl Default for Browser {
    fn default() -> Self {
        let (year, month, _) = utc_date(std::time::SystemTime::now(), 0);
        Self { year, month, feed: Bulletin {
            news: vec![Article { title: lfmt!("Oteryn {}", crate::RELEASE_ID), body: tr!("Sign in through your browser. Use the calendar to browse operator-published events. This calendar has no published events.").into() }],
            events: Vec::new(),
        } }
    }
}

/// Ctrl+Alt+Shift+L opens the read-only login news/calendar while playing.
pub fn open_key(events: &[oteryn_input_actions::NormalizedInputEvent]) -> bool {
    use oteryn_input_actions::{ButtonState, Modifier, Modifiers, NormalizedInputEvent};
    let modifiers = Modifiers::one(Modifier::Control)
        .with(Modifier::Alt)
        .with(Modifier::Shift);
    events.iter().any(|event| matches!(event, NormalizedInputEvent::Key { code, state: ButtonState::Pressed, modifiers: held, repeat: false } if code.get() == 15 && *held == modifiers))
}
