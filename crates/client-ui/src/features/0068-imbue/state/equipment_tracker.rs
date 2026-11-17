//! Read-only equipment timers from the admitted inspection protocol.
use super::ItemHandle;
use crate::inspection::{Reply, decode_reply};
use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GearScope {
    pub handle: ItemHandle,
    pub definition_ref: u32,
    pub label: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timer {
    pub slot: String,
    pub description: String,
    pub minutes: u32,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GearRecord {
    pub name: String,
    pub timers: Vec<Timer>,
    pub ended: Vec<String>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EquipmentTracker {
    pub available: bool,
    pub pending: Option<ItemHandle>,
    pub scope: Vec<GearScope>,
    pub records: BTreeMap<ItemHandle, GearRecord>,
    pub feedback: Option<&'static str>,
    cursor: usize,
    next_due: Option<Instant>,
}
impl EquipmentTracker {
    pub fn reconcile(&mut self, scope: Vec<GearScope>) {
        self.records.retain(|handle, _| {
            scope.iter().any(|gear| {
                gear.handle == *handle
                    && self.scope.iter().any(|old| {
                        old.handle == *handle && old.definition_ref == gear.definition_ref
                    })
            })
        });
        if self
            .pending
            .is_some_and(|h| !scope.iter().any(|g| g.handle == h))
        {
            self.pending = None;
        }
        if self.scope != scope {
            self.cursor = 0;
            self.scope = scope;
        }
    }
    pub fn refresh_now(&mut self) {
        self.next_due = None;
    }
    pub fn next(&mut self, now: Instant) -> Option<ItemHandle> {
        if !self.available || self.pending.is_some() || self.next_due.is_some_and(|at| now < at) {
            return None;
        }
        let handle = self.scope.get(self.cursor)?.handle;
        self.cursor = (self.cursor + 1) % self.scope.len();
        self.pending = Some(handle);
        self.next_due = Some(now + Duration::from_secs(1));
        Some(handle)
    }
    pub fn accept(&mut self, response: &ServerMechanicResponse) {
        let Some(handle) = self.pending.take() else {
            return;
        };
        let Some(scope) = self.scope.iter().find(|gear| gear.handle == handle) else {
            return;
        };
        let reply = matches!(
            response.status,
            CommandStatus::Accepted | CommandStatus::DuplicateReplay
        )
        .then(|| decode_reply(&response.payload).ok())
        .flatten();
        let Some(Reply {
            status: 1, items, ..
        }) = reply
        else {
            self.feedback = Some("Equipment inspection refused; showing the last confirmed timers");
            return;
        };
        let [item] = items.as_slice() else {
            self.feedback = Some("Invalid equipment inspection");
            return;
        };
        if item.slot != 0 || item.definition_ref != scope.definition_ref {
            self.feedback = Some("Equipment changed during inspection; refresh again");
            return;
        }
        let mut timers = Vec::new();
        for (label, value) in &item.attributes {
            if !label.starts_with("Imbuement Slot ") || value == "empty" {
                continue;
            }
            let Some(timer) = parse_timer(label, value) else {
                self.feedback = Some("Invalid imbuement time in equipment inspection");
                return;
            };
            timers.push(timer);
        }
        let mut ended = self.records.get(&handle).map_or_else(Vec::new, |old| {
            old.ended
                .iter()
                .chain(old.timers.iter().map(|t| &t.slot))
                .filter(|slot| !timers.iter().any(|timer| timer.slot == **slot))
                .cloned()
                .collect::<Vec<_>>()
        });
        ended.sort();
        ended.dedup();
        self.records.insert(
            handle,
            GearRecord {
                name: item.name.clone(),
                timers,
                ended,
            },
        );
        self.feedback = None;
    }
}
fn parse_timer(slot: &str, value: &str) -> Option<Timer> {
    let index = slot.strip_prefix("Imbuement Slot ")?.parse::<u8>().ok()?;
    if !(1..=3).contains(&index) {
        return None;
    }
    let (description, time) = value.rsplit_once(", lasts ")?;
    let (hours, minutes) = time.strip_suffix("h while fighting.")?.split_once(':')?;
    let minutes = minutes.parse::<u32>().ok().filter(|value| *value < 60)?;
    let total = hours
        .parse::<u32>()
        .ok()?
        .checked_mul(60)?
        .checked_add(minutes)?;
    Some(Timer {
        slot: slot.into(),
        description: description.into(),
        minutes: total,
    })
}
pub fn text(tracker: &EquipmentTracker, maximum_ms: u32) -> String {
    let mut out = format!(
        "Equipped imbuements — {} / {} items inspected\r\nTimes are server-reported whole minutes while fighting.\r\n",
        tracker.records.len(),
        tracker.scope.len()
    );
    let mut shown = false;
    for scope in &tracker.scope {
        let Some(record) = tracker.records.get(&scope.handle) else {
            continue;
        };
        for timer in &record.timers {
            if maximum_ms != 0 && u64::from(timer.minutes) * 60_000 > u64::from(maximum_ms) {
                continue;
            }
            shown = true;
            out.push_str(&format!(
                "{} — {} — {}: {} — {}{}\r\n",
                scope.label,
                record.name,
                timer.slot,
                timer.description,
                if timer.minutes == 0 {
                    "less than 1 minute".into()
                } else {
                    format!("{} minutes", timer.minutes)
                },
                if timer.minutes <= 5 {
                    " — LOW TIME"
                } else {
                    ""
                }
            ));
        }
        for slot in &record.ended {
            shown = true;
            out.push_str(&format!(
                "{} — {} — {slot}: ALERT — no longer active\r\n",
                scope.label, record.name
            ));
        }
    }
    if !shown {
        out.push_str("No reported equipped imbuements match this filter\r\n");
    }
    if tracker.pending.is_some() {
        out.push_str("Refreshing equipment from the server\r\n");
    }
    if let Some(message) = tracker.feedback {
        out.push_str(message);
    }
    out
}
