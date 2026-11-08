//! Read-only operator bulletin, suitable for packaging alongside a client release.
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Article {
    pub title: String,
    pub body: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub article: Article,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Bulletin {
    pub news: Vec<Article>,
    pub events: Vec<Event>,
}
fn text(value: &Value, key: &str) -> Result<String, String> {
    let s = value[key]
        .as_str()
        .ok_or_else(|| format!("Missing {key}"))?;
    if s.trim().is_empty() || s.len() > 4096 || s.contains('\0') {
        return Err(format!("Invalid {key}"));
    }
    Ok(s.to_owned())
}
fn number(value: &Value, key: &str) -> Result<u16, String> {
    value[key]
        .as_u64()
        .and_then(|n| u16::try_from(n).ok())
        .ok_or_else(|| format!("Invalid {key}"))
}
pub fn days_in_month(year: u16, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        _ => 0,
    }
}
impl Bulletin {
    pub fn parse(json: &str) -> Result<Self, String> {
        if json.len() > 256 * 1024 {
            return Err("Bulletin exceeds 256 KiB".into());
        }
        let root: Value = serde_json::from_str(json).map_err(|_| "Invalid bulletin JSON")?;
        let list = |key: &str| -> Result<&Vec<Value>, String> {
            root[key]
                .as_array()
                .filter(|items| items.len() <= 64)
                .ok_or_else(|| format!("Invalid {key} list"))
        };
        let mut feed = Self::default();
        for item in list("news")? {
            feed.news.push(Article {
                title: text(item, "title")?,
                body: text(item, "body")?,
            });
        }
        for item in list("events")? {
            let year = number(item, "year")?;
            let month = u8::try_from(number(item, "month")?).map_err(|_| "Invalid month")?;
            let day = u8::try_from(number(item, "day")?).map_err(|_| "Invalid day")?;
            if year == 0 || year > 9999 || day == 0 || day > days_in_month(year, month) {
                return Err("Invalid event date".into());
            }
            feed.events.push(Event {
                year,
                month,
                day,
                article: Article {
                    title: text(item, "title")?,
                    body: text(item, "body")?,
                },
            });
        }
        feed.events.sort_by_key(|e| (e.year, e.month, e.day));
        Ok(feed)
    }
    pub fn news_text(&self) -> String {
        if self.news.is_empty() {
            return "No news published.".into();
        }
        self.news
            .iter()
            .map(|a| format!("{}\r\n{}", a.title, a.body))
            .collect::<Vec<_>>()
            .join("\r\n\r\n")
    }
    pub fn month_text(&self, year: u16, month: u8) -> String {
        let mut result = format!("Events {year:04}-{month:02}\r\n");
        let mut found = false;
        for event in &self.events {
            if event.year == year && event.month == month {
                found = true;
                result.push_str(&format!(
                    "{}: {} — {}\r\n",
                    event.day, event.article.title, event.article.body
                ));
            }
        }
        if !found {
            result.push_str("No events published for this month.");
        }
        result
    }
}
