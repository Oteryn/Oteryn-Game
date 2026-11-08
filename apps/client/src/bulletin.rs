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
            return Err("Invalid calendar month".into());
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
    pub fn news(&self) -> String {
        self.feed.news_text()
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
impl Default for Browser {
    fn default() -> Self {
        let mut days = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            / 86400;
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
        Self { year, month, feed: Bulletin {
            news: vec![Article { title: format!("Oteryn {}", crate::RELEASE_ID), body: "Sign in through your browser. Use the calendar to browse operator-published events. This calendar has no published events.".into() }],
            events: Vec::new(),
        } }
    }
}
