//! Explicit local operator publication; no web identity, distribution or gameplay authority.
use serde_json::Value;
use std::{io::Read, path::PathBuf, sync::Arc, time::Duration};
use tokio::sync::Semaphore;

pub struct BulletinSource {
    path: PathBuf,
    reads: Arc<Semaphore>,
}
impl BulletinSource {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            reads: Arc::new(Semaphore::new(1)),
        }
    }
    /// At most one blocking file read per source, even if the filesystem stalls.
    pub async fn load(&self) -> Option<String> {
        let permit = self.reads.clone().try_acquire_owned().ok()?;
        let path = self.path.clone();
        tokio::time::timeout(
            Duration::from_millis(200),
            tokio::task::spawn_blocking(move || {
                let _permit = permit;
                let file = std::fs::File::open(path).ok()?;
                let mut document = String::new();
                file.take(oteryn_protocol_oteryn::bulletin::MAX_DOCUMENT_BYTES as u64 + 1)
                    .read_to_string(&mut document)
                    .ok()?;
                valid(&document).then_some(document)
            }),
        )
        .await
        .ok()?
        .ok()?
    }
}
fn keys(value: &Value, expected: &[&str]) -> bool {
    value.as_object().is_some_and(|object| {
        object.len() == expected.len() && expected.iter().all(|key| object.contains_key(*key))
    })
}
fn article(item: &Value) -> bool {
    ["title", "body"].iter().all(|key| {
        item[*key].as_str().is_some_and(|text| {
            !text.trim().is_empty() && text.len() <= 4096 && !text.contains('\0')
        })
    })
}
fn valid(document: &str) -> bool {
    if document.len() > oteryn_protocol_oteryn::bulletin::MAX_DOCUMENT_BYTES {
        return false;
    }
    let Ok(root) = serde_json::from_str::<Value>(document) else {
        return false;
    };
    if !keys(&root, &["news", "events"]) {
        return false;
    }
    let (Some(news), Some(events)) = (root["news"].as_array(), root["events"].as_array()) else {
        return false;
    };
    if news.len() > 64
        || events.len() > 64
        || news
            .iter()
            .any(|item| !keys(item, &["title", "body"]) || !article(item))
    {
        return false;
    }
    events.iter().all(|item| {
        if !keys(item, &["year", "month", "day", "title", "body"]) || !article(item) {
            return false;
        }
        let (Some(year), Some(month), Some(day)) = (
            item["year"].as_u64(),
            item["month"].as_u64(),
            item["day"].as_u64(),
        ) else {
            return false;
        };
        let days = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if year.is_multiple_of(4)
                && (!year.is_multiple_of(100) || year.is_multiple_of(400)) =>
            {
                29
            }
            2 => 28,
            _ => 0,
        };
        (1..=9999).contains(&year) && day > 0 && day <= days
    })
}
#[cfg(test)]
mod tests;
